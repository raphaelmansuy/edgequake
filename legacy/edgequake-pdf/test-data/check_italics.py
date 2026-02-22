#!/usr/bin/env python3
"""Check italic markers in the generated markdown"""
import re
import sys

text = open(sys.argv[1]).read()
# Match *text* but not **text**
italics = re.findall(r"(?<!\*)\*(?!\*)[^*]+\*(?!\*)", text)
print(f"Found {len(italics)} italics")
for i in italics[:30]:
    print(f"  {repr(i[:80])}")
