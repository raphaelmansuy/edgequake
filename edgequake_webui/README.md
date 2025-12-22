# EdgeQuake WebUI

A modern web interface for the EdgeQuake RAG (Retrieval-Augmented Generation) framework, built with Next.js 16, React 19, and Tailwind CSS.

## Features

- **Knowledge Graph Visualization**: Interactive graph viewer with Sigma.js for exploring entities and relationships
- **Document Management**: Upload, process, and manage documents in the knowledge base
- **Query Interface**: Natural language query interface with streaming responses
- **API Explorer**: Test EdgeQuake API endpoints interactively
- **Multi-tenant Support**: Switch between tenants and workspaces
- **Dark/Light Theme**: System-aware theme with manual override
- **Responsive Design**: Works on desktop and mobile devices

## Tech Stack

- **Framework**: Next.js 16.1.0 (App Router)
- **UI Library**: React 19.2.3
- **Styling**: Tailwind CSS 4.1.18 + shadcn/ui
- **State Management**: Zustand 5.0.9
- **Data Fetching**: TanStack React Query 5.90.12
- **Graph Visualization**: Sigma.js 3.0.2 + Graphology
- **Icons**: Lucide React

## Getting Started

### Prerequisites

- Node.js 20+ or Bun 1.1+
- EdgeQuake API server running (default: http://localhost:8080)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd edgequake_webui

# Install dependencies
bun install
# or
npm install

# Copy environment file
cp .env.local.example .env.local

# Start development server
bun run dev
# or
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) to view the application.

### Environment Variables

| Variable                          | Description                      | Default   |
| --------------------------------- | -------------------------------- | --------- |
| `NEXT_PUBLIC_API_URL`             | EdgeQuake API base URL           | `/api/v1` |
| `NEXT_PUBLIC_ENABLE_DEMO_MODE`    | Enable demo mode without backend | `false`   |
| `NEXT_PUBLIC_ENABLE_API_EXPLORER` | Show API Explorer in navigation  | `true`    |

## Project Structure

```
src/
├── app/                    # Next.js App Router pages
│   ├── (auth)/            # Authentication pages
│   │   └── login/
│   └── (dashboard)/       # Main application pages
│       ├── graph/         # Knowledge graph viewer
│       ├── documents/     # Document management
│       ├── query/         # Query interface
│       ├── api-explorer/  # API testing
│       └── settings/      # User settings
├── components/
│   ├── graph/             # Graph visualization components
│   ├── documents/         # Document management components
│   ├── layout/            # Layout components (sidebar, header)
│   ├── query/             # Query interface components
│   ├── shared/            # Shared/common components
│   └── ui/                # shadcn/ui components
├── lib/
│   ├── api/               # API client and endpoints
│   └── utils.ts           # Utility functions
├── providers/             # React context providers
├── stores/                # Zustand stores
└── types/                 # TypeScript type definitions
```

## Available Scripts

- `bun run dev` - Start development server with Turbopack
- `bun run build` - Build production bundle
- `bun run start` - Start production server
- `bun run lint` - Run ESLint

## API Integration

The WebUI expects the EdgeQuake API server to be running. Configure the API URL in `.env.local`:

```env
NEXT_PUBLIC_API_URL=http://localhost:8080
```

### Supported Endpoints

- **Health**: `GET /health`
- **Auth**: `POST /auth/login`, `POST /auth/logout`
- **Documents**: `GET/POST /documents`, `DELETE /documents/:id`
- **Entities**: `GET /entities`, `GET /entities/:id`
- **Relationships**: `GET /relationships`
- **Query**: `POST /query`, `POST /query/stream`
- **Graph**: `GET /graph`
- **Pipeline**: `GET /pipeline/status`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run linting: `bun run lint`
5. Build to verify: `bun run build`
6. Submit a pull request

## License

MIT License - see [LICENSE](../LICENSE) for details.
