-- Enable WAL mode
PRAGMA journal_mode=WAL;

-- Create tables
CREATE TABLE media (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    release_year INTEGER,
    youtube_id TEXT NOT NULL,
    spotify_uri TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE questions (
   id INTEGER PRIMARY KEY,
   type TEXT NOT NULL CHECK (type IN ('color', 'character', 'text', 'year')),
   text TEXT NOT NULL,
   media_id INTEGER REFERENCES media(id) NOT NULL,
   image_url TEXT,
   created_at TEXT DEFAULT (datetime('now')),
   is_active INTEGER DEFAULT 1
);

CREATE TABLE question_options (
   id INTEGER PRIMARY KEY,
   question_id INTEGER REFERENCES questions(id) NOT NULL,
   text TEXT NOT NULL,
   is_correct INTEGER NOT NULL, -- boolean
   created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE question_sets (
   id INTEGER PRIMARY KEY,
   name TEXT NOT NULL,
   created_at TEXT DEFAULT (datetime('now')),
   is_active INTEGER DEFAULT 1
);

CREATE TABLE question_set_items (
   question_set_id INTEGER NOT NULL,
   question_id INTEGER NOT NULL,
   position INTEGER NOT NULL,
   PRIMARY KEY (question_set_id, question_id),
   FOREIGN KEY (question_set_id) REFERENCES question_sets(id),
   FOREIGN KEY (question_id) REFERENCES questions(id)
);

-- Indexes
CREATE INDEX idx_questions_media ON questions(media_id);
CREATE INDEX idx_questions_active ON questions(is_active);
CREATE INDEX idx_question_options_question ON question_options(question_id);
CREATE INDEX idx_question_set_items_ordered ON question_set_items(question_set_id, position);
CREATE INDEX idx_question_sets_active ON question_sets(is_active);
