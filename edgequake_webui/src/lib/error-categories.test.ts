/**
 * @module error-categories.test
 * @description Unit tests for error categorization utility
 *
 * @implements OODA-10 - Test coverage for error categorization
 */

import { describe, expect, it } from 'vitest';

import {
  categorizeError,
  getCategoryColor,
  type ErrorCategory,
} from '@/lib/error-categories';

describe('categorizeError', () => {
  describe('LLM rate limit errors', () => {
    it('detects rate limit messages', () => {
      const errors = [
        'API rate limit exceeded',
        'Rate-limit reached for requests',
        'Too many requests, please slow down',
        'Error 429: Quota exceeded',
        'TPM limit exceeded for model',
        'RPM limit hit',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
        expect(result.isTransient).toBe(true);
      }
    });
  });

  describe('LLM API/auth errors', () => {
    it('detects API key errors', () => {
      const errors = [
        'Invalid API key provided',
        'Authentication failed',
        'Unauthorized: check your credentials',
        'Invalid token for OpenAI',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
        expect(result.isTransient).toBe(false);
      }
    });

    it('detects provider errors', () => {
      const errors = [
        'OpenAI API error',
        'Ollama connection failed',
        'LLM error during extraction',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
      }
    });
  });

  describe('LLM context length errors', () => {
    it('detects context too long errors', () => {
      const errors = [
        'Context length exceeded',
        'Input too long for model',
        'Maximum tokens exceeded',
        'Token limit reached',
        'Maximum context window exceeded',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('llm');
        expect(result.isTransient).toBe(false);
      }
    });
  });

  describe('embedding errors', () => {
    it('detects embedding dimension errors', () => {
      const errors = [
        'Embedding dimension mismatch',
        'Vector dimension does not match',
        'Error in embedding process',
        'Failed to encode text',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('embedding');
        expect(result.isTransient).toBe(false);
      }
    });
  });

  describe('storage/database errors', () => {
    it('detects database errors', () => {
      const errors = [
        'Database connection failed',
        'PostgreSQL error',
        'Connection refused to database',
        'Unique constraint violation',
        'Storage error occurred',
        'Deadlock detected',
        'Transaction aborted',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('storage');
        expect(result.isTransient).toBe(true);
      }
    });
  });

  describe('pipeline/parsing errors', () => {
    it('detects parse errors', () => {
      const errors = [
        'Parse error in document',
        'Invalid format detected',
        'Chunk error during processing',
        'Failed to extract entities',
        'Malformed input',
        'Invalid content type',
        'Empty content in document',
        'No text found in PDF',
        'PDF error: corrupt file',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('pipeline');
        expect(result.isTransient).toBe(false);
      }
    });
  });

  describe('network errors', () => {
    it('detects timeout errors', () => {
      const errors = [
        'Request timeout',
        'Connection timed out',
        'Network error occurred',
        'Connection reset by peer',
        'ECONNREFUSED',
        'ETIMEDOUT',
        'Failed to fetch resource',
        'Host unreachable',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('network');
        expect(result.isTransient).toBe(true);
      }
    });
  });

  describe('unknown errors', () => {
    it('falls back to unknown for unrecognized errors', () => {
      const errors = [
        'Something went wrong',
        'Unexpected error',
        'Internal server error',
        '',
      ];

      for (const msg of errors) {
        const result = categorizeError(msg);
        expect(result.category).toBe('unknown');
      }
    });
  });

  describe('summary extraction', () => {
    it('extracts first line as summary', () => {
      const result = categorizeError('First line\nSecond line\nThird line');
      expect(result.summary).toBe('First line');
    });

    it('removes Error: prefix', () => {
      const result = categorizeError('Error: Something bad happened');
      expect(result.summary).toBe('Something bad happened');
    });

    it('truncates long summaries', () => {
      const longMsg = 'A'.repeat(200);
      const result = categorizeError(longMsg);
      expect(result.summary.length).toBeLessThanOrEqual(100);
      expect(result.summary).toContain('...');
    });

    it('provides fallback for empty message', () => {
      const result = categorizeError('');
      expect(result.summary).toBe('An error occurred');
    });
  });

  describe('suggestions', () => {
    it('provides appropriate suggestion for each category', () => {
      const categories: ErrorCategory[] = [
        'llm',
        'embedding',
        'storage',
        'pipeline',
        'network',
        'unknown',
      ];

      for (const cat of categories) {
        // Force category by using known pattern
        let msg: string;
        switch (cat) {
          case 'llm':
            msg = 'rate limit';
            break;
          case 'embedding':
            msg = 'embedding error';
            break;
          case 'storage':
            msg = 'database error';
            break;
          case 'pipeline':
            msg = 'parse error';
            break;
          case 'network':
            msg = 'timeout';
            break;
          default:
            msg = 'something happened';
        }

        const result = categorizeError(msg);
        expect(result.suggestion).toBeTruthy();
        expect(result.suggestion.length).toBeGreaterThan(10);
      }
    });
  });
});

describe('getCategoryColor', () => {
  it('returns colors for all categories', () => {
    const categories: ErrorCategory[] = [
      'llm',
      'embedding',
      'storage',
      'pipeline',
      'network',
      'unknown',
    ];

    for (const cat of categories) {
      const colors = getCategoryColor(cat);
      expect(colors).toHaveProperty('bg');
      expect(colors).toHaveProperty('text');
      expect(colors).toHaveProperty('border');
      expect(colors.bg).toContain('bg-');
      expect(colors.text).toContain('text-');
    }
  });

  it('returns different colors for each category', () => {
    const llmColors = getCategoryColor('llm');
    const storageColors = getCategoryColor('storage');
    const pipelineColors = getCategoryColor('pipeline');

    expect(llmColors.bg).not.toBe(storageColors.bg);
    expect(storageColors.bg).not.toBe(pipelineColors.bg);
  });
});
