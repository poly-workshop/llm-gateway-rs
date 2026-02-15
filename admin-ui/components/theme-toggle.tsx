"use client";

import * as React from "react";
import { useTheme } from "next-themes";
import { Button } from "@/components/ui/button";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = React.useState(false);

  // Avoid hydration mismatch
  React.useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return (
      <div className="flex h-7 w-full items-center justify-between rounded border border-border bg-muted/30 px-2 text-xs">
        <span className="text-muted-foreground">Theme</span>
        <span className="text-muted-foreground">...</span>
      </div>
    );
  }

  const cycleTheme = () => {
    if (theme === "light") setTheme("dark");
    else if (theme === "dark") setTheme("system");
    else setTheme("light");
  };

  const getLabel = () => {
    if (theme === "light") return "Light";
    if (theme === "dark") return "Dark";
    return "System";
  };

  return (
    <Button
      variant="outline"
      size="sm"
      className="w-full justify-between px-2 text-xs h-7"
      onClick={cycleTheme}
    >
      <span className="text-muted-foreground">Theme</span>
      <span className="flex items-center gap-1">
        <span>{getLabel()}</span>
      </span>
    </Button>
  );
}
