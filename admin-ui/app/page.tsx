"use client";

import { useEffect, useState, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useAdminKey } from "@/lib/admin-key-context";
import * as api from "@/lib/api";

export default function DashboardPage() {
  const { isConfigured } = useAdminKey();
  const [providerCount, setProviderCount] = useState<number | null>(null);
  const [modelCount, setModelCount] = useState<number | null>(null);
  const [keyCount, setKeyCount] = useState<number | null>(null);
  const [activeKeyCount, setActiveKeyCount] = useState<number | null>(null);

  const fetchCounts = useCallback(async () => {
    if (!isConfigured) return;
    try {
      const [providers, models, keys] = await Promise.all([
        api.listProviders(),
        api.listModels(),
        api.listKeys(),
      ]);
      setProviderCount(providers.length);
      setModelCount(models.length);
      setKeyCount(keys.length);
      setActiveKeyCount(keys.filter((k) => k.is_active).length);
    } catch {
      // silently fail
    }
  }, [isConfigured]);

  useEffect(() => {
    fetchCounts();
  }, [fetchCounts]);

  if (!isConfigured) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Enter your Admin Key in the sidebar to get started.
      </div>
    );
  }

  const stats = [
    { label: "Providers", value: providerCount },
    { label: "Models", value: modelCount },
    { label: "Total Keys", value: keyCount },
    { label: "Active Keys", value: activeKeyCount },
  ];

  return (
    <div>
      <div className="mb-8">
        <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          LLM Gateway overview
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map((s) => (
          <Card key={s.label}>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                {s.label}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">
                {s.value === null ? "—" : s.value}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}