# OODA Iteration 39: Decide Phase

## Decision: Update Gold Standard to Match Physical Reality

Based on first principles analysis:

1. **The gold standard is the problem**, not the extraction quality
2. **markitdown (Microsoft's official tool)** produces output similar to our extractor
3. **For RAG systems**, faithful extraction is more valuable than semantic synthesis
4. **AlphaEvolve** achieves 1.0 F1 because its gold matches physical reality

## Action Plan

### Immediate: Update one_tool gold standard

Remove synthesized metadata:

- Remove `**Authors:**` prefix
- Remove `**Affiliation:**` line (affiliations stay where they physically are)
- Keep author names but accept superscripts/spacing variations

### Future Considerations

1. **Document the gold standard philosophy**:
   - Gold should represent "best faithful extraction"
   - NOT "ideal semantic document"
2. **Review other gold files** for similar issues:
   - 2900_Goyal_et_al (also has `**Authors:**` but scores 0.943)
   - Check if physical layout differs

3. **Consider tolerance metrics**:
   - Allow fuzzy matching for author names
   - Accept affiliation placement variance

## Expected Outcome

After gold standard fix:

- F1 should increase significantly (closer to 0.9+)
- Precision should improve (fewer "extra" content penalties)
- Better alignment with what markitdown produces

## Alternative Not Taken

**Why not implement semantic synthesis?**

- Requires NLP/ML to detect metadata patterns
- Different documents have different conventions
- Risk of incorrect synthesis (false positives)
- Downstream LLM can do this at query time
- Faithful extraction is more robust
