/**
 * @module DocumentsPage
 * @description Document ingestion and management page route.
 *
 * @implements FEAT0001 - Document ingestion
 * @see DocumentManager component for full implementation
 */
import { DocumentManager } from '@/components/documents/document-manager';

export default function DocumentsPage() {
  return <DocumentManager />;
}
