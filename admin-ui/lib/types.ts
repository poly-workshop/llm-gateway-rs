// Types matching the Rust backend API responses

export interface ProviderInfo {
  id: string;
  name: string;
  kind: string;
  base_url: string;
  api_key_preview: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  kind: string;
  base_url?: string;
  api_key: string;
}

export interface UpdateProviderRequest {
  name?: string;
  kind?: string;
  base_url?: string;
  api_key?: string;
  is_active?: boolean;
}

export interface ModelInfo {
  id: string;
  name: string;
  provider_id: string;
  provider_name: string | null;
  provider_model_name: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateModelRequest {
  name: string;
  provider_id: string;
  provider_model_name?: string;
}

export interface UserKeyInfo {
  id: string;
  name: string;
  key_prefix: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface UserKeyCreated {
  id: string;
  name: string;
  key: string;
  key_prefix: string;
  created_at: string;
}

export interface CreateKeyRequest {
  name: string;
}

export interface ApiError {
  error: {
    message: string;
  };
}

// ── Request Logs ─────────────────────────────────────────────

export interface RequestLogInfo {
  id: string;
  request_id: string | null;
  user_key_id: string | null;
  model_requested: string;
  model_sent: string;
  provider_id: string | null;
  provider_kind: string | null;
  status_code: number;
  is_error: boolean;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  total_tokens: number | null;
  latency_ms: number;
  is_stream: boolean;
  request_body: unknown | null;
  response_body: unknown | null;
  error_message: string | null;
  created_at: string;
}

export interface LogListResponse {
  data: RequestLogInfo[];
  total: number;
  page: number;
  per_page: number;
}

export interface ListLogsParams {
  page?: number;
  per_page?: number;
  key_id?: string;
  model?: string;
}
