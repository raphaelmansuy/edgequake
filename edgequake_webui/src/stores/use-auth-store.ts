/**
 * @module use-auth-store
 * @description Zustand store for authentication state management.
 * Handles JWT tokens, user info, and login/logout actions.
 *
 * @implements UC0501 - User authenticates via login form
 * @implements UC0505 - User logs out and clears session
 * @implements FEAT0870 - JWT token management
 * @implements FEAT0871 - Token expiration detection
 *
 * @enforces BR0501 - Protected routes require authentication
 * @enforces BR0502 - Expired tokens trigger logout
 * @enforces BR0505 - Tokens stored securely in localStorage
 *
 * @see {@link docs/use_cases.md} UC0501, UC0505
 */
"use client";

import { clearTokens, getTokens, setTokens } from "@/lib/api/client";
import { STORE_VERSIONS, ZUSTAND_STORAGE_KEYS } from "@/lib/storage-keys";
import type { AuthState, LoginResponse } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AuthStoreState extends AuthState {
  /** Tracks if store has been hydrated from localStorage */
  _hasHydrated: boolean;
}

interface AuthStoreActions {
  login: (response: LoginResponse) => void;
  logout: () => void;
  updateUser: (user: Partial<LoginResponse["user"]>) => void;
  isTokenExpired: () => boolean;
  initializeFromStorage: () => void;
  setHasHydrated: (hydrated: boolean) => void;
}

type AuthStore = AuthStoreState & AuthStoreActions;

const initialState: AuthStoreState = {
  isAuthenticated: false,
  user: null,
  accessToken: null,
  refreshToken: null,
  expiresAt: null,
  _hasHydrated: false,
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

      setHasHydrated: (hydrated) => set({ _hasHydrated: hydrated }),
    }),
    {
      name: ZUSTAND_STORAGE_KEYS.AUTH_STORE,
      version: STORE_VERSIONS[ZUSTAND_STORAGE_KEYS.AUTH_STORE],
      partialize: (state) => ({
        isAuthenticated: state.isAuthenticated,
        user: state.user,
        expiresAt: state.expiresAt,
      }),
      /**
       * Migration function for handling schema changes
       */
      migrate: (persistedState: unknown, version: number) => {
        const state = persistedState as Partial<AuthStoreState>;

        if (version === 0) {
          // Future migrations go here
        }

        return state as AuthStoreState;
      },
      /**
       * Callback when hydration finishes
       */
      onRehydrateStorage: () => {
        return (state, error) => {
          if (error) {
            console.error("[AuthStore] Hydration failed:", error);
          }
          state?.setHasHydrated(true);

          // Sync tokens to API client after hydration
          state?.initializeFromStorage();
        };
      },
    }
  )
);

/**
 * Selector for hydration state
 */
export const useAuthStoreHydrated = () => {
  return useAuthStore((state) => state._hasHydrated);
};

export default useAuthStore;
