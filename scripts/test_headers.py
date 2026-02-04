#!/usr/bin/env python3
"""Test pattern detection for headers."""


def is_roman_numeral_header(text):
    # Must have at least 3 chars: 'I. X'
    if len(text) < 4:
        return False

    chars = list(text)
    i = 0

    # Collect Roman numeral characters (I, V, X)
    has_roman = False
    while i < len(chars) and chars[i] in "IVX":
        has_roman = True
        i += 1

    if not has_roman:
        return False

    # Must be followed by '.' and space
    if i + 2 > len(chars):
        return False
    if chars[i] != "." or chars[i + 1] != " ":
        return False

    # Rest should be mostly uppercase (section title)
    rest = "".join(chars[i + 2 :])
    uppercase_count = sum(1 for c in rest if c.isupper())
    alpha_count = sum(1 for c in rest if c.isalpha())

    # At least 50% uppercase indicates a section title
    return alpha_count > 0 and (uppercase_count / alpha_count) >= 0.5


def is_letter_subsection_header(text):
    if len(text) < 4:
        return False
    chars = list(text)
    first, second, third = chars[0], chars[1], chars[2]
    if first.isupper() and second == "." and third == " ":
        if len(chars) >= 4:
            return chars[3].isupper()
    return False


# Test cases
tests = [
    "I. INTRODUCTION",
    "II. RELATED WORKS",
    "III. MODULAR TELEOPERATION",
    "IV. CHOICE POLICY",
    "V. EXPERIMENTS",
    "VI. CONCLUSION",
    "A. Humanoid Manipulation",
    "B. Policy Representations",
    "A. Background",
]

print("Testing header detection patterns:")
print("-" * 60)
for t in tests:
    roman = is_roman_numeral_header(t)
    letter = is_letter_subsection_header(t)
    print(f"{t[:35]:35} Roman={roman:5}  Letter={letter}")
