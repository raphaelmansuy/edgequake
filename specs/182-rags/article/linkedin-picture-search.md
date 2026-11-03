# We added classical RAG to EdgeQuake. Here is why that is not a downgrade.

*Find a photograph by describing it. Find it again by showing a different photograph of
the same thing. No captions, no tags, no graph.*

---

EdgeQuake started as a GraphRAG system. You ingest documents, an LLM extracts entities and
relationships, and answers come from walking that graph. It is the right shape for
questions that need to connect named things: *which suppliers ship to the plant that had
the recall?*

Then people started uploading photographs.

Not figures inside a PDF — actual images. A picture of a bike they wanted to find again, a
screenshot of a dashboard, a photo of a whiteboard. And the graph had nothing to say about
any of it. There were no entities to extract from a picture of a cappuccino. The pipeline
ran, found nothing, and the user concluded the product could not read images.

They were right, in a sense. It could not. It could read the *filename*.

So we shipped a second workspace type, and this post is about the honest engineering behind
it: what we added, why a graph is the wrong tool for a photo library, and the three bugs
that only appeared once we pointed a real embedding model at real pictures.

---

## The claim, in one table

Twenty photographs. A person writes a sentence describing one of them. The system returns
that photograph.

| Query a human wrote | Top hit | Runner-up |
|---|---|---|
| a path through the woods with sunlight between the trees | forest path | desert road |
| a small wooden boat on a turquoise mountain lake | alpine lake | orange poppies |
| a cat staring straight at the camera | cat portrait | golden retriever puppy |
| slices of pizza with melted cheese and basil | pizza | cappuccino |
| two cups of coffee with heart shapes drawn in the foam | cappuccino | espresso |
| a happy hound dog panting with its tongue out | beagle | golden retriever puppy |

Twenty out of twenty, first place. Latency 2.6 to 4.7 seconds, dominated by one call to the
embedding model.

Look at the near-misses, because they are the interesting part. "Pizza" returns pizza
first, then *cappuccino* — two Italian-looking photographs. "Coffee with hearts in the
foam" returns cappuccino, then espresso. "Orange wild flowers" returns the poppies, then a
sunset over a cornfield. The model is not grouping "food" or "outdoors". It is separating
one photograph from a similar one.

And image-to-image works with no words at all. Send a photograph the corpus has never seen,
with an empty text query:

| Probe (not in the library) | Top hit | Rank 2 |
|---|---|---|
| tabby cat on a sofa | the corpus cat | golden retriever puppy |
| terrier puppy on a rug | golden retriever puppy | beagle |
| snow peaks at twilight | Milky Way | mountain peaks |
| two wood-fired pizzas | pizza | cappuccino |

The dog probe puts both dogs above the cat. A text index cannot make that distinction at
all, because both pictures have the same caption: none.

---

## Why a graph was the wrong tool here

A photo library has no named things. That is not a limitation of our extractor; it is the
nature of the data. There is no entity in a picture of a wave.

Running GraphRAG anyway costs real money. For every document, the pipeline calls a chat
model to extract entities, calls it again to glean what it missed, writes summaries, and
builds community reports. On twenty pictures that is eighty-odd chat calls producing a
graph with zero useful nodes.

So the new workspace type skips all of it:

```
Normal RAG workspace, 20 pictures:
  chat-model calls during ingest:  0
  chunks stored:                  20
  vectors stored:                 20
  wall clock, end to end:         97 s
```

Ninety-seven seconds for twenty pictures including twenty embedding round-trips to Vertex.
Zero chat calls. The ingest is: decode, chunk, embed, store.

Retrieval is equally plain. Two legs, run together:

- **dense** — cosine over the embeddings
- **BM25** — a real lexical index over the chunk text

Then reciprocal rank fusion, then an optional reranker. No keyword-extraction LLM, no graph
seed, no entity expansion.

That is classical RAG. It is cheaper, it is faster, and for a corpus of pictures it is
strictly better than what we had.

---

## The part that matters: one vector space for pixels and words

Here is the actual trick, and it is not ours — it is the embedding model's.

`gemini-embedding-2` maps text and images into the same 1536-dimensional space. So a
sentence and a photograph can be compared by cosine similarity, because they are both
just vectors there.

```
POST /api/v1/query          (JSON, text only)
{
  "query": "slices of pizza with melted cheese and basil",
  "document_filter": { "doc_kinds": ["image"] }
}
```

```
POST /api/v1/query          (multipart, picture only)
  request:     {"query": "", "document_filter": {"doc_kinds": ["image"]}}
  query_image: probe-pizza.jpg
```

Same route. Same filter. The second one has an empty text query, so the only signal is the
image vector. The image is never stored as a document — it is held in memory for the
search.

The workspace tells the client what it can do, so nothing has to guess from a profile name:

```json
"rag_profile_capabilities": {
  "extracts_graph": false,
  "embeds_images": true,
  "writes_bm25": true,
  "queries_graph": false,
  "shows_graph_ui": false
}
```

And it fails closed. Ask a text-only workspace to search by picture and you get a 422 with
a sentence that names the fix:

```json
{
  "code": "image_query_not_supported",
  "message": "this workspace embeds text only; search by image needs 'text and images'"
}
```

Not a silent fallback to text, which would return an arbitrary photograph and look like a
working feature.

---

## Four bugs that only real pictures found

This is the part I would want to read if I were evaluating the feature.

### 1. The "Matching images" grid was invisible

We have a nice UI surface: a thumbnail grid of the pictures an answer cited. It was built
during the feature work and it worked perfectly.

In tests. Against mocked routes.

Against a real backend it never appeared, ever. The cause:

```ts
// how the client decided "is this citation a picture?"
const IMAGE_EXTENSION = /\.(png|jpe?g|webp|gif)$/i;
return IMAGE_EXTENSION.test(chunk.file_path ?? '');
```

Reasonable. Except `file_path` is filled in by the server with the document **title**, not a
path. And an image's title is its caption:

```
"Slices of margherita pizza with fresh basil"
```

No extension. So the regex answered `false` for every real photograph.

The fix was not to loosen the regex. It was to stop inferring. The server already knows the
document kind, the MIME type, and what the chunk was embedded from — it just was not saying
so. Now it does, on every citation:

```json
{
  "file_path": "Slices of margherita pizza with fresh basil",
  "doc_kind": "image",
  "mime_type": "image/jpeg",
  "source_modality": "image"
}
```

The client reads `source_modality`. No filename parsing anywhere.

**Lesson:** if a UI has to parse a string to learn a fact the database already stores, that
fact belongs in the API response.

### 2. …and then it vanished again, after a reload

The new fields fixed the live stream. One page reload brought the bug back.

The client renders citations from the *persisted* conversation context, not from the SSE
event, once a conversation is stored. That struct had been carrying `page_start` since an
earlier spec — added for exactly this reason, with a comment explaining that dropping it
"silently breaks page grouping after reload" — and my three new fields were not on it.

So the picture arrived with a live answer and disappeared from history. The same fix, for
the same reason, one layer down.

**Lesson:** a field that survives the stream must also survive the database, and the
existing comments in a codebase usually name this hazard before you hit it.

### 3. The Send button was dead for picture-only search

The composer enabled Send when a picture was attached. You clicked it, the chip stayed,
nothing happened.

The submit guard:

```ts
const text = input.trim();
if (!text) return;          // ← exits before anything can be sent
```

A picture-only search is a *supported request*. The button knew that. The guard did not.
Two pieces of code, two different opinions about whether an empty message is valid.

We pulled the rule into one tested function:

```ts
composerSubmitIntent({ text, hasSearchImage, phase })
  → "send" | "queue" | "none"
```

Now the button and the guard read the same rule, and there are unit tests pinning the case
that used to fail.

### 4. The UI was writing the answer

This is the one I am least proud of, and the one that took longest to find.

To make a picture-only turn readable in the conversation, the UI invented a sentence:

> "Find matches for this image"

and sent it as the user's message. It looked harmless. It was not.

On a normal workspace the server embeds the message as a **text** query vector and fuses it
with the image vector at equal weight. So the invented sentence was a second, competing
search. Measured on our corpus:

| What the client sent | Pizza probe returns |
|---|---|
| `"Find matches for this image"` + image | beagle, retriever, **pizza is 4th** |
| empty message + image | **pizza**, cappuccino, wave |

The label we added for cosmetics made the feature look broken. The pizza was still in the
results — it had just been outvoted by a sentence no human wrote.

The fix separates the two concerns:

```ts
{ message: "", display: "Find matches for this image" }
```

`message` goes to the server. `display` goes on screen. They are different jobs.

And then it happened a second time, in a place nobody would look. The request still ranked
the pizza 4th even with an empty message — because the UI sends `language: "en"`, and a
helper appended a response directive to *every* query:

```rust
format!("{query}\n\n[IMPORTANT: You MUST respond in {lang_name}]")
```

Applied to `""`, that manufactures a text query where the caller had deliberately supplied
none. One line, written years earlier for a good reason, silently re-weighted a search mode
it had never heard of.

**Lesson:** any text a UI or middleware synthesises is a query. If you would not let the
user type it, do not let the client send it.

---

## What `score` is not

One more honesty note, because it produced two real UI bugs.

On a normal workspace, `score` is an **RRF fusion score**:

```
score = w_dense/(60 + rank_dense) + w_bm25/(60 + rank_bm25)
```

With twenty candidates the denominator is always 61 to 80, so the only possible values are
five fixed numbers: `0.01639, 0.01613, 0.01587, 0.01562, 0.01538`.

The grid printed `score * 100`, so a **perfect match showed "2%"** and an unrelated picture
showed "1%". The citations badge averaged the same numbers and labelled a perfect match
"Mentioned (1%)". Both were real, both were meaningless: RRF encodes rank, and a percentage
of a rank is not a similarity.

The grid now shows no percentage for a rank score, and the badge reports how many distinct
sources came back instead of inventing a confidence. If you want the real numbers, they are
cosines from the vendor endpoint:

| Probe | Winner | Score | Runner-up | Score |
|---|---|---|---|---|
| pizza | pizza | 0.813 | cappuccino | 0.753 |
| mountain | milky-way | 0.833 | mountain-peaks | 0.827 |
| cat | cat | 0.769 | retriever puppy | 0.724 |
| dog | retriever puppy | 0.752 | beagle | 0.709 |

Note the mountain pair: 0.833 and 0.827. Both are snow peaks at night. Claiming one is
"the" right answer would be noise dressed as precision — so the acceptance test accepts
either.

---

## When to pick which

**Normal RAG** when the corpus is documents, pictures, or both, and the question is
"find me the passage / picture that matches this."

**Graph RAG** when the question needs to connect named things across documents — "which
suppliers ship to the plant that had the recall" — and you are willing to pay extraction
cost at ingest for it.

They are different data shapes, not tiers. A photo library is not a downgrade of a
knowledge graph; it is a different problem that a graph could never solve.

Both live behind the same API, the same Query UI, and the same citation model. The
workspace states which one it is, in a capability object, and endpoints that do not apply
say so:

```json
{ "code": "profile_has_no_graph" }   // 409, graph route on a chunk index
```

---

## Try it

```bash
export GOOGLE_CLOUD_PROJECT=<your-project>
make dev-bg
```

Then create a workspace with `rag_profile: normal` and
`embedding_modalities: "text,image"`, upload some pictures, and ask for one.

Everything in this post is reproducible from
`specs/182-rags/`: the 20-image corpus with licences, the driver script, the
request/response transcript, and the acceptance checks.

```bash
cd specs/182-rags/e2e
python3 run_demo.py       # 24/24 retrieval checks against a live server
```

If it prints a number lower than 24, we want the log.

---

*EdgeQuake v0.39.0. Twenty photographs, twenty text queries, four image probes, zero
chat-model calls at ingest. The failures above are real, were shipped, and now have tests.*
