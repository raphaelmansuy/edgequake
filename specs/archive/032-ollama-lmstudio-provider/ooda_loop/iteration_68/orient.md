# Orient: Hybrid Mode Analysis

## Query Results

**Query:** "Summarize all the people and organizations mentioned"
**Mode:** hybrid

### Answer Quality

The hybrid mode produced a comprehensive summary:

**People identified:**

- Sarah Chen (researcher at EdgeQuake Labs, OpenAI, works on GraphRAG)
- Michael Wong (team member, previously at Google, worked on Neo4j)

**Organizations identified:**

- EdgeQuake Labs (research lab using LightRAG)
- OpenAI (research organization)
- Microsoft (located in Redmond)
- Google (where Michael Wong worked)

### Performance Metrics

- Embedding time: 2650ms (higher due to hybrid processing)
- Retrieval time: 116ms
- Generation time: 2195ms
- Total: 4963ms
- Sources: 29 retrieved

## Observations

1. Hybrid mode combines local + global retrieval
2. More comprehensive than local-only
3. Higher latency due to dual retrieval strategy
