#!/usr/bin/env python3
"""Prove edgequake migrate from every published release onto this branch.

One cell is one published release and one engine (age or plain). The old
GHCR image creates the schema. A two-node fixture is inserted into the store
that release's readers use. This branch's `edgequake migrate` then runs twice,
and `upgrade_visibility` writes a new document through the API pool path.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import threading
import time
import uuid
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

REPO = "raphaelmansuy/edgequake"
LOG_ROOT = Path(
    "/cursor/stores/bc-e2f26b1c-fbee-4b13-8d5a-0895e4ddabbc/media/migration-upgrade-proof"
)
WORK = Path("/workspace")
MIGRATE_BIN = WORK / "edgequake/target/release/edgequake"
PROBE_BIN = WORK / "edgequake/target/release/examples/upgrade_visibility"
NETWORK = "eq-proof"
AGE_CONTAINER = "eq-proof-age"
PLAIN_CONTAINER = "eq-proof-plain"
AGE_HOST = "127.0.0.1"
AGE_PORT = 5433
PLAIN_PORT = 5434
USER = "edgequake"
PASSWORD = "edgequake_secret"
AGE_LOCK = threading.Lock()

AGE_SEED = r"""
CREATE EXTENSION IF NOT EXISTS age CASCADE;
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
SELECT CASE
  WHEN EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = 'eq_eq_default_graph')
  THEN 'exists'
  ELSE (SELECT ag_catalog.create_graph('eq_eq_default_graph')::text)
END;
SELECT CASE
  WHEN EXISTS (
    SELECT 1 FROM ag_catalog.ag_label l
    JOIN ag_catalog.ag_graph g ON g.graphid = l.graph
    WHERE g.name = 'eq_eq_default_graph' AND l.name = 'Node'
  ) THEN 'exists'
  ELSE (SELECT ag_catalog.create_vlabel('eq_eq_default_graph', 'Node')::text)
END;
SELECT CASE
  WHEN EXISTS (
    SELECT 1 FROM ag_catalog.ag_label l
    JOIN ag_catalog.ag_graph g ON g.graphid = l.graph
    WHERE g.name = 'eq_eq_default_graph' AND l.name = 'EDGE'
  ) THEN 'exists'
  ELSE (SELECT ag_catalog.create_elabel('eq_eq_default_graph', 'EDGE')::text)
END;
INSERT INTO eq_eq_default_graph."Node" (properties) VALUES
  ('{"node_id": "PRE_A", "entity_type": "PERSON", "description": "pre-upgrade fixture", "tenant_id": "default", "workspace_id": "default"}'::agtype),
  ('{"node_id": "PRE_B", "entity_type": "PERSON", "description": "pre-upgrade fixture", "tenant_id": "default", "workspace_id": "default"}'::agtype);
INSERT INTO eq_eq_default_graph."EDGE" (start_id, end_id, properties)
SELECT a.id, b.id,
  '{"source_id": "PRE_A", "target_id": "PRE_B", "relation_type": "KNOWS", "weight": 1.0, "description": "pre-upgrade fixture"}'::agtype
FROM eq_eq_default_graph."Node" a
JOIN eq_eq_default_graph."Node" b
  ON ag_catalog.agtype_to_json(a.properties)->>'node_id' = 'PRE_A'
 AND ag_catalog.agtype_to_json(b.properties)->>'node_id' = 'PRE_B';
"""

PLAIN_SEED = r"""
INSERT INTO public.eq_graph_nodes
  (namespace, node_id, tenant_id, workspace_id, entity_type, properties)
VALUES
  ('default', 'PRE_A', 'default', 'default', 'PERSON',
   '{"node_id":"PRE_A","entity_type":"PERSON","description":"pre-upgrade fixture","tenant_id":"default","workspace_id":"default"}'),
  ('default', 'PRE_B', 'default', 'default', 'PERSON',
   '{"node_id":"PRE_B","entity_type":"PERSON","description":"pre-upgrade fixture","tenant_id":"default","workspace_id":"default"}')
ON CONFLICT (namespace, node_id) DO NOTHING;
INSERT INTO public.eq_graph_edges
  (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
VALUES
  ('default', 'PRE_A', 'PRE_B', 'KNOWS', 'default', 'default', 1.0,
   '{"source_id":"PRE_A","target_id":"PRE_B","relation_type":"KNOWS","weight":1.0,"description":"pre-upgrade fixture"}')
ON CONFLICT (namespace, source_id, target_id, rel_type) DO NOTHING;
"""


def run(cmd: list[str], timeout: int = 180, env: dict | None = None) -> subprocess.CompletedProcess:
    merged = os.environ.copy()
    if env:
        merged.update(env)
    return subprocess.run(
        cmd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
        env=merged,
        check=False,
    )


def psql(container: str, database: str, sql: str, timeout: int = 60) -> subprocess.CompletedProcess:
    return run(
        [
            "docker",
            "exec",
            "-e",
            f"PGPASSWORD={PASSWORD}",
            container,
            "psql",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            USER,
            "-d",
            database,
            "-c",
            sql,
        ],
        timeout=timeout,
    )


def psql_file(container: str, database: str, sql: str, timeout: int = 60) -> subprocess.CompletedProcess:
    return run(
        [
            "docker",
            "exec",
            "-i",
            "-e",
            f"PGPASSWORD={PASSWORD}",
            container,
            "psql",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            USER,
            "-d",
            database,
        ],
        timeout=timeout,
    ) if False else _psql_stdin(container, database, sql, timeout)


def _psql_stdin(container: str, database: str, sql: str, timeout: int) -> subprocess.CompletedProcess:
    merged = os.environ.copy()
    return subprocess.run(
        [
            "docker",
            "exec",
            "-i",
            "-e",
            f"PGPASSWORD={PASSWORD}",
            container,
            "psql",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            USER,
            "-d",
            database,
        ],
        input=sql,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
        check=False,
    )


def scalar(container: str, database: str, sql: str) -> str | None:
    proc = run(
        [
            "docker",
            "exec",
            "-e",
            f"PGPASSWORD={PASSWORD}",
            container,
            "psql",
            "-q",
            "-tA",
            "-U",
            USER,
            "-d",
            database,
            "-c",
            sql,
        ],
        timeout=30,
    )
    if proc.returncode != 0:
        return None
    line = proc.stdout.strip().splitlines()
    return line[-1].strip() if line else ""


def db_name(version: str, engine: str) -> str:
    safe = version.replace(".", "_")
    return f"p{safe}_{engine}"[:63]


def local_url(engine: str, database: str) -> str:
    port = AGE_PORT if engine == "age" else PLAIN_PORT
    return f"postgres://{USER}:{PASSWORD}@{AGE_HOST}:{port}/{database}"


def docker_url(engine: str, database: str) -> str:
    host = AGE_CONTAINER if engine == "age" else PLAIN_CONTAINER
    return f"postgres://{USER}:{PASSWORD}@{host}:5432/{database}"


def container_for(engine: str) -> str:
    return AGE_CONTAINER if engine == "age" else PLAIN_CONTAINER


def published_releases() -> list[dict]:
    proc = run(
        [
            "gh",
            "release",
            "list",
            "--repo",
            REPO,
            "--limit",
            "200",
            "--json",
            "tagName,name,isDraft,isPrerelease,publishedAt",
        ],
        timeout=60,
    )
    if proc.returncode != 0:
        raise SystemExit(proc.stdout)
    rows = json.loads(proc.stdout)
    published = [row for row in rows if not row.get("isDraft")]
    published.sort(key=lambda row: row["publishedAt"])
    return published


def image_exists(version: str) -> bool:
    proc = run(
        ["docker", "manifest", "inspect", f"ghcr.io/{REPO}:{version}"],
        timeout=60,
    )
    return proc.returncode == 0


def ensure_image(version: str, log_dir: Path) -> tuple[bool, str]:
    image = f"ghcr.io/{REPO}:{version}"
    proc = run(["docker", "pull", image], timeout=600)
    (log_dir / "docker-pull.log").write_text(proc.stdout)
    if proc.returncode == 0:
        return True, image
    local = run(["docker", "image", "inspect", image], timeout=30)
    if local.returncode == 0:
        return True, f"{image} (already local; pull failed)"
    return False, f"no published image {image}: {tail(proc.stdout)}"


def tail(text: str, limit: int = 800) -> str:
    text = text.strip()
    if len(text) <= limit:
        return text
    return text[-limit:]


def reset_database(engine: str, database: str) -> None:
    container = container_for(engine)
    psql(
        container,
        "postgres",
        f"SELECT pg_terminate_backend(pid) FROM pg_stat_activity "
        f"WHERE datname = '{database}' AND pid <> pg_backend_pid();",
    )
    drop = psql(container, "postgres", f'DROP DATABASE IF EXISTS "{database}";')
    if drop.returncode != 0:
        raise RuntimeError(drop.stdout)
    create = psql(container, "postgres", f'CREATE DATABASE "{database}";')
    if create.returncode != 0:
        raise RuntimeError(create.stdout)


def boot_old(version: str, engine: str, database: str, log_dir: Path) -> tuple[bool, str]:
    """Start the published image. Stop it once schema creation finishes.

    Images since the migrate verb exit after `migrate`. Older images ignore
    that argument and stay up as the API, which is what created their schema.
    """
    image = f"ghcr.io/{REPO}:{version}"
    url = docker_url(engine, database)
    name = f"eqboot-{database}"[:63]
    run(["docker", "rm", "-f", name], timeout=30)
    started = run(
        [
            "docker",
            "run",
            "-d",
            "--name",
            name,
            "--network",
            NETWORK,
            "-e",
            f"DATABASE_URL={url}",
            "-e",
            "EDGEQUAKE_MIGRATE_CLI=1",
            "-e",
            "EDGEQUAKE_LLM_PROVIDER=mock",
            "-e",
            "EDGEQUAKE_EMBEDDING_PROVIDER=mock",
            "-e",
            "EDGEQUAKE_ALLOW_MOCK_PROVIDER=1",
            "-e",
            "EDGEQUAKE_AUTH_ENABLED=false",
            "-e",
            "EDGEQUAKE_DEV_MODE=true",
            "-e",
            "RUST_LOG=info",
            image,
            "migrate",
        ],
        timeout=60,
    )
    if started.returncode != 0:
        (log_dir / "old-migrate.log").write_text(started.stdout)
        return False, f"could not start {image}: {tail(started.stdout)}"
    last_head = None
    stable = 0
    logs = ""
    deadline = time.time() + 120
    how = "edgequake migrate"
    while time.time() < deadline:
        time.sleep(2)
        logs = run(["docker", "logs", name], timeout=20).stdout
        running = run(
            ["docker", "inspect", "-f", "{{.State.Running}} {{.State.ExitCode}}", name],
            timeout=20,
        ).stdout.strip()
        head = schema_head(engine, database)
        present = schema_present(engine, database)
        exited = running.startswith("false")
        if exited:
            how = "edgequake migrate" if present else "image exited before creating schema"
            break
        if present and head not in (None, "", "unknown", "none") and head == last_head:
            stable += 1
            if stable >= 2:
                how = "server boot (image has no exiting migrate verb)"
                run(["docker", "stop", "-t", "5", name], timeout=30)
                break
        else:
            stable = 0
        last_head = head
    else:
        run(["docker", "stop", "-t", "5", name], timeout=30)
        logs = run(["docker", "logs", name], timeout=20).stdout
    (log_dir / "old-migrate.log").write_text(logs)
    run(["docker", "rm", "-f", name], timeout=30)
    if schema_present(engine, database):
        return True, how
    return False, f"published image did not create a database: {tail(logs)}"


def migrate_verb_missing(text: str) -> bool:
    lowered = text.lower()
    needles = (
        "unexpected argument",
        "unrecognized subcommand",
        "unrecognized argument",
        "unknown command",
        "found argument 'migrate'",
        "found argument \"migrate\"",
    )
    return any(needle in lowered for needle in needles)


def schema_present(engine: str, database: str) -> bool:
    value = scalar(
        container_for(engine),
        database,
        "SELECT to_regclass('public.documents') IS NOT NULL "
        "OR to_regclass('public._sqlx_migrations') IS NOT NULL;",
    )
    return value == "t"


def seed_mode(engine: str, database: str) -> str:
    container = container_for(engine)
    age = scalar(
        container,
        database,
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age');",
    )
    cutover = scalar(
        container,
        database,
        "SELECT to_regclass('public.eq_graph_cutover') IS NOT NULL;",
    )
    nodes = scalar(
        container,
        database,
        "SELECT to_regclass('public.eq_graph_nodes') IS NOT NULL;",
    )
    status = None
    if cutover == "t":
        status = scalar(
            container,
            database,
            "SELECT status FROM public.eq_graph_cutover WHERE id = 1;",
        )
    if age == "t" and status in (None, "", "age"):
        return "age"
    if nodes == "t" and status == "ready":
        return "plain"
    if nodes == "t" and age != "t":
        return "plain"
    if age == "t":
        return "age"
    return "none"


def seed(engine: str, database: str, mode: str, log_dir: Path) -> tuple[bool, str]:
    if mode == "none":
        (log_dir / "seed.log").write_text("no graph store at this release\n")
        return True, "no graph fixture: this release has no graph store on this engine"
    sql = AGE_SEED if mode == "age" else PLAIN_SEED
    last = ""
    for attempt in (1, 2):
        proc = _psql_stdin(container_for(engine), database, sql, 60)
        last = proc.stdout
        (log_dir / "seed.log").write_text(last)
        if proc.returncode == 0:
            return True, mode
        transient = "server closed the connection" in last or "not yet accepting connections" in last
        if attempt == 1 and transient:
            time.sleep(5)
            continue
        break
    return False, f"graph fixture seed failed ({mode}): {tail(last)}"


def fingerprint(engine: str, database: str) -> str:
    sql = r"""
SELECT COALESCE((
  SELECT md5(string_agg(version::text || ':' || encode(checksum, 'hex') || ':' || success::text, ',' ORDER BY version))
  FROM public._sqlx_migrations
), 'no-ledger') || '|' || COALESCE((
  SELECT md5(string_agg(n.nspname || '.' || c.relname || ':' || c.relkind, ',' ORDER BY 1))
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
    AND n.nspname NOT LIKE 'pg_%'
    AND c.relkind IN ('r', 'v', 'm', 'i', 'S', 'f')
), 'no-catalog') || '|' || COALESCE((
  SELECT status || ':' || age_absent::text FROM public.eq_graph_cutover WHERE id = 1
), 'no-cutover');
"""
    return scalar(container_for(engine), database, sql) or "fingerprint-failed"


def schema_head(engine: str, database: str) -> str:
    return (
        scalar(
            container_for(engine),
            database,
            "SELECT COALESCE(max(version)::text, 'none') FROM public._sqlx_migrations WHERE success;",
        )
        or "unknown"
    )


def run_fixed_migrate(url: str, log_path: Path) -> subprocess.CompletedProcess:
    proc = run(
        [str(MIGRATE_BIN), "migrate"],
        timeout=600,
        env={"DATABASE_URL": url, "RUST_LOG": "info"},
    )
    log_path.write_text(proc.stdout)
    return proc


def applied_count(text: str) -> int | None:
    count = None
    for line in text.splitlines():
        if "applied_this_run=" in line:
            part = line.split("applied_this_run=", 1)[1].split()[0]
            try:
                count = int(part.strip("."))
            except ValueError:
                pass
        elif line.startswith("applied_this_run:"):
            try:
                count = int(line.split(":", 1)[1].strip())
            except ValueError:
                pass
    return count


def run_cell(release: dict, engine: str) -> dict:
    version = release["tagName"].lstrip("v")
    tag = release["tagName"]
    log_dir = LOG_ROOT / f"{tag}-{engine}"
    log_dir.mkdir(parents=True, exist_ok=True)
    result_path = log_dir / "result.json"
    database = db_name(version, engine)
    cell = {
        "tag": tag,
        "version": version,
        "name": release["name"],
        "published_at": release["publishedAt"],
        "engine": engine,
        "database": database,
        "status": "fail",
        "boot": None,
        "seed": None,
        "schema_before": None,
        "schema_after": None,
        "idempotent": None,
        "visibility": None,
        "reason": "",
    }
    try:
        ok, detail = ensure_image(version, log_dir)
        if not ok:
            cell["status"] = "cannot_boot"
            cell["reason"] = detail
            cell["boot"] = "no image"
            result_path.write_text(json.dumps(cell, indent=2))
            return cell
        reset_database(engine, database)
        if engine == "age":
            AGE_LOCK.acquire()
        try:
            booted, how = boot_old(version, engine, database, log_dir)
            cell["boot"] = how
            if booted:
                cell["schema_before"] = schema_head(engine, database)
                mode = seed_mode(engine, database)
                seeded, seed_detail = seed(engine, database, mode, log_dir)
                cell["seed"] = seed_detail
            else:
                mode = "none"
                seeded, seed_detail = False, ""
        finally:
            if engine == "age":
                AGE_LOCK.release()
        if not booted:
            cell["status"] = "cannot_boot"
            cell["reason"] = how
            result_path.write_text(json.dumps(cell, indent=2))
            return cell
        if not seeded:
            cell["status"] = "fail"
            cell["reason"] = seed_detail
            result_path.write_text(json.dumps(cell, indent=2))
            return cell
        url = local_url(engine, database)
        first = run_fixed_migrate(url, log_dir / "fixed-migrate-1.log")
        if first.returncode != 0:
            cell["reason"] = f"fixed migrate failed: {tail(first.stdout)}"
            result_path.write_text(json.dumps(cell, indent=2))
            return cell
        before = fingerprint(engine, database)
        (log_dir / "fingerprint-before.txt").write_text(before + "\n")
        second = run_fixed_migrate(url, log_dir / "fixed-migrate-2.log")
        after = fingerprint(engine, database)
        (log_dir / "fingerprint-after.txt").write_text(after + "\n")
        second_applied = applied_count(second.stdout)
        applied_work = 'step="apply_ok"' in second.stdout or "… applied in" in second.stdout
        idempotent = (
            second.returncode == 0
            and before == after
            and not applied_work
            and second_applied in (None, 0)
        )
        cell["idempotent"] = idempotent
        cell["schema_after"] = schema_head(engine, database)
        if not idempotent:
            cell["reason"] = (
                f"second migrate is not a no-op (exit {second.returncode}, "
                f"applied_this_run={second_applied}, fingerprint_same={before == after})"
            )
            result_path.write_text(json.dumps(cell, indent=2))
            return cell
        doc_id = str(uuid.uuid4())
        probe_env = {
            "DATABASE_URL": url,
            "DOC_ID": doc_id,
            "FIXTURE_NODE": "PRE_A",
            "FIXTURE_OTHER": "PRE_B",
            "EXPECT_FIXTURE": "0" if mode == "none" else "1",
            "RUST_LOG": "warn",
        }
        probe = run([str(PROBE_BIN)], timeout=180, env=probe_env)
        (log_dir / "visibility.log").write_text(probe.stdout)
        proof = parse_proof(probe.stdout)
        cell["visibility"] = proof
        write_plans(log_dir, proof)
        if probe.returncode != 0 or not proof or not proof.get("pass"):
            reasons = (proof or {}).get("reasons") or [tail(probe.stdout)]
            cell["reason"] = "; ".join(str(item) for item in reasons)
            result_path.write_text(json.dumps(cell, indent=2))
            return cell
        cell["status"] = "pass"
        cell["reason"] = "idempotent migrate; plain graph only; new document visible; AGE plans absent"
        if mode == "none":
            cell["reason"] += "; this engine had no graph store before upgrade"
        result_path.write_text(json.dumps(cell, indent=2))
        return cell
    except subprocess.TimeoutExpired as exc:
        cell["status"] = "fail"
        cell["reason"] = f"timed out: {exc.cmd}"
        result_path.write_text(json.dumps(cell, indent=2))
        return cell
    except Exception as exc:  # noqa: BLE001 — one cell must not kill the matrix
        cell["status"] = "fail"
        cell["reason"] = f"{type(exc).__name__}: {exc}"
        result_path.write_text(json.dumps(cell, indent=2))
        return cell


def write_plans(log_dir: Path, proof: dict | None) -> None:
    lines: list[str] = []
    for item in (proof or {}).get("explains") or []:
        lines.append("SQL")
        lines.append(str(item.get("sql") or ""))
        lines.append("PLAN")
        lines.append(str(item.get("plan") or ""))
        lines.append("")
    text = "\n".join(lines).strip()
    if not text:
        text = "no explain plans"
    (log_dir / "plans.txt").write_text(text + "\n")


def parse_proof(text: str) -> dict | None:
    for line in reversed(text.splitlines()):
        if line.startswith("PROOF "):
            try:
                return json.loads(line[len("PROOF ") :])
            except json.JSONDecodeError:
                return None
    return None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--only", default="", help="comma-separated tags, e.g. v0.26.5")
    parser.add_argument("--engines", default="age,plain")
    parser.add_argument("--jobs", type=int, default=2)
    args = parser.parse_args()
    if not MIGRATE_BIN.exists() or not PROBE_BIN.exists():
        raise SystemExit(f"missing binaries under {MIGRATE_BIN.parent}")
    LOG_ROOT.mkdir(parents=True, exist_ok=True)
    releases = published_releases()
    (LOG_ROOT / "releases.json").write_text(json.dumps(releases, indent=2))
    wanted = {item.strip() for item in args.only.split(",") if item.strip()}
    engines = [item.strip() for item in args.engines.split(",") if item.strip()]
    if wanted:
        releases = [row for row in releases if row["tagName"] in wanted]
    tasks = [(row, engine) for row in releases for engine in engines]
    print(f"cells {len(tasks)} jobs {args.jobs}", flush=True)
    results = []
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = [pool.submit(run_cell, row, engine) for row, engine in tasks]
        for future in as_completed(futures):
            cell = future.result()
            results.append(cell)
            print(
                f"{cell['status']:12} {cell['tag']} {cell['engine']} {cell['reason'][:180]}",
                flush=True,
            )
    by_key = {}
    for path in sorted(LOG_ROOT.glob("*/result.json")):
        try:
            row = json.loads(path.read_text())
        except json.JSONDecodeError:
            continue
        if row.get("tag") and row.get("engine"):
            by_key[(row["tag"], row["engine"])] = row
    for cell in results:
        by_key[(cell["tag"], cell["engine"])] = cell
    merged = sorted(by_key.values(), key=lambda row: (row.get("published_at") or "", row["tag"], row["engine"]))
    summary = {
        "pass": sum(1 for cell in merged if cell["status"] == "pass"),
        "fail": sum(1 for cell in merged if cell["status"] == "fail"),
        "cannot_boot": sum(1 for cell in merged if cell["status"] == "cannot_boot"),
        "cells": len(merged),
    }
    (LOG_ROOT / "summary.json").write_text(json.dumps({"summary": summary, "results": merged}, indent=2))
    print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
