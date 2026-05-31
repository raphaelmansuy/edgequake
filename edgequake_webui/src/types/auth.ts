/** Authentication types. */

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: {
    id?: string;
    user_id?: string;
    username: string;
    email?: string;
    role: string;
    roles?: string[];
    is_active?: boolean;
    created_at?: string;
    updated_at?: string;
    last_login_at?: string | null;
  };
}

export interface AuthState {
  isAuthenticated: boolean;
  user: LoginResponse["user"] | null;
  accessToken: string | null;
  refreshToken: string | null;
  expiresAt: number | null;
}
