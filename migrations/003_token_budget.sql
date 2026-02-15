-- Add token budget columns to user_keys
ALTER TABLE user_keys ADD COLUMN token_budget BIGINT NULL;        -- NULL = unlimited
ALTER TABLE user_keys ADD COLUMN tokens_used  BIGINT NOT NULL DEFAULT 0;
