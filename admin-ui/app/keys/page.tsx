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

export default function KeysPage() {
  const { isConfigured } = useAdminKey();
  const [keys, setKeys] = useState<UserKeyInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  // Create form
  const [showCreate, setShowCreate] = useState(false);
  const [createName, setCreateName] = useState("");
  const [creating, setCreating] = useState(false);

  // Newly created/rotated key (shown once)
  const [newKey, setNewKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

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
      const result = await api.createKey({ name: createName });
      setNewKey(result.key);
      setCopied(false);
      setCreateName("");
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

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // fallback
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
          <DataTableHead>Created</DataTableHead>
          <DataTableHead>Updated</DataTableHead>
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
                  <Badge variant={k.is_active ? "default" : "destructive"}>
                    {k.is_active ? "Active" : "Revoked"}
                  </Badge>
                </DataTableCell>
                <DataTableCell>
                  <span className="text-muted-foreground">
                    {new Date(k.created_at).toLocaleDateString()}
                  </span>
                </DataTableCell>
                <DataTableCell>
                  <span className="text-muted-foreground">
                    {new Date(k.updated_at).toLocaleDateString()}
                  </span>
                </DataTableCell>
                <DataTableCell className="text-right">
                  <div className="flex items-center justify-end gap-1">
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
