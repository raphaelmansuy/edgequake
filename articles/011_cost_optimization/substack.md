# Why I Obsess Over LLM Costs (And You Should Too)

_A behind-the-scenes look at building cost visibility into EdgeQuake_

---

Hey friends,

I have a confession: I check my OpenAI dashboard more often than my bank account.

There's something deeply satisfying about knowing exactly where each dollar goes. Maybe it's my engineering brain, or maybe it's trauma from that one time a prototype cost more than my car payment.

This week, I want to share why I built cost tracking into EdgeQuake from day one—and what I learned along the way.

---

## The Wake-Up Call

A few years ago, I was consulting with a fintech startup. Smart team, great product vision, first RAG deployment to production.

First week: Beautiful results. GPT-4 was extracting compliance data perfectly.

Second week: Still great. Queries were fast, users were happy.

Third week: The monthly bill arrived. **$10,247.**

They had processed about 50,000 documents. At roughly $0.20 per document, the math checked out—but nobody had checked the math beforehand.

The project was nearly killed on the spot.

I decided right then: any RAG system I build will have cost visibility baked in. Not as an afterthought. Not as a "nice to have." As a core feature.

---

## The Three Principles

When I designed EdgeQuake's cost tracking, I built it around three principles:

**1. Track everything**

Not just total cost—per-operation cost. I want to know that "entity extraction" is 60% of my spend while "embedding" is only 10%. That tells me where to focus optimization.

**2. Predict before you commit**

Before processing a batch of documents, I want to see an estimate. "This will cost approximately $14" is a lot more comfortable than "let's see what happens."

**3. Alert before it hurts**

Budget thresholds with alerts. If I set a $100 monthly budget and I'm at $80, I want to know before I'm at $150.

---

## The Implementation (For the Curious)

Here's the core calculation. It's embarrassingly simple:

```rust
pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
    let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
    let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
    input_cost + output_cost
}
```

The magic isn't in the math—it's in applying it consistently to every single LLM call.

Every extraction, every gleaning pass, every embedding request flows through a `CostTracker` that logs tokens and calculates costs in real-time.

The result is a `CostBreakdown` that looks like this:

```
job_id: "doc-abc123"
model: "gpt-4o-mini"
operations:
  extraction: {calls: 4, tokens: 8000, cost: $0.0008}
  gleaning:   {calls: 2, tokens: 2000, cost: $0.0002}
  embedding:  {calls: 1, tokens: 3000, cost: $0.0001}
total_cost_usd: $0.0011
```

Every document, every operation, every token—tracked.

---

## What I Learned

### Lesson 1: Model selection is the biggest lever

This table changed how I think about LLM costs:

| Model       | Cost per 10K Docs |
| ----------- | ----------------- |
| gpt-4o-mini | $14               |
| gpt-4o      | $467              |

That's a 33x difference.

And here's the kicker: for entity extraction (which is basically pattern matching at scale), the quality difference is negligible. Both achieve the same ~40% entity deduplication rate in our tests.

We were paying premium prices for no premium benefit.

### Lesson 2: Embeddings are sneaky

Everyone focuses on LLM costs, but embeddings add up quietly:

- text-embedding-3-small: $0.00002 per 1K tokens
- text-embedding-3-large: $0.00013 per 1K tokens

That's 6.5x more for the large model. For most RAG use cases, the smaller model works fine.

### Lesson 3: Local models change everything

With Ollama integration, the marginal cost per document becomes $0.

Yes, there's hardware cost. Yes, there's electricity. Yes, it's slower.

But for batch processing workloads—especially with data sovereignty requirements—local models are a game-changer.

---

## A Personal Note

There's something philosophically interesting about cost tracking.

In the old world of software, infrastructure costs were fixed. You bought servers, you knew what you'd pay.

In the LLM world, costs are variable. Every API call has a price. Every token matters.

This creates anxiety for some people. "What if my bill explodes?" "What if I can't predict costs?"

But I see it differently.

Variable costs mean you only pay for what you use. If your application is idle, your bill is zero. If it's busy, you're generating value.

The key is visibility. When you know exactly what each operation costs, you can make intelligent tradeoffs.

Do you want faster results? Pay for the larger model.
Do you want cheaper results? Use gpt-4o-mini.
Do you want free results? Run Ollama locally.

The choice is yours—but only if you have the data to make it.

---

## Reader Q&A

**Q: How do you handle pricing changes from OpenAI/Anthropic?**

A: The pricing table is configurable. When providers update prices (which they do often, usually downward), we update the `default_model_pricing()` function. You can also override with custom pricing for enterprise agreements.

**Q: What about costs for fine-tuned models?**

A: Same structure—just add your model with its pricing. The `ModelPricing` struct works for any model.

**Q: How accurate are the cost estimates?**

A: Very accurate for single documents. For batches, there's some variance based on document length, but estimates are typically within 10%.

---

## Try It Yourself

If any of this resonates, EdgeQuake is open source:

```bash
git clone https://github.com/raphaelmansuy/edgequake
make dev
```

Navigate to `http://localhost:3000/costs` and watch the dashboard in real-time as you process documents.

---

## Next Week

I'll be diving into production deployment patterns—how to take EdgeQuake from a local dev setup to handling real traffic.

Stay tuned, and as always, hit reply with questions. I read every email.

—Raphaël

---

_P.S. EdgeQuake implements the LightRAG algorithm from [arXiv:2410.05779](https://arxiv.org/abs/2410.05779). Big thanks to Guo et al. for the foundational research._

---

**Share this newsletter**: If you found this useful, forward it to a colleague who's wrestling with LLM costs. They'll thank you.
