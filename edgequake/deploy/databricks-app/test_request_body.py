"""Proxy body framing for Databricks Apps uploads."""

import io
import unittest

from app import BodyReadError, read_request_body


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
        with self.assertRaises(BodyReadError):
            read_request_body({"Content-Length": "10"}, io.BytesIO(b"abc"))

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
