-- Request logs for tracking all proxy calls
CREATE TABLE IF NOT EXISTS request_logs (
    id                  TEXT PRIMARY KEY,
    request_id          TEXT,
    user_key_id         TEXT,
    user_key_hash       TEXT NOT NULL,
    model_requested     TEXT NOT NULL,
    model_sent          TEXT NOT NULL,
    provider_id         TEXT,
    provider_kind       TEXT,
    status_code         INTEGER NOT NULL,
    is_error            INTEGER NOT NULL DEFAULT 0,
    prompt_tokens       INTEGER,
    completion_tokens   INTEGER,
    total_tokens        INTEGER,
    latency_ms          INTEGER NOT NULL,
    is_stream           INTEGER NOT NULL DEFAULT 0,
    request_body        TEXT,
    response_body       TEXT,
    error_message       TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_request_logs_user_key ON request_logs (user_key_hash);
CREATE INDEX IF NOT EXISTS idx_request_logs_model ON request_logs (model_requested);
