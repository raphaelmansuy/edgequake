# How I Explained Graph-RAG to a Skeptical Lawyer

_A conversation that changed how I think about knowledge systems_

---

Hey friends,

Last month I had coffee with a litigation partner at a top-tier law firm. Let's call her Sarah.

Sarah had heard about AI for legal research. She was skeptical. Her firm had tried "AI search" tools before, and they were, in her words, "garbage that returned everything and nothing."

I told her about Graph-RAG. She gave me the look that lawyers give when they think you're about to oversell something.

This is the conversation that followed.

---

## "So it's just better search?"

**Sarah**: "Every vendor says they have 'better search.' What's actually different?"

**Me**: "Let me give you an example. Say you're doing due diligence on an acquisition. You have 50,000 contracts to review. What's a typical query?"

**Sarah**: "Something like... find all contracts where our client could be liable for unlimited damages."

**Me**: "Right. And with your current system, you'd search for 'unlimited liability' or 'indemnification' and get..."

**Sarah**: "Two thousand results. Which a paralegal then has to manually filter for the actual risk conditions."

**Me**: "What if you could ask: 'Show me contracts where Party A has unlimited liability AND termination allows exit without cause within 30 days'?"

**Sarah**: [Pause] "That would be... actually useful."

---

## "How does it know the relationships?"

**Sarah**: "But how does it know that 'unlimited liability' is connected to 'termination without cause'? Those might be in completely different sections of the contract."

**Me**: "That's the key insight. When we process a contract, we don't just store the text. We extract entities and relationships."

I drew a quick diagram on a napkin:

```
CONTRACT_2024_001
    │
    ├──involves──▶ ACME_CORP (Party A)
    ├──involves──▶ BETA_INC (Party B)
    │
    ├──has_clause──▶ INDEMNIFICATION
    │                    └── type: UNLIMITED
    │                    └── applies_to: PARTY_A
    │
    └──has_clause──▶ TERMINATION
                         └── type: WITHOUT_CAUSE
                         └── notice_days: 30
```

**Sarah**: "So you're building a... database of relationships?"

**Me**: "A knowledge graph. Every entity becomes a node. Every relationship becomes an edge. Then we can traverse the graph to answer complex questions."

---

## "What about the other 49,999 contracts?"

**Sarah**: "But you've shown me one contract. We have 50,000. Do you do this for each one?"

**Me**: "Yes. Automatically. The LLM reads each contract, extracts entities and relationships, and adds them to the graph."

**Sarah**: "And how much does that cost?"

**Me**: "About $0.0014 per contract with the efficient model. So your 50,000 contracts would cost about $70 to process."

**Sarah**: [Long pause] "That's... less than an hour of paralegal time."

**Me**: "And the graph is permanent. Once processed, every future query is nearly instant."

---

## "But can I trust it?"

**Sarah**: "Here's my concern. If I rely on this system and it misses a contract with a material risk, I'm the one who's liable. How do I know it's not missing things?"

**Me**: "Two things. First, we do multiple extraction passes on each document. The first pass gets the obvious entities. The second pass catches the subtle ones. This is called 'gleaning.'"

**Sarah**: "Like a second review."

**Me**: "Exactly. Second, we combine graph search with vector similarity. So even if an entity is named differently—'unlimited indemnification' vs 'uncapped liability'—we catch it through semantic matching."

**Sarah**: "And I can audit what it found?"

**Me**: "Every query returns document references with line numbers. You can trace any result back to the source text."

---

## "What about confidentiality?"

**Sarah**: "Our client data is extremely sensitive. Attorney-client privilege. I can't send contracts to some cloud API."

**Me**: "You don't have to. We support running the language model locally, on your own servers. Nothing leaves your network."

**Sarah**: "And it still works?"

**Me**: "The quality is slightly lower than the cloud models, but for entity extraction—which is mostly pattern matching—it's comparable. And the cost per document drops to zero after the hardware investment."

---

## The Verdict

By the end of our coffee, Sarah was convinced enough to run a pilot.

We processed 10,000 contracts from one of her M&A matters. The results:

- **Query**: "Contracts with unlimited liability + termination without cause"
- **Old system**: 847 results (required manual review)
- **Graph-RAG**: 47 results (verified accurate)

Time saved: approximately 40 paralegal hours.

More importantly, the system found 3 contracts with concerning clause combinations that the manual review had missed. Those contracts ended up being renegotiated before closing.

---

## What I Learned

**1. Lawyers care about precision, not just recall.**

Getting 2,000 results is worse than useless. It creates work without providing value. Graph-RAG's ability to filter on relationships is the killer feature.

**2. Auditability is non-negotiable.**

Every result needs a paper trail. "The AI said so" doesn't hold up in court. Document references and source text must be traceable.

**3. Data sovereignty isn't optional.**

For legal, healthcare, and finance, on-premise deployment is a requirement, not a nice-to-have. The ability to run everything locally changes the conversation from "we can't use AI" to "when do we start?"

---

## Try It Yourself

EdgeQuake is open source under Apache 2.0:

```bash
git clone https://github.com/raphaelmansuy/edgequake
make dev
```

Upload some contracts. Ask a relationship question. See what you find.

---

## Next Week

I'm going to share a similar conversation with a hospital CMIO about clinical notes analysis. Spoiler: the HIPAA concerns are real, and the solution is surprisingly similar.

Stay tuned.

—Raphaël

---

_P.S. EdgeQuake implements the LightRAG algorithm from [arXiv:2410.05779](https://arxiv.org/abs/2410.05779). Thanks to the researchers who made this possible._

---

**Share this newsletter**: Know a lawyer who's frustrated with "AI search" tools? Forward this email.
