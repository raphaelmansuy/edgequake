# LinkedIn Post: Production Deployment

---

Your RAG demo "works perfectly."

Your production deployment? That's a different story.

I've watched this pattern play out repeatedly:

→ ML team builds incredible RAG prototype
→ Demo impresses leadership
→ SRE team inherits it for production
→ 3 months later, they're still building:

• Health endpoints (Kubernetes needs these)
• Connection pooling (or enjoy 3am pages)
• Graceful shutdown (data corruption is fun)
• Runbooks (for when things break)
• Multi-stage Docker (security matters)

The ML team built a great system.
The ops team had to make it production-ready.

This is backwards.

---

When we built EdgeQuake, we started from production requirements:

✅ Three health endpoints: /health, /ready, /live
→ Kubernetes knows if your app is alive

✅ Built-in connection pooling
→ No more PostgreSQL connection exhaustion at scale

✅ Multi-stage Docker build
→ ~100MB image, non-root user, locked dependencies

✅ Stateless API design
→ Scale with replica count, no session affinity

✅ 316-line runbook included
→ Alert thresholds, backup procedures, incident response

✅ Graceful shutdown handlers
→ SIGTERM drains connections, no mid-transaction kills

---

The difference:

Before: "Demo works, production takes 3 months"
After: "docker-compose up and we're production-ready"

---

RAG frameworks optimize for notebooks.
EdgeQuake optimizes for production.

Your SRE team will thank you.

---

🔗 Open source: github.com/raphaelmansuy/edgequake

#RAG #MLOps #DevOps #Kubernetes #Production #GraphRAG #Rust
