# We Cut Our LLM Bill by 97%. Here's the Math.

A fintech startup I know deployed GPT-4 RAG to production. First month's bill: $10,247.

They processed 50,000 documents. At ~$0.20 per document with GPT-4, the math was brutal.

**This is how most RAG projects die.**

Here's what we learned building EdgeQuake:

---

**The 33x Cost Difference**

| Model       | 10K Docs Cost |
| ----------- | ------------- |
| gpt-4o-mini | $14           |
| gpt-4o      | $467          |

Same entity extraction quality. Same 40% deduplication rate. 33x cheaper.

---

**The Formula**

```
Cost = (input_tokens / 1000) × $0.00015
     + (output_tokens / 1000) × $0.0006
```

Typical document: ~$0.0014.

---

**Five Optimization Strategies**

1. **Model selection**: gpt-4o-mini vs gpt-4o = 33x savings
2. **Embedding choice**: text-embedding-3-small = 6.5x savings
3. **Smart chunking**: Fewer calls = lower cost
4. **Caching**: Skip unchanged documents entirely
5. **Local models**: Ollama = $0 marginal cost

---

**What Most Frameworks Miss**

LangChain: No built-in cost tracking
LlamaIndex: Token counting only
GraphRAG: Nothing

EdgeQuake: Real-time cost per operation, budget alerts, historical trends.

---

**The Insight**

Cost tracking isn't about saving money.

It's about enabling production deployment.

When you know exactly what each document costs:
→ Budget accurately for new projects
→ Price products that depend on RAG
→ Scale confidently without surprise bills

---

EdgeQuake is open source (Apache 2.0).

Built-in cost observability. $0.0014 per document.

Try it: github.com/raphaelmansuy/edgequake

---

What cost optimization strategies have worked for your LLM deployments?

#RAG #LLM #CostOptimization #AI #MachineLearning #StartupLife
