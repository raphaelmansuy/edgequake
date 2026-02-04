#!/usr/bin/env python3
"""Check how PyMuPDF extracts text spans for author names."""

import fitz

doc = fitz.open('/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.pdf')
page = doc[0]

print("=== Looking at Block 3 (author names) ===")
data = page.get_text('dict')
block3 = data['blocks'][3]  # 0-indexed, so block 3 from output
print(f"Block 3 has {len(block3.get('lines', []))} lines")

for line_idx, line in enumerate(block3.get('lines', [])):
    print(f"\nLine {line_idx}:")
    print(f"  bbox: {line.get('bbox')}")
    for span_idx, span in enumerate(line.get('spans', [])):
        text = span.get('text', '')
        origin = span.get('origin', (0,0))
        bbox = span.get('bbox', (0,0,0,0))
        print(f"  Span {span_idx}: text={text!r}")
        print(f"           origin=({origin[0]:.1f}, {origin[1]:.1f})")
        print(f"           bbox=({bbox[0]:.1f}, {bbox[1]:.1f}, {bbox[2]:.1f}, {bbox[3]:.1f})")

# Also check the actual block numbers
print("\n=== Checking which blocks contain author names ===")
for block_idx, block in enumerate(data.get('blocks', [])):
    if 'lines' in block:
        for line in block['lines']:
            for span in line.get('spans', []):
                text = span.get('text', '')
                if any(name in text for name in ['Zhening', 'Jeong', 'Chen', 'Xuelin', 'Hyeonho', 'Yulia', 'Tuanfeng', 'Joan', 'Chun-Hao']):
                    bbox = span.get('bbox', (0,0,0,0))
                    print(f"Block {block_idx}: text={text!r} bbox=({bbox[0]:.1f}, {bbox[1]:.1f}, {bbox[2]:.1f}, {bbox[3]:.1f})")

print("=== Done ===")
