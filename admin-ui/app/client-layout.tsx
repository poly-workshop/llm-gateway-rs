"use client";

import { AdminKeyProvider } from "@/lib/admin-key-context";
import { Sidebar } from "@/components/sidebar";

export function ClientLayout({ children }: { children: React.ReactNode }) {
  return (
    <AdminKeyProvider>
      <div className="flex h-screen overflow-hidden">
        <Sidebar />
        <main className="flex-1 overflow-y-auto p-6">{children}</main>
      </div>
    </AdminKeyProvider>
  );
}
