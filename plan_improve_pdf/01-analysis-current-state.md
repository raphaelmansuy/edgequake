# Analysis of Current State (SotaBackend)

## Overview

The current `SotaBackend` in `edgequake-pdf` is a robust text extractor that handles:

- **Character Encoding:** Correctly maps WinAnsi and other encodings to Unicode.
- **Layout Analysis:** Detects single vs. two-column layouts using vertical projection profiles.
- **Reading Order:** Reorders text based on column detection (Left -> Right).
- **Block Classification:** Identifies Headers, List Items, and Body Text based on font size and patterns.
- **Artifact Removal:** Filters headers and footers based on Y-coordinates and content patterns.

## Limitations

1.  **No Table Detection:** The current implementation treats all text as prose. Tables are flattened into lines, often destroying their structure (e.g., merging adjacent cells in a row into a single line, or interleaving columns if the column detection logic misfires on the table).
2.  **No Graphical Object Extraction:** The backend currently only extracts `TextElement`s (from `TJ`, `Tj`, `"` operators). It ignores graphical operators (`m`, `l`, `re`, `S`, `f`) which are crucial for Lattice-based table detection.
3.  **Hardcoded Thresholds:** Column detection and footer filtering rely on hardcoded values (e.g., `affiliation_threshold = 80.0`), which may not generalize to all documents.

## The "One Tool Is Enough" Challenge

The goal is to achieve SOTA table extraction _without_ Machine Learning. This requires implementing advanced heuristics that can rival ML-based approaches (like LayoutLM) for standard document types.

## Key Missing Components

1.  **Graphical Operator Parsing:** Need to parse `m` (move), `l` (line), `re` (rectangle) to find table borders.
2.  **Table Detection Logic:**
    - **Lattice:** Using graphical lines to find grids.
    - **Stream:** Using text alignment to find "implied" grids.
3.  **Table Extraction:** Converting the detected grid + text into a structured format (Markdown table).
4.  **Exclusion Logic:** Ensuring text inside tables is not also extracted as body text.
