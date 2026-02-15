-- User keys: gateway-issued API keys for end users
CREATE TABLE IF NOT EXISTS user_keys (
    id          UUID PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    key_hash    VARCHAR(64)  NOT NULL,
    key_prefix  VARCHAR(16)  NOT NULL,
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_keys_key_hash ON user_keys (key_hash);
CREATE INDEX IF NOT EXISTS idx_user_keys_is_active ON user_keys (is_active);

-- Providers: each represents an LLM API backend (OpenAI, OpenRouter, DashScope, etc.)
CREATE TABLE IF NOT EXISTS providers (
    id          UUID PRIMARY KEY,
    name        VARCHAR(255) NOT NULL UNIQUE,
    kind        VARCHAR(50)  NOT NULL DEFAULT 'openai',  -- openai | openrouter | dashscope
    base_url    VARCHAR(512) NOT NULL,                    -- e.g. https://api.openai.com/v1
    api_key     VARCHAR(512) NOT NULL,
    is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Models: maps user-facing model names to a provider
CREATE TABLE IF NOT EXISTS models (
    id                   UUID PRIMARY KEY,
    name                 VARCHAR(255) NOT NULL UNIQUE,    -- user-facing name, e.g. "gpt-4o"
    provider_id          UUID         NOT NULL REFERENCES providers(id),
    provider_model_name  VARCHAR(255),                    -- actual name on provider side (NULL = same as name)
    is_active            BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_models_name ON models (name) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_models_provider_id ON models (provider_id);
