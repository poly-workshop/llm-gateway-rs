-- Add input/output token cost coefficients to models
-- Default 1.0 means 1 raw token = 1 budget token
ALTER TABLE models ADD COLUMN input_token_coefficient  DOUBLE PRECISION NOT NULL DEFAULT 1.0;
ALTER TABLE models ADD COLUMN output_token_coefficient DOUBLE PRECISION NOT NULL DEFAULT 1.0;
