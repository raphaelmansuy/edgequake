"""Databricks App launcher for the EdgeQuake server.

Writes a boot report to the Unity Catalog volume, then replaces this process
with the server binary. If the binary cannot start, this process serves the
report on DATABRICKS_APP_PORT so the failure stays visible.
"""

import os
import stat
import subprocess
import sys
import urllib.request


def main() -> None:
    raw_volume = os.environ.get("EDGEQUAKE_BINARY_PATH", "")
    lines = [
        "platform " + subprocess.getoutput("uname -a"),
        subprocess.getoutput("ldd --version | head -1"),
        "env " + ",".join(sorted(k for k in os.environ if k.startswith(("PG", "DATABRICKS", "EDGE", "LAKE")))),
        "volume_prefix " + raw_volume[:12],
        "volume_root " + volume_root(),
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
    publish(lines)
    if "not found" in lines[-1]:
        serve("\n".join(lines))
        return
    os.chmod(local, os.stat(local).st_mode | stat.S_IEXEC)
    os.execv(local, [local])


def volume_root() -> str:
    raw = os.environ.get("EDGEQUAKE_BINARY_PATH", "").strip().rstrip("/")
    if raw.startswith("/Volumes/"):
        return raw
    if raw.count(".") == 2 and "/" not in raw:
        catalog, schema, name = raw.split(".")
        return f"/Volumes/{catalog}/{schema}/{name}"
    return raw


def download(local: str) -> None:
    volume = volume_root()
    if not volume:
        raise RuntimeError("EDGEQUAKE_BINARY_PATH is empty")
    host = os.environ["DATABRICKS_HOST"].rstrip("/")
    request = urllib.request.Request(
        host + "/api/2.0/fs/files" + volume + "/edgequake",
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
    host = os.environ["DATABRICKS_HOST"].rstrip("/")
    request = urllib.request.Request(
        host + "/api/2.0/fs/files" + volume + "/boot-report.txt",
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
    host = os.environ["DATABRICKS_HOST"].rstrip("/")
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
