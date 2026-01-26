#!/usr/bin/env python3
"""
Fix CreateWorkspaceRequest usages in test files to use helper functions.
SPEC-032: Ollama/LM Studio provider integration
"""

import re


def fix_workspace_request(content: str) -> str:
    """Replace struct initialization with builder pattern."""

    # Pattern to match CreateWorkspaceRequest { ... } blocks
    pattern = r'CreateWorkspaceRequest\s*\{\s*name:\s*"([^"]+)"\.to_string\(\)\s*,\s*slug:\s*None\s*,\s*description:\s*None\s*,\s*max_documents:\s*None\s*,?\s*\}'

    def replacement(m):
        name = m.group(1)
        return f'test_workspace_request("{name}")'

    content = re.sub(pattern, replacement, content, flags=re.DOTALL)

    # Pattern for slug: Some("...")
    pattern2 = r'CreateWorkspaceRequest\s*\{\s*name:\s*"([^"]+)"\.to_string\(\)\s*,\s*slug:\s*Some\("([^"]+)"\.to_string\(\)\)\s*,\s*description:\s*None\s*,\s*max_documents:\s*None\s*,?\s*\}'

    def replacement2(m):
        name = m.group(1)
        slug = m.group(2)
        return f'test_workspace_request_with_slug("{name}", "{slug}")'

    content = re.sub(pattern2, replacement2, content, flags=re.DOTALL)

    # Pattern with format! for name
    pattern3 = r'CreateWorkspaceRequest\s*\{\s*name:\s*format!\("([^"]+)",\s*([^)]+)\)\s*,\s*slug:\s*None\s*,\s*description:\s*None\s*,\s*max_documents:\s*None\s*,?\s*\}'

    def replacement3(m):
        fmt = m.group(1)
        arg = m.group(2)
        return f'test_workspace_request(&format!("{fmt}", {arg}))'

    content = re.sub(pattern3, replacement3, content, flags=re.DOTALL)
