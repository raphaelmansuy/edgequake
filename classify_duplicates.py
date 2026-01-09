#!/usr/bin/env python3
"""
Classify frontend duplicates into Category A (cross-cutting) vs Category B (collisions).
OODA Loop 74 - Duplicate Classification
"""
import json

with open("/tmp/frontend_dupes.json") as f:
    data = json.load(f)

dupes = data.get("duplicates", {})

# Classify duplicates
category_a = []  # Cross-cutting (same feature across layers) - ACCEPT
category_b = []  # True collisions (different features, same ID) - FIX

for feat_id, entries in sorted(dupes.items()):
    files = [e["file"] for e in entries]

    # Analyze file layer distribution
    layers = set()
    for f in files:
        if "/types/" in f:
            layers.add("types")
        elif "/stores/" in f:
            layers.add("stores")
        elif "/hooks/" in f:
            layers.add("hooks")
        elif "/providers/" in f:
            layers.add("providers")
        elif "/app/" in f:
            layers.add("pages")
        elif "/components/" in f:
            layers.add("components")
        elif "/lib/" in f:
            layers.add("lib")

    # If feature spans multiple layers (types, stores, hooks, components, etc.)
    # it's a cross-cutting concern - ACCEPT as Category A
    if len(layers) >= 2:
        category_a.append((feat_id, len(entries), layers, files))
    else:
        category_b.append((feat_id, len(entries), layers, files))

print("=" * 60)
print("DUPLICATE CLASSIFICATION REPORT")
print("=" * 60)
print()
print(
    f"Category A (Cross-cutting, ACCEPT): {len(category_a)} features ({sum(c[1] for c in category_a)} annotations)"
)
print(
    f"Category B (Collisions, FIX): {len(category_b)} features ({sum(c[1] for c in category_b)} annotations)"
)
print()
print("-" * 60)
print("CATEGORY A - Cross-cutting features (same feature across layers)")
print("These are INTENTIONAL: a feature like FEAT0001 (Document Ingestion)")
print("correctly appears in types, stores, components, and API lib.")
print("-" * 60)
for feat_id, count, layers, files in category_a:
    print(f"  {feat_id}: {count}x across {sorted(layers)}")
print()
print("-" * 60)
print("CATEGORY B - Single-layer duplicates (potential collisions)")
print("These need investigation - same layer, same feature ID.")
print("-" * 60)
for feat_id, count, layers, files in category_b:
    print(f"  {feat_id}: {count}x in {sorted(layers)}")
    for f in files:
        print(f"      - {f}")
print()

# Summary
total_a = sum(c[1] for c in category_a)
total_b = sum(c[1] for c in category_b)
print("=" * 60)
print("RECOMMENDATION")
print("=" * 60)
print(f"Accept as cross-cutting: {len(category_a)} feature IDs ({total_a} annotations)")
print(f"Investigate collisions:  {len(category_b)} feature IDs ({total_b} annotations)")
print()
if len(category_b) == 0:
    print("✅ ALL DUPLICATES ARE CROSS-CUTTING (Category A)")
    print("   No true collisions found! Uniqueness score will be recalculated.")
else:
    print("⚠️  Category B duplicates need migration to unique IDs.")
