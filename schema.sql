-- Database schema, embedded in the binary (src/db.rs) and run every time the database is opened.
-- Dates are ISO text (YYYY-MM-DD); status holds the status name (e.g. 'In selection').

CREATE TABLE IF NOT EXISTS applications (
    id               INTEGER PRIMARY KEY,
    company          TEXT NOT NULL,
    role             TEXT NOT NULL DEFAULT '',
    sector           TEXT NOT NULL DEFAULT '',
    role_type        TEXT NOT NULL DEFAULT '',
    location         TEXT NOT NULL DEFAULT '',
    channel          TEXT NOT NULL DEFAULT '',
    contact          TEXT NOT NULL DEFAULT '',
    sent_date        TEXT,
    status           TEXT NOT NULL,
    next_action      TEXT NOT NULL DEFAULT '',
    next_action_date TEXT,
    salary           TEXT NOT NULL DEFAULT '',
    cv               TEXT NOT NULL DEFAULT '',
    link             TEXT NOT NULL DEFAULT '',
    notes            TEXT NOT NULL DEFAULT ''
);
