#!/usr/bin/env python3
"""Pick the image-to-image probes by measurement, not by hope.

Embeds every corpus picture and a set of candidate probes once with the real Vertex
`gemini-embedding-2` endpoint, caches the vectors, then prints the ranking each probe
produces. A probe is usable for the article only if its top-1 is defensible to a human
looking at the two pictures.

    python3 pick_probes.py            # measure and print
    python3 pick_probes.py --refresh  # ignore the cache
"""
from __future__ import annotations

import argparse
import base64
import json
import math
import subprocess
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
IMAGES = ROOT / "images"
CACHE = HERE / "cache" / "vectors.json"
PROJECT = "mobioos-506015"
URL = (
    "https://aiplatform.googleapis.com/v1/projects/"
    f"{PROJECT}/locations/global/publishers/google/models/gemini-embedding-2:embedContent"
)
DIM = 1536


def token() -> str:
    return subprocess.run(
        ["gcloud", "auth", "print-access-token"], capture_output=True, text=True, check=True
    ).stdout.strip()


def embed_image(path: Path, tok: str) -> list[float]:
    blob = base64.b64encode(path.read_bytes()).decode()
    body = {
        "content": {"parts": [{"inlineData": {"mimeType": "image/jpeg", "data": blob}}]},
        "embedContentConfig": {"outputDimensionality": DIM},
    }
    req = urllib.request.Request(
        URL,
        data=json.dumps(body).encode(),
        method="POST",
        headers={"Authorization": f"Bearer {tok}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.load(r)["embedding"]["values"]


def cos(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(y * y for y in b))
    return dot / (na * nb)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--refresh", action="store_true")
    args = ap.parse_args()

    corpus = sorted(IMAGES.glob("*.jpg"))
    extra = sorted((IMAGES / "probes").glob("*.jpg")) + sorted(Path("/tmp/probes").glob("*.jpg"))
    # A truncated download (a 404 body saved as .jpg) fails at the vendor with a 400.
    probes = [p for p in extra if p.stat().st_size > 8000]

    cache = {}
    if CACHE.exists() and not args.refresh:
        cache = json.loads(CACHE.read_text())
    tok = token()

    todo = [p for p in corpus + probes if str(p) not in cache]
    for i, path in enumerate(todo, 1):
        try:
            cache[str(path)] = embed_image(path, tok)
        except Exception as e:  # noqa: BLE001 - report and keep the rest
            print(f"  SKIP {path.name}: {e}", flush=True)
            continue
        print(f"  embedded {i}/{len(todo)} {path.name}", flush=True)
        CACHE.parent.mkdir(parents=True, exist_ok=True)
        CACHE.write_text(json.dumps(cache))
    CACHE.parent.mkdir(parents=True, exist_ok=True)
    CACHE.write_text(json.dumps(cache))

    names = {str(p): p.name for p in corpus}
    print()
    for probe in probes:
        if str(probe) not in cache:
            continue
        vec = cache[str(probe)]
        ranked = sorted(
            ((cos(vec, cache[str(c)]), c.name) for c in corpus if str(c) in cache), reverse=True
        )
        print(f"\n{probe.name}")
        for score, name in ranked[:5]:
            print(f"   {score:.4f}  {name}")


if __name__ == "__main__":
    main()
