#!/usr/bin/env python3
"""Validate PROVIDER-ACCESS documentation and pinned source evidence without dependencies."""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

SPEC = Path(__file__).resolve().parents[1]
ROOT = SPEC.parents[1]
ERRORS: list[str] = []
DOCUMENTS = {
    'README.md',
    'design/why.md',
    'design/current-state-audit.md',
    'design/architecture.md',
    'design/contracts.md',
    'design/consistency-and-migration.md',
    'design/algorithms-and-budgets.md',
    'implementation/feasibility-and-releases.md',
    'implementation/work-packages.md',
    'implementation/step-by-step-guide.md',
    'implementation/persistence-and-provider-recipes.md',
    'validation/edge-cases-and-contract-tests.md',
    'validation/e2e-test-specification.md',
    'validation/definition-of-done.md',
    'references/official-sources.md',
    'evidence/implementation-status.md',
}


def require(condition: bool, message: str) -> None:
    if not condition:
        ERRORS.append(message)


def heading_ids(text: str) -> set[str]:
    result: set[str] = set()
    seen: dict[str, int] = {}
    for heading in re.findall(r'^#{1,6}\s+(.+)$', text, re.M):
        slug = re.sub(r'[^\w\- ]', '', heading.lower()).replace(' ', '-')
        count = seen.get(slug, 0)
        seen[slug] = count + 1
        result.add(slug if count == 0 else f'{slug}-{count}')
    return result


def check_markdown(path: Path) -> int:
    content = path.read_text()
    require(content.startswith('# WHY'), f'{path.name}: must start with WHY')
    require('<!--' not in content, f'{path.name}: unresolved placeholder')
    require(len(re.findall(r'^```', content, re.M)) % 2 == 0,
            f'{path.name}: unbalanced fences')
    for number, line in enumerate(content.splitlines(), 1):
        require(line == line.rstrip(), f'{path.name}:{number}: trailing whitespace')
    link_count = 0
    for target in re.findall(r'\[[^\]]*\]\(([^)]+)\)', content):
        if urlsplit(target).scheme:
            require(target.startswith('https://'), f'{path.name}: non-HTTPS URL {target}')
            continue
        link_count += 1
        raw_path, _, anchor = unquote(target).partition('#')
        dest = (path.parent / raw_path).resolve() if raw_path else path
        require(dest.exists(), f'{path.name}: missing local link {target}')
        if dest.is_file() and anchor and dest.suffix == '.md':
            require(anchor in heading_ids(dest.read_text()),
                    f'{path.name}: missing heading {target}')
    for block in re.findall(r'```ascii\n(.*?)```', content, re.S):
        require(block.isascii(), f'{path.name}: non-ASCII diagram')
        require('\t' not in block, f'{path.name}: tabs in diagram')
        borders: list[int] | None = None
        for line in block.splitlines():
            if re.fullmatch(r'[ +\-]+', line) and '+' in line:
                borders = [i for i, char in enumerate(line) if char == '+']
            elif line.startswith('|') and borders:
                require(len(line) > borders[-1] and all(line[i] == '|' for i in borders),
                        f'{path.name}: misaligned box: {line!r}')
    return link_count


def check_evidence() -> int:
    manifest = json.loads((SPEC / 'evidence/code-evidence.json').read_text())
    require(bool(re.fullmatch(r'[0-9a-f]{40}', manifest['commit'])), 'invalid baseline commit')
    ids: set[str] = set()
    for item in manifest['entries']:
        require(item['id'] not in ids, f"duplicate evidence {item['id']}")
        ids.add(item['id'])
        source = ROOT / item['path']
        require(source.is_file(), f"missing evidence source {item['path']}")
        if not source.is_file():
            continue
        raw = source.read_bytes()
        require(hashlib.sha256(raw).hexdigest() == item['sha256'],
                f"{item['id']}: source changed; re-audit before refreshing hash")
        lines = raw.decode().splitlines()
        line = item['line']
        require(0 < line <= len(lines) and item['needle'] in lines[line - 1],
                f"{item['id']}: invalid line/needle")
    require(ids == {f'E{i:02}' for i in range(1, 35)}, 'expected E01-E34')
    return len(ids)


def check_traceability() -> None:
    findings = (SPEC / 'design/current-state-audit.md').read_text()
    principles = (SPEC / 'design/why.md').read_text()
    plan = (SPEC / 'implementation/work-packages.md').read_text()
    validation = (SPEC / 'validation/edge-cases-and-contract-tests.md').read_text()
    expected = {f'PROVIDER-ACCESS-F{i:02}' for i in range(1, 15)}
    defined = set(re.findall(r'^\| (PROVIDER-ACCESS-F\d+) \|', findings, re.M))
    mapped = set(re.findall(r'^\| (PROVIDER-ACCESS-F\d+) \|', validation, re.M))
    require(defined == expected == mapped, 'F01-F14 finding/traceability coverage differs')
    tests = set(re.findall(r'^\| (PROVIDER-ACCESS-T\d+) \|', validation, re.M))
    require(tests == {f'PROVIDER-ACCESS-T{i:02}' for i in range(1, 21)}, 'expected T01-T20')
    work = set(re.findall(r'^## (PROVIDER-ACCESS-W\d+) ', plan, re.M))
    require(work == {f'PROVIDER-ACCESS-W{i}' for i in range(10)}, 'expected W0-W9')
    invariants = set(re.findall(r'^\| (PROVIDER-ACCESS-I\d+) \|', principles, re.M))
    require(invariants == {f'PROVIDER-ACCESS-I{i}' for i in range(1, 9)}, 'expected I1-I8')
    for row in validation.splitlines():
        if row.startswith('| PROVIDER-ACCESS-F'):
            cells = row.split('|')[1:-1]
            for token in re.findall(r'\b(?:I\d+|W\d+|T\d+)\b', row):
                pool = invariants if token[0] == 'I' else work if token[0] == 'W' else tests
                require(f'PROVIDER-ACCESS-{token}' in pool, f'unknown traceability reference {token}')
            require(len(cells) == 4 and all(c.strip() for c in cells), 'incomplete traceability row')


def check_delivery(manifest: dict | None = None) -> None:
    if manifest is None:
        manifest = json.loads((SPEC / 'evidence/delivery-matrix.json').read_text())
    guide = (SPEC / 'implementation/step-by-step-guide.md').read_text()
    e2e = (SPEC / 'validation/e2e-test-specification.md').read_text()
    step_ids = {f'J{i:02}' for i in range(1, 25)}
    scenario_ids = {f'PROVIDER-ACCESS-E2E{i:02}' for i in range(1, 16)}
    release_ids = {f'R{i}' for i in range(5)}
    profile_ids = {'P0', 'P1', 'P2a', 'P2b', 'P3', 'P4'}
    require(manifest.get('schema_version') == 1, 'unsupported delivery schema')
    require(set(manifest['profiles']) == profile_ids, 'expected all six compositions')
    require(set(re.findall(r'^## (J\d+) ', guide, re.M)) == step_ids,
            'guide must define J01-J24')
    require(set(re.findall(r'^### (PROVIDER-ACCESS-E2E\d+) ', e2e, re.M)) == scenario_ids,
            'E2E document must define E2E01-E2E15')
    steps = {s['id']: s for s in manifest['steps']}
    scenarios = {s['id']: s for s in manifest['scenarios']}
    releases = {r['id']: r for r in manifest['releases']}
    require(set(steps) == step_ids and len(manifest['steps']) == 24,
            'delivery step IDs missing or duplicated')
    require(set(scenarios) == scenario_ids and len(manifest['scenarios']) == 15,
            'delivery scenario IDs missing or duplicated')
    require(set(releases) == release_ids and len(manifest['releases']) == 5,
            'expected R0-R4 releases')
    work_ids = {f'W{i}' for i in range(10)}
    covered_tests: set[str] = set()
    for step in steps.values():
        require(step['work'] in work_ids, f"unknown package for {step['id']}")
        require(step['release'] in release_ids, f"unknown release for {step['id']}")
        for dep in step['depends_on']:
            require(dep in steps and dep < step['id'],
                    f"{step['id']}: dependency must precede step: {dep}")
            if dep in steps:
                require(steps[dep]['release'] <= step['release'],
                        f"{step['id']}: dependency belongs to later release")
    for scenario in scenarios.values():
        require(scenario['first_full_release'] in release_ids,
                f"unknown E2E release: {scenario['id']}")
        require(bool(scenario['work']) and set(scenario['work']) <= work_ids,
                f"invalid E2E work mapping: {scenario['id']}")
        require(bool(scenario['tests']), f"empty E2E proof: {scenario['id']}")
        covered_tests.update(scenario['tests'])
    require(covered_tests == {f'T{i:02}' for i in range(1, 21)},
            'E2E scenarios must cover exactly T01-T20')
    previous_profiles: set[str] = set()
    for key in sorted(releases):
        release = releases[key]
        profiles = set(release['required_profiles'])
        require(bool(profiles) and previous_profiles <= profiles <= profile_ids,
                f'{key}: profile regression coverage lost or invalid')
        previous_profiles = profiles
        expected_steps = {s['id'] for s in steps.values() if s['release'] <= key}
        expected_scenarios = {s['id'] for s in scenarios.values()
                              if s['first_full_release'] <= key}
        require(set(release['required_steps']) == expected_steps,
                f'{key}: incomplete or premature step gate')
        require(set(release['full_scenarios']) == expected_scenarios,
                f'{key}: incomplete or premature E2E gate')
    require(previous_profiles == profile_ids, 'R4 must certify all six compositions')


def main() -> int:
    files = sorted(SPEC.rglob('*.md'))
    require({str(p.relative_to(SPEC)) for p in files} == DOCUMENTS,
            'document directory structure differs from the owned topic layout')
    allowed_files = DOCUMENTS | {
        'scripts/validate_spec.py', 'evidence/code-evidence.json',
        'evidence/delivery-matrix.json', 'evidence/validation.txt',
        'evidence/implementation-status.md',
    }
    actual_files = {str(p.relative_to(SPEC)) for p in SPEC.rglob('*') if p.is_file()}
    require(actual_files == allowed_files, 'unexpected or missing specification artifact')
    links = sum(check_markdown(p) for p in files)
    evidence = check_evidence()
    check_traceability()
    check_delivery()
    if ERRORS:
        print('\n'.join(ERRORS), file=sys.stderr)
        return 1
    print(f'PASS: {len(files)} documents, {links} local links, {evidence} source anchors; '
          'ASCII boxes and F01-F14 / I1-I8 / W0-W9 / T01-T20 traceability valid; '
          '24 ordered steps, 15 E2E scenarios, 5 releases, 6 compositions valid.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
