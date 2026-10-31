import { describe, expect, it } from 'vitest';
import { capabilitiesForDraft, GRAPH_CAPABILITIES } from '@/constants/rag-profile';
import {
  destinationForChord,
  destinationGroupsFor,
  destinationsFor,
} from '../destinations';

const normal = capabilitiesForDraft('normal', 'text');
const hrefs = (list: readonly { href: string }[]) => list.map((d) => d.href);

describe('destinations by retrieval type', () => {
  it('a graph workspace, or one not loaded yet, keeps every destination', () => {
    expect(hrefs(destinationsFor(GRAPH_CAPABILITIES))).toContain('/graph');
    expect(hrefs(destinationsFor(undefined))).toContain('/knowledge');
  });

  it('a normal workspace offers neither the graph nor the knowledge page', () => {
    const offered = hrefs(destinationsFor(normal));
    expect(offered).not.toContain('/graph');
    expect(offered).not.toContain('/knowledge');
    expect(offered).toEqual(expect.arrayContaining(['/documents', '/query', '/workspace']));
  });

  it('removes the entry, it does not disable it', () => {
    const items = destinationGroupsFor(normal).flatMap((g) => g.items);
    expect(hrefs(items)).not.toContain('/graph');
  });

  it('keeps every group that still has an item', () => {
    expect(destinationGroupsFor(normal).map((g) => g.id)).toEqual(['primary', 'knowledge', 'system']);
  });

  it('the G then G chord does nothing on a normal workspace', () => {
    expect(destinationForChord('g', normal)).toBeUndefined();
    expect(destinationForChord('g', GRAPH_CAPABILITIES)?.href).toBe('/graph');
    expect(destinationForChord('d', normal)?.href).toBe('/documents');
  });
});
