/**
 * Multipart upload transport with byte progress + scaled timeout (SPEC-038).
 * SSOT for FormData POSTs that need upload.onprogress (fetch cannot).
 */

import { getRuntimeApiBaseUrl } from "@/lib/runtime-config";
import {
  adoptTraceparentFromResponse,
  ApiRequestError,
  AuthError,
  buildHeaders,
  clearTokens,
  dispatchAuthFailure,
  handleErrorResponse,
  NetworkError,
} from "@/lib/api/client";
import { getTokens, setTokens } from "@/lib/api/client-context";
import { uploadTimeoutMs } from "./upload-timeout";

export type MultipartUploadPhase = "transfer" | "admit";

export interface MultipartUploadProgress {
  loaded: number;
  total: number;
  phase: MultipartUploadPhase;
}

export interface MultipartUploadOptions {
  fileSizeBytes?: number;
  timeoutMs?: number;
  onProgress?: (progress: MultipartUploadProgress) => void;
  silent?: boolean;
}

function resolveUrl(endpoint: string): string {
  return endpoint.startsWith("http")
    ? endpoint
    : `${getRuntimeApiBaseUrl()}${endpoint}`;
}

async function tryRefreshToken(): Promise<boolean> {
  const { refreshToken: refresh } = getTokens();
  if (!refresh) return false;

  try {
    const response = await fetch(`${getRuntimeApiBaseUrl()}/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: refresh }),
    });
    if (!response.ok) {
      clearTokens();
      dispatchAuthFailure();
      return false;
    }
    const data = (await response.json()) as {
      access_token: string;
      refresh_token: string;
    };
    setTokens(data.access_token, data.refresh_token);
    return true;
  } catch {
    clearTokens();
    dispatchAuthFailure();
    return false;
  }
}

function parseJsonResponse<T>(xhr: XMLHttpRequest): T {
  const text = xhr.responseText;
  return text ? (JSON.parse(text) as T) : ({} as T);
}

function xhrPostMultipart<T>(
  url: string,
  formData: FormData,
  timeoutMs: number,
  onProgress: MultipartUploadOptions["onProgress"],
  silent: boolean,
  isRetry: boolean,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", url);
    xhr.timeout = timeoutMs;

    const headers = buildHeaders(undefined, formData);
    headers.forEach((value, key) => {
      xhr.setRequestHeader(key, value);
    });

    let admitNotified = false;

    xhr.upload.onprogress = (event) => {
      if (!onProgress) return;
      const total = event.lengthComputable ? event.total : 0;
      const loaded = event.loaded;
      if (total > 0 && loaded >= total && !admitNotified) {
        admitNotified = true;
        onProgress({ loaded, total, phase: "admit" });
        return;
      }
      onProgress({
        loaded,
        total: total > 0 ? total : loaded,
        phase: "transfer",
      });
    };

    xhr.onload = async () => {
      if (xhr.status === 401 && !isRetry) {
        const refreshed = await tryRefreshToken();
        if (refreshed) {
          try {
            const retried = await xhrPostMultipart<T>(
              url,
              formData,
              timeoutMs,
              onProgress,
              silent,
              true,
            );
            resolve(retried);
          } catch (err) {
            reject(err);
          }
          return;
        }
        reject(new AuthError());
        return;
      }

      if (xhr.status >= 200 && xhr.status < 300) {
        const traceHeader = xhr.getResponseHeader("traceparent");
        if (traceHeader) {
          adoptTraceparentFromResponse(
            new Response(null, { headers: { traceparent: traceHeader } }),
          );
        }
        try {
          resolve(parseJsonResponse<T>(xhr));
        } catch {
          reject(new ApiRequestError("Invalid JSON response", xhr.status));
        }
        return;
      }

      try {
        const err = await handleErrorResponse(
          new Response(xhr.responseText, {
            status: xhr.status,
            statusText: xhr.statusText,
            headers: { "Content-Type": "application/json" },
          }),
          { silent },
        );
        reject(err);
      } catch {
        reject(new ApiRequestError(xhr.statusText || "Request failed", xhr.status));
      }
    };

    xhr.onerror = () => reject(new NetworkError());
    xhr.ontimeout = () =>
      reject(new NetworkError(`Upload timed out after ${timeoutMs}ms`));

    xhr.send(formData);
  });
}

/** POST multipart/form-data with byte-level progress and size-scaled timeout. */
export async function postMultipart<T>(
  endpoint: string,
  formData: FormData,
  options: MultipartUploadOptions = {},
): Promise<T> {
  const fileSizeBytes = options.fileSizeBytes ?? 0;
  const timeoutMs = options.timeoutMs ?? uploadTimeoutMs(fileSizeBytes);
  const url = resolveUrl(endpoint);
  return xhrPostMultipart<T>(
    url,
    formData,
    timeoutMs,
    options.onProgress,
    options.silent ?? false,
    false,
  );
}
