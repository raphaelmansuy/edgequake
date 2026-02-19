#!/usr/bin/env python3
"""Test font detection patterns for the PDF fonts."""

test_fonts = [
    "CMSY10",
    "CMSY8",
    "CMSY7",
    "CMSY6",
    "CMMI10",
    "CMMI7",
    "CMMI8",
    "CMMI6",
    "CMMI5",
    "CMMIB8",
    "NimbusRomNo9L-ReguItal",
    "NimbusRomNo9L-MediItal",
    "CMBX10",
]

for font in test_fonts:
    lower = font.lower()
    is_italic = (
        "italic" in lower
        or "oblique" in lower
        or "ital" in lower
        or "sfti" in lower
        or "cmti" in lower
        or "cmmi" in lower
        or "cmsy" in lower
        or "cmmib" in lower
        or "-italic" in lower
    )

    is_bold = (
        "bold" in lower
        or "black" in lower
        or "heavy" in lower
        or "sfbx" in lower
        or "cmbx" in lower
        or "medi" in lower
        or "-bold" in lower
    )

    print(f"{font}: bold={is_bold}, italic={is_italic}")
