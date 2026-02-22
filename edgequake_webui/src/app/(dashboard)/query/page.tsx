/**
 * @module QueryPage
 * @description RAG query interface page route.
 *
 * @implements FEAT0007 - Natural language query processing
 * @see QueryInterface component for full implementation
 */
import { QueryInterface } from '@/components/query/query-interface';
import { Suspense } from 'react';

// WHY: QueryInterface uses useSearchParams() for conversation deep-linking.
// Next.js App Router requires useSearchParams to be wrapped in a Suspense
// boundary at the page level to prevent build-time prerender errors.
export default function QueryPage() {
  return (
    <Suspense fallback={<div className="h-full" />}>
      <QueryInterface />
    </Suspense>
  );
}
