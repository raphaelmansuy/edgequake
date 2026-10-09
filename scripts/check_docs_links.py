#!/usr/bin/env python3
"""Offline docs gate: relative links resolve, frontmatter has title+description.

Usage: python3 scripts/check_docs_links.py [path ...]   (default: docs/)
Exit 1 on any failure. External (http/https/mailto) links are not fetched.
"""
import os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LINK = re.compile(r"(?<!\!)\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)|!\[[^\]]*\]\(([^)\s]+)\)")
FENCE = re.compile(r"^```.*?^```", re.S | re.M)

def md_files(paths):
    for p in paths:
        p = os.path.join(ROOT, p) if not os.path.isabs(p) else p
        if os.path.isfile(p):
            yield p
        for d, _, fs in os.walk(p):
            for f in fs:
                if f.endswith((".md", ".mdx")):
                    yield os.path.join(d, f)

def check(path):
    errs = []
    text = open(path, encoding="utf-8").read()
    rel = os.path.relpath(path, ROOT)
    fm = re.match(r"---\n(.*?)\n---\n", text, re.S)
    if not fm:
        errs.append(f"{rel}: missing frontmatter")
    else:
        for key in ("title", "description"):
            if not re.search(rf"^{key}:\s*\S", fm.group(1), re.M):
                errs.append(f"{rel}: frontmatter missing {key}")
    body = FENCE.sub("", text)
    body = re.sub(r"`[^`\n]*`", "", body)
    for m in LINK.finditer(body):
        url = (m.group(1) or m.group(2)).split("#")[0]
        if not url or url.startswith(("http://", "https://", "mailto:", "/", "<")):
            continue
        target = os.path.normpath(os.path.join(os.path.dirname(path), url))
        if not os.path.exists(target):
            errs.append(f"{rel}: broken link -> {url}")
    return errs

if __name__ == "__main__":
    targets = sys.argv[1:] or ["docs"]
    errs = [e for f in sorted(set(md_files(targets))) for e in check(f)]
    print("\n".join(errs))
    print(f"docs check: {len(errs)} problem(s)")
    sys.exit(1 if errs else 0)
