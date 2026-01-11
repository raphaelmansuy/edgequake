#!/usr/bin/env python3
"""
Fix CreateWorkspaceRequest usages by adding missing embedding fields.
SPEC-032: Ollama/LM Studio provider integration
"""

import re
import sys

def fix_workspace_request(content: str) -> str:
    """Add missing embedding fields to CreateWorkspaceRequest structs."""
    
    # Pattern to match CreateWorkspaceRequest { ... } blocks that are missing embedding fields
    # Matches blocks ending with max_documents: ...} or max_documents: ...,}
    
    # Handle max_documents: None, } and max_documents: Some(...), }
    pattern = r'(CreateWorkspaceRequest\s*\{[^}]*max_documents:\s*(?:None|Some\([^)]+\)))\s*,?\s*\}'
    
    def add_embedding_fields(m):
        inner = m.group(1)
        # Check if embedding fields are already present
        if 'embedding_model' in inner:
            return m.group(0)
        return inner + """,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
        }"""
    
    content = re.sub(pattern, add_embedding_fields, content, flags=re.DOTALL)
    
    # Also replace test_workspace_request() calls with CreateWorkspaceRequest::new()
    content = re.sub(
        r'test_workspace_request\("([^"]+)"\)',
        r'CreateWorkspaceRequest::new("\1")',
        content
    )
    
    # Replace test_workspace_request_with_slug() calls
    content = re.sub(
        r'test_workspace_request_with_slug\("([^"]+)",\s*"([^"]+)"\)',
        lambda m: f'''CreateWorkspaceRequest {{
            name: "{m.group(1)}".to_string(),
            slug: Some("{m.group(2)}".to_string()),
            description: None,
            max_documents: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
        }}''',
        content
    )
    
    return content


if __name__ == "__main__":
    filepath = sys.argv[1]
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    fixed = fix_workspace_request(content)
    
    with open(filepath, 'w') as f:
        f.write(fixed)
    
    print(f"Fixed {filepath}")
