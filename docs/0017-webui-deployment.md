# EdgeQuake WebUI Deployment

> Guide for building, containerizing, and deploying the Next.js frontend.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## 1. Build Strategy

We use **Next.js Standalone Output** mode (`output: "standalone"` in `next.config.ts`).
This automatically traces dependencies and generates a minimal `server.js` file and `node_modules` folder needed for production, reducing the deployment artifact size significantly (typically ~100MB vs ~1GB).

### 1.1 The Artifacts

After running `pnpm build`, the relevant outputs are:

1.  `.next/standalone/`: The minimal node server.
2.  `.next/static/`: Static JS/CSS files (Client components).
3.  `public/`: Static assets (images, locales).

---

## 2. Environment Variables

Configuration is handled via strict environment variables. These must be present at **build time** (if used in client code) or **runtime** (for server side).

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `NEXT_PUBLIC_API_URL` | Build/Run | URL of the Backend API | `http://localhost:8080/api/v1` |
| `NEXT_PUBLIC_DEBUG_MODE` | Build | Enable verbose logging | `false` |
| `NEXT_PUBLIC_WS_URL` | Build/Run | (Optional) Explicit WebSocket URL | Auto-derived from API_URL |

> **Critical**: `NEXT_PUBLIC_` variables are inlined into the JS bundle at **build time**. If you are building a Docker image to run in different environments (Staging vs Prod), you cannot rely on build-time env vars alone. You must either rebuild per env or use a runtime config injection strategy (uncommon in Next.js). **EdgeQuake recommends rebuilding for each environment.**

---

## 3. Docker Deployment

Since we use standalone mode, the Dockerfile is multi-stage and efficient.

### 3.1 Recommended Dockerfile

```dockerfile
# Stage 1: Builder
FROM node:20-alpine AS builder
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile
COPY . .
ENV NEXT_PUBLIC_API_URL=http://api-service:8080/api/v1
RUN pnpm build

# Stage 2: Runner
FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production

# Copy standalone server
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

EXPOSE 3000
CMD ["node", "server.js"]
```

---

## 4. Nginx Reverse Proxy Protocol

In production, EdgeQuake is typically deployed behind Nginx to serve both Frontend and Backend on the same port (443).

### 4.1 Routing Rules

```nginx
server {
    listen 80;
    server_name edgequake.internal;

    # 1. Frontend (Next.js)
    location / {
        proxy_pass http://webui-service:3000;
        proxy_set_header Host $host;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    # 2. Backend API
    location /api/ {
        proxy_pass http://backend-service:8080/api/;
        # Streaming support (SSE)
        proxy_buffering off;
        proxy_cache off;
    }
}
```

---

## 5. Streaming & Timeouts

The WebUI relies heavily on **Server-Sent Events (SSE)** for AI generation.

**Infrastructure Requirements**:
1.  **No Buffering**: Your load balancer (Nginx/AWS ALB) must disable buffering for `/api/query` endpoints.
2.  **Long Timeouts**: Set idle timeouts to at least **60 seconds**.
3.  **HTTP/2**: Strictly recommended for multiplexing multiple streams (e.g., chat + graph loading).

---

## 6. CDN & Static Caching

For maximum performance, the `.next/static` folder can be uploaded to a CDN (S3/CloudFront).

To specific a CDN asset prefix, set in `next.config.ts`:
```typescript
const nextConfig = {
  assetPrefix: process.env.CDN_URL || '',
}
```

This ensures `_next/static/chunks/*.js` are loaded from the CDN edge.
