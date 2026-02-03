#!/usr/bin/env python3
"""Investigate PDF font encodings for Apple-Sandbox-Guide."""

import sys

from pypdf import PdfReader

pdf_path = (
    sys.argv[1]
    if len(sys.argv) > 1
    else "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/Apple-Sandbox-Guide-v1.0.pdf"
)
reader = PdfReader(pdf_path)

# Check multiple pages for fonts with /Differences
for page_num in range(min(5, len(reader.pages))):
    page = reader.pages[page_num]
    resources = page.get("/Resources", {})
    fonts = resources.get("/Font", {})

    print(f"\n=== Page {page_num + 1} Fonts ===")
    for font_name, font_ref in fonts.items():
        font = font_ref.get_object()
        enc = font.get("/Encoding")
        base_font = font.get("/BaseFont")

        enc_info = "None"
        if enc:
            if hasattr(enc, "get_object"):
                enc_obj = enc.get_object()
                if hasattr(enc_obj, "keys"):
                    if "/Differences" in enc_obj:
                        diffs = enc_obj["/Differences"]
                        enc_info = f"Dict with /Differences ({len(diffs)} entries)"
                    else:
                        enc_info = f"Dict: {list(enc_obj.keys())}"
                else:
                    enc_info = str(enc_obj)
            else:
                enc_info = str(enc)

        # Check for FontDescriptor and FontFile2
        ff2_info = ""
        font_desc = font.get("/FontDescriptor")
        if font_desc:
            fd = font_desc.get_object()
            ff2 = fd.get("/FontFile2")
            if ff2:
                ff2_obj = ff2.get_object()
                if hasattr(ff2_obj, "__len__"):
                    ff2_info = f" [FontFile2: {len(ff2_obj._data)} bytes]"
                else:
                    ff2_info = " [FontFile2: YES]"

        print(f"  {font_name}: {base_font} - Encoding: {enc_info}{ff2_info}")
