-- Enable WAL mode
PRAGMA journal_mode=WAL;

-- Create tables
CREATE TABLE Media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    artist TEXT, 
    spotify_uri TEXT,
    youtube_id TEXT NOT NULL,
    release_year INTEGER,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE QuestionTypes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE Questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER REFERENCES Media(id),
    question_type_id INTEGER REFERENCES QuestionTypes(id) NOT NULL,
    question_text TEXT,
    image_url TEXT,
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE QuestionOptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER REFERENCES Questions(id) NOT NULL,
    option_text TEXT NOT NULL,
    is_correct INTEGER NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE Characters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    image_url TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE QuestionCharacterOptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_option_id INTEGER REFERENCES QuestionOptions(id) NOT NULL,
    character_id INTEGER REFERENCES Characters(id) NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE QuestionSets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE TABLE QuestionSetItems (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    question_set_id INTEGER REFERENCES QuestionSets(id) NOT NULL,
    question_id INTEGER REFERENCES Questions(id) NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    deleted_at TEXT
);

-- Create indexes
CREATE INDEX idx_questions_media ON Questions(media_id);
CREATE INDEX idx_questions_type ON Questions(question_type_id);
CREATE INDEX idx_questions_active ON Questions(is_active);
CREATE INDEX idx_question_options_question ON QuestionOptions(question_id);
CREATE INDEX idx_question_character_options_character ON QuestionCharacterOptions(character_id);
CREATE INDEX idx_question_character_options_option ON QuestionCharacterOptions(question_option_id);
CREATE INDEX idx_question_set_items_set ON QuestionSetItems(question_set_id);
CREATE INDEX idx_question_sets_active ON QuestionSets(is_active);

-- Insert initial question types
INSERT INTO QuestionTypes (name, description) VALUES
    ('color', 'Questions about colors in songs'),
    ('character', 'Questions about identifying characters from media'),
    ('text', 'Standard text-based questions with text alternatives'),
    ('year', 'Guess the release year of the song');
