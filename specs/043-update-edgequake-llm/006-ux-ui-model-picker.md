# SPEC-043 — UX/UI Model Picker

## Screen: Workspace Edit Configuration

```
+------------------------------------------------------------------+
|  Default Workspace                                    [Active]    |
|  Default knowledge base                                           |
|                          [Refresh]  [Cancel]  [Save]              |
+------------------------------------------------------------------+
|  LLM Configuration                                                |
|  +------------------------------------------------------------+  |
|  | Provider:  [ Mistral AI          v ]  (● online  11 models) |  |
|  | Search:    [ mistral small________________________ ] 🔍    |  |
|  | Filters:   [Vision] [Tools] [Streaming] [>100K ctx]         |  |
|  | +----------------------------------------------------------+|  |
|  | | ● Mistral Small Latest     mistral-small-latest          ||  |
|  | |   128K ctx · tools · stream                              ||  |
|  | | ○ Magistral Medium         magistral-medium-latest       ||  |
|  | +----------------------------------------------------------+|  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  Provider Status (clickable)                                      |
|  +------------------------------------------------------------+  |
|  | (●) OpenAI 15   (●) Mistral 11   (●) Ollama 23   (×) MiniMax |  |
|  |     ^ filter      ^ selected     local only      offline     |  |
|  +------------------------------------------------------------+  |
+------------------------------------------------------------------+
```

## Component tree

```
ModelPickerPanel (shared)
├── ProviderFilterBar
│   ├── ProviderChip (status dot + model count, click = filter)
│   └── ProviderSelect (dropdown fallback)
├── ModelSearchInput (debounced)
├── CapabilityFilterChips
│   └── vision | tools | thinking | embedding | streaming
└── ModelResultList
    ├── ModelResultRow (display_name, id, caps, cost hint)
    └── EmptyState / LoadingSkeleton

Consumers:
├── workspace/llm-model-selector.tsx      → wraps ModelPickerPanel
├── workspace/embedding-model-selector.tsx
├── query/provider-model-selector.tsx
└── settings/server-llm-config-card.tsx
```

## Interaction rules

| Action | Behavior |
| ------ | -------- |
| Click provider chip | Set `providerFilter`, clear search or keep |
| Type in search | Debounce 200ms → `GET /models/search?q=` |
| Toggle capability chip | AND-filter via query params |
| Select model | Emit `{ provider, model, fullId }` |
| Offline provider | Chip red; models hidden unless "show unavailable" |
| **↓ / ↑** | Move highlight in list; **loop** at ends (cmdk) |
| **Enter** | Select highlighted model; close dropdown |
| **Escape** | Close dropdown |
| **Mouse wheel** | Scroll list only (`overscroll-contain`; no page scroll) |
| Open dropdown | Focus search input automatically |

See [010-model-picker-keyboard-scroll.md](./010-model-picker-keyboard-scroll.md) for assessment + E2E proof.

## Provider Status Hub (replaces flat badges)

```
Before (badges only):
  [OpenAI (15)] [Mistral (11)] ...

After (hub):
+------------------------------------------------+
| Provider Status                    [Refresh]   |
| Filter workspace models by provider health     |
+------------------------------------------------+
| > OpenAI        ● connected    15 models  [>] |
|   └─ Requires: OPENAI_API_KEY ✓               |
| > Mistral AI    ● connected    11 models  [v] |  ← expanded
|   └─ Default: mistral-small-latest            |
| > MiniMax       × offline       4 models      |
+------------------------------------------------+
```

## ASCII: Settings server config

```
+------------------------------------------------+
| Server LLM Defaults              [Save]        |
| Persisted in database (no restart for defaults)|
+------------------------------------------------+
| LLM Provider     [ ollama           v ]          |
| LLM Model        [ gemma4:latest    v ]  🔍     |
| Embedding Prov.  [ ollama           v ]          |
| Embedding Model  [ embeddinggemma   v ]  🔍     |
| Vision Provider  [ ollama           v ]          |
| Vision Model     [ gemma4:latest    v ]  🔍     |
|                                                |
| Application Attribution                        |
| App ID    [ edgequake________________ ]        |
| App Name  [ EdgeQuake_________________ ]        |
| App URL   [ http://localhost:3000____ ]        |
+------------------------------------------------+
```

## Cross-refs

- [006-ux-ui-model-picker.md](./006-ux-ui-model-picker.md) — this doc
- [007-settings-server-config.md](./007-settings-server-config.md) — save API
- WebUI: `components/models/model-picker-panel.tsx` (new)
