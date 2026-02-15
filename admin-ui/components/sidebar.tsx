"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useAdminKey } from "@/lib/admin-key-context";
import { useState } from "react";

const navItems = [
  { href: "/", label: "Dashboard" },
  { href: "/providers", label: "Providers" },
  { href: "/models", label: "Models" },
  { href: "/keys", label: "User Keys" },
  { href: "/logs", label: "Request Logs" },
];

export function Sidebar() {
  const pathname = usePathname();
  const { adminKey, setAdminKey, isConfigured } = useAdminKey();
  const [keyInput, setKeyInput] = useState("");
  const [showKey, setShowKey] = useState(false);

  return (
    <aside className="flex h-screen w-56 shrink-0 flex-col border-r bg-background">
      <div className="border-b px-4 py-3">
        <h1 className="text-sm font-semibold tracking-tight">LLM Gateway</h1>
        <p className="text-xs text-muted-foreground">Admin Console</p>
      </div>

      <nav className="flex-1 space-y-0.5 p-2">
        {navItems.map((item) => (
          <Link key={item.href} href={item.href}>
            <Button
              variant={pathname === item.href ? "secondary" : "ghost"}
              size="sm"
              className="w-full justify-start"
            >
              {item.label}
            </Button>
          </Link>
        ))}
      </nav>

      <div className="border-t p-3">
        <label className="mb-1.5 block text-xs text-muted-foreground">
          Admin Key
        </label>
        {isConfigured ? (
          <div className="space-y-1.5">
            <div className="flex items-center gap-1">
              <Input
                type={showKey ? "text" : "password"}
                value={adminKey}
                readOnly
                className="flex-1 font-mono text-[10px]"
              />
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setShowKey(!showKey)}
                title={showKey ? "Hide" : "Show"}
              >
                {showKey ? "◉" : "◎"}
              </Button>
            </div>
            <Button
              variant="outline"
              size="xs"
              className="w-full"
              onClick={() => {
                setAdminKey("");
                setKeyInput("");
              }}
            >
              Clear
            </Button>
          </div>
        ) : (
          <form
            className="space-y-1.5"
            onSubmit={(e) => {
              e.preventDefault();
              if (keyInput.trim()) setAdminKey(keyInput.trim());
            }}
          >
            <Input
              type="password"
              placeholder="Enter admin key..."
              value={keyInput}
              onChange={(e) => setKeyInput(e.target.value)}
              className="font-mono text-[10px]"
            />
            <Button type="submit" size="xs" className="w-full">
              Connect
            </Button>
          </form>
        )}
      </div>
    </aside>
  );
}
