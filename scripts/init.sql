-- Database initialization script for Judge Ma Di
-- Automatically executed on first Postgres boot via /docker-entrypoint-initdb.d/

CREATE TABLE IF NOT EXISTS task (
    id         TEXT PRIMARY KEY,
    title      TEXT NOT NULL DEFAULT '',
    full_score INTEGER NOT NULL DEFAULT 100,
    private    BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS submission (
    id           SERIAL PRIMARY KEY,
    task_id      TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'In Queue',
    submitted_at TIMESTAMPTZ DEFAULT NOW(),
    time         INTEGER NOT NULL DEFAULT 0,
    memory       INTEGER NOT NULL DEFAULT 0,
    code         BYTEA NOT NULL,
    score        INTEGER NOT NULL DEFAULT 0,
    result       JSONB NOT NULL DEFAULT '[]'::jsonb,
    language     TEXT NOT NULL,
    private      BOOLEAN NOT NULL DEFAULT FALSE
);

-- Partial index optimizing FIFO worker polling (FOR UPDATE SKIP LOCKED)
CREATE INDEX IF NOT EXISTS idx_submission_queue
ON submission (submitted_at ASC, id ASC)
WHERE status = 'In Queue';

-- Default test fixture task
INSERT INTO task (id, title, full_score)
VALUES ('a_plus_b', 'A + B Problem', 100)
ON CONFLICT (id) DO NOTHING;
