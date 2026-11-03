#!/usr/bin/env python3
"""Verify the SPEC-182 probe choices against the corpus, using cached vectors.

`pick_probes.py` measures; this script asserts. It reads `cache/vectors.json` and fails
when a probe's top-1 is not the picture a human would choose. Run it after changing the
corpus or a probe so the article never quotes a stale ranking.

    python3 check_probes.py
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
IMAGES = HERE.parent / "images"
CACHE = HERE / "cache" / "vectors.json"

# The top-1 a human would accept, and why the choice is not a coin flip.
#
# `expect` is a *set*. A set is honest where two pictures are genuinely the same answer:
# the mountain probe lands on the Milky Way shot or the sea-of-clouds shot, both of which
# are snow peaks at night, and the two cosine scores differ by less than 0.01. Claiming
# one of them is "the" right answer would be noise dressed as precision.
EXPECTED = {
    "probe-cat.jpg": (
        {"08-cat-portrait.jpg"},
        "a cat photograph finds the cat, not the dogs",
    ),
    "probe-dog.jpg": (
        {"13-golden-retriever-puppy.jpg"},
        "a puppy finds the puppy, and beats the cat",
    ),
    "probe-mountain.jpg": (
        {"06-mountain-peaks.jpg", "09-milky-way.jpg"},
        "a snow peak at dusk finds a snow peak, not the beach",
    ),
    "probe-pizza.jpg": (
        {"12-pizza-margherita.jpg"},
        "a pizza finds the pizza, not the coffee",
    ),
}

# A winner that beats the runner-up by less than this is too close to quote as a ranking.
# The mountain pair is exempt: they are both correct answers, so the margin proves nothing.
MIN_MARGIN = 0.02
# Unrelated pictures score about 0.65-0.70 in this corpus; a real match clears this.
BASELINE = 0.72


def cos(a, b):
    return sum(x * y for x, y in zip(a, b)) / math.sqrt(
        sum(x * x for x in a) * sum(y * y for y in b)
    )


def main() -> int:
    if not CACHE.exists():
        print("no cache; run pick_probes.py first")
        return 1
    cache = json.loads(CACHE.read_text())
    corpus = [k for k in cache if Path(k).parent == IMAGES.resolve()]

    rows, failed = [], 0
    for probe, (accept, why) in EXPECTED.items():
        key = str((IMAGES / "probes" / probe).resolve())
        if key not in cache:
            print(f"SKIP {probe}: not embedded")
            failed += 1
            continue
        ranked = sorted(((cos(cache[key], cache[c]), Path(c).name) for c in corpus), reverse=True)
        (s1, got), (s2, second) = ranked[0], ranked[1]
        # The runner-up must be outside the accepted set for the margin to mean anything.
        margin_matters = second not in accept
        ok = got in accept and s1 >= BASELINE and (not margin_matters or (s1 - s2) >= MIN_MARGIN)
        failed += 0 if ok else 1
        rows.append(
            {
                "probe": probe,
                "accepted": sorted(accept),
                "got": got,
                "score": round(s1, 4),
                "runner_up": second,
                "runner_up_score": round(s2, 4),
                "margin": round(s1 - s2, 4),
                "margin_required": margin_matters,
                "why": why,
                "pass": ok,
            }
        )
        note = "" if margin_matters else "  [both answers valid]"
        print(
            f"{'PASS' if ok else 'FAIL'}  {probe} -> {got} ({s1:.4f}), "
            f"runner-up {second} ({s2:.4f}), margin {s1 - s2:+.4f}{note}"
        )

    (HERE.parent / "api" / "probe-matrix.json").write_text(json.dumps(rows, indent=2))
    print(f"\n{len(rows) - failed}/{len(rows)} probes behave")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
