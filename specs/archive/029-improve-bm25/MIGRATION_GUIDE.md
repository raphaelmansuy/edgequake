# BM25 Migration Guide

## Overview

This guide helps users upgrade from the legacy BM25 implementation to the enhanced version in EdgeQuake.

## Migration Steps

1. **Update EdgeQuake** to the latest version (BM25 improvements included)
2. **Review API changes**:
   - New domain presets available (see API reference)
   - Phrase boosting and Unicode normalization enabled by default
   - Stopword filtering and stemming now configurable
3. **Test your queries**:
   - Run your existing queries and compare results
   - Use the `BM25_ENHANCED` env var to toggle new features if needed

## Breaking Changes
- Default tokenization is now Unicode-aware
- Phrase boosting may affect ranking for multi-word queries
- Some stopwords may be filtered by default

## New Features
- 8 domain-specific parameter presets
- HybridReranker and RRF integration
- API reference and doc examples for all public methods

## Troubleshooting
- If results differ significantly, try adjusting presets or disabling enhancements with `BM25_ENHANCED=0`
- For legacy compatibility, see the API reference for configuration options

## Support
For questions or issues, open an issue on GitHub or consult the API reference.
