"use client";

import { useEffect, useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/components/page-header";
import {
  DataTable,
  DataTableHeader,
  DataTableHead,
  DataTableBody,
  DataTableRow,
  DataTableCell,
  DataTableEmpty,
} from "@/components/data-table";
import { useAdminKey } from "@/lib/admin-key-context";
import * as api from "@/lib/api";
import type { ProviderInfo } from "@/lib/types";

const PROVIDER_KINDS = [
  { value: "openai", label: "OpenAI", url: "https://api.openai.com/v1" },
  { value: "openrouter", label: "OpenRouter", url: "https://openrouter.ai/api/v1" },
  { value: "dashscope", label: "DashScope", url: "https://dashscope.aliyuncs.com/compatible-mode/v1" },
];

export default function ProvidersPage() {
  const { isConfigured } = useAdminKey();
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  // Create form
  const [showCreate, setShowCreate] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createKind, setCreateKind] = useState("openai");
  const [createBaseUrl, setCreateBaseUrl] = useState("");
  const [createApiKey, setCreateApiKey] = useState("");
  const [creating, setCreating] = useState(false);

  const fetchProviders = useCallback(async () => {
    if (!isConfigured) return;
    setLoading(true);
    setError("");
    try {
      const data = await api.listProviders();
      setProviders(data);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to load providers");
    } finally {
      setLoading(false);
    }
  }, [isConfigured]);

  useEffect(() => {
    fetchProviders();
  }, [fetchProviders]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreating(true);
    setError("");
    try {
      await api.createProvider({
        name: createName,
        kind: createKind,
        base_url: createBaseUrl || undefined,
        api_key: createApiKey,
      });
      setCreateName("");
      setCreateKind("openai");
      setCreateBaseUrl("");
      setCreateApiKey("");
      setShowCreate(false);
      fetchProviders();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to create provider");
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this provider? All associated models will break.")) return;
    try {
      await api.deleteProvider(id);
      fetchProviders();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to delete provider");
    }
  };

  if (!isConfigured) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Enter your Admin Key in the sidebar to get started.
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Providers"
        description="Manage upstream LLM API providers"
      >
        <Button size="sm" onClick={() => setShowCreate(!showCreate)}>
          {showCreate ? "Cancel" : "+ Add Provider"}
        </Button>
      </PageHeader>

      {error && (
        <div className="mb-4 border border-destructive/50 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          {error}
        </div>
      )}

      {showCreate && (
        <Card className="mb-6">
          <CardHeader>
            <CardTitle>New Provider</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleCreate} className="grid grid-cols-2 gap-3">
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  Name
                </label>
                <Input
                  placeholder="e.g. openai-main"
                  value={createName}
                  onChange={(e) => setCreateName(e.target.value)}
                  required
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  Kind
                </label>
                <select
                  className="h-8 w-full border border-input bg-transparent px-2.5 text-xs outline-none focus-visible:border-ring"
                  value={createKind}
                  onChange={(e) => {
                    setCreateKind(e.target.value);
                    const kind = PROVIDER_KINDS.find((k) => k.value === e.target.value);
                    if (kind) setCreateBaseUrl(kind.url);
                  }}
                >
                  {PROVIDER_KINDS.map((k) => (
                    <option key={k.value} value={k.value}>
                      {k.label}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  Base URL{" "}
                  <span className="text-muted-foreground/60">(optional, defaults by kind)</span>
                </label>
                <Input
                  placeholder={
                    PROVIDER_KINDS.find((k) => k.value === createKind)?.url
                  }
                  value={createBaseUrl}
                  onChange={(e) => setCreateBaseUrl(e.target.value)}
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  API Key
                </label>
                <Input
                  type="password"
                  placeholder="sk-..."
                  value={createApiKey}
                  onChange={(e) => setCreateApiKey(e.target.value)}
                  required
                />
              </div>
              <div className="col-span-2 flex justify-end">
                <Button type="submit" size="sm" disabled={creating}>
                  {creating ? "Creating..." : "Create Provider"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      <DataTable>
        <DataTableHeader>
          <DataTableHead>Name</DataTableHead>
          <DataTableHead>Kind</DataTableHead>
          <DataTableHead>Base URL</DataTableHead>
          <DataTableHead>API Key</DataTableHead>
          <DataTableHead>Status</DataTableHead>
          <DataTableHead>Created</DataTableHead>
          <DataTableHead className="text-right">Actions</DataTableHead>
        </DataTableHeader>
        <DataTableBody>
          {loading ? (
            <DataTableEmpty message="Loading..." />
          ) : providers.length === 0 ? (
            <DataTableEmpty message="No providers configured yet." />
          ) : (
            providers.map((p) => (
              <DataTableRow key={p.id}>
                <DataTableCell>
                  <span className="font-medium">{p.name}</span>
                </DataTableCell>
                <DataTableCell>
                  <Badge variant="outline">{p.kind}</Badge>
                </DataTableCell>
                <DataTableCell>
                  <span className="font-mono text-[10px] text-muted-foreground">
                    {p.base_url}
                  </span>
                </DataTableCell>
                <DataTableCell>
                  <code className="text-[10px]">{p.api_key_preview}</code>
                </DataTableCell>
                <DataTableCell>
                  <Badge variant={p.is_active ? "default" : "destructive"}>
                    {p.is_active ? "Active" : "Inactive"}
                  </Badge>
                </DataTableCell>
                <DataTableCell>
                  <span className="text-muted-foreground">
                    {new Date(p.created_at).toLocaleDateString()}
                  </span>
                </DataTableCell>
                <DataTableCell className="text-right">
                  <Button
                    variant="destructive"
                    size="xs"
                    onClick={() => handleDelete(p.id)}
                  >
                    Delete
                  </Button>
                </DataTableCell>
              </DataTableRow>
            ))
          )}
        </DataTableBody>
      </DataTable>
    </div>
  );
}
