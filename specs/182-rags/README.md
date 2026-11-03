# SPEC-182 — RAG for pictures: the article pack

This folder is not a feature spec. The feature shipped in
[SPEC-180](../180-normal-rag/README.md) / v0.39.0. This pack is the **evidence and the
story** for one claim:

> A chunk index that embeds pictures and text into the same vector space lets a person
> find a photograph by describing it, and find it again by showing a different photograph
> of the same thing — with no captions, no tags, no graph, and no chat-model call at
> ingest time.

Everything here exists to make that claim checkable rather than asserted.

## What is in the box

| Path | What it is |
|---|---|
| [`images/`](images/MANIFEST.md) | 20 curated photographs + 4 out-of-corpus probes, with licences |
| [`api/RESULTS.md`](api/RESULTS.md) | Every call, every ranking, every latency — 24/24 checks, plus the six defects the run found |
| [`e2e/evidence.sh`](e2e/evidence.sh) | One command: starts Postgres + backend, runs the probe and retrieval suites |
| [`e2e/capture.sh`](e2e/capture.sh) | One command: starts the stack, captures all 11 screenshots, stops it |
| [`e2e/run_demo.py`](e2e/run_demo.py) | The driver that produced the API results against a live server |
| [`e2e/pick_probes.py`](e2e/pick_probes.py) | Measures probe→corpus cosines so the probes are chosen by data |
| [`e2e/check_probes.py`](e2e/check_probes.py) | Asserts the probe margins the article quotes |
| [`screenshots/`](screenshots/README.md) | The `round-02` captures used in the article, with a manifest of what each proves |
| [`article/`](article/) | The LinkedIn piece (2,246 words) |

## Status

Both suites are green on EdgeQuake `v0.39.0` against Postgres 16 and live Vertex
`gemini-embedding-2`:

```
4/4 probes behave
24/24 retrieval checks passed in 99.5s
5/5 screenshot tests passed
```

## The three questions this pack answers

**1. Is image search actually surfaced in the API, or only in the UI?**
It is in the API. `POST /api/v1/query` accepts `multipart/form-data` with a `query_image`
file and an *empty* text query, so the image vector is the only signal. The same route
takes JSON for text-only search, and one `document_filter` narrows either to
`doc_kinds: ["image"]`. `GET /models/embedding-support` answers "can this pair embed
pictures, here?" before a workspace is created. See [RESULTS §1, §4, §5](api/RESULTS.md).

**2. Are the results real, or is the demo flattering itself?**
The corpus is built with six clusters that contain deliberate near-misses: two coffee
pictures, three dog pictures, four mountain pictures, two portraits, pizza next to
cappuccino. A retriever that only grouped "outdoors" or "food" would fail those. All 20
text queries return their intended picture at rank 1, and the runner-up is almost always
the *right kind of confusion* (pizza→cappuccino, beach→wave). The four image probes were
chosen by measuring cosines first ([`probe-matrix.json`](api/probe-matrix.json)), not by
trial and error in the UI.

**3. Why add classical RAG at all, next to GraphRAG?**
Because most corpora do not need a graph, and paying for one is not free. A Normal
workspace runs no extraction, no gleaning, no summaries and no community reports, so
ingest of 20 pictures took 97 s end-to-end including 20 vendor embedding calls, with
**zero chat-model calls**. GraphRAG's advantage is multi-hop reasoning over named things;
a photo library has no named things. Choosing the cheaper shape when the graph is empty
is the design, not a downgrade — the capability object returned at creation says which
shape you got, and a graph endpoint on a chunk index answers `409 profile_has_no_graph`
instead of pretending.

## Reproduce it

```bash
export GOOGLE_CLOUD_PROJECT=<your-project>

# evidence: 4/4 probes + 24/24 retrieval checks (starts and stops the stack)
bash specs/182-rags/e2e/evidence.sh

# screenshots: 5 tests, 11 images (starts and stops the stack)
bash specs/182-rags/e2e/capture.sh
```

Both scripts exit non-zero on any miss, so either can gate a release.

## What this pack does *not* claim

- **Not a benchmark.** 20 pictures and 24 checks is a demonstration with real numbers, not
  a recall study. There is no annotated ground truth beyond "a human looking at the two
  pictures agrees".
- **Not caption-free retrieval of everything.** The pictures here have one dominant
  subject. A picture of a busy street would need captions or a vision pass; the pipeline
  supports image-plus-text inputs, but this corpus does not exercise that path.
- **Not a graph comparison.** Nothing here measures GraphRAG quality. The claim is about
  cost and shape, not accuracy versus a graph workspace.
- **`score` is not a cosine.** It is an RRF fusion score in a narrow band. See the last
  section of [RESULTS](api/RESULTS.md) before quoting any number as similarity — and note
  that treating it as one caused two UI defects, both now fixed.
