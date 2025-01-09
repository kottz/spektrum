import pandas as pd
import sqlite3
from typing import List, Dict
import argparse


class GameDatabaseConverter:
    def __init__(self, db_path: str):
        self.conn = sqlite3.connect(db_path)
        self.cursor = self.conn.cursor()

    def close(self):
        self.conn.close()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def import_color_questions(self, csv_path: str):
        df = pd.read_csv(csv_path)

        for _, row in df.iterrows():
            # Insert media
            self.cursor.execute(
                """
                INSERT INTO Media (title, artist, spotify_uri, youtube_id)
                VALUES (?, ?, ?, ?)
            """,
                (row["title"], row["artist"], row["spotify_uri"], row["youtube_id"]),
            )
            media_id = self.cursor.lastrowid

            # Get color question type id
            self.cursor.execute("SELECT id FROM QuestionTypes WHERE name = 'color'")
            color_type_id = self.cursor.fetchone()[0]

            # Insert question
            self.cursor.execute(
                """
                INSERT INTO Questions (media_id, question_type_id, is_active)
                VALUES (?, ?, 1)
            """,
                (media_id, color_type_id),
            )
            question_id = self.cursor.lastrowid

            # Insert correct color option
            self.cursor.execute(
                """
                INSERT INTO QuestionOptions (question_id, option_text, is_correct)
                VALUES (?, ?, 1)
            """,
                (question_id, row["color"]),
            )

        self.conn.commit()

    def import_character_questions(self, csv_path: str):
        df = pd.read_csv(csv_path)

        for _, row in df.iterrows():
            # Insert media
            self.cursor.execute(
                """
                INSERT INTO Media (title, spotify_uri, youtube_id)
                VALUES (?, ?, ?)
            """,
                (row["song"], row["spotify_uri"], row["youtube_id"]),
            )
            media_id = self.cursor.lastrowid

            # Get character question type id
            self.cursor.execute("SELECT id FROM QuestionTypes WHERE name = 'character'")
            char_type_id = self.cursor.fetchone()[0]

            # Insert question
            self.cursor.execute(
                """
                INSERT INTO Questions (media_id, question_type_id, is_active)
                VALUES (?, ?, 1)
            """,
                (media_id, char_type_id),
            )
            question_id = self.cursor.lastrowid

            # Insert correct character
            self.cursor.execute(
                """
                INSERT INTO QuestionOptions (question_id, option_text, is_correct)
                VALUES (?, ?, 1)
            """,
                (question_id, row["correct_character"]),
            )

            # Insert other characters
            if pd.notna(row["other_characters"]):
                for char in row["other_characters"].split(";"):
                    if char:
                        self.cursor.execute(
                            """
                            INSERT INTO QuestionOptions (question_id, option_text, is_correct)
                            VALUES (?, ?, 0)
                        """,
                            (question_id, char),
                        )

        self.conn.commit()

    def export_color_questions(self, output_path: str):
       query = """
       SELECT 
           m.title,
           m.artist, 
           qo.option_text as color,
           m.spotify_uri,
           m.youtube_id
       FROM Media m
       JOIN Questions q ON m.id = q.media_id
       JOIN QuestionTypes qt ON q.question_type_id = qt.id
       JOIN QuestionOptions qo ON q.id = qo.question_id
       WHERE qt.name = 'color'
       AND qo.is_correct = 1
       AND m.deleted_at IS NULL
       AND q.deleted_at IS NULL
       AND qo.deleted_at IS NULL
       """
       
       df = pd.read_sql_query(query, self.conn)
       df = df.reset_index(drop=True)
       df.index += 1

       # Reorder columns to match exact format
       columns = ['title', 'artist', 'color', 'spotify_uri', 'youtube_id']
       df = df[columns]
       
       df.to_csv(output_path, index=True, index_label='id')


    def export_character_questions(self, output_path: str):
        query = """
        SELECT 
            q.id as question_id,
            m.title as song,
            qo_correct.option_text as correct_character,
            m.spotify_uri,
            m.youtube_id
        FROM Media m
        JOIN Questions q ON m.id = q.media_id
        JOIN QuestionTypes qt ON q.question_type_id = qt.id
        JOIN QuestionOptions qo_correct ON q.id = qo_correct.question_id
        WHERE qt.name = 'character'
        AND qo_correct.is_correct = 1
        AND m.deleted_at IS NULL
        AND q.deleted_at IS NULL
        AND qo_correct.deleted_at IS NULL
        """
        
        df = pd.read_sql_query(query, self.conn)
        
        # Get wrong options for each question
        for idx, row in df.iterrows():
            wrong_options_query = """
            SELECT option_text
            FROM QuestionOptions
            WHERE question_id = ?
            AND is_correct = 0
            AND deleted_at IS NULL
            ORDER BY id
            """
            wrong_options = pd.read_sql_query(
                wrong_options_query, 
                self.conn, 
                params=(row['question_id'],)
            )
            df.at[idx, 'other_characters'] = ';'.join(wrong_options['option_text'])
        
        # Reset index and add difficulty column
        df = df.reset_index(drop=True)
        df.index += 1
        
        # Reorder columns and drop question_id
        columns = ['song', 'correct_character', 'other_characters', 'spotify_uri', 'youtube_id']
        df = df.drop('question_id', axis=1)[columns]
        
        df.to_csv(output_path, index=True, index_label='id')


def main():
    parser = argparse.ArgumentParser(description="Game Database CSV Converter")
    parser.add_argument("--db", required=True, help="Path to SQLite database")
    parser.add_argument(
        "--mode", choices=["import", "export"], required=True, help="Mode of operation"
    )
    parser.add_argument(
        "--type", choices=["color", "character"], required=True, help="Question type"
    )
    parser.add_argument(
        "--file",
        required=True,
        help="CSV file path (input for import, output for export)",
    )

    args = parser.parse_args()

    with GameDatabaseConverter(args.db) as converter:
        if args.mode == "import":
            if args.type == "color":
                converter.import_color_questions(args.file)
            else:
                converter.import_character_questions(args.file)
        else:  # export
            if args.type == "color":
                converter.export_color_questions(args.file)
            else:
                converter.export_character_questions(args.file)


if __name__ == "__main__":
    main()
