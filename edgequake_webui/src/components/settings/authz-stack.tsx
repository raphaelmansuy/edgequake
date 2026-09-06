'use client';

/**
 * Responsive list chrome for SPEC-146 Settings — same lg (1024) contract as
 * document inventory (`INVENTORY_*` in document-table-columns.ts).
 * Cards below lg; table from lg up.
 */

import type { ReactNode } from 'react';
import {
  INVENTORY_CARDS_CLASS,
  INVENTORY_TABLE_CLASS,
} from '@/lib/documents/document-table-columns';

export const AUTHZ_STACK_CARDS_CLASS = INVENTORY_CARDS_CLASS;
export const AUTHZ_STACK_TABLE_CLASS = INVENTORY_TABLE_CLASS;

export interface AuthzStackProps {
  /** Dense table (≥ lg). */
  table: ReactNode;
  /** Stacked cards (&lt; lg). */
  cards: ReactNode;
  /** Optional test id on the cards wrapper. */
  cardsTestId?: string;
  /** Optional test id on the table wrapper. */
  tableTestId?: string;
  className?: string;
}

export function AuthzStack({
  table,
  cards,
  cardsTestId,
  tableTestId,
  className,
}: AuthzStackProps) {
  return (
    <div className={className}>
      <div className={AUTHZ_STACK_CARDS_CLASS} data-testid={cardsTestId}>
        {cards}
      </div>
      <div
        className={`${AUTHZ_STACK_TABLE_CLASS} border rounded-md overflow-hidden`}
        data-testid={tableTestId}
      >
        {table}
      </div>
    </div>
  );
}
