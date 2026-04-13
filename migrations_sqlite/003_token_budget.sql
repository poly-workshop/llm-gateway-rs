-- Add token budget columns to user_keys
ALTER TABLE user_keys ADD COLUMN token_budget INTEGER NULL;
ALTER TABLE user_keys ADD COLUMN tokens_used  INTEGER NOT NULL DEFAULT 0;
