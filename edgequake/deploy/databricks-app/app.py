"""Databricks App launcher for the EdgeQuake server.

Downloads the server binary from the Unity Catalog volume, writes a boot
report, and starts the server as a child. Python stays process 1 so a fast
exit is still visible: the report is stored on the volume and served on
DATABRICKS_APP_PORT. A healthy child keeps that port.
"""

import http.client
import json
import os
import re
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import urllib.request

# EdgeQuake binds here. Python owns DATABRICKS_APP_PORT so GET / is a page.
INTERNAL_PORT = 18080
UI_PORT = 3000
HOP_HEADERS = {
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
}

SAFE_VALUES = (
    "LAKEBASE_ENDPOINT",
    "PGHOST",
    "PGPORT",
    "PGDATABASE",
    "PGSSLMODE",
    "PGUSER",
    "PGAPPNAME",
    "DATABRICKS_HOST",
    "DATABRICKS_APP_PORT",
    "DATABRICKS_APP_NAME",
    "EDGEQUAKE_DB_AUTH",
    "EDGEQUAKE_LLM_PROVIDER",
    "EDGEQUAKE_LLM_MODEL",
    "EDGEQUAKE_EMBEDDING_PROVIDER",
    "EDGEQUAKE_EMBEDDING_MODEL",
    "EDGEQUAKE_EMBEDDING_DIMENSION",
    "EDGEQUAKE_LLM_OMIT_TEMPERATURE",
    "HOST",
    "PORT",
)
SECRET_LEN = (
    "JWT_SECRET",
    "DATABRICKS_CLIENT_ID",
    "DATABRICKS_CLIENT_SECRET",
    "DATABRICKS_TOKEN",
    "PGPASSWORD",
)

# Keep request bodies under the API upload cap (50 MiB) with room for framing.
MAX_PROXY_BODY = 64 * 1024 * 1024


class BodyReadError(Exception):
    """The inbound HTTP body could not be framed."""


def read_request_body(headers, rfile) -> bytes | None:
    """Read a POST body, including chunked uploads.

    The Apps proxy sometimes forwards a browser upload as chunked encoding
    and also sets Content-Length. Reading Content-Length bytes of a chunked
    stream stops inside the file part, and the API reports a multipart parse
    error. Chunked encoding wins. A declared length that ends early is a
    client error, not a short buffer forwarded upstream.
    """
    encoding = _header(headers, "Transfer-Encoding")
    if "chunked" in encoding.lower():
        return read_chunked_body(rfile)
    raw_length = _header(headers, "Content-Length") or "0"
    try:
        length = int(raw_length)
    except ValueError as exc:
        raise BodyReadError("invalid Content-Length") from exc
    if length < 0:
        raise BodyReadError("invalid Content-Length")
    if length == 0:
        return None
    if length > MAX_PROXY_BODY:
        raise BodyReadError("request body exceeds limit")
    payload = rfile.read(length)
    if len(payload) != length:
        raise BodyReadError(
            f"request body shorter than Content-Length ({len(payload)} of {length})"
        )
    return payload


_UPLOADS_LOCK = threading.Lock()
_UPLOADS: dict[str, dict[str, object]] = {}
_MAX_UPLOAD_SESSIONS = 32
_UPLOAD_TTL_SECONDS = 600


def reset_uploads_for_tests() -> None:
    with _UPLOADS_LOCK:
        for item in _UPLOADS.values():
            path = item.get("path")
            if isinstance(path, str):
                try:
                    os.remove(path)
                except OSError:
                    pass
        _UPLOADS.clear()


def _upload_token(headers, name: str) -> str:
    return _header(headers, name).strip()


def _safe_upload_id(value: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9]{16,64}", value or ""):
        raise BodyReadError("invalid upload id")
    return value


def _disposition(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"').replace("\r", "").replace("\n", "")


def _drop_upload(upload_id: str) -> None:
    item = _UPLOADS.pop(upload_id, None)
    if not item:
        return
    path = item.get("path")
    if isinstance(path, str):
        try:
            os.remove(path)
        except OSError:
            pass


def accept_upload_chunk(headers, body: bytes) -> dict[str, object]:
    """Collect browser slices that stay under the Apps gateway body limit.

    A large Content-Length upload is closed early by the gateway. A 128KiB
    body arrived as 78021 bytes, and a 100KB JSON document arrived as 16375
    of 103316. The page sends one small JSON description and then slices
    under that cutoff. When the last slice arrives, a file upload becomes
    one multipart body and a raw upload (JSON document) is the original
    bytes, both forwarded on localhost.
    """
    upload_id = _safe_upload_id(_upload_token(headers, "X-EdgeQuake-Upload-Id"))
    role = _upload_token(headers, "X-EdgeQuake-Upload-Role").lower()
    now = time.time()
    with _UPLOADS_LOCK:
        expired = [
            key
            for key, item in _UPLOADS.items()
            if now - float(item.get("created") or 0) > _UPLOAD_TTL_SECONDS
        ]
        for key in expired:
            _drop_upload(key)
        if role == "fields":
            return _accept_upload_fields(upload_id, body, now)
        if role == "bytes":
            return _accept_upload_bytes(upload_id, headers, body)
        raise BodyReadError("invalid upload role")


def _accept_upload_fields(upload_id: str, body: bytes, now: float) -> dict[str, object]:
    if upload_id in _UPLOADS:
        raise BodyReadError("upload already started")
    if len(_UPLOADS) >= _MAX_UPLOAD_SESSIONS:
        raise BodyReadError("too many uploads")
    try:
        meta = json.loads(body.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise BodyReadError("invalid upload description") from exc
    if not isinstance(meta, dict):
        raise BodyReadError("invalid upload description")
    try:
        total = int(meta.get("total"))
    except (TypeError, ValueError) as exc:
        raise BodyReadError("invalid upload length") from exc
    if total < 0 or total > MAX_PROXY_BODY:
        raise BodyReadError("request body exceeds limit")
    raw_fields = meta.get("fields") or {}
    if not isinstance(raw_fields, dict):
        raise BodyReadError("invalid upload field")
    fields = {}
    for key, value in raw_fields.items():
        if not isinstance(key, str) or not isinstance(value, str):
            raise BodyReadError("invalid upload field")
        fields[key] = value
    mode = str(meta.get("mode") or "file")
    if mode not in {"file", "raw"}:
        raise BodyReadError("invalid upload role")
    filename = str(meta.get("filename") or "upload")
    field = str(meta.get("field") or "file")
    content_type = str(meta.get("content_type") or "application/octet-stream")
    if any(char in content_type for char in "\r\n"):
        raise BodyReadError("invalid upload type")
    handle = tempfile.NamedTemporaryFile(prefix="eq-upload-", delete=False)
    path = handle.name
    handle.close()
    os.chmod(path, stat.S_IRUSR | stat.S_IWUSR)
    _UPLOADS[upload_id] = {
        "path": path,
        "received": 0,
        "total": total,
        "filename": filename,
        "field": field,
        "content_type": content_type,
        "fields": fields,
        "mode": mode,
        "created": now,
    }
    return {"done": False}


def _accept_upload_bytes(upload_id: str, headers, body: bytes) -> dict[str, object]:
    item = _UPLOADS.get(upload_id)
    if item is None:
        raise BodyReadError("unknown upload")
    try:
        offset = int(_upload_token(headers, "X-EdgeQuake-Upload-Offset"))
        total = int(_upload_token(headers, "X-EdgeQuake-Upload-Total"))
    except ValueError as exc:
        raise BodyReadError("invalid upload length") from exc
    if total != item["total"] or offset != item["received"]:
        raise BodyReadError("upload chunk out of order")
    if offset + len(body) > int(item["total"]):
        raise BodyReadError("request body exceeds limit")
    path = item["path"]
    if not isinstance(path, str):
        raise BodyReadError("unknown upload")
    with open(path, "ab") as handle:
        handle.write(body)
    item["received"] = offset + len(body)
    if item["received"] < item["total"]:
        return {"done": False}
    with open(path, "rb") as handle:
        file_bytes = handle.read()
    if item.get("mode") == "raw":
        content_type = str(item.get("content_type") or "application/json")
        _drop_upload(upload_id)
        return {"done": True, "content_type": content_type, "body": file_bytes}
    content_type, payload = _multipart_from_upload(upload_id, item, file_bytes)
    _drop_upload(upload_id)
    return {"done": True, "content_type": content_type, "body": payload}


def _multipart_from_upload(upload_id: str, item: dict[str, object], file_bytes: bytes) -> tuple[str, bytes]:
    boundary = "----eqproxy" + upload_id
    chunks: list[bytes] = []
    fields = item["fields"]
    if isinstance(fields, dict):
        for key, value in fields.items():
            chunks.append(
                (
                    f'--{boundary}\r\nContent-Disposition: form-data; name="{_disposition(str(key))}"\r\n\r\n'
                    f"{value}\r\n"
                ).encode()
            )
    field = _disposition(str(item["field"]))
    filename = _disposition(str(item["filename"]))
    content_type = str(item["content_type"])
    chunks.append(
        (
            f'--{boundary}\r\nContent-Disposition: form-data; name="{field}"; filename="{filename}"\r\n'
            f"Content-Type: {content_type}\r\n\r\n"
        ).encode()
    )
    chunks.append(file_bytes)
    chunks.append(f"\r\n--{boundary}--\r\n".encode())
    return f"multipart/form-data; boundary={boundary}", b"".join(chunks)


def read_chunked_body(rfile, limit: int = MAX_PROXY_BODY) -> bytes:
    body = bytearray()
    while True:
        line = rfile.readline(64 * 1024)
        if not line:
            raise BodyReadError("truncated chunked body")
        size_token = line.split(b";", 1)[0].strip()
        try:
            size = int(size_token, 16)
        except ValueError as exc:
            raise BodyReadError("invalid chunk size") from exc
        if size < 0:
            raise BodyReadError("invalid chunk size")
        if size == 0:
            while True:
                trailer = rfile.readline(64 * 1024)
                if trailer in (b"\r\n", b"\n", b""):
                    break
            return bytes(body)
        if len(body) + size > limit:
            raise BodyReadError("chunked body exceeds limit")
        body.extend(_read_exact(rfile, size))
        _read_chunk_delimiter(rfile)


def _read_exact(rfile, size: int) -> bytes:
    buf = bytearray()
    while len(buf) < size:
        chunk = rfile.read(size - len(buf))
        if not chunk:
            raise BodyReadError("truncated chunked body")
        buf.extend(chunk)
    return bytes(buf)


def _read_chunk_delimiter(rfile) -> None:
    first = rfile.read(1)
    if first == b"\n":
        return
    if first == b"\r" and rfile.read(1) == b"\n":
        return
    raise BodyReadError("invalid chunk delimiter")


def _header(headers, name: str) -> str:
    if headers is None:
        return ""
    getter = getattr(headers, "get", None)
    if getter is None:
        return ""
    value = getter(name)
    if value:
        return str(value)
    # email.message is case-insensitive. A plain dict is not.
    if hasattr(headers, "items"):
        for key, item in headers.items():
            if str(key).lower() == name.lower() and item:
                return str(item)
    return ""


def main() -> None:
    raw_volume = os.environ.get("EDGEQUAKE_BINARY_PATH", "")
    lines = [
        "platform " + subprocess.getoutput("uname -a"),
        subprocess.getoutput("ldd --version | head -1"),
        "volume_prefix " + raw_volume[:12],
        "volume_root " + volume_root(),
        *safe_env(),
    ]
    local = "/tmp/edgequake"
    try:
        download(local)
        lines.append(subprocess.getoutput(f"ldd {local}"))
    except Exception as exc:
        lines.append(f"download-error {type(exc).__name__}: {exc}")
        publish(lines)
        serve("\n".join(lines))
        return
    if "not found" in lines[-1]:
        publish(lines)
        serve("\n".join(lines))
        return
    os.chmod(local, os.stat(local).st_mode | stat.S_IEXEC)
    env = os.environ.copy()
    env.setdefault("RUST_LOG", "info")
    # Serving boot only verifies the ledger. It does not ALTER tables, so an
    # app role can serve a schema an admin already migrated. Migrate runs
    # only when that gate refuses (exit 78).
    code = run_server(local, env, lines, serve_on_exit=False)
    if code != 78:
        serve("\n".join(lines))
        return
    holder = start_holder("EdgeQuake is applying the Lakebase schema.\n")
    try:
        run_migrate(local, env, lines)
    finally:
        stop_holder(holder)
    run_server(local, env, lines, serve_on_exit=True)


def safe_env() -> list[str]:
    rows = [f"{key}={os.environ.get(key, '<unset>')}" for key in SAFE_VALUES]
    rows.extend(f"{key}_len={len(os.environ.get(key, ''))}" for key in SECRET_LEN)
    return rows


def redact(text: str) -> str:
    text = re.sub(r"postgres(?:ql)?://\S+", "postgresql://<redacted>", text, flags=re.I)
    text = re.sub(r"(?i)bearer\s+\S+", "Bearer <redacted>", text)
    text = re.sub(
        r"(?i)(token|secret|password|authorization)(['\"]?\s*[:=]\s*['\"]?)[^\s,'\"\\]+",
        r"\1\2<redacted>",
        text,
    )
    return text


def run_migrate(local: str, env: dict[str, str], lines: list[str]) -> int:
    proc = subprocess.Popen(
        [local, "migrate"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        env=env,
    )
    chunks: list[bytes] = []
    threading.Thread(target=lambda: collect(proc, chunks), daemon=True).start()
    deadline = time.time() + 600
    while proc.poll() is None and time.time() < deadline:
        time.sleep(2)
    if proc.poll() is None:
        proc.kill()
        proc.wait()
    code = proc.returncode if proc.returncode is not None else 1
    lines.append(f"migrate_exit={code}")
    lines.append(redact(b"".join(chunks).decode(errors="replace"))[-60_000:])
    publish(lines)
    return code


def collect(proc: subprocess.Popen[bytes], chunks: list[bytes]) -> None:
    assert proc.stdout is not None
    for chunk in iter(lambda: proc.stdout.readline(), b""):
        chunks.append(chunk)


def start_holder(text: str):
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    body = text.encode()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, fmt: str, *args: object) -> None:
            print(fmt % args, flush=True)

    port = int(os.environ.get("DATABRICKS_APP_PORT") or os.environ.get("PORT") or "8000")

    class ReuseServer(ThreadingHTTPServer):
        allow_reuse_address = True

    httpd = ReuseServer(("0.0.0.0", port), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def stop_holder(httpd: object) -> None:
    if httpd is None:
        return
    httpd.shutdown()
    httpd.server_close()


def api_path(url_path: str) -> bool:
    bare = url_path.split("?", 1)[0]
    if bare in {"/health", "/ready", "/live", "/metrics", "/api"}:
        return True
    return bare.startswith(("/api/", "/swagger-ui", "/api-docs", "/ws/", "/mcp"))


def home_path(url_path: str) -> bool:
    return url_path.split("?", 1)[0] in ("", "/")


def home_page() -> bytes:
    path = os.path.join(os.path.dirname(__file__), "index.html")
    with open(path, "rb") as handle:
        return handle.read()


def start_proxy(ui_port: int | None) -> object:
    """Serve the public port.

    API paths go to EdgeQuake. Everything else goes to the web UI when it is
    running, and `/` falls back to the small home page when it is not.
    """
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    page = home_page()

    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"
        # Unbuffered so a WebSocket frame is not stuck in the header buffer.
        rbufsize = 0

        def do_GET(self) -> None:
            self._dispatch()

        def do_POST(self) -> None:
            self._dispatch()

        def do_PUT(self) -> None:
            self._dispatch()

        def do_PATCH(self) -> None:
            self._dispatch()

        def do_DELETE(self) -> None:
            self._dispatch()

        def do_HEAD(self) -> None:
            self._dispatch()

        def _dispatch(self) -> None:
            if self.headers.get("Upgrade", "").lower() == "websocket":
                self._tunnel(INTERNAL_PORT if api_path(self.path) else (ui_port or INTERNAL_PORT))
                return
            if api_path(self.path):
                self._forward(INTERNAL_PORT)
                return
            if ui_port is not None:
                self._forward(ui_port)
                return
            if self.command in ("GET", "HEAD") and home_path(self.path):
                self._home()
                return
            self._forward(INTERNAL_PORT)

        def _tunnel(self, port: int) -> None:
            """Pass the raw request through, including the WebSocket handshake.

            http.client drops hop-by-hop Upgrade and Connection headers, so the
            API answered 400 and the browser reported the connection as lost.
            """
            import socket

            upstream = socket.create_connection(("127.0.0.1", port), timeout=30)
            upstream.settimeout(None)
            lines = [f"{self.command} {self.path} {self.request_version}"]
            cookie = self.headers.get("Cookie", "")
            token = session_token(cookie)
            protocol = self.headers.get("Sec-WebSocket-Protocol", "")
            for key, value in self.headers.items():
                if token and key.lower() == "sec-websocket-protocol":
                    continue
                lines.append(f"{key}: {value}")
            if token and "edgequake.bearer" not in protocol:
                lines.append(f"Sec-WebSocket-Protocol: edgequake.bearer, {token}")
            elif token and protocol:
                lines.append(f"Sec-WebSocket-Protocol: {protocol}")
            raw = ("\r\n".join(lines) + "\r\n\r\n").encode("iso-8859-1")
            length = int(self.headers.get("Content-Length", "0") or "0")
            if length:
                raw += self.rfile.read(length)
            upstream.sendall(raw)
            client = self.connection
            self.close_connection = True

            def pump(src: socket.socket, dst: socket.socket) -> None:
                try:
                    while True:
                        chunk = src.recv(65536)
                        if not chunk:
                            break
                        dst.sendall(chunk)
                except Exception:
                    pass
                try:
                    dst.shutdown(socket.SHUT_WR)
                except Exception:
                    pass

            backward = threading.Thread(target=pump, args=(upstream, client), daemon=True)
            backward.start()
            pump(client, upstream)
            backward.join(timeout=2)
            upstream.close()

        def _home(self) -> None:
            body = b"" if self.command == "HEAD" else page
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(page)))
            self.send_header("Connection", "close")
            self.end_headers()
            if body:
                self.wfile.write(body)

        def _forward(self, port: int) -> None:
            try:
                payload = read_request_body(self.headers, self.rfile)
                forced_type = None
                if _header(self.headers, "X-EdgeQuake-Upload-Id"):
                    outcome = accept_upload_chunk(self.headers, payload or b"")
                    if not outcome["done"]:
                        self._json(202, {"message": "chunk accepted", "status": 202})
                        return
                    payload = outcome["body"]
                    forced_type = str(outcome["content_type"])
            except BodyReadError as exc:
                # The upload client only reads a JSON `message`. A text/plain
                # body with an empty HTTP/2 status line becomes "Request failed".
                self._json(400, {"message": str(exc), "status": 400, "code": "BAD_REQUEST"})
                return
            headers = {
                key: value
                for key, value in self.headers.items()
                if key.lower() not in HOP_HEADERS
                and key.lower() not in {"content-length", "expect"}
                and not key.lower().startswith("x-edgequake-upload-")
            }
            if forced_type:
                headers["Content-Type"] = forced_type
            apply_session_authorization(headers)
            public_host = headers.get("Host") or self.headers.get("Host", "")
            headers["X-Forwarded-Proto"] = "https"
            if public_host:
                headers["X-Forwarded-Host"] = public_host
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=600)
            try:
                conn.request(self.command, self.path, body=payload, headers=headers)
                response = conn.getresponse()
                raw_type = ""
                for key, value in response.getheaders():
                    if key.lower() == "content-type":
                        raw_type = value
                body = b"" if self.command == "HEAD" else response.read()
                body = inject_fetch_shim(body, raw_type)
                self.send_response(response.status)
                for key, value in response.getheaders():
                    if key.lower() in HOP_HEADERS or key.lower() == "content-length":
                        continue
                    self.send_header(key, value)
                session_cookie = self._session_cookie(body)
                if session_cookie:
                    self.send_header("Set-Cookie", session_cookie)
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Connection", "close")
                self.end_headers()
                if body:
                    self.wfile.write(body)
            except Exception as exc:
                message = f"upstream {port} is not reachable: {exc}".encode()
                self.send_response(502)
                self.send_header("Content-Type", "text/plain")
                self.send_header("Content-Length", str(len(message)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(message)
            finally:
                conn.close()

        def _json(self, status: int, payload: dict[str, object]) -> None:
            message = json.dumps(payload).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(message)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(message)

        def _session_cookie(self, body: bytes) -> str:
            path = self.path.split("?", 1)[0]
            if path == "/api/v1/auth/logout":
                return f"{SESSION_COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0"
            # Login, refresh, and OIDC/handoff all return access_token. A fixed
            # path list missed the callback, so the cookie backup never existed
            # for that session and only fetch() kept working.
            if not path.startswith("/api/v1/auth/"):
                return ""
            try:
                import json

                token = json.loads(body).get("access_token")
            except Exception:
                return ""
            if not isinstance(token, str) or not token:
                return ""
            return f"{SESSION_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax"

        def log_message(self, fmt: str, *args: object) -> None:
            print(fmt % args, flush=True)

    port = int(os.environ.get("DATABRICKS_APP_PORT") or os.environ.get("PORT") or "8000")

    class ReuseServer(ThreadingHTTPServer):
        allow_reuse_address = True

    httpd = ReuseServer(("0.0.0.0", port), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def start_ui(lines: list[str]) -> int | None:
    """Start the Next.js UI beside the API. Return its port, or None."""
    node = shutil.which("node")
    lines.append("node " + (node or "missing"))
    if not node:
        return None
    archive = "/tmp/webui.tar.gz"
    try:
        download_volume_file("webui.tar.gz", archive)
    except Exception as exc:
        lines.append(f"ui-download {type(exc).__name__}: {exc}")
        return None
    root = "/tmp/edgequake-webui"
    shutil.rmtree(root, ignore_errors=True)
    os.makedirs(root, exist_ok=True)
    with tarfile.open(archive, "r:gz") as bundle:
        bundle.extractall(root)
    server = os.path.join(root, "server.js")
    if not os.path.exists(server):
        for dirpath, _, files in os.walk(root):
            if "server.js" in files:
                root = dirpath
                server = os.path.join(dirpath, "server.js")
                break
    if not os.path.exists(server):
        lines.append("ui-missing server.js")
        return None
    ui_env = os.environ.copy()
    ui_env["PORT"] = str(UI_PORT)
    ui_env["HOSTNAME"] = "0.0.0.0"
    ui_env["NODE_ENV"] = "production"
    ui_env["NEXT_PUBLIC_AUTH_ENABLED"] = "true"
    ui_env["NEXT_PUBLIC_DISABLE_DEMO_LOGIN"] = "true"
    ui_env["EDGEQUAKE_API_URL"] = ""
    log = open("/tmp/webui.log", "ab")
    proc = subprocess.Popen(
        [node, server],
        cwd=root,
        env=ui_env,
        stdout=log,
        stderr=subprocess.STDOUT,
    )
    deadline = time.time() + 45
    while time.time() < deadline:
        if proc.poll() is not None:
            lines.append(f"ui-exit {proc.returncode}")
            return None
        try:
            conn = http.client.HTTPConnection("127.0.0.1", UI_PORT, timeout=2)
            conn.request("GET", "/login")
            status = conn.getresponse().status
            conn.close()
            if status < 500:
                lines.append(f"ui-ready {status}")
                return UI_PORT
        except Exception:
            time.sleep(1)
    lines.append("ui-timeout")
    return UI_PORT if proc.poll() is None else None


def run_server(
    local: str,
    env: dict[str, str],
    lines: list[str],
    serve_on_exit: bool,
) -> int | None:
    """Keep Python as process 1 and retain the child's stdout.

    Returns the child exit code. A healthy server does not return.
    """
    child_env = env.copy()
    # The public port serves the home page. The API listens beside it.
    child_env["PORT"] = str(INTERNAL_PORT)
    child_env["HOST"] = "127.0.0.1"
    proc = subprocess.Popen(
        [local],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        env=child_env,
    )
    ui_port = start_ui(lines)
    proxy = start_proxy(ui_port)
    try:
        code = _wait_server(proc, lines, serve_on_exit)
    finally:
        stop_holder(proxy)
    if serve_on_exit and code is not None:
        serve("\n".join(lines))
    return code


SESSION_COOKIE = "eq_access"


def session_token(cookie_header: str) -> str:
    """EdgeQuake access token carried in a cookie the Apps proxy forwards."""
    for part in cookie_header.split(";"):
        name, _, value = part.strip().partition("=")
        if name == SESSION_COOKIE and value:
            return value
    return ""


def header_value(headers: dict[str, str], name: str) -> str:
    for key, value in headers.items():
        if key.lower() == name.lower() and value:
            return value
    return ""


def apply_session_authorization(headers: dict[str, str]) -> None:
    """Put the EdgeQuake bearer back on the internal request.

    The Apps proxy replaces Authorization. The page copies its token into
    X-EdgeQuake-Authorization, and login stores the same token in eq_access.
    """
    token = header_value(headers, "X-EdgeQuake-Authorization")
    if token.lower().startswith("bearer "):
        token = token.split(" ", 1)[1].strip()
    if not token:
        token = session_token(header_value(headers, "Cookie"))
    if token:
        headers["Authorization"] = "Bearer " + token


def load_fetch_shim() -> bytes:
    """Auth header copy plus sliced uploads.

    The Apps gateway replaces Authorization, and it closes a large
    Content-Length body before Python finishes reading it. A browser JSON
    document of 103316 bytes arrived as 16375. The shim splits FormData and
    JSON bodies into 8KiB slices and this proxy forwards one body on
    localhost. The script is injected into HTML, so the packaged web UI does
    not have to be rebuilt.
    """
    path = os.path.join(os.path.dirname(__file__), "fetch_shim.js")
    with open(path, "rb") as handle:
        script = handle.read().strip()
    if b"</script" in script.lower():
        raise RuntimeError("fetch shim must not close the script element")
    return b"<script>" + script + b"</script>"


FETCH_SHIM = load_fetch_shim()


def inject_fetch_shim(body: bytes, content_type: str) -> bytes:
    if "text/html" not in content_type.lower() or b"<head" not in body.lower():
        return body
    start = body.lower().find(b"<head")
    end = body.find(b">", start)
    if end < 0:
        return body
    return body[: end + 1] + FETCH_SHIM + body[end + 1 :]


def _wait_server(
    proc: subprocess.Popen[bytes],
    lines: list[str],
    serve_on_exit: bool,
) -> int | None:
    chunks: list[bytes] = []

    def reader() -> None:
        assert proc.stdout is not None
        for chunk in iter(lambda: proc.stdout.readline(), b""):
            chunks.append(chunk)
            if sum(len(item) for item in chunks) > 300_000:
                break

    threading.Thread(target=reader, daemon=True).start()
    stopping = {"value": False}

    def forward(signum: int, _frame: object) -> None:
        stopping["value"] = True
        proc.send_signal(signum)

    signal.signal(signal.SIGTERM, forward)
    signal.signal(signal.SIGINT, forward)

    # Publish the first snapshot once the server has had time to boot, then
    # refresh the redacted tail while it runs. A single 45s snapshot hides
    # later request failures, and a shutdown snapshot can look like the new
    # process exited.
    next_publish = time.time() + 45
    while True:
        code = proc.poll()
        now = time.time()
        if code is not None or now >= next_publish:
            snapshot = lines + server_tail(chunks, code)
            publish(snapshot)
            next_publish = now + 20
            if code is not None:
                lines[:] = snapshot
                return None if stopping["value"] else code
        time.sleep(1)


def server_tail(chunks: list[bytes], code: int | None) -> list[str]:
    text = redact(b"".join(chunks).decode(errors="replace"))
    tail = text[-80_000:]
    return ["--- server ---", tail, f"exit={code}"]


def volume_root() -> str:
    raw = os.environ.get("EDGEQUAKE_BINARY_PATH", "").strip().rstrip("/")
    if raw.startswith("/Volumes/"):
        return raw
    if raw.count(".") == 2 and "/" not in raw:
        catalog, schema, name = raw.split(".")
        return f"/Volumes/{catalog}/{schema}/{name}"
    return raw


def workspace_url() -> str:
    """Apps inject DATABRICKS_HOST with no scheme. urllib requires one."""
    raw = os.environ["DATABRICKS_HOST"].strip().rstrip("/")
    if raw.startswith("https://") or raw.startswith("http://"):
        return raw
    return "https://" + raw


def download(local: str) -> None:
    download_volume_file("edgequake", local)


def download_volume_file(name: str, local: str) -> None:
    volume = volume_root()
    if not volume:
        raise RuntimeError("EDGEQUAKE_BINARY_PATH is empty")
    request = urllib.request.Request(
        workspace_url() + "/api/2.0/fs/files" + volume + "/" + name,
        headers={"Authorization": "Bearer " + workspace_token()},
    )
    with urllib.request.urlopen(request, timeout=180) as response, open(local, "wb") as out:
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            out.write(chunk)


def publish(lines: list[str]) -> None:
    volume = volume_root()
    if not volume:
        return
    body = "\n".join(lines).encode()
    request = urllib.request.Request(
        workspace_url() + "/api/2.0/fs/files" + volume + "/boot-report.txt",
        data=body,
        method="PUT",
        headers={
            "Authorization": "Bearer " + workspace_token(),
            "Content-Type": "text/plain",
        },
    )
    try:
        urllib.request.urlopen(request, timeout=30).read()
    except Exception as exc:
        print(f"report-write {type(exc).__name__}: {exc}", flush=True)


def serve(text: str) -> None:
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            body = text.encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, fmt: str, *args) -> None:
            print(fmt % args, flush=True)

    port = int(os.environ.get("DATABRICKS_APP_PORT") or os.environ.get("PORT") or "8000")
    ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()


def workspace_token() -> str:
    import base64
    import json

    client_id = os.environ.get("DATABRICKS_CLIENT_ID", "")
    client_secret = os.environ.get("DATABRICKS_CLIENT_SECRET", "")
    host = workspace_url()
    if client_id and client_secret:
        basic = base64.b64encode(f"{client_id}:{client_secret}".encode()).decode()
        request = urllib.request.Request(
            host + "/oidc/v1/token",
            data=b"grant_type=client_credentials&scope=all-apis",
            headers={
                "Authorization": "Basic " + basic,
                "Content-Type": "application/x-www-form-urlencoded",
            },
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)["access_token"]
    token = os.environ.get("DATABRICKS_TOKEN", "")
    if not token:
        sys.exit("App identity is missing")
    return token


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        import traceback

        text = traceback.format_exc()
        print(text, flush=True)
        try:
            publish(["crash", text])
        except Exception:
            pass
        serve(text)
