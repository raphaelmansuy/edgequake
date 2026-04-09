# ADR-005 — UX Design System for Terminal Wizards

**Status:** Accepted  
**Date:** 2026-04-09  
**Version:** EdgeQuake v0.9.12

---

## Context

Terminal-based installers vary wildly in quality. The goals for EdgeQuake's wizard:

1. **Premium feel** — confident, informative, not chatty
2. **Predictable** — every interaction follows the same pattern
3. **Scannable** — colour and indentation guide the eye
4. **Safe** — destructive operations require explicit confirmation
5. **Portable** — works without colours, still readable

## Design Tokens

| Token      | ANSI Code  | Semantic meaning                      |
| ---------- | ---------- | ------------------------------------- |
| `C_BOLD`   | `\033[1m`  | Headings, important values            |
| `C_DIM`    | `\033[2m`  | Decorative separators, secondary text |
| `C_RESET`  | `\033[0m`  | Returns to default style              |
| `C_GREEN`  | `\033[32m` | Success, confirmation                 |
| `C_YELLOW` | `\033[33m` | Warning, advisory                     |
| `C_RED`    | `\033[31m` | Error, failure                        |
| `C_BLUE`   | `\033[34m` | Informational, progress               |
| `C_CYAN`   | `\033[36m` | Interactive elements (menu keys)      |

All tokens collapse to empty string when stdout is not a TTY.

## Component Library

### `ui_banner()`

Displayed once at the very top. Box-drawing border, product name, version.

```
  ╔══════════════════════════════════════════╗
  ║   EdgeQuake Setup Wizard    v0.9.12      ║
  ╚══════════════════════════════════════════╝
```

### `ui_section(label)`

Section divider with bold label and dim horizontal rule. Creates visual grouping.

```
  ▸ Pre-flight Checks
  ──────────────────────────────────────────────────

```

### `ui_ok(msg)` / `ui_info(msg)` / `ui_warn(msg)` / `ui_fail(msg)`

Line-level status messages. All left-padded 2 spaces + icon + 2 spaces.

```
  ✓  Docker: Docker version 26.1.4
  →  Downloading compose file…
  ⚠  OPENAI_API_KEY is not set.
  ✗  API did not become healthy within 90s.
```

### `ui_menu(prompt, opt1, opt2, ...)`

Numbered choice list. Reads from `/dev/tty`. Loops until valid input.

```
  Which LLM provider do you want to use?

    [1]  OpenAI   — cloud API (GPT-5 family), requires OPENAI_API_KEY
    [2]  Ollama   — fully local, free to run, requires Ollama daemon on port 11434

  Enter choice (1–2): _
```

Rules:
- Options indented 4 spaces. Numbers in cyan brackets.
- Prompt line flush after the list, preceded by a blank line.
- Invalid input: `ui_warn` then re-prompt (no repeat of the full menu).

### `ui_confirm(prompt, default)`

Yes/No question. Default shown in brackets (`[Y/n]` or `[y/N]`). Empty input → default.

```
  Continue without Ollama running? [y/N]:
```

### Success Banner (`print_summary`)

End-state after successful start. Bold green double-rule, all URLs listed.

```
  ══════════════════════════════════════════
  ✅  EdgeQuake is running!
  ══════════════════════════════════════════

  🌐  Web UI:    http://localhost:3000
  🔗  API:       http://localhost:8080
  📚  Swagger:   http://localhost:8080/swagger-ui
  🏥  Health:    http://localhost:8080/health

  🤖  Provider:  OpenAI
  🧠  LLM:       gpt-5-mini
  📐  Embedding: text-embedding-3-small
```

## Structural Rules

1. **Sections are the primary grouping unit.** Everything within a section is indented.
2. **Blank lines are used deliberately.** One before a menu, one after the last item,
   none between consecutive `ui_ok` / `ui_info` lines within the same step.
3. **Never use `echo`.** Always `printf` for portability.
4. **Emojis in success/summary only.** Arrow icons (`→`, `✓`, `⚠`, `✗`) in status lines.
5. **Destructive operations use ALL-CAPS confirmation string** ("DELETE"), not y/n.

## Tested Configurations

| Terminal                    | OS             | Result                              |
| --------------------------- | -------------- | ----------------------------------- |
| Terminal.app (default)      | macOS 15       | Full colour + Unicode ✓             |
| iTerm2                      | macOS 15       | Full colour + Unicode ✓             |
| GNOME Terminal              | Ubuntu 24.04   | Full colour + Unicode ✓             |
| VS Code integrated terminal | macOS / Linux  | Full colour + Unicode ✓             |
| ssh (PTY allocated)         | Linux → Linux  | Full colour + Unicode ✓             |
| Piped to file (`            | tee out.txt`)  | Any                                 | No colour, readable ASCII ✓ |
| CI (no TTY)                 | GitHub Actions | Clear error with env-var guidance ✓ |
