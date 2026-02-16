import type {
  ProviderInfo,
  CreateProviderRequest,
  UpdateProviderRequest,
  ModelInfo,
  CreateModelRequest,
  UpdateModelRequest,
  UserKeyInfo,
  UserKeyCreated,
  CreateKeyRequest,
  UpdateKeyRequest,
  LogListResponse,
  ListLogsParams,
} from "./types";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

function getAdminKey(): string {
  if (typeof window !== "undefined") {
    return localStorage.getItem("admin_key") || "";
  }
  return "";
}

async function request<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const adminKey = getAdminKey();
  const res = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${adminKey}`,
      ...options.headers,
    },
  });

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    const message =
      body?.error?.message || `Request failed with status ${res.status}`;
    throw new Error(message);
  }

  if (res.status === 204) return undefined as T;
  return res.json();
}

// ── Providers ────────────────────────────────────────────────

export async function listProviders(): Promise<ProviderInfo[]> {
  return request<ProviderInfo[]>("/admin/providers");
}

export async function createProvider(
  data: CreateProviderRequest
): Promise<ProviderInfo> {
  return request<ProviderInfo>("/admin/providers", {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function updateProvider(
  id: string,
  data: UpdateProviderRequest
): Promise<ProviderInfo> {
  return request<ProviderInfo>(`/admin/providers/${id}`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

export async function deleteProvider(id: string): Promise<void> {
  return request<void>(`/admin/providers/${id}`, { method: "DELETE" });
}

// ── Models ───────────────────────────────────────────────────

export async function listModels(): Promise<ModelInfo[]> {
  return request<ModelInfo[]>("/admin/models");
}

export async function createModel(
  data: CreateModelRequest
): Promise<ModelInfo> {
  return request<ModelInfo>("/admin/models", {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function deleteModel(id: string): Promise<void> {
  return request<void>(`/admin/models/${id}`, { method: "DELETE" });
}

export async function updateModel(
  id: string,
  data: UpdateModelRequest
): Promise<ModelInfo> {
  return request<ModelInfo>(`/admin/models/${id}`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

// ── User Keys ────────────────────────────────────────────────

export async function listKeys(): Promise<UserKeyInfo[]> {
  return request<UserKeyInfo[]>("/admin/keys");
}

export async function createKey(
  data: CreateKeyRequest
): Promise<UserKeyCreated> {
  return request<UserKeyCreated>("/admin/keys", {
    method: "POST",
    body: JSON.stringify(data),
  });
}

export async function rotateKey(id: string): Promise<UserKeyCreated> {
  return request<UserKeyCreated>(`/admin/keys/${id}/rotate`, {
    method: "POST",
  });
}

export async function deleteKey(id: string): Promise<void> {
  return request<void>(`/admin/keys/${id}`, { method: "DELETE" });
}

export async function updateKey(
  id: string,
  data: UpdateKeyRequest
): Promise<UserKeyInfo> {
  return request<UserKeyInfo>(`/admin/keys/${id}`, {
    method: "PUT",
    body: JSON.stringify(data),
  });
}

// ── Request Logs ─────────────────────────────────────────────

export async function listLogs(
  params: ListLogsParams = {}
): Promise<LogListResponse> {
  const searchParams = new URLSearchParams();
  if (params.page) searchParams.set("page", String(params.page));
  if (params.per_page) searchParams.set("per_page", String(params.per_page));
  if (params.key_id) searchParams.set("key_id", params.key_id);
  if (params.model) searchParams.set("model", params.model);
  const qs = searchParams.toString();
  return request<LogListResponse>(`/admin/logs${qs ? `?${qs}` : ""}`);
}
