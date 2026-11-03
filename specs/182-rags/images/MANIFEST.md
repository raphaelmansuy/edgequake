# Image corpus — SPEC-182

Twenty photographs, each with one dominant, nameable subject. The point is *contrast*: a
text query names a thing, and exactly one picture in the corpus is that thing.

## Why these pictures

Image embedding fails in a demo for two boring reasons. Either the corpus is
homogeneous — twenty landscapes, so every query returns a landscape and nobody is
convinced — or the subjects are abstract, so the top hit looks like luck.

So the set is built on **six clusters with internal near-misses**:

| Cluster | Pictures | The near-miss it proves |
|---|---|---|
| Animals | cat, beagle, golden-retriever puppy | "a dog" must beat "a cat"; "puppy" must land on the retriever |
| Mountains | misty highlands, alpine lake, snow peaks, Milky Way | four mountain pictures, four very different queries |
| Water & coast | tropical beach, ocean wave | "beach" vs "wave" are not the same picture |
| Food | espresso, pizza, cappuccino | "coffee" must split between two coffee pictures |
| People | man portrait, woman portrait | gender and framing are separable |
| Structure | desert road, bike, architecture, forest path, poppies, cornfield | colour and material queries |

Every picture is at least 900 px on the short side, so a UI thumbnail is sharp and a
zoom to 125% still holds. All are JPEG, progressive, `4:2:0`, stripped of EXIF, under
330 KB. That keeps a 20-document ingest inside a minute on a laptop and keeps the
screenshots free of a loading shimmer.

## Attribution

Unsplash photos are used under the [Unsplash License](https://unsplash.com/license):
free for commercial and non-commercial use, no permission needed, attribution
appreciated. Wikimedia Commons photos carry the licence named per row.

| File | Subject | Source | Licence |
|---|---|---|---|
| `01-forest-path.jpg` | Sunlit dirt path through tall forest | Unsplash `1441974231531-c6227db76b6e` | Unsplash |
| `02-misty-highlands.jpg` | Green valley, low cloud, winding road | Unsplash `1470071459604-3b5ec3a7fe05` | Unsplash |
| `03-desert-road.jpg` | Two-lane road through red rock | Unsplash `1500530855697-b586d89ba3ee` | Unsplash |
| `04-man-portrait.jpg` | Smiling man, grey sweater, studio light | Unsplash `1500648767791-00dcc994a43e` | Unsplash |
| `05-alpine-lake.jpg` | Turquoise lake, rowing boat, peaks | Unsplash `1501785888041-af3ef285b470` | Unsplash |
| `06-mountain-peaks.jpg` | Snow peaks above a sea of cloud | Unsplash `1506905925346-21bda4d32df4` | Unsplash |
| `07-tropical-beach.jpg` | White sand, turquoise surf | Unsplash `1507525428034-b723cf961d3e` | Unsplash |
| `08-cat-portrait.jpg` | Black-and-white cat on a green ground | Unsplash `1514888286974-6c03e2ca1dba` | Unsplash |
| `09-milky-way.jpg` | Milky Way over snow peaks and forest | Unsplash `1519681393784-d120267933ba` | Unsplash |
| `10-woman-portrait.jpg` | Woman, striped shirt, dusk bokeh | Unsplash `1544005313-94ddf0286df2` | Unsplash |
| `11-espresso.jpg` | Espresso cup, top-down, white ground | Wikimedia `Espresso Coffee 01.jpg` | CC0 |
| `12-pizza-margherita.jpg` | Two pizza slices, basil, white plate | Wikimedia `Margherita pizza on plate.jpg` | CC BY 4.0 |
| `13-golden-retriever-puppy.jpg` | Retriever puppy resting on a ledge | Wikimedia `Callie the golden retriever puppy.jpg` | CC BY 3.0 |
| `14-cornfield-sunset.jpg` | Sun setting over a green crop field | Wikimedia `Sunflower field at sunset.jpg` | CC0 |
| `15-vintage-bicycle.jpg` | White Peugeot bike, dark wall | Unsplash `1485965120184-e220f721d03e` | Unsplash |
| `16-orange-poppies.jpg` | Orange poppies against blue sky | Unsplash `1490750967868-88aa4486c946` | Unsplash |
| `17-ocean-wave.jpg` | Breaking wave, late light | Unsplash `1500375592092-40eb2168fd21` | Unsplash |
| `18-modern-architecture.jpg` | White geometric facade, low angle | Unsplash `1487958449943-2429e8be8625` | Unsplash |
| `19-cappuccino-latte-art.jpg` | Two cappuccinos, latte art, wood table | Unsplash `1509042239860-f550ce710b93` | Unsplash |
| `20-beagle-portrait.jpg` | Beagle, tongue out, shallow depth | Unsplash `1543466835-00a7907e9de1` | Unsplash |

## Regenerating

`normalize.sh` re-applies the exact geometry and encoding above to every file, so a
replacement image lands in the same shape as its neighbours:

```bash
cd specs/182-rags/images && ./normalize.sh
```

## Captions

`captions.json` gives each picture a one-line caption. The demo uses it for two
purposes, and they are deliberately separate:

1. **Metadata provenance** — the caption is the document *title*, which is what a
   human reads in the documents list. It is never embedded.
2. **Text-to-image proof** — the query in the demo is written by a *human*, from the
   picture alone, not copied from this file. Otherwise the demo would be measuring a
   string match, not an embedding.
