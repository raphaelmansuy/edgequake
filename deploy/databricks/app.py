"""Databricks App launcher for the EdgeQuake server.

Downloads the server binary from the Unity Catalog volume, writes a boot
report, and starts the server as a child. Python stays process 1 so a fast
exit is still visible: the report is stored on the volume and served on
DATABRICKS_APP_PORT. A healthy child keeps that port.
"""

import os
import re
import signal
import stat
import subprocess
import sys
import threading
import time
import urllib.request

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


def run_server(
    local: str,
    env: dict[str, str],
    lines: list[str],
    serve_on_exit: bool,
) -> int | None:
    """Keep Python as process 1 and retain the child's stdout.

    Returns the child exit code. A healthy server does not return.
    """
    proc = subprocess.Popen(
        [local],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        env=env,
    )
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

    published = False
    deadline = time.time() + 45
    while True:
        code = proc.poll()
        if code is not None or (not published and time.time() >= deadline):
            snapshot = lines + server_tail(chunks, code)
            publish(snapshot)
            published = True
            if code is not None:
                lines[:] = snapshot
                if serve_on_exit and not stopping["value"]:
                    serve("\n".join(snapshot))
                return code
        if published:
            code = proc.wait()
            snapshot = lines + server_tail(chunks, code)
            publish(snapshot)
            lines[:] = snapshot
            if serve_on_exit and not stopping["value"]:
                serve("\n".join(snapshot))
            return code
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
    volume = volume_root()
    if not volume:
        raise RuntimeError("EDGEQUAKE_BINARY_PATH is empty")
    request = urllib.request.Request(
        workspace_url() + "/api/2.0/fs/files" + volume + "/edgequake",
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
