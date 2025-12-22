"use client";

import { clearTokens, getTokens, setTokens } from "@/lib/api/client";
import type { AuthState, LoginResponse } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AuthStore extends AuthState {
  // Actions
  login: (response: LoginResponse) => void;
  logout: () => void;
  updateUser: (user: Partial<LoginResponse["user"]>) => void;
  isTokenExpired: () => boolean;
  initializeFromStorage: () => void;
}

const initialState: AuthState = {
  isAuthenticated: false,
  user: null,
  accessToken: null,
  refreshToken: null,
  expiresAt: null,
};

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      ...initialState,

      login: (response: LoginResponse) => {
        const expiresAt = Date.now() + response.expires_in * 1000;

        // Store tokens in localStorage via client
        setTokens(response.access_token, response.refresh_token);

        set({
          isAuthenticated: true,
          user: response.user,
          accessToken: response.access_token,
          refreshToken: response.refresh_token,
          expiresAt,
        });
      },

      logout: () => {
        clearTokens();
        set(initialState);
      },

      updateUser: (userData) => {
        set((state) => ({
          user: state.user ? { ...state.user, ...userData } : null,
        }));
      },

      isTokenExpired: () => {
        const { expiresAt } = get();
        if (!expiresAt) return true;
        // Add 5 minute buffer
        return Date.now() > expiresAt - 5 * 60 * 1000;
      },

      initializeFromStorage: () => {
        const { accessToken, refreshToken } = getTokens();
        if (accessToken && refreshToken) {
          set({
            isAuthenticated: true,
            accessToken,
            refreshToken,
          });
        }
      },
    }),
    {
      name: "edgequake-auth",
      partialize: (state) => ({
        isAuthenticated: state.isAuthenticated,
        user: state.user,
        expiresAt: state.expiresAt,
      }),
    }
  )
);

export default useAuthStore;
