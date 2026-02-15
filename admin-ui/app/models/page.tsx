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
import type { ModelInfo, ProviderInfo } from "@/lib/types";

export default function ModelsPage() {
  const { isConfigured } = useAdminKey();
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  // Create form
  const [showCreate, setShowCreate] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createProviderId, setCreateProviderId] = useState("");
  const [createProviderModelName, setCreateProviderModelName] = useState("");
  const [creating, setCreating] = useState(false);

  const fetchData = useCallback(async () => {
    if (!isConfigured) return;
    setLoading(true);
    setError("");
    try {
      const [modelsData, providersData] = await Promise.all([
        api.listModels(),
        api.listProviders(),
      ]);
      setModels(modelsData);
      setProviders(providersData);
      if (providersData.length > 0 && !createProviderId) {
        setCreateProviderId(providersData[0].id);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to load data");
    } finally {
      setLoading(false);
    }
  }, [isConfigured, createProviderId]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreating(true);
    setError("");
    try {
      await api.createModel({
        name: createName,
        provider_id: createProviderId,
        provider_model_name: createProviderModelName || undefined,
      });
      setCreateName("");
      setCreateProviderModelName("");
      setShowCreate(false);
      fetchData();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to create model");
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this model mapping?")) return;
    try {
      await api.deleteModel(id);
      fetchData();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to delete model");
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
        title="Models"
        description="Map user-facing model names to providers"
      >
        <Button size="sm" onClick={() => setShowCreate(!showCreate)}>
          {showCreate ? "Cancel" : "+ Add Model"}
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
            <CardTitle>New Model</CardTitle>
          </CardHeader>
          <CardContent>
            {providers.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                No providers available. Create a provider first.
              </p>
            ) : (
              <form onSubmit={handleCreate} className="grid grid-cols-2 gap-3">
                <div>
                  <label className="mb-1 block text-xs text-muted-foreground">
                    Model Name
                  </label>
                  <Input
                    placeholder="e.g. gpt-4o"
                    value={createName}
                    onChange={(e) => setCreateName(e.target.value)}
                    required
                  />
                  <p className="mt-0.5 text-[10px] text-muted-foreground/60">
                    User-facing name used in API requests
                  </p>
                </div>
                <div>
                  <label className="mb-1 block text-xs text-muted-foreground">
                    Provider
                  </label>
                  <select
                    className="h-8 w-full border border-input bg-transparent px-2.5 text-xs outline-none focus-visible:border-ring"
                    value={createProviderId}
                    onChange={(e) => setCreateProviderId(e.target.value)}
                  >
                    {providers.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name} ({p.kind})
                      </option>
                    ))}
                  </select>
                </div>
                <div className="col-span-2">
                  <label className="mb-1 block text-xs text-muted-foreground">
                    Provider Model Name{" "}
                    <span className="text-muted-foreground/60">
                      (optional — defaults to model name)
                    </span>
                  </label>
                  <Input
                    placeholder="e.g. gpt-4o-2024-08-06"
                    value={createProviderModelName}
                    onChange={(e) =>
                      setCreateProviderModelName(e.target.value)
                    }
                  />
                  <p className="mt-0.5 text-[10px] text-muted-foreground/60">
                    Actual model identifier sent to the provider. Leave empty if same as model name.
                  </p>
                </div>
                <div className="col-span-2 flex justify-end">
                  <Button type="submit" size="sm" disabled={creating}>
                    {creating ? "Creating..." : "Create Model"}
                  </Button>
                </div>
              </form>
            )}
          </CardContent>
        </Card>
      )}

      <DataTable>
        <DataTableHeader>
          <DataTableHead>Model Name</DataTableHead>
          <DataTableHead>Provider</DataTableHead>
          <DataTableHead>Provider Model Name</DataTableHead>
          <DataTableHead>Status</DataTableHead>
          <DataTableHead>Created</DataTableHead>
          <DataTableHead className="text-right">Actions</DataTableHead>
        </DataTableHeader>
        <DataTableBody>
          {loading ? (
            <DataTableEmpty message="Loading..." />
          ) : models.length === 0 ? (
            <DataTableEmpty message="No models configured yet." />
          ) : (
            models.map((m) => (
              <DataTableRow key={m.id}>
                <DataTableCell>
                  <code className="font-medium">{m.name}</code>
                </DataTableCell>
                <DataTableCell>
                  <Badge variant="outline">
                    {m.provider_name || m.provider_id.slice(0, 8)}
                  </Badge>
                </DataTableCell>
                <DataTableCell>
                  <code className="text-[10px] text-muted-foreground">
                    {m.provider_model_name || "—"}
                  </code>
                </DataTableCell>
                <DataTableCell>
                  <Badge variant={m.is_active ? "default" : "destructive"}>
                    {m.is_active ? "Active" : "Inactive"}
                  </Badge>
                </DataTableCell>
                <DataTableCell>
                  <span className="text-muted-foreground">
                    {new Date(m.created_at).toLocaleDateString()}
                  </span>
                </DataTableCell>
                <DataTableCell className="text-right">
                  <Button
                    variant="destructive"
                    size="xs"
                    onClick={() => handleDelete(m.id)}
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
