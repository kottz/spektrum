import pandas as pd
import json
import sqlite3
from typing import List, Dict, Optional
import argparse
from dataclasses import dataclass, asdict, field
from enum import Enum
from pathlib import Path


class QuestionType(str, Enum):
    COLOR = "color"
    CHARACTER = "character"
    TEXT = "text"
    YEAR = "year"


@dataclass
class Media:
    id: int
    title: str
    artist: str
    spotify_uri: str | None
    youtube_id: str
    release_year: int | None = None


@dataclass
class Question:
    id: int
    media_id: int
    question_type: QuestionType
    question_text: Optional[str]
    image_url: Optional[str]
    is_active: bool


@dataclass
class QuestionOption:
    id: int
    question_id: int
    option_text: str
    is_correct: bool


@dataclass
class QuestionSet:
    id: int
    name: str
    question_ids: List[int]

    @classmethod
    def create_default(cls, questions):
        return cls(id=1, name="All Questions", question_ids=[q.id for q in questions])


@dataclass
class StoredData:
    media: List[Media] = field(default_factory=list)
    questions: List[Question] = field(default_factory=list)
    options: List[QuestionOption] = field(default_factory=list)
    sets: List[QuestionSet] = field(default_factory=list)


class Converter:
    def __init__(self):
        self.data = StoredData()
        self.next_media_id = 1
        self.next_question_id = 1
        self.next_option_id = 1

    def reset_ids(self):
        if self.data.media:
            self.next_media_id = max(m.id for m in self.data.media) + 1
        if self.data.questions:
            self.next_question_id = max(q.id for q in self.data.questions) + 1
        if self.data.options:
            self.next_option_id = max(o.id for o in self.data.options) + 1

    def load_json(self, filename: str):
        with open(filename, "r", encoding="utf-8") as f:
            data = json.load(f)
            self.data = StoredData(
                media=[Media(**m) for m in data["media"]],
                questions=[Question(**q) for q in data["questions"]],
                options=[QuestionOption(**o) for o in data["options"]],
                sets=[QuestionSet(**s) for s in data["sets"]],
            )
        self.reset_ids()

    def save_json(self, filename: str):
        with open(filename, "w", encoding="utf-8") as f:
            json.dump(asdict(self.data), f, indent=2, ensure_ascii=False)

    def load_csv(self, color_csv: str, character_csv: str):
        self.data = StoredData()

        # Load color questions
        if Path(color_csv).exists():
            df_color = pd.read_csv(color_csv)
            for _, row in df_color.iterrows():
                media = Media(
                    id=self.next_media_id,
                    title=row["title"],
                    artist=row["artist"],
                    spotify_uri=row["spotify_uri"],
                    youtube_id=row["youtube_id"],
                    release_year=None,
                )
                self.next_media_id += 1

                question = Question(
                    id=self.next_question_id,
                    media_id=media.id,
                    question_type=QuestionType.COLOR,
                    question_text=None,
                    image_url=None,
                    is_active=True,
                )
                self.next_question_id += 1

                option = QuestionOption(
                    id=self.next_option_id,
                    question_id=question.id,
                    option_text=row["color"],
                    is_correct=True,
                )
                self.next_option_id += 1

                self.data.media.append(media)
                self.data.questions.append(question)
                self.data.options.append(option)

        # Load character questions
        if Path(character_csv).exists():
            df_char = pd.read_csv(character_csv)
            for _, row in df_char.iterrows():
                media = Media(
                    id=self.next_media_id,
                    title=row["song"],
                    artist="",
                    spotify_uri=row["spotify_uri"],
                    youtube_id=row["youtube_id"],
                    release_year=None,
                )
                self.next_media_id += 1

                question = Question(
                    id=self.next_question_id,
                    media_id=media.id,
                    question_type=QuestionType.CHARACTER,
                    question_text=None,
                    image_url=None,
                    is_active=True,
                )
                self.next_question_id += 1

                option = QuestionOption(
                    id=self.next_option_id,
                    question_id=question.id,
                    option_text=row["correct_character"],
                    is_correct=True,
                )
                self.next_option_id += 1

                self.data.media.append(media)
                self.data.questions.append(question)
                self.data.options.append(option)

                if pd.notna(row["other_characters"]):
                    for char in row["other_characters"].split(";"):
                        if char:
                            option = QuestionOption(
                                id=self.next_option_id,
                                question_id=question.id,
                                option_text=char.strip(),
                                is_correct=False,
                            )
                            self.next_option_id += 1
                            self.data.options.append(option)

        # Create default question set
        if not self.data.sets:
            self.data.sets.append(QuestionSet.create_default(self.data.questions))

    def save_csv(self, color_csv: str, character_csv: str):
        # Export color questions
        color_rows = []
        for question in self.data.questions:
            if question.question_type == QuestionType.COLOR and question.is_active:
                media = next(m for m in self.data.media if m.id == question.media_id)
                correct_option = next(
                    o
                    for o in self.data.options
                    if o.question_id == question.id and o.is_correct
                )

                color_rows.append(
                    {
                        "title": media.title,
                        "artist": media.artist,
                        "color": correct_option.option_text,
                        "spotify_uri": media.spotify_uri,
                        "youtube_id": media.youtube_id,
                    }
                )

        df_color = pd.DataFrame(color_rows)
        df_color.index += 1
        df_color.to_csv(color_csv, index=True, index_label="id")

        # Export character questions
        char_rows = []
        for question in self.data.questions:
            if question.question_type == QuestionType.CHARACTER and question.is_active:
                media = next(m for m in self.data.media if m.id == question.media_id)

                correct_option = next(
                    o
                    for o in self.data.options
                    if o.question_id == question.id and o.is_correct
                )
                incorrect_options = [
                    o.option_text
                    for o in self.data.options
                    if o.question_id == question.id and not o.is_correct
                ]

                char_rows.append(
                    {
                        "song": media.title,
                        "correct_character": correct_option.option_text,
                        "other_characters": ";".join(incorrect_options),
                        "spotify_uri": media.spotify_uri,
                        "youtube_id": media.youtube_id,
                    }
                )

        df_char = pd.DataFrame(char_rows)
        df_char.index += 1
        df_char.to_csv(character_csv, index=True, index_label="id")

    def load_sqlite(self, filename: str):
        conn = sqlite3.connect(filename)
        self.data = StoredData()

        # Load media
        media_df = pd.read_sql_query(
            "SELECT id, title, artist, release_year, spotify_uri, youtube_id FROM media",
            conn,
        )
        self.data.media = [Media(**row) for row in media_df.to_dict("records")]

        # Load questions
        questions_df = pd.read_sql_query(
            "SELECT id, media_id, type as question_type, text as question_text, "
            "image_url, is_active FROM questions",
            conn,
        )
        self.data.questions = [
            Question(
                id=row["id"],
                media_id=row["media_id"],
                question_type=QuestionType(row["question_type"]),
                question_text=row["question_text"],
                image_url=row["image_url"],
                is_active=bool(row["is_active"]),
            )
            for row in questions_df.to_dict("records")
        ]

        # Load options
        options_df = pd.read_sql_query(
            "SELECT id, question_id, text as option_text, is_correct FROM question_options",
            conn,
        )
        self.data.options = [
            QuestionOption(
                id=row["id"],
                question_id=row["question_id"],
                option_text=row["option_text"],
                is_correct=bool(row["is_correct"]),
            )
            for row in options_df.to_dict("records")
        ]

        # Load question sets
        sets_df = pd.read_sql_query(
            "SELECT id, name FROM question_sets WHERE is_active = 1", conn
        )

        for set_row in sets_df.to_dict("records"):
            # Get question IDs for this set
            items_df = pd.read_sql_query(
                "SELECT question_id FROM question_set_items "
                "WHERE question_set_id = ? ORDER BY position",
                conn,
                params=(set_row["id"],),
            )
            question_ids = items_df["question_id"].tolist()

            self.data.sets.append(
                QuestionSet(
                    id=set_row["id"], name=set_row["name"], question_ids=question_ids
                )
            )

        conn.close()
        self.reset_ids()

    def save_sqlite(self, filename: str):
        conn = sqlite3.connect(filename)

        # Create tables
        conn.execute("""
        CREATE TABLE media (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            release_year INTEGER,
            youtube_id TEXT NOT NULL,
            spotify_uri TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        )
        """)

        conn.execute("""
        CREATE TABLE questions (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL CHECK (type IN ('color', 'character', 'text', 'year')),
            text TEXT,
            media_id INTEGER REFERENCES media(id) NOT NULL,
            image_url TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            is_active INTEGER DEFAULT 1
        )
        """)

        conn.execute("""
        CREATE TABLE question_options (
            id INTEGER PRIMARY KEY,
            question_id INTEGER REFERENCES questions(id) NOT NULL,
            text TEXT NOT NULL,
            is_correct INTEGER NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        )
        """)

        # Add the missing tables for sets
        conn.execute("""
        CREATE TABLE question_sets (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            is_active INTEGER DEFAULT 1
        )
        """)

        conn.execute("""
        CREATE TABLE question_set_items (
            question_set_id INTEGER NOT NULL,
            question_id INTEGER NOT NULL,
            position INTEGER NOT NULL,
            PRIMARY KEY (question_set_id, question_id),
            FOREIGN KEY (question_set_id) REFERENCES question_sets(id),
            FOREIGN KEY (question_id) REFERENCES questions(id)
        )
        """)

        # Insert data (existing code)
        for media in self.data.media:
            conn.execute(
                "INSERT INTO media (id, title, artist, release_year, spotify_uri, youtube_id) "
                "VALUES (?, ?, ?, ?, ?, ?)",
                (
                    media.id,
                    media.title,
                    media.artist,
                    media.release_year,
                    media.spotify_uri,
                    media.youtube_id,
                ),
            )

        for question in self.data.questions:
            q_type = (
                question.question_type.value
                if hasattr(question.question_type, "value")
                else question.question_type
            )
            conn.execute(
                "INSERT INTO questions (id, media_id, type, text, image_url, is_active) "
                "VALUES (?, ?, ?, ?, ?, ?)",
                (
                    question.id,
                    question.media_id,
                    q_type,
                    question.question_text,
                    question.image_url,
                    question.is_active,
                ),
            )

        for option in self.data.options:
            conn.execute(
                "INSERT INTO question_options (id, question_id, text, is_correct) "
                "VALUES (?, ?, ?, ?)",
                (option.id, option.question_id, option.option_text, option.is_correct),
            )

        # Add the set data
        for question_set in self.data.sets:
            conn.execute(
                "INSERT INTO question_sets (id, name) VALUES (?, ?)",
                (question_set.id, question_set.name),
            )

            # Insert question set items with position
            for position, question_id in enumerate(question_set.question_ids):
                conn.execute(
                    "INSERT INTO question_set_items (question_set_id, question_id, position) "
                    "VALUES (?, ?, ?)",
                    (question_set.id, question_id, position),
                )

        conn.commit()
        conn.close()


def get_format(filename: str) -> str:
    ext = filename.lower().split(".")[-1]
    if ext == "json":
        return "json"
    elif ext == "db":
        return "sqlite"
    elif ext == "csv":
        return "csv"
    else:
        raise ValueError(f"Unsupported file format: {ext}")


def get_csv_filenames(base_filename: str) -> tuple[str, str]:
    base = base_filename.rsplit(".", 1)[0]
    return f"{base}_color.csv", f"{base}_character.csv"


def check_file_exists(filepath: str) -> bool:
    return Path(filepath).exists()


def main():
    parser = argparse.ArgumentParser(description="Game Data Converter")
    parser.add_argument("input", help="Input file (*.json, *.db, or *.csv)")
    parser.add_argument("output", help="Output file (*.json, *.db, or *.csv)")

    args = parser.parse_args()

    input_format = get_format(args.input)
    output_format = get_format(args.output)

    # Check if output files exist
    if output_format == "csv":
        color_out, char_out = get_csv_filenames(args.output)
        if check_file_exists(color_out) or check_file_exists(char_out):
            print(
                f"Error: Output CSV files already exist: {color_out} and/or {char_out}"
            )
            print("Please remove them first or specify a different output name.")
            return
    else:
        if check_file_exists(args.output):
            print(f"Error: Output file already exists: {args.output}")
            print("Please remove it first or specify a different output name.")
            return

    converter = Converter()

    # Load input
    try:
        if input_format == "json":
            converter.load_json(args.input)
        elif input_format == "sqlite":
            converter.load_sqlite(args.input)
        elif input_format == "csv":
            color_in, char_in = get_csv_filenames(args.input)
            if not (check_file_exists(color_in) or check_file_exists(char_in)):
                print(f"Error: Input CSV files not found: {color_in} and/or {char_in}")
                return
            converter.load_csv(color_in, char_in)

        # Save output
        if output_format == "json":
            converter.save_json(args.output)
        elif output_format == "sqlite":
            converter.save_sqlite(args.output)
        elif output_format == "csv":
            converter.save_csv(color_out, char_out)

        print("Conversion completed successfully.")

    except Exception as e:
        print(f"Error during conversion: {str(e)}")
        return


if __name__ == "__main__":
    main()
