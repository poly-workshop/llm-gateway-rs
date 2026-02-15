"use client";

import { useEffect, useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import type { RequestLogInfo } from "@/lib/types";

export default function LogsPage() {
  const { isConfigured } = useAdminKey();
  const [logs, setLogs] = useState<RequestLogInfo[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [perPage] = useState(25);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  // Filters
  const [filterModel, setFilterModel] = useState("");
  const [filterKeyId, setFilterKeyId] = useState("");

  // Detail view
  const [selectedLog, setSelectedLog] = useState<RequestLogInfo | null>(null);

  const fetchLogs = useCallback(async () => {
    if (!isConfigured) return;
    setLoading(true);
    setError("");
    try {
      const result = await api.listLogs({
        page,
        per_page: perPage,
        model: filterModel || undefined,
        key_id: filterKeyId || undefined,
      });
      setLogs(result.data);
      setTotal(result.total);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to load logs");
    } finally {
      setLoading(false);
    }
  }, [isConfigured, page, perPage, filterModel, filterKeyId]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const totalPages = Math.ceil(total / perPage);

  const handleApplyFilter = () => {
    setPage(1);
    fetchLogs();
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
        title="Request Logs"
        description="View proxy request history and usage"
      >
        <Button size="sm" variant="outline" onClick={fetchLogs}>
          Refresh
        </Button>
      </PageHeader>

      {error && (
        <div className="mb-4 border border-destructive/50 bg-destructive/10 px-3 py-2 text-xs text-destructive">
          {error}
        </div>
      )}

      {/* Filters */}
      <div className="mb-4 flex items-end gap-3">
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">
            Model
          </label>
          <Input
            placeholder="e.g. gpt-4o"
            value={filterModel}
            onChange={(e) => setFilterModel(e.target.value)}
            className="w-40"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">
            Key ID
          </label>
          <Input
            placeholder="UUID"
            value={filterKeyId}
            onChange={(e) => setFilterKeyId(e.target.value)}
            className="w-56"
          />
        </div>
        <Button size="sm" variant="outline" onClick={handleApplyFilter}>
          Filter
        </Button>
        {(filterModel || filterKeyId) && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setFilterModel("");
              setFilterKeyId("");
              setPage(1);
            }}
          >
            Clear
          </Button>
        )}
      </div>

      <DataTable>
        <DataTableHeader>
          <DataTableHead>Time</DataTableHead>
          <DataTableHead>Model</DataTableHead>
          <DataTableHead>Provider</DataTableHead>
          <DataTableHead>Status</DataTableHead>
          <DataTableHead>Stream</DataTableHead>
          <DataTableHead>Tokens</DataTableHead>
          <DataTableHead>Latency</DataTableHead>
          <DataTableHead className="text-right">Actions</DataTableHead>
        </DataTableHeader>
        <DataTableBody>
          {loading ? (
            <DataTableEmpty message="Loading..." />
          ) : logs.length === 0 ? (
            <DataTableEmpty message="No request logs found." />
          ) : (
            logs.map((log) => (
              <DataTableRow key={log.id}>
                <DataTableCell>
                  <span className="text-[10px] text-muted-foreground">
                    {new Date(log.created_at).toLocaleString()}
                  </span>
                </DataTableCell>
                <DataTableCell>
                  <span className="font-medium text-xs">
                    {log.model_requested}
                  </span>
                  {log.model_requested !== log.model_sent && (
                    <span className="ml-1 text-[10px] text-muted-foreground">
                      → {log.model_sent}
                    </span>
                  )}
                </DataTableCell>
                <DataTableCell>
                  <span className="text-xs">{log.provider_kind ?? "—"}</span>
                </DataTableCell>
                <DataTableCell>
                  <Badge
                    variant={log.is_error ? "destructive" : "default"}
                  >
                    {log.status_code}
                  </Badge>
                </DataTableCell>
                <DataTableCell>
                  <span className="text-xs">
                    {log.is_stream ? "SSE" : "JSON"}
                  </span>
                </DataTableCell>
                <DataTableCell>
                  {log.total_tokens != null ? (
                    <span className="text-xs tabular-nums">
                      {log.prompt_tokens ?? 0} / {log.completion_tokens ?? 0} ={" "}
                      {log.total_tokens}
                    </span>
                  ) : (
                    <span className="text-xs text-muted-foreground">—</span>
                  )}
                </DataTableCell>
                <DataTableCell>
                  <span className="text-xs tabular-nums">
                    {log.latency_ms}ms
                  </span>
                </DataTableCell>
                <DataTableCell className="text-right">
                  <Button
                    variant="outline"
                    size="xs"
                    onClick={() => setSelectedLog(log)}
                  >
                    Details
                  </Button>
                </DataTableCell>
              </DataTableRow>
            ))
          )}
        </DataTableBody>
      </DataTable>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="mt-4 flex items-center justify-between text-xs text-muted-foreground">
          <div>
            Page {page} of {totalPages} ({total} total)
          </div>
          <div className="flex gap-1">
            <Button
              size="xs"
              variant="outline"
              disabled={page <= 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              size="xs"
              variant="outline"
              disabled={page >= totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}

      {/* Detail Modal */}
      {selectedLog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <Card className="m-4 max-h-[80vh] w-full max-w-2xl overflow-auto">
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle className="text-sm">
                Request Log Detail
              </CardTitle>
              <Button
                variant="ghost"
                size="xs"
                onClick={() => setSelectedLog(null)}
              >
                ✕
              </Button>
            </CardHeader>
            <CardContent className="space-y-4 text-xs">
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <span className="text-muted-foreground">ID:</span>{" "}
                  <span className="font-mono">{selectedLog.id}</span>
                </div>
                <div>
                  <span className="text-muted-foreground">Request ID:</span>{" "}
                  <span className="font-mono">
                    {selectedLog.request_id ?? "—"}
                  </span>
                </div>
                <div>
                  <span className="text-muted-foreground">Model:</span>{" "}
                  {selectedLog.model_requested}
                  {selectedLog.model_requested !== selectedLog.model_sent &&
                    ` → ${selectedLog.model_sent}`}
                </div>
                <div>
                  <span className="text-muted-foreground">Provider:</span>{" "}
                  {selectedLog.provider_kind ?? "—"}
                </div>
                <div>
                  <span className="text-muted-foreground">Status:</span>{" "}
                  <Badge
                    variant={
                      selectedLog.is_error ? "destructive" : "default"
                    }
                  >
                    {selectedLog.status_code}
                  </Badge>
                </div>
                <div>
                  <span className="text-muted-foreground">Stream:</span>{" "}
                  {selectedLog.is_stream ? "Yes" : "No"}
                </div>
                <div>
                  <span className="text-muted-foreground">Latency:</span>{" "}
                  {selectedLog.latency_ms}ms
                </div>
                <div>
                  <span className="text-muted-foreground">Tokens:</span>{" "}
                  {selectedLog.total_tokens != null
                    ? `${selectedLog.prompt_tokens ?? 0} / ${selectedLog.completion_tokens ?? 0} = ${selectedLog.total_tokens}`
                    : "—"}
                </div>
                <div>
                  <span className="text-muted-foreground">Key ID:</span>{" "}
                  <span className="font-mono">
                    {selectedLog.user_key_id ?? "—"}
                  </span>
                </div>
                <div>
                  <span className="text-muted-foreground">Time:</span>{" "}
                  {new Date(selectedLog.created_at).toLocaleString()}
                </div>
              </div>

              {selectedLog.error_message && (
                <div>
                  <div className="mb-1 font-medium text-destructive">
                    Error Message
                  </div>
                  <pre className="whitespace-pre-wrap rounded bg-destructive/10 p-2 font-mono text-[10px]">
                    {selectedLog.error_message}
                  </pre>
                </div>
              )}

              {selectedLog.request_body != null && (
                <div>
                  <div className="mb-1 font-medium">Request Body</div>
                  <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded bg-muted/50 p-2 font-mono text-[10px]">
                    {JSON.stringify(selectedLog.request_body, null, 2)}
                  </pre>
                </div>
              )}

              {selectedLog.response_body != null && (
                <div>
                  <div className="mb-1 font-medium">Response Body</div>
                  <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded bg-muted/50 p-2 font-mono text-[10px]">
                    {JSON.stringify(selectedLog.response_body, null, 2)}
                  </pre>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
