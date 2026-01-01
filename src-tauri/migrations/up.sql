-- migrations/20251221_0001_initial/up.sql

CREATE TABLE IF NOT EXISTS papers (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    authors TEXT,
    file_path TEXT NOT NULL,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS excerpts (
    id TEXT PRIMARY KEY,
    paper_id TEXT NOT NULL,
    content TEXT NOT NULL,
    page_number INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
);

-- For spaced repetition cards
CREATE TABLE IF NOT EXISTS cards (
    id TEXT PRIMARY KEY,
    excerpt_id TEXT NOT NULL,
    front TEXT NOT NULL,      -- Question / cloze
    back TEXT NOT NULL,       -- Answer / full text
    due_date DATETIME NOT NULL,
    interval INTEGER DEFAULT 1,
    ease_factor REAL DEFAULT 2.5,
    reps INTEGER DEFAULT 0,
    lapses INTEGER DEFAULT 0,
    FOREIGN KEY (excerpt_id) REFERENCES excerpts(id) ON DELETE CASCADE
);
