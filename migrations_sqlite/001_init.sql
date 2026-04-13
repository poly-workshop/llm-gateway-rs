-- User keys: gateway-issued API keys for end users
CREATE TABLE IF NOT EXISTS user_keys (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    key_hash    TEXT NOT NULL,
    key_prefix  TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_keys_key_hash ON user_keys (key_hash);
CREATE INDEX IF NOT EXISTS idx_user_keys_is_active ON user_keys (is_active);

-- Providers: each represents an LLM API backend (OpenAI, OpenRouter, DashScope, etc.)
CREATE TABLE IF NOT EXISTS providers (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    kind        TEXT NOT NULL DEFAULT 'openai',
    base_url    TEXT NOT NULL,
    api_key     TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Models: maps user-facing model names to a provider
CREATE TABLE IF NOT EXISTS models (
    id                   TEXT PRIMARY KEY,
    name                 TEXT NOT NULL UNIQUE,
    provider_id          TEXT NOT NULL REFERENCES providers(id),
    provider_model_name  TEXT,
    is_active            INTEGER NOT NULL DEFAULT 1,
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_models_name ON models (name) WHERE is_active = 1;
CREATE INDEX IF NOT EXISTS idx_models_provider_id ON models (provider_id);
