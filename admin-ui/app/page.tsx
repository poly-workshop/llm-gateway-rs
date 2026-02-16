"use client";

import { useEffect, useState, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useAdminKey } from "@/lib/admin-key-context";
import * as api from "@/lib/api";
import type { DashboardStats } from "@/lib/types";
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

/** Read CSS custom-property values so recharts SVG attributes get real colors. */
function useCssColors(vars: string[]): Record<string, string> {
  const [colors, setColors] = useState<Record<string, string>>({});
  useEffect(() => {
    const root = document.documentElement;
    const style = getComputedStyle(root);
    const map: Record<string, string> = {};
    for (const v of vars) {
      map[v] = style.getPropertyValue(v).trim() || "#888";
    }
    setColors(map);

    // Re-read on theme change (class mutation on <html>)
    const observer = new MutationObserver(() => {
      const s = getComputedStyle(root);
      const m: Record<string, string> = {};
      for (const v of vars) {
        m[v] = s.getPropertyValue(v).trim() || "#888";
      }
      setColors(m);
    });
    observer.observe(root, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return colors;
}

const CSS_VARS = [
  "--chart-1",
  "--chart-2",
  "--chart-3",
  "--chart-4",
  "--destructive",
  "--popover",
  "--border",
  "--popover-foreground",
  "--muted-foreground",
];

export default function DashboardPage() {
  const { isConfigured } = useAdminKey();
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [counts, setCounts] = useState<{
    providers: number;
    models: number;
    keys: number;
    activeKeys: number;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const c = useCssColors(CSS_VARS);

  const fetchData = useCallback(async () => {
    if (!isConfigured) return;
    setLoading(true);
    try {
      const [statsData, providers, models, keys] = await Promise.all([
        api.getStats(),
        api.listProviders(),
        api.listModels(),
        api.listKeys(),
      ]);
      setStats(statsData);
      setCounts({
        providers: providers.length,
        models: models.length,
        keys: keys.length,
        activeKeys: keys.filter((k) => k.is_active).length,
      });
    } catch {
      // silently fail
    } finally {
      setLoading(false);
    }
  }, [isConfigured]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  if (!isConfigured) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Enter your Admin Key in the sidebar to get started.
      </div>
    );
  }

  if (loading && !stats) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Loading...
      </div>
    );
  }

  const errorRate =
    stats && stats.total_requests_24h > 0
      ? ((stats.total_errors_24h / stats.total_requests_24h) * 100).toFixed(1)
      : "0";

  const summaryCards = [
    {
      label: "Requests (24h)",
      value: stats ? formatNumber(stats.total_requests_24h) : "—",
    },
    {
      label: "Error Rate (24h)",
      value: stats ? `${errorRate}%` : "—",
      alert: stats != null && parseFloat(errorRate) > 5,
    },
    {
      label: "Tokens (24h)",
      value: stats ? formatNumber(stats.total_tokens_24h) : "—",
    },
    {
      label: "Avg Latency (24h)",
      value: stats ? `${stats.avg_latency_24h}ms` : "—",
    },
    { label: "Total Requests", value: stats ? formatNumber(stats.total_requests) : "—" },
    { label: "Providers", value: counts ? String(counts.providers) : "—" },
    { label: "Models", value: counts ? String(counts.models) : "—" },
    {
      label: "Active Keys",
      value: counts ? `${counts.activeKeys} / ${counts.keys}` : "—",
    },
  ];

  return (
    <div>
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          LLM Gateway overview
        </p>
      </div>

      {/* Summary cards */}
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4 mb-6">
        {summaryCards.map((s) => (
          <Card key={s.label}>
            <CardHeader className="pb-1 pt-4 px-4">
              <CardTitle className="text-xs font-medium text-muted-foreground">
                {s.label}
              </CardTitle>
            </CardHeader>
            <CardContent className="px-4 pb-4">
              <div
                className={`text-2xl font-bold ${
                  "alert" in s && s.alert ? "text-destructive" : ""
                }`}
              >
                {s.value}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Charts row 1: Requests per hour + Latency per hour */}
      {stats && stats.requests_per_hour.length > 0 && (
        <div className="grid gap-4 lg:grid-cols-2 mb-6">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Requests &amp; Errors (24h)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={240}>
                <AreaChart data={stats.requests_per_hour}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis
                    dataKey="hour"
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                  />
                  <YAxis tick={{ fontSize: 10 }} className="fill-muted-foreground" />
                  <Tooltip
                    contentStyle={{
                      fontSize: 12,
                      borderRadius: 6,
                      backgroundColor: c["--popover"],
                      borderColor: c["--border"],
                      color: c["--popover-foreground"],
                    }}
                  />
                  <Legend wrapperStyle={{ fontSize: 11 }} />
                  <Area
                    type="monotone"
                    dataKey="requests"
                    name="Requests"
                    stroke={c["--chart-1"]}
                    fill={c["--chart-1"]}
                    fillOpacity={0.15}
                    strokeWidth={2}
                  />
                  <Area
                    type="monotone"
                    dataKey="errors"
                    name="Errors"
                    stroke={c["--destructive"]}
                    fill={c["--destructive"]}
                    fillOpacity={0.1}
                    strokeWidth={2}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Avg Latency &amp; Tokens (24h)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={240}>
                <AreaChart data={stats.requests_per_hour}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis
                    dataKey="hour"
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                  />
                  <YAxis
                    yAxisId="latency"
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                    label={{
                      value: "ms",
                      position: "insideTopLeft",
                      fontSize: 10,
                      className: "fill-muted-foreground",
                    }}
                  />
                  <YAxis
                    yAxisId="tokens"
                    orientation="right"
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                    label={{
                      value: "tokens",
                      position: "insideTopRight",
                      fontSize: 10,
                      className: "fill-muted-foreground",
                    }}
                  />
                  <Tooltip
                    contentStyle={{
                      fontSize: 12,
                      borderRadius: 6,
                      backgroundColor: c["--popover"],
                      borderColor: c["--border"],
                      color: c["--popover-foreground"],
                    }}
                  />
                  <Legend wrapperStyle={{ fontSize: 11 }} />
                  <Area
                    yAxisId="latency"
                    type="monotone"
                    dataKey="avg_latency"
                    name="Latency (ms)"
                    stroke={c["--chart-3"]}
                    fill={c["--chart-3"]}
                    fillOpacity={0.12}
                    strokeWidth={2}
                  />
                  <Area
                    yAxisId="tokens"
                    type="monotone"
                    dataKey="tokens"
                    name="Tokens"
                    stroke={c["--chart-2"]}
                    fill={c["--chart-2"]}
                    fillOpacity={0.12}
                    strokeWidth={2}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Charts row 2: Model usage */}
      {stats && stats.model_usage.length > 0 && (
        <div className="grid gap-4 lg:grid-cols-2 mb-6">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Requests by Model (7d)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer
                width="100%"
                height={Math.max(200, stats.model_usage.length * 36)}
              >
                <BarChart
                  data={stats.model_usage}
                  layout="vertical"
                  margin={{ left: 10, right: 20 }}
                >
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis
                    type="number"
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                  />
                  <YAxis
                    dataKey="model"
                    type="category"
                    width={120}
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                  />
                  <Tooltip
                    contentStyle={{
                      fontSize: 12,
                      borderRadius: 6,
                      backgroundColor: c["--popover"],
                      borderColor: c["--border"],
                      color: c["--popover-foreground"],
                    }}
                    formatter={(value?: number) => formatNumber(value ?? 0)}
                  />
                  <Legend wrapperStyle={{ fontSize: 11 }} />
                  <Bar
                    dataKey="requests"
                    name="Requests"
                    fill={c["--chart-1"]}
                    radius={[0, 3, 3, 0]}
                  />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">Tokens by Model (7d)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer
                width="100%"
                height={Math.max(200, stats.model_usage.length * 36)}
              >
                <BarChart
                  data={stats.model_usage}
                  layout="vertical"
                  margin={{ left: 10, right: 20 }}
                >
                  <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                  <XAxis
                    type="number"
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                    tickFormatter={formatNumber}
                  />
                  <YAxis
                    dataKey="model"
                    type="category"
                    width={120}
                    tick={{ fontSize: 10 }}
                    className="fill-muted-foreground"
                  />
                  <Tooltip
                    contentStyle={{
                      fontSize: 12,
                      borderRadius: 6,
                      backgroundColor: c["--popover"],
                      borderColor: c["--border"],
                      color: c["--popover-foreground"],
                    }}
                    formatter={(value?: number) => formatNumber(value ?? 0)}
                  />
                  <Legend wrapperStyle={{ fontSize: 11 }} />
                  <Bar
                    dataKey="tokens"
                    name="Tokens"
                    fill={c["--chart-2"]}
                    radius={[0, 3, 3, 0]}
                  />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Provider usage */}
      {stats && stats.provider_usage.length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Requests by Provider (7d)</CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer
              width="100%"
              height={Math.max(160, stats.provider_usage.length * 40)}
            >
              <BarChart
                data={stats.provider_usage}
                layout="vertical"
                margin={{ left: 10, right: 20 }}
              >
                <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                <XAxis
                  type="number"
                  tick={{ fontSize: 10 }}
                  className="fill-muted-foreground"
                />
                <YAxis
                  dataKey="provider"
                  type="category"
                  width={100}
                  tick={{ fontSize: 10 }}
                  className="fill-muted-foreground"
                />
                <Tooltip
                  contentStyle={{
                    fontSize: 12,
                    borderRadius: 6,
                    backgroundColor: c["--popover"],
                    borderColor: c["--border"],
                    color: c["--popover-foreground"],
                  }}
                />
                <Legend wrapperStyle={{ fontSize: 11 }} />
                <Bar
                  dataKey="requests"
                  name="Requests"
                  fill={c["--chart-4"]}
                  radius={[0, 3, 3, 0]}
                />
                <Bar
                  dataKey="errors"
                  name="Errors"
                  fill={c["--destructive"]}
                  radius={[0, 3, 3, 0]}
                />
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      )}
    </div>
  );
}