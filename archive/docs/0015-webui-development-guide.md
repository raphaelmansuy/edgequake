# EdgeQuake WebUI Development Guide

> Setup, workflow, testing, and contribution limits for the Next.js frontend.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## 1. Getting Started

### 1.1 Prerequisites

-   **Node.js**: v20.x or higher (LTS recommended).
-   **Package Manager**: `pnpm` v9+ (Mandatory, lockfile is `pnpm-lock.yaml`).
-   **Backend**: Running EdgeQuake API (localhost:8080).

### 1.2 Installation

```bash
cd edgequake_webui
pnpm install
```

### 1.3 Environment Setup

Create `.env.local` based on `.env.example`:

```bash
# API Connection
NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1

# Feature Flags
NEXT_PUBLIC_ENABLE_ANALYTICS=false
NEXT_PUBLIC_DEBUG_MODE=true
```

---

## 2. Development Workflow

### 2.1 Running Locally

```bash
# Start dev server on http://localhost:3000
pnpm dev
```

### 2.2 Running with Backend (Recommended)

Use the root Makefile to start the full stack:

```bash
# In project root
make dev
```
This ensures the Rust backend, Postgres database, and Next.js frontend are synchronized.

### 2.3 Type Checking & Linting

We enforce strict TypeScript and ESLint rules.

```bash
# Check types without building
pnpm typecheck

# Run linter
pnpm lint
```

---

## 3. Building for Production

We use a custom build script to ensure safety checks pass before emitting artifacts.

```bash
# Runs typecheck -> tests -> build
pnpm build:safe
```

The output is a standalone node server in `.next/standalone`.

### Docker Build

Refer to `Dockerfile.webui` in the root. The build is multi-stage to minimize image size.

---

## 4. Testing Strategy

### 4.1 Unit Tests (Vitest)

Used for logic-heavy utilities, hooks, and reducers.

```bash
# Run all unit tests
pnpm test

# Watch mode
pnpm test:watch
```

**Where to write tests:**
-   `src/lib/**/*.test.ts`
-   `src/hooks/**/*.test.ts`
-   `src/stores/**/*.test.ts`
-   Co-located with components: `MyComponent.test.tsx`

### 4.2 End-to-End Tests (Playwright)

Used for critical user flows (Login -> Ingest -> Query).

```bash
# Run headless
pnpm test:e2e

# Run with UI debugger
pnpm test:e2e:ui
```

**Test Artifacts**: Snapshots and videos are saved to `test-results/`.

---

## 5. Folder Structure Conventions

We use a feature-sliced architecture variant.

```
src/
├── app/                 # Next.js App Router (Routing only)
├── components/          # React Components
│   ├── ui/              # Dumb primitives (shadcn)
│   ├── shared/          # Smart reusable molecules
│   └── [feature]/       # Domain feature components
├── lib/                 # Pure functions, API clients
│   ├── api/
│   ├── utils/
│   └── websocket/
├── hooks/               # React Hooks (Composition)
├── stores/              # Zustand Store definitions
└── types/               # TypeScript interfaces
```

**Rules**:
1.  **Colocation**: If a utility is only used by one component, keep it in that component's folder (if modularized) or file.
2.  **No Cyclic Deps**: `stores` should not import `components`.
3.  **Barrel Exports**: Use `index.ts` sparingly; prefer direct imports to enable tree-shaking.

---

## 6. Styling (Tailwind CSS v4)

We use Tailwind v4. Configuration is zero-config where possible.

-   **Classes**: Use `className="p-4 flex..."`.
-   **Conditions**: Use `cn()` helper: `cn("base-class", isActive && "active-class")`.
-   **Animations**: Define in `tailwind.config.ts` or use `tw-animate-css` classes.

**Design Tokens**:
The theme implementation is based on CSS variables defined in `src/app/globals.css`.

---

## 7. Common Issues & Troubleshooting

### Hydration Mismatch
*Symptom*: "Text content does not match server-rendered HTML".
*Fix*: Ensure you check `isMounted` or `useStoreHydration` when rendering browser-specific data (like `localStorage` values) in the initial render.

### Graph Canvas Blank
*Symptom*: Sigma container is empty.
*Fix*: The container needs explicit `height`. Ensure the parent `div` has `h-full` or fixed height.

### Max Listeners Exceeded
*Symptom*: Warning in console.
*Fix*: Check `useEffect` cleanups in WebSocket subscribers. Ensure you `unsubscribe()` on unmount.
