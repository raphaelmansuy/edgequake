import { describe, expect, it } from 'vitest';
import {
  activeProfilePhase,
  isStageOutsideProfile,
  phaseIndexOf,
  phasesForProfile,
  runProfilePhases,
} from '../profile-phases';

const labels = (profile: 'graph' | 'normal') =>
  phasesForProfile(profile).map((p) => p.label);

describe('profile phases', () => {
  it('GraphRAG shows four phases, including extraction', () => {
    expect(labels('graph')).toEqual(['Admit', 'Prepare', 'Extract', 'Index']);
  });

  it('classical RAG has no extraction or graph phase', () => {
    expect(labels('normal')).toEqual(['Admit', 'Prepare', 'Index']);
    for (const phase of phasesForProfile('normal')) {
      expect(phase.label.toLowerCase()).not.toContain('extract');
      expect(phase.hint.toLowerCase()).not.toContain('graph');
      expect(phase.hint.toLowerCase()).not.toContain('entit');
    }
  });

  it('normal phases say what really happens: embed and store', () => {
    const index = phasesForProfile('normal').find((p) => p.id === 'index');
    expect(index?.stages).toContain('embedding');
    expect(index?.stages).toContain('storing');
    expect(index?.hint).toMatch(/embed/i);
  });

  it('a chunking run is Prepare in both profiles', () => {
    expect(activeProfilePhase('graph', 'chunking')).toBe('prepare');
    expect(activeProfilePhase('normal', 'chunking')).toBe('prepare');
  });

  it('extracting lands on Extract for graph and does not exist for normal', () => {
    expect(activeProfilePhase('graph', 'extracting')).toBe('extract');
    // A normal workspace never has that phase; it maps to the last one it does have.
    expect(phasesForProfile('normal').some((p) => p.id === 'extract')).toBe(false);
    expect(activeProfilePhase('normal', 'extracting')).toBe('index');
  });

  it('gleaning and merging stay inside Extract on a graph workspace', () => {
    const phases = phasesForProfile('graph');
    const extract = phaseIndexOf(phases, 'extracting');
    expect(phaseIndexOf(phases, 'gleaning')).toBe(extract);
    expect(phaseIndexOf(phases, 'merging')).toBe(extract);
  });

  it('embedding and storing are Index for both profiles', () => {
    expect(activeProfilePhase('graph', 'embedding')).toBe('index');
    expect(activeProfilePhase('normal', 'embedding')).toBe('index');
    expect(activeProfilePhase('normal', 'storing')).toBe('index');
  });

  it('an unknown stage falls back to the first phase, never crashes', () => {
    expect(activeProfilePhase('graph', undefined)).toBe('admit');
    expect(activeProfilePhase('normal', 'totally-new-stage')).toBe('admit');
  });

  it('reports a graph-only stage on a normal workspace', () => {
    expect(isStageOutsideProfile('normal', 'extracting')).toBe(true);
    expect(isStageOutsideProfile('normal', 'gleaning')).toBe(true);
    expect(isStageOutsideProfile('normal', 'merging')).toBe(true);
    // Shared stages are never "outside" the profile.
    expect(isStageOutsideProfile('normal', 'embedding')).toBe(false);
    expect(isStageOutsideProfile('normal', 'storing')).toBe(false);
    expect(isStageOutsideProfile('graph', 'extracting')).toBe(false);
    expect(isStageOutsideProfile('graph', 'embedding')).toBe(false);
  });

  it('resolves a run to its phases and index', () => {
    const run = runProfilePhases({ stage: 'embedding' }, 'normal');
    expect(run.phases).toHaveLength(3);
    expect(run.activeIndex).toBe(2);
    expect(run.activeId).toBe('index');
  });
});
