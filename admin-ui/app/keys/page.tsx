"use client";

import { useEffect, useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
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
import type { UserKeyInfo } from "@/lib/types";

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

export default function KeysPage() {
  const { isConfigured } = useAdminKey();
  const [keys, setKeys] = useState<UserKeyInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  // Create form
  const [showCreate, setShowCreate] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createBudget, setCreateBudget] = useState("");
  const [creating, setCreating] = useState(false);

  // Newly created/rotated key (shown once)
  const [newKey, setNewKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  // Edit budget
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editBudget, setEditBudget] = useState("");

  const fetchKeys = useCallback(async () => {
    if (!isConfigured) return;
    setLoading(true);
    setError("");
    try {
      const data = await api.listKeys();
      setKeys(data);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to load keys");
    } finally {
      setLoading(false);
    }
  }, [isConfigured]);

  useEffect(() => {
    fetchKeys();
  }, [fetchKeys]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreating(true);
    setError("");
    try {
      const budgetNum = createBudget.trim() ? parseInt(createBudget, 10) : null;
      const result = await api.createKey({
        name: createName,
        token_budget: budgetNum && !isNaN(budgetNum) ? budgetNum : null,
      });
      setNewKey(result.key);
      setCopied(false);
      setCreateName("");
      setCreateBudget("");
      setShowCreate(false);
      fetchKeys();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to create key");
    } finally {
      setCreating(false);
    }
  };

  const handleRotate = async (id: string) => {
    if (!confirm("Rotate this key? The old key will be immediately invalidated.")) return;
    try {
      const result = await api.rotateKey(id);
      setNewKey(result.key);
      setCopied(false);
      fetchKeys();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to rotate key");
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Revoke this key? It will stop working immediately.")) return;
    try {
      await api.deleteKey(id);
      fetchKeys();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to delete key");
    }
  };

  const handleUpdateBudget = async (id: string) => {
    try {
      const budgetNum = editBudget.trim() ? parseInt(editBudget, 10) : null;
      await api.updateKey(id, {
        token_budget: budgetNum && !isNaN(budgetNum) ? budgetNum : null,
      });
      setEditingId(null);
      setEditBudget("");
      fetchKeys();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to update budget");
    }
  };

  const handleResetUsage = async (id: string, k: UserKeyInfo) => {
    if (!confirm(`Reset token usage for "${k.name}" to 0?`)) return;
    try {
      await api.updateKey(id, {
        token_budget: k.token_budget,
        reset_usage: true,
      });
      fetchKeys();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to reset usage");
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // fallback
    }
  };

  const startEditBudget = (k: UserKeyInfo) => {
    setEditingId(k.id);
    setEditBudget(k.token_budget != null ? String(k.token_budget) : "");
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
        title="User Keys"
        description="Issue and manage API keys for gateway users"
      >
        <Button size="sm" onClick={() => setShowCreate(!showCreate)}>
          {showCreate ? "Cancel" : "+ Create Key"}
        </Button>
      </PageHeader>

      {error && (
        <div className="mb-4 border border-destructive/50 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          {error}
        </div>
      )}

      {newKey && (
        <Card className="mb-6 border-green-500/30 bg-green-500/5">
          <CardHeader>
            <CardTitle className="text-green-600">
              Key Created Successfully
            </CardTitle>
            <CardDescription>
              Copy this key now — it will not be shown again.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <code className="flex-1 break-all bg-muted/50 px-3 py-2 font-mono text-xs">
                {newKey}
              </code>
              <Button
                variant="outline"
                size="sm"
                onClick={() => copyToClipboard(newKey)}
              >
                {copied ? "Copied!" : "Copy"}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setNewKey(null)}
              >
                Dismiss
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {showCreate && (
        <Card className="mb-6">
          <CardHeader>
            <CardTitle>New User Key</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleCreate} className="flex items-end gap-3">
              <div className="flex-1">
                <label className="mb-1 block text-xs text-muted-foreground">
                  Key Name
                </label>
                <Input
                  placeholder="e.g. my-app, team-backend"
                  value={createName}
                  onChange={(e) => setCreateName(e.target.value)}
                  required
                />
              </div>
              <div className="w-48">
                <label className="mb-1 block text-xs text-muted-foreground">
                  Token Budget (optional)
                </label>
                <Input
                  type="number"
                  placeholder="Unlimited"
                  value={createBudget}
                  onChange={(e) => setCreateBudget(e.target.value)}
                  min={0}
                />
              </div>
              <Button type="submit" size="sm" disabled={creating}>
                {creating ? "Creating..." : "Generate Key"}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      <DataTable>
        <DataTableHeader>
          <DataTableHead>Name</DataTableHead>
          <DataTableHead>Key Prefix</DataTableHead>
          <DataTableHead>Status</DataTableHead>
          <DataTableHead>Token Usage</DataTableHead>
          <DataTableHead>Created</DataTableHead>
          <DataTableHead className="text-right">Actions</DataTableHead>
        </DataTableHeader>
        <DataTableBody>
          {loading ? (
            <DataTableEmpty message="Loading..." />
          ) : keys.length === 0 ? (
            <DataTableEmpty message="No user keys created yet." />
          ) : (
            keys.map((k) => (
              <DataTableRow key={k.id}>
                <DataTableCell>
                  <span className="font-medium">{k.name}</span>
                </DataTableCell>
                <DataTableCell>
                  <code className="text-[10px]">{k.key_prefix}</code>
                </DataTableCell>
                <DataTableCell>
                  <Badge variant={k.is_active ? "success" : "destructive"}>
                    {k.is_active ? "Active" : "Revoked"}
                  </Badge>
                </DataTableCell>
                <DataTableCell>
                  {editingId === k.id ? (
                    <div className="flex items-center gap-1">
                      <Input
                        type="number"
                        className="h-7 w-28 text-xs"
                        placeholder="Unlimited"
                        value={editBudget}
                        onChange={(e) => setEditBudget(e.target.value)}
                        min={0}
                        autoFocus
                        onKeyDown={(e) => {
                          if (e.key === "Enter") handleUpdateBudget(k.id);
                          if (e.key === "Escape") setEditingId(null);
                        }}
                      />
                      <Button
                        variant="outline"
                        size="xs"
                        onClick={() => handleUpdateBudget(k.id)}
                      >
                        Save
                      </Button>
                      <Button
                        variant="ghost"
                        size="xs"
                        onClick={() => setEditingId(null)}
                      >
                        Cancel
                      </Button>
                    </div>
                  ) : (
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs tabular-nums">
                        {formatTokens(k.tokens_used)}
                        {k.token_budget != null && (
                          <span className="text-muted-foreground">
                            {" / "}
                            {formatTokens(k.token_budget)}
                          </span>
                        )}
                        {k.token_budget == null && (
                          <span className="text-muted-foreground"> / ∞</span>
                        )}
                      </span>
                      {k.token_budget != null &&
                        k.tokens_used >= k.token_budget && (
                          <Badge variant="destructive" className="text-[10px] px-1 py-0">
                            Exhausted
                          </Badge>
                        )}
                      {k.is_active && (
                        <Button
                          variant="ghost"
                          size="xs"
                          className="h-5 px-1 text-[10px]"
                          onClick={() => startEditBudget(k)}
                        >
                          Edit
                        </Button>
                      )}
                    </div>
                  )}
                </DataTableCell>
                <DataTableCell>
                  <span className="text-muted-foreground">
                    {new Date(k.created_at).toLocaleDateString()}
                  </span>
                </DataTableCell>
                <DataTableCell className="text-right">
                  <div className="flex items-center justify-end gap-1">
                    {k.is_active && k.tokens_used > 0 && (
                      <Button
                        variant="outline"
                        size="xs"
                        onClick={() => handleResetUsage(k.id, k)}
                      >
                        Reset Usage
                      </Button>
                    )}
                    {k.is_active && (
                      <Button
                        variant="outline"
                        size="xs"
                        onClick={() => handleRotate(k.id)}
                      >
                        Rotate
                      </Button>
                    )}
                    {k.is_active && (
                      <Button
                        variant="destructive"
                        size="xs"
                        onClick={() => handleDelete(k.id)}
                      >
                        Revoke
                      </Button>
                    )}
                  </div>
                </DataTableCell>
              </DataTableRow>
            ))
          )}
        </DataTableBody>
      </DataTable>
    </div>
  );
}
