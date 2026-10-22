"""Proxy body framing for Databricks Apps uploads."""

import io
import json
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


_SHIM_BEHAVIOR = r"""
const fs = require("fs");
const posts = [];
function mockResponse(status, text) {
  return {
    status: status,
    statusText: status === 202 ? "Accepted" : "OK",
    headers: { get: function () { return null; } },
    text: function () { return Promise.resolve(text); },
  };
}
function record(url, init) {
  const headers = init.headers;
  const role = headers.get("x-edgequake-upload-role");
  let size = 0;
  let text = "";
  const body = init.body;
  if (typeof body === "string") {
    size = Buffer.byteLength(body);
    text = body;
  } else if (body instanceof Uint8Array) {
    size = body.byteLength;
    text = Buffer.from(body).toString("utf8");
  } else if (body && typeof body.size === "number") {
    size = body.size;
  }
  posts.push({
    role: role,
    size: size,
    text: text,
    credentials: init.credentials,
    auth: headers.get("x-edgequake-authorization"),
  });
  if (!role) return mockResponse(200, JSON.stringify({ ok: true, native: true }));
  if (role === "bytes") {
    const offset = Number(headers.get("x-edgequake-upload-offset"));
    const total = Number(headers.get("x-edgequake-upload-total"));
    if (offset + size >= total) {
      return mockResponse(202, JSON.stringify({ document_id: "doc-1", status: "processing" }));
    }
  }
  return mockResponse(202, JSON.stringify({ message: "chunk accepted", status: 202 }));
}
global.window = global;
global.fetch = function (url, init) { return Promise.resolve(record(url, init)); };
function XMLHttpRequest() {}
XMLHttpRequest.prototype.open = function () {};
XMLHttpRequest.prototype.setRequestHeader = function () {};
XMLHttpRequest.prototype.send = function () {};
global.XMLHttpRequest = XMLHttpRequest;
function FileLike(size) {
  this.size = size;
  this.name = "gcc.pdf";
  this.type = "application/pdf";
}
FileLike.prototype.slice = function (start, end) { return new Uint8Array(end - start); };
function FormDataMock() { this.parts = []; }
FormDataMock.prototype.append = function (key, value) { this.parts.push([key, value]); };
FormDataMock.prototype.forEach = function (cb) {
  this.parts.forEach(function (pair) { cb(pair[1], pair[0]); });
};
global.FormData = FormDataMock;
eval(fs.readFileSync(process.argv[1], "utf8"));
function summarize(from) {
  const slice = posts.slice(from);
  let max = 0;
  let fields = 0;
  let bytes = 0;
  const chunks = [];
  let mode = null;
  let total = null;
  let type = null;
  let filename = null;
  let credentials = null;
  let auth = null;
  slice.forEach(function (post) {
    if (post.size > max) max = post.size;
    if (post.role === "fields") {
      fields += 1;
      const meta = JSON.parse(post.text);
      mode = Object.prototype.hasOwnProperty.call(meta, "mode") ? meta.mode : null;
      total = meta.total;
      type = meta.content_type;
      filename = meta.filename || null;
    }
    if (post.role === "bytes") {
      bytes += 1;
      chunks.push(Buffer.from(post.text, "utf8"));
    }
    credentials = post.credentials;
    auth = post.auth;
  });
  return {
    max: max,
    fields: fields,
    bytes: bytes,
    joined: Buffer.concat(chunks).toString("utf8"),
    mode: mode,
    total: total,
    type: type,
    filename: filename,
    credentials: credentials,
    auth: auth,
  };
}
(async function () {
  const exact = "a".repeat(8 * 1024);
  const exactResponse = await window.fetch("/api/v1/documents", {
    method: "POST",
    headers: { Authorization: "Bearer t", "Content-Type": "application/json" },
    credentials: "include",
    body: exact,
  });
  const exactBody = JSON.parse(await exactResponse.text());
  const payload = JSON.stringify({ content: "m".repeat(9000), source_type: "markdown" });
  const jsonStart = posts.length;
  const jsonResponse = await window.fetch("/api/v1/documents", {
    method: "POST",
    headers: { Authorization: "Bearer t", "Content-Type": "application/json" },
    credentials: "include",
    body: payload,
  });
  const jsonBody = JSON.parse(await jsonResponse.text());
  const json = summarize(jsonStart);
  const formStart = posts.length;
  const xhr = new XMLHttpRequest();
  xhr.open("POST", "/api/v1/documents/pdf");
  xhr.setRequestHeader("Authorization", "Bearer t");
  const form = new FormDataMock();
  form.append("title", "gcc");
  form.append("file", new FileLike(20000));
  const formResult = await new Promise(function (resolve, reject) {
    xhr.onload = function () { resolve({ status: xhr.status, text: xhr.responseText }); };
    xhr.onerror = function () { reject(new Error(xhr.__eqErr || "xhr error")); };
    xhr.send(form);
  });
  const formInfo = summarize(formStart);
  process.stdout.write(JSON.stringify({
    exact_posts: jsonStart,
    exact_size: posts[0].size,
    exact_sliced: Boolean(posts[0].role),
    exact_native: exactBody.native === true,
    json_mode: json.mode,
    json_total: json.total,
    json_payload: Buffer.byteLength(payload),
    json_type: json.type,
    json_max: json.max,
    json_joined: json.joined === payload,
    json_document: jsonBody.document_id,
    json_credentials: json.credentials,
    json_auth: json.auth,
    json_fields: json.fields,
    json_slices: json.bytes,
    form_max: formInfo.max,
    form_slices: formInfo.bytes,
    form_filename: formInfo.filename,
    form_status: formResult.status,
    form_text: formResult.text,
  }));
})().catch(function (error) {
  console.error(error && error.stack ? error.stack : String(error));
  process.exit(1);
});
"""


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
        self.assertIn(b"8 * 1024", script)
        self.assertIn(b'"raw"', script)
        self.assertNotIn(b"48 * 1024", script)
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

    def test_raw_slices_reassemble_json(self) -> None:
        reset_uploads_for_tests()
        payload = b'{"content":"' + (b"m" * 100) + b'","source_type":"markdown"}'
        upload_id = "rawJSON0123456789"
        started = accept_upload_chunk(
            {
                "X-EdgeQuake-Upload-Id": upload_id,
                "X-EdgeQuake-Upload-Role": "fields",
            },
            (
                b'{"mode":"raw","content_type":"application/json","total":%d}'
                % len(payload)
            ),
        )
        self.assertFalse(started["done"])
        offset = 0
        step = 8
        done: dict[str, object] | None = None
        while offset < len(payload):
            piece = payload[offset : offset + step]
            outcome = accept_upload_chunk(
                {
                    "X-EdgeQuake-Upload-Id": upload_id,
                    "X-EdgeQuake-Upload-Role": "bytes",
                    "X-EdgeQuake-Upload-Offset": str(offset),
                    "X-EdgeQuake-Upload-Total": str(len(payload)),
                },
                piece,
            )
            offset += len(piece)
            done = outcome
            if offset < len(payload):
                self.assertFalse(outcome["done"])
        assert done is not None
        self.assertTrue(done["done"])
        self.assertEqual(done["body"], payload)
        self.assertEqual(done["content_type"], "application/json")
        self.assertNotIn(b"multipart/form-data", str(done["content_type"]).encode())
        reset_uploads_for_tests()

    def test_unknown_upload_mode_is_rejected(self) -> None:
        reset_uploads_for_tests()
        with self.assertRaises(BodyReadError):
            accept_upload_chunk(
                {
                    "X-EdgeQuake-Upload-Id": "abcDEF0123456789",
                    "X-EdgeQuake-Upload-Role": "fields",
                },
                b'{"mode":"stream","total":1}',
            )
        reset_uploads_for_tests()

    def test_fetch_shim_slices_json_and_form_under_8kib(self) -> None:
        script_path = Path(__file__).with_name("fetch_shim.js")
        node = subprocess.run(
            ["node", "-e", _SHIM_BEHAVIOR, str(script_path)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(node.returncode, 0, node.stderr)
        report = json.loads(node.stdout)
        self.assertEqual(report["exact_posts"], 1)
        self.assertEqual(report["exact_size"], 8 * 1024)
        self.assertFalse(report["exact_sliced"])
        self.assertEqual(report["json_mode"], "raw")
        self.assertEqual(report["json_total"], report["json_payload"])
        self.assertEqual(report["json_type"], "application/json")
        self.assertLessEqual(report["json_max"], 8 * 1024)
        self.assertEqual(report["json_joined"], True)
        self.assertEqual(report["json_document"], "doc-1")
        self.assertEqual(report["json_credentials"], "include")
        self.assertEqual(report["json_auth"], "Bearer t")
        self.assertEqual(report["json_fields"], 1)
        self.assertGreater(report["json_slices"], 1)
        self.assertEqual(report["form_max"], 8 * 1024)
        self.assertGreater(report["form_slices"], 1)
        self.assertEqual(report["form_filename"], "gcc.pdf")
        self.assertEqual(report["form_status"], 202)
        self.assertIn("doc-1", report["form_text"])

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
