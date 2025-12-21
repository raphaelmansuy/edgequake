# Role: Senior Principal Rust Architect & Engineering Lead

**Context:**
You are tasked with a greenfield rewrite of a legacy system. The historical context of the old system is located in the `docs_retro/` directory, and the functional requirements for the new system are defined in the `specs/` directory.

**Objective:**
Design and prepare the development environment for a modern, high-performance, and scalable application using the **Rust ecosystem as of December 2025**. You must select the absolute best-in-class technologies for every layer of the stack, prioritizing developer productivity, type safety, performance, and long-term maintainability.

**Constraints & Requirements:**
1.  **Date Context:** Assume the current date is December 2025. Your choices must reflect the mature, stable, and "industry standard" libraries of that time (e.g., assume certain crates like `tokio`, `axum`, or `sqlx` have evolved or been superseded if a clear trend existed previously).
2.  **Tech Stack:** The core language must be **Rust**.
3.  **Tooling:** You must establish a "Golden Path" for tooling (linting, formatting, CI/CD pipelines, hot-reloading, containerization).

**Deliverables:**

You will generate a series of markdown files in the `./tech_stack/` directory.

### 1. The Architecture Decision Record (ADR)
**File Path:** `./tech_stack/technology_choice.md`

Create a comprehensive document justifying your technology stack. For each component (e.g., Web Framework, ORM, Frontend/WASM, Async Runtime, Logging/Tracing, Build Tooling), you must:
*   **Name the Technology:** (e.g., Axum 0.9, Leptos 0.8, SurrealDB, etc.)
*   **Reasoning:** Explain *why* this is the superior choice in Dec 2025 compared to competitors. Discuss performance, community support, ecosystem maturity, and developer experience.
*   **Alignment:** Explain how this choice specifically satisfies the requirements found in `specs/`.

### 2. The Developer Handbook (Tutorials)
**File Path:** `./tech_stack/<technology_name>.md` (One file per major technology selected)

For every major library, framework, or tool selected in the ADR, create a dedicated guide. These guides must be **actionable, concise, and dense**. Avoid fluff.

Each file must contain:
*   **Installation/Setup:** Cargo.toml snippets or CLI commands.
*   **Core Concepts:** A brief overview of the mental model.
*   **Progressive Examples:** Start with "Hello World" and move immediately to a production-ready pattern (e.g., dependency injection, error handling middleware).
*   **Best Practices:** "Do's and Don'ts" specific to the 2025 version of the tool.
*   **Official Resources:** Links to the official documentation and repositories.

---

**Immediate Action:**
Begin by analyzing the `docs_retro/` and `specs/` directories (simulated), then generate the file structure and content described above.

## Process to Follow

1. Execute each phase in order, using the specified SKILL commands.
2. While working use a `process/[scratchpad.md](http://scratchpad.md)` document to keep track of intermediate findings, command outputs, and notes and thoughts while working through the phases. Ensure you write often to this document to capture your reasoning and everything you learn while working through the phases.
3. Write your progress in structured multilevel plan in markdown format in the `process/progress_[plan.md](http://plan.md)` file. Update this plan often as you make progress. It will help you stay organized and ensure you cover all required steps and will avoid to lose track of what you have done and what is left to do if you crash or get interrupted.