"""Proxy body framing for Databricks Apps uploads."""

import io
import subprocess
import unittest
from pathlib import Path

from app import (
    FETCH_SHIM,
    BodyReadError,
    accept_upload_chunk,
    inject_fetch_shim,
    read_request_body,
    reset_uploads_for_tests,
)


def chunked(payload: bytes, size: int = 5) -> bytes:
    raw = bytearray()
    for offset in range(0, len(payload), size):
        piece = payload[offset : offset + size]
        raw.extend(f"{len(piece):x}\r\n".encode())
        raw.extend(piece)
        raw.extend(b"\r\n")
    raw.extend(b"0\r\n\r\n")
    return bytes(raw)


class ReadRequestBodyTest(unittest.TestCase):
    def test_content_length_round_trip(self) -> None:
        payload = b"small-pdf"
        body = read_request_body(
            {"Content-Length": str(len(payload))},
            io.BytesIO(payload),
        )
        self.assertEqual(body, payload)

    def test_empty_body_is_absent(self) -> None:
        self.assertIsNone(read_request_body({}, io.BytesIO(b"")))

    def test_short_content_length_is_rejected(self) -> None:
        with self.assertRaises(BodyReadError) as caught:
            read_request_body({"Content-Length": "10"}, io.BytesIO(b"abc"))
        self.assertIn("3 of 10", str(caught.exception))

    def test_fetch_shim_streams_form_data_and_injects(self) -> None:
        script_path = Path(__file__).with_name("fetch_shim.js")
        script = script_path.read_bytes()
        self.assertIn(b"X-EdgeQuake-Upload-Role", script)
        self.assertIn(b"FormData", script)
        self.assertIn(b"128 * 1024", script)
        self.assertNotIn(b"</script", script.lower())
        self.assertIn(script.strip(), FETCH_SHIM)
        page = inject_fetch_shim(b"<html><head><title>x</title></head></html>", "text/html")
        self.assertIn(b"X-EdgeQuake-Upload-Role", page)
        check = subprocess.run(
            ["node", "--check", str(script_path)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(check.returncode, 0, check.stderr)

    def test_sliced_upload_becomes_one_multipart(self) -> None:
        reset_uploads_for_tests()
        upload_id = "abcDEF0123456789"
        headers = {
            "X-EdgeQuake-Upload-Id": upload_id,
            "X-EdgeQuake-Upload-Role": "fields",
        }
        description = (
            b'{"filename":"gcc-ai-opportunities-report.pdf","field":"file",'
            b'"content_type":"application/pdf","total":8,"fields":{"title":"gcc"}}'
        )
        started = accept_upload_chunk(headers, description)
        self.assertFalse(started["done"])
        first = accept_upload_chunk(
            {
                "X-EdgeQuake-Upload-Id": upload_id,
                "X-EdgeQuake-Upload-Role": "bytes",
                "X-EdgeQuake-Upload-Offset": "0",
                "X-EdgeQuake-Upload-Total": "8",
            },
            b"%PDF-1",
        )
        self.assertFalse(first["done"])
        done = accept_upload_chunk(
            {
                "X-EdgeQuake-Upload-Id": upload_id,
                "X-EdgeQuake-Upload-Role": "bytes",
                "X-EdgeQuake-Upload-Offset": "6",
                "X-EdgeQuake-Upload-Total": "8",
            },
            b".4",
        )
        self.assertTrue(done["done"])
        body = done["body"]
        self.assertIsInstance(body, bytes)
        self.assertIn(b'name="title"', body)
        self.assertIn(b"gcc", body)
        self.assertIn(b'filename="gcc-ai-opportunities-report.pdf"', body)
        self.assertIn(b"%PDF-1.4", body)
        self.assertIn(upload_id.encode(), str(done["content_type"]).encode())
        reset_uploads_for_tests()

    def test_chunked_body_ignores_a_misleading_content_length(self) -> None:
        payload = b"hello world, this is a file part"
        wire = chunked(payload, size=7)
        body = read_request_body(
            {
                "Transfer-Encoding": "chunked",
                "Content-Length": str(len(payload)),
            },
            io.BytesIO(wire),
        )
        self.assertEqual(body, payload)

    def test_large_chunked_body_round_trip(self) -> None:
        payload = b"A" * (256 * 1024)
        body = read_request_body(
            {"Transfer-Encoding": "chunked"},
            io.BytesIO(chunked(payload, size=8192)),
        )
        self.assertEqual(body, payload)


if __name__ == "__main__":
    unittest.main()
