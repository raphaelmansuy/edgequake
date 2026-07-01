/**
 * @module ApiExplorerPage
 * @description API Explorer route — OpenAPI-native, always in sync with live API.
 *
 * @implements UC0901  - Developer tests API endpoints
 * @implements FEAT0639 - Interactive API testing
 * @implements FEAT0640 - Request/response visualization
 * @implements FEAT-035 - OpenAPI-native integration (replaces hardcoded explorer)
 *
 * @enforces DRY - no hardcoded endpoint list; /api-docs/openapi.json is SSOT
 * @enforces SRP - page delegates to ApiExplorerView
 * @enforces OCP - new endpoints appear automatically (no frontend change)
 */
'use client';

import { ApiExplorerView } from '@/components/api-explorer/api-explorer-view';
import '@scalar/api-reference-react/style.css';

export default function ApiExplorerPage() {
  return (
    <div className="h-full min-h-0">
      <ApiExplorerView />
    </div>
  );
}
