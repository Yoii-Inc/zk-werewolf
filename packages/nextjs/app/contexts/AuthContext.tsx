"use client";

import React, { createContext, useContext, useEffect, useState } from "react";

interface User {
  id: string;
  username: string;
  email: string;
}

interface AuthContextType {
  isAuthenticated: boolean;
  token: string | null;
  user: User | null;
  login: (token: string, userData: User) => void;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [token, setToken] = useState<string | null>(null);
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    // 環境に応じてストレージを選択
    const isTestEnv = process.env.NODE_ENV === "development";
    const storage = isTestEnv ? sessionStorage : localStorage;

    // ページ読み込み時にストレージからユーザー情報を取得
    const storedToken = storage.getItem("token");
    const storedUser = storage.getItem("user");
    if (storedToken && storedUser) {
      setToken(storedToken);
      setUser(JSON.parse(storedUser));
    }
  }, []);

  const login = (newToken: string, userData: User) => {
    const isTestEnv = process.env.NODE_ENV === "development";
    const storage = isTestEnv ? sessionStorage : localStorage;

    setToken(newToken);
    setUser(userData);
    storage.setItem("token", newToken);
    storage.setItem("user", JSON.stringify(userData));
  };

  const logout = () => {
    const isTestEnv = process.env.NODE_ENV === "development";
    const storage = isTestEnv ? sessionStorage : localStorage;

    setToken(null);
    setUser(null);
    storage.removeItem("token");
    storage.removeItem("user");
  };

  return (
    <AuthContext.Provider
      value={{
        isAuthenticated: !!token,
        token,
        user,
        login,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}
