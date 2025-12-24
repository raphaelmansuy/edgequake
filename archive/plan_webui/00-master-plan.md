# EdgeQuake WebUI - Master Plan

> Comprehensive specification for building the EdgeQuake WebUI, adapted from LightRAG WebUI with modern technology stack and enhanced UX.

**Version**: 1.0.0 | **Date**: December 2025 | **Status**: Active Development

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Project Overview](#project-overview)
3. [Document Index](#document-index)
4. [Technology Stack](#technology-stack)
5. [Migration Strategy](#migration-strategy)
6. [Timeline & Milestones](#timeline--milestones)
7. [Success Criteria](#success-criteria)

---

## Executive Summary

The EdgeQuake WebUI project aims to create a modern, production-ready web interface for the EdgeQuake RAG framework. This interface will provide:

- **Knowledge Graph Visualization**: Interactive graph exploration with Sigma.js
- **Document Management**: Upload, process, and manage documents in the knowledge base
- **RAG Query Interface**: Chat-like interface for retrieval-augmented generation
- **API Explorer**: Interactive API documentation and testing

### Key Differentiators from LightRAG WebUI

| Feature              | LightRAG WebUI    | EdgeQuake WebUI                              |
| -------------------- | ----------------- | -------------------------------------------- |
| Framework            | Vite + React 19   | **Next.js 15** with App Router               |
| Rendering            | SPA (client-side) | **SSR/SSG hybrid** with streaming            |
| State Management     | Zustand           | Zustand + **React Server Components**        |
| Styling              | Tailwind CSS 4    | Tailwind CSS 4 + **shadcn/ui v2**            |
| API Integration      | Axios             | **Server Actions** + React Query             |
| Theming              | Manual toggle     | **Native next-themes** with system detection |
| Internationalization | i18next           | **next-intl** with locale routing            |
| Build Tool           | Vite              | Next.js + Turbopack                          |

---

## Project Overview

### Vision

Create the best-in-class web interface for Graph-RAG systems, featuring:

1. **Performance Excellence**: Sub-second interactions with optimistic updates
2. **Developer Experience**: Type-safe APIs, clear architecture, comprehensive testing
3. **User Experience**: Intuitive navigation, responsive design, accessibility (WCAG 2.1 AA)
4. **Enterprise Ready**: Multi-tenant support, authentication, audit logging

### Scope

#### In Scope

- Complete WebUI functionality matching LightRAG WebUI
- Enhanced UX with modern design patterns
- Full integration with EdgeQuake Rust API
- Multi-tenant and knowledge base selection
- Real-time streaming for query responses
- Export/import capabilities

#### Out of Scope (v1)

- Mobile native applications
- Offline-first capabilities
- Custom theme builder
- Plugin/extension system

---

## Document Index

### Core Specifications

| Document              | Description                                             | Link                                                       |
| --------------------- | ------------------------------------------------------- | ---------------------------------------------------------- |
| Architecture Overview | System architecture, component hierarchy, data flow     | [01-architecture.md](./01-architecture.md)                 |
| API Integration       | EdgeQuake API mapping, type definitions, error handling | [02-api-integration.md](./02-api-integration.md)           |
| Component Mapping     | LightRAG → EdgeQuake component migration guide          | [03-component-mapping.md](./03-component-mapping.md)       |
| UI/UX Improvements    | Enhanced user experience, new features                  | [04-ui-ux-improvements.md](./04-ui-ux-improvements.md)     |
| Technology Migration  | Vite → Next.js migration steps                          | [05-technology-migration.md](./05-technology-migration.md) |
| Testing Strategy      | Unit, integration, E2E testing plan                     | [06-testing-strategy.md](./06-testing-strategy.md)         |

### Implementation Guides

| Document       | Description                                         | Link                                           |
| -------------- | --------------------------------------------------- | ---------------------------------------------- |
| Setup Guide    | Project initialization, dependencies, configuration | [10-setup-guide.md](./10-setup-guide.md)       |
| File Structure | Directory layout, naming conventions                | [11-file-structure.md](./11-file-structure.md) |
| Deployment     | Docker, standalone, Vercel deployment               | [12-deployment.md](./12-deployment.md)         |

---

## Technology Stack

### Core Framework

```yaml
Runtime: Node.js 22 LTS
Package Manager: Bun 1.1+
Framework: Next.js 15.1+
React: 19.0+ (stable)
TypeScript: 5.7+
```

### UI Libraries

```yaml
Styling: Tailwind CSS 4.0
Components: shadcn/ui (latest)
Icons: Lucide React
Graph: Sigma.js + @react-sigma/*
Charts: Recharts (optional)
```

### State & Data

```yaml
Client State: Zustand 5.0
Server State: TanStack Query v5
Forms: React Hook Form + Zod
Streaming: Server-Sent Events / fetch streaming
```

### Development Tools

```yaml
Bundler: Turbopack (Next.js built-in)
Linting: ESLint 9 flat config
Formatting: Prettier
Testing: Vitest + Playwright
CI/CD: GitHub Actions
```

---

## Migration Strategy

### Phase 1: Foundation (Week 1)

- [ ] Initialize Next.js 15 project with App Router
- [ ] Configure Tailwind CSS 4 and shadcn/ui
- [ ] Set up project structure matching new architecture
- [ ] Create base layouts (root, auth, dashboard)
- [ ] Implement theme provider with next-themes

### Phase 2: Core Features (Week 2)

- [ ] Port API client layer with proper typing
- [ ] Implement Zustand stores (settings, tenant, graph)
- [ ] Build authentication flow (login, JWT handling)
- [ ] Create shared UI components from shadcn/ui

### Phase 3: Main Features (Week 3-4)

- [ ] Knowledge Graph Viewer with Sigma.js
- [ ] Document Manager with upload/pagination
- [ ] RAG Query Interface with streaming
- [ ] API Documentation viewer

### Phase 4: Enhancement (Week 5)

- [ ] Performance optimization
- [ ] Accessibility audit
- [ ] Error boundaries and fallbacks
- [ ] Analytics integration

### Phase 5: Polish (Week 6)

- [ ] Comprehensive testing
- [ ] Documentation
- [ ] Docker deployment setup
- [ ] Final QA

---

## Timeline & Milestones

```
Week 1: Foundation
├── Day 1-2: Project setup, dependencies
├── Day 3-4: Base layout, routing
└── Day 5: Theme, basic components

Week 2: Core Systems
├── Day 1-2: API layer, types
├── Day 3-4: State management
└── Day 5: Authentication

Week 3: Graph & Documents
├── Day 1-3: Knowledge Graph Viewer
├── Day 4-5: Document Manager

Week 4: Query & Polish
├── Day 1-3: RAG Query Interface
├── Day 4-5: API Explorer

Week 5-6: Testing & Deployment
├── Testing suite
├── Performance optimization
├── Docker setup
└── Documentation
```

---

## Success Criteria

### Functional Requirements

- [ ] All LightRAG WebUI features are available
- [ ] Real-time streaming works reliably
- [ ] Graph visualization handles 10,000+ nodes
- [ ] Document upload supports 50MB+ files
- [ ] Multi-tenant switching is seamless

### Performance Requirements

- [ ] First Contentful Paint < 1.5s
- [ ] Time to Interactive < 3s
- [ ] Core Web Vitals pass
- [ ] Graph renders 1000 nodes in < 2s

### Quality Requirements

- [ ] 80%+ test coverage
- [ ] Zero critical accessibility issues
- [ ] TypeScript strict mode passes
- [ ] ESLint with zero warnings

---

## Quick Start

```bash
# Clone and navigate
cd edgequake_webui

# Install dependencies
bun install

# Set up environment
cp .env.example .env.local
# Edit .env.local with your EdgeQuake API URL

# Start development server
bun dev

# Open in browser
open http://localhost:3000
```

---

## Related Documentation

- [EdgeQuake API Reference](../docs/0003-api-reference.md)
- [EdgeQuake Architecture](../docs/0002-architecture-overview.md)
- [LightRAG WebUI Source](../lightrag_webui/) (reference implementation)

---

**Next Steps**: Begin with [01-architecture.md](./01-architecture.md) for the detailed system architecture.
