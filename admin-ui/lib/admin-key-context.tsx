"use client";

import { createContext, useContext, useState, useEffect, useCallback } from "react";

interface AdminKeyContextType {
  adminKey: string;
  setAdminKey: (key: string) => void;
  isConfigured: boolean;
}

const AdminKeyContext = createContext<AdminKeyContextType>({
  adminKey: "",
  setAdminKey: () => {},
  isConfigured: false,
});

export function AdminKeyProvider({ children }: { children: React.ReactNode }) {
  const [adminKey, setAdminKeyState] = useState("");

  useEffect(() => {
    const stored = localStorage.getItem("admin_key");
    if (stored) setAdminKeyState(stored);
  }, []);

  const setAdminKey = useCallback((key: string) => {
    setAdminKeyState(key);
    localStorage.setItem("admin_key", key);
  }, []);

  return (
    <AdminKeyContext.Provider
      value={{ adminKey, setAdminKey, isConfigured: adminKey.length > 0 }}
    >
      {children}
    </AdminKeyContext.Provider>
  );
}

export function useAdminKey() {
  return useContext(AdminKeyContext);
}
