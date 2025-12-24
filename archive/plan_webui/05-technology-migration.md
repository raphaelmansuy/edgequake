# EdgeQuake WebUI - Technology Migration Guide

> Step-by-step migration from Vite/React SPA to Next.js 15 App Router.

**Parent Document**: [00-master-plan.md](./00-master-plan.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Technology Stack Comparison](#technology-stack-comparison)
3. [Migration Steps](#migration-steps)
4. [Configuration Migration](#configuration-migration)
5. [Breaking Changes](#breaking-changes)
6. [Testing Migration](#testing-migration)

---

## Overview

This guide details the migration from the current LightRAG WebUI technology stack to the modernized EdgeQuake WebUI stack.

### Key Migrations

| From             | To                 |
| ---------------- | ------------------ |
| Vite 6.3.6       | Next.js 15.1.x     |
| React Router 6.x | Next.js App Router |
| HashRouter       | File-based routing |
| Client-only SPA  | SSR/SSG hybrid     |
| Axios            | Native fetch       |
| i18next          | next-intl          |
| Custom theme     | next-themes        |

---

## Technology Stack Comparison

### Current Stack (LightRAG)

From [package.json](../lightrag_webui/package.json):

```json
{
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "react-router-dom": "^7.6.1",
    "axios": "^1.9.0",
    "zustand": "^5.0.0",
    "@sigma/core": "^3.0.0",
    "graphology": "^0.26.0",
    "i18next": "^25.2.1",
    "tailwindcss": "^4.1.11"
  },
  "devDependencies": {
    "vite": "^6.3.6",
    "typescript": "^5.8.3"
  }
}
```

### Target Stack (EdgeQuake)

```json
{
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "next": "^15.1.0",
    "zustand": "^5.0.0",
    "@sigma/core": "^3.0.0",
    "graphology": "^0.26.0",
    "next-intl": "^3.25.0",
    "next-themes": "^0.4.4",
    "@tanstack/react-query": "^5.62.0"
  },
  "devDependencies": {
    "typescript": "^5.7.2",
    "tailwindcss": "^4.0.0"
  }
}
```

---

## Migration Steps

### Step 1: Initialize Next.js Project

```bash
cd /Users/raphaelmansuy/Github/03-working/edgequake
bunx create-next-app@latest edgequake_webui \
  --typescript \
  --tailwind \
  --eslint \
  --app \
  --src-dir \
  --import-alias "@/*" \
  --use-bun
```

### Step 2: Install Dependencies

```bash
cd edgequake_webui

# Core dependencies
bun add zustand @tanstack/react-query next-themes next-intl

# Graph visualization
bun add @sigma/core graphology graphology-layout-forceatlas2 graphology-communities-louvain

# UI components
bunx shadcn@latest init
bunx shadcn@latest add button card dialog input select table tabs textarea tooltip \
  alert alert-dialog badge checkbox command dropdown-menu hover-card popover \
  scroll-area separator sheet skeleton slider switch toast

# Utilities
bun add clsx tailwind-merge lucide-react react-dropzone date-fns

# Markdown rendering
bun add react-markdown rehype-highlight remark-gfm

# Dev dependencies
bun add -d @types/node
```

### Step 3: Project Structure Setup

Create the directory structure from [01-architecture.md](./01-architecture.md):

```bash
mkdir -p src/{app,components,lib,hooks,stores,providers,types}
mkdir -p src/app/{(auth),(dashboard)}
mkdir -p src/app/(auth)/{login,select-tenant}
mkdir -p src/app/(dashboard)/{graph,documents,query,api-explorer,settings}
mkdir -p src/components/{ui,layout,graph,documents,query,shared}
mkdir -p src/lib/{api,utils,graph}
```

### Step 4: Configure Next.js

Create `next.config.ts`:

```typescript
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Enable React 19 features
  experimental: {
    reactCompiler: true,
    ppr: "incremental",
  },

  // API proxy to EdgeQuake backend
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${process.env.EDGEQUAKE_API_URL}/api/:path*`,
      },
    ];
  },

  // Environment variables
  env: {
    EDGEQUAKE_API_URL: process.env.EDGEQUAKE_API_URL || "http://localhost:3000",
  },
};

export default nextConfig;
```

### Step 5: Configure Tailwind CSS 4

Create `tailwind.config.ts`:

```typescript
import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["class"],
  content: ["./src/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        // ... more theme colors
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
```

### Step 6: Configure TypeScript

Update `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
```

---

## Configuration Migration

### Environment Variables

**LightRAG** ([.env.example](../lightrag_webui/env.local.sample)):

```env
VITE_API_BASE_URL=http://localhost:9621
VITE_ENABLE_LOGIN=false
```

**EdgeQuake**:

```env
# API Configuration
EDGEQUAKE_API_URL=http://localhost:3000

# Feature Flags
NEXT_PUBLIC_ENABLE_AUTH=true
NEXT_PUBLIC_ENABLE_MULTI_TENANT=true

# Optional: Analytics
NEXT_PUBLIC_POSTHOG_KEY=
```

### Routing Configuration

**LightRAG** ([AppRouter.tsx](../lightrag_webui/src/AppRouter.tsx)):

```tsx
<HashRouter>
  <Routes>
    <Route path="/login" element={<LoginPage />} />
    <Route path="/" element={<App />} />
  </Routes>
</HashRouter>
```

**EdgeQuake** (file-based):

```
src/app/
├── (auth)/
│   ├── login/page.tsx
│   └── select-tenant/page.tsx
├── (dashboard)/
│   ├── layout.tsx
│   ├── page.tsx           # Redirect to /graph
│   ├── graph/page.tsx
│   ├── documents/page.tsx
│   ├── query/page.tsx
│   ├── api-explorer/page.tsx
│   └── settings/page.tsx
├── layout.tsx
└── error.tsx
```

### i18n Configuration

**LightRAG** ([i18n.ts](../lightrag_webui/src/i18n.ts)):

```typescript
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: { en: {...}, zh: {...} },
    fallbackLng: 'en',
  });
```

**EdgeQuake** (`src/i18n/config.ts`):

```typescript
import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async ({ locale }) => ({
  messages: (await import(`./messages/${locale}.json`)).default,
}));
```

### Theme Configuration

**LightRAG** ([ThemeProvider.tsx](../lightrag_webui/src/components/ThemeProvider.tsx)):

```tsx
// Custom implementation with localStorage

export default function ThemeProvider({ children }) {
  const [theme, setTheme] = useState(() => {
    return localStorage.getItem("theme") || "system";
  });
  // ...
}
```

**EdgeQuake** (`src/providers/theme-provider.tsx`):

```tsx
"use client";

import { ThemeProvider as NextThemesProvider } from "next-themes";
import type { ThemeProviderProps } from "next-themes";

export function ThemeProvider({ children, ...props }: ThemeProviderProps) {
  return <NextThemesProvider {...props}>{children}</NextThemesProvider>;
}
```

---

## Breaking Changes

### 1. No `window` in Server Components

**Problem**: Graph components access `window` directly.

**Solution**: Use dynamic import with `ssr: false`:

```tsx
// app/(dashboard)/graph/page.tsx
import dynamic from "next/dynamic";

const GraphViewer = dynamic(() => import("@/components/graph/graph-viewer"), {
  ssr: false,
  loading: () => <GraphSkeleton />,
});

export default function GraphPage() {
  return <GraphViewer />;
}
```

### 2. useRouter Changes

**Problem**: React Router `useNavigate` → Next.js `useRouter`.

**LightRAG**:

```tsx
import { useNavigate } from "react-router-dom";
const navigate = useNavigate();
navigate("/login");
```

**EdgeQuake**:

```tsx
import { useRouter } from "next/navigation";
const router = useRouter();
router.push("/login");
```

### 3. Query Parameters

**LightRAG**:

```tsx
import { useSearchParams } from "react-router-dom";
const [searchParams] = useSearchParams();
const query = searchParams.get("q");
```

**EdgeQuake**:

```tsx
import { useSearchParams } from "next/navigation";
const searchParams = useSearchParams();
const query = searchParams.get("q");
```

### 4. Client Components

**Problem**: Hooks can only be used in client components.

**Solution**: Add `'use client'` directive:

```tsx
"use client";

import { useState, useEffect } from "react";
import { useSettingsStore } from "@/stores/use-settings-store";

export function SettingsPanel() {
  const theme = useSettingsStore((s) => s.theme);
  // ...
}
```

### 5. Metadata API

**LightRAG** (index.html):

```html
<title>LightRAG WebUI</title> <meta name="description" content="..." />
```

**EdgeQuake** (layout.tsx):

```tsx
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "EdgeQuake",
  description: "Knowledge Graph RAG Platform",
};
```

### 6. Static Assets

**LightRAG**: `public/` accessed via `/`
**EdgeQuake**: Same, but use `next/image` for images:

```tsx
import Image from "next/image";

<Image src="/logo.svg" alt="EdgeQuake" width={120} height={40} />;
```

---

## Testing Migration

### Test Framework Change

**LightRAG**: Vitest
**EdgeQuake**: Keep Vitest (or use Jest with Next.js)

### Configure Vitest for Next.js

Create `vitest.config.ts`:

```typescript
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },
});
```

### E2E Testing with Playwright

Install Playwright:

```bash
bun add -d @playwright/test
bunx playwright install
```

Create `playwright.config.ts`:

```typescript
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  webServer: {
    command: "bun run dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
  },
  use: {
    baseURL: "http://localhost:3000",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "mobile", use: { ...devices["Pixel 5"] } },
  ],
});
```

---

## Deployment Migration

### Docker Configuration

Create `Dockerfile`:

```dockerfile
FROM oven/bun:1 AS base

# Install dependencies
FROM base AS deps
WORKDIR /app
COPY package.json bun.lockb ./
RUN bun install --frozen-lockfile

# Build
FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
ENV NEXT_TELEMETRY_DISABLED=1
RUN bun run build

# Production
FROM base AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

COPY --from=builder /app/public ./public
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static

EXPOSE 3000
ENV PORT=3000

CMD ["bun", "run", "server.js"]
```

### Next.js Output Configuration

Update `next.config.ts`:

```typescript
const nextConfig: NextConfig = {
  output: "standalone", // For Docker deployment
  // ...
};
```

---

## Related Documents

- **Previous**: [04-ui-ux-improvements.md](./04-ui-ux-improvements.md) - UX enhancements
- **Implementation Start**: [01-architecture.md](./01-architecture.md) - Reference architecture
- **API Guide**: [02-api-integration.md](./02-api-integration.md) - API integration
