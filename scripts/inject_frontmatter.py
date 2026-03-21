#!/usr/bin/env python3
"""Inject title frontmatter into docs markdown files that lack it."""
import os
import re

docs_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "docs")
count = 0

for root, dirs, files in os.walk(docs_dir):
    for f in files:
        if not f.endswith(".md"):
            continue
        path = os.path.join(root, f)
        with open(path, "r") as fh:
            content = fh.read()

        # Check if it already has frontmatter with title
        if content.startswith("---"):
            fm_end = content.find("---", 3)
            if fm_end != -1:
                fm = content[3:fm_end]
                if "title:" in fm or "title :" in fm:
                    continue  # Already has title
                # Has frontmatter but no title
                rest = content[fm_end + 3 :].strip()
                h1_match = re.search(r"^#\s+(.+)", rest, re.MULTILINE)
                if h1_match:
                    title = h1_match.group(1).strip()
                else:
                    title = (
                        os.path.splitext(f)[0]
                        .replace("-", " ")
                        .replace("_", " ")
                        .title()
                    )
                # Inject title into existing frontmatter
                new_fm = (
                    "---\ntitle: "
                    + repr(title)
                    + "\n"
                    + fm.strip()
                    + "\n---"
                )
                new_content = new_fm + content[fm_end + 3 :]
                with open(path, "w") as fh:
                    fh.write(new_content)
                count += 1
                print(f"ADDED_TITLE: {path} -> {title}")
                continue

        # No frontmatter at all
        h1_match = re.search(r"^#\s+(.+)", content, re.MULTILINE)
        if h1_match:
            title = h1_match.group(1).strip()
        else:
            title = (
                os.path.splitext(f)[0]
                .replace("-", " ")
                .replace("_", " ")
                .title()
            )

        new_content = "---\ntitle: " + repr(title) + "\n---\n\n" + content
        with open(path, "w") as fh:
            fh.write(new_content)
        count += 1
        print(f"INJECTED: {path} -> {title}")

print(f"\nTotal files modified: {count}")
