/**
 * Documents inventory table column widths (SPEC-099 / table-fixed layout).
 *
 * Shared by header + body `<colgroup>` so columns stay aligned.
 * Title must claim an explicit % — an empty `<col />` collapses under
 * pressure and nowrap cells spill into Status (overlapping headers/badges).
 *
 * SPEC-146: when showAbacColumns, Class + Share claim width from Title/Created.
 *
 * Layout breakpoint (SPEC-146 tablet DoD): dense table only from `lg` (1024px).
 * Below that, inventory cards keep Class/Share/Owner readable (tablet-768).
 */
export const INVENTORY_TABLE_MIN_BREAKPOINT = 'lg' as const;

/** Tailwind classes — cards visible below lg. */
export const INVENTORY_CARDS_CLASS = 'flex flex-col gap-3 lg:hidden';

/** Tailwind classes — table chrome visible from lg up. */
export const INVENTORY_TABLE_CLASS = 'hidden lg:block';

export const DOCUMENT_TABLE_COL_PERCENTS = {
  default: {
    checkbox: '3%',
    title: '30%',
    status: '16%',
    entities: '8%',
    created: '14%',
    updated: '14%',
    actions: '15%',
  },
  withCost: {
    checkbox: '3%',
    title: '24%',
    status: '15%',
    entities: '7%',
    cost: '8%',
    created: '13%',
    updated: '13%',
    actions: '17%',
  },
  withAbac: {
    checkbox: '3%',
    title: '18%',
    status: '11%',
    class: '7%',
    share: '7%',
    owner: '8%',
    entities: '6%',
    created: '11%',
    updated: '11%',
    actions: '18%',
  },
  withCostAndAbac: {
    checkbox: '3%',
    title: '16%',
    status: '10%',
    class: '6%',
    share: '6%',
    owner: '7%',
    entities: '5%',
    cost: '6%',
    created: '10%',
    updated: '10%',
    actions: '21%',
  },
} as const;

export type DocumentTableLayoutKey = keyof typeof DOCUMENT_TABLE_COL_PERCENTS;

export function documentTableLayoutKey(
  showCostColumn: boolean,
  showAbacColumns: boolean,
): DocumentTableLayoutKey {
  if (showCostColumn && showAbacColumns) return 'withCostAndAbac';
  if (showAbacColumns) return 'withAbac';
  if (showCostColumn) return 'withCost';
  return 'default';
}

export function documentTableCols(
  showCostColumn: boolean,
  showAbacColumns = false,
) {
  return DOCUMENT_TABLE_COL_PERCENTS[
    documentTableLayoutKey(showCostColumn, showAbacColumns)
  ];
}

/** Sum of column percents — must be 100 for all layouts. */
export function documentTableColPercentSum(
  showCostColumn: boolean,
  showAbacColumns = false,
): number {
  const cols = documentTableCols(showCostColumn, showAbacColumns);
  return Object.values(cols).reduce(
    (sum, value) => sum + Number.parseFloat(value),
    0,
  );
}
