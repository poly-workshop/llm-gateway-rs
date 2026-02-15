-- Request logs for tracking all proxy calls
CREATE TABLE request_logs (
    id                  UUID PRIMARY KEY,
    request_id          VARCHAR(64),
    user_key_id         UUID,
    user_key_hash       VARCHAR(64) NOT NULL,
    model_requested     VARCHAR(255) NOT NULL,
    model_sent          VARCHAR(255) NOT NULL,
    provider_id         UUID,
    provider_kind       VARCHAR(50),
    status_code         SMALLINT NOT NULL,
    is_error            BOOLEAN NOT NULL DEFAULT FALSE,
    prompt_tokens       INTEGER,
    completion_tokens   INTEGER,
    total_tokens        INTEGER,
    latency_ms          INTEGER NOT NULL,
    is_stream           BOOLEAN NOT NULL DEFAULT FALSE,
    request_body        JSONB,
    response_body       JSONB,
    error_message       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_request_logs_created_at ON request_logs (created_at DESC);
CREATE INDEX idx_request_logs_user_key ON request_logs (user_key_hash);
CREATE INDEX idx_request_logs_model ON request_logs (model_requested);
