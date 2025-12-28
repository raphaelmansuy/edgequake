# EdgeQuake Skills (Legacy - Moved to .github/skills/)

⚠️ **MIGRATION NOTICE**: All formal skill definitions have been moved to [`.github/skills/`](../.github/skills/). This directory is kept for historical reference only.

For current skills and documentation, see the [.github/skills/](../.github/skills/) directory.

## Active Skills in .github/skills/

- [Reverse Documentation Skill](../.github/skills/reverse-documentation/SKILL.md)
- [Makefile Dev Workflow Skill](../.github/skills/makefile-dev-workflow/SKILL.md)
- [Playwright UX/UI Capture Skill](../.github/skills/playwright-ux-ui-capture/SKILL.md)
- [UX/UI Analysis Skills](../.github/skills/ux-ui-analyze-single-page/SKILL.md)
- [CopilotKit Integration Skill](../.github/skills/copilotkit-nextjs-integration/SKILL.md)

---

## Legacy Content

### [Reverse Documentation](./reverse-documentation/) (Moved to .github/skills/)

Automatically generate comprehensive documentation for Rust and TypeScript codebases by analyzing code structure, patterns, and relationships.

**Languages Supported:**

- Rust (traits, async patterns, error handling)
- TypeScript (React components, hooks, Next.js)

**Use Cases:**

- Document entire crates or modules
- Generate API documentation
- Create component documentation
- Extract architecture diagrams
- Generate migration guides

**Example Usage:**

```
Generate comprehensive documentation for the edgequake-storage crate
```

## Skill Structure

Each skill contains:

- `README.md` - Overview and usage guide
- Language-specific directories with:
  - `skill.md` - Skill description and capabilities
  - `instructions.md` - Detailed instructions for AI assistants
- `examples/` - Real-world usage examples

## How to Use Skills

Simply mention what you want to document in natural language:

**For Rust:**

```
Document the GraphStorage trait and all its implementations
```

**For TypeScript:**

```
Document all React components in the workspace management module
```

The AI assistant will automatically:

1. Discover and analyze the relevant code
2. Extract patterns and relationships
3. Generate comprehensive documentation
4. Verify examples compile and work

## Creating New Skills

To create a new skill:

1. Create a directory under `skills/`
2. Add a README.md describing the skill
3. Add language-specific subdirectories
4. Include `skill.md` and `instructions.md` files
5. Provide examples in `examples/` directory
6. Update this index

## Best Practices

- Keep skills focused on specific tasks
- Provide clear examples
- Include language-specific patterns
- Document expected outputs
- Include verification steps

## Contributing

When adding or improving skills:

1. Follow the existing structure
2. Test with real code examples
3. Update documentation
4. Add examples showing usage
5. Verify AI assistants can use the skill effectively
