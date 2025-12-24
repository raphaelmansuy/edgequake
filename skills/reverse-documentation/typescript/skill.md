# Reverse Documentation Skill for TypeScript

This skill enables you to analyze TypeScript/React codebases and generate comprehensive documentation by understanding code structure, patterns, and relationships.

## Purpose

Automatically generate documentation for TypeScript projects by analyzing:

- Module structure and exports
- Type definitions and interfaces
- React components and hooks
- Function signatures and implementations
- API endpoints and data flow
- State management patterns
- Test coverage

## Usage

To use this skill, provide one or more TypeScript files or directories and specify the documentation format you want.

### Example Commands

```
Generate comprehensive documentation for the edgequake_webui React app
```

```
Document all API hooks in the src/hooks directory
```

```
Create component documentation for the workspace management UI
```

## Skill Workflow

When you invoke this skill, the agent will:

1. **Analyze Code Structure**

   - Parse TypeScript/TSX source files
   - Identify modules, types, interfaces, and functions
   - Extract React components and hooks
   - Understand component hierarchies

2. **Extract Metadata**

   - Read package.json for dependencies and scripts
   - Identify framework (Next.js, Vite, React)
   - Extract version information and metadata
   - Understand build configuration

3. **Understand Patterns**

   - Identify React patterns (Context, Hooks, HOCs)
   - Recognize state management (useState, useReducer, Zustand, etc.)
   - Understand data fetching patterns (SWR, React Query, fetch)
   - Detect component composition patterns
   - Identify TypeScript utility types

4. **Generate Documentation**
   - Create component documentation with props
   - Generate hook documentation with usage examples
   - Document API types and interfaces
   - Create architecture diagrams (optional)
   - Generate README files and Storybook stories

## Output Formats

The skill can generate documentation in multiple formats:

- **TSDoc Comments**: Standard TypeScript JSDoc comments
- **Markdown Files**: README.md, COMPONENTS.md, API.md
- **Storybook Stories**: Component stories with variants
- **API Documentation**: Type definitions and schemas
- **Architecture Diagrams**: Mermaid diagrams showing data flow

## Configuration

You can customize the documentation generation by specifying:

```yaml
scope: "public" # or "all" for private items too
format: "markdown" # or "inline" or "both"
include_examples: true
include_props: true
include_stories: true
include_diagrams: true
depth: "comprehensive" # or "brief" or "detailed"
framework: "react" # or "vue" or "angular"
```

## Best Practices

1. **Document Components First**: Focus on reusable components
2. **Include Prop Examples**: Show all prop variations
3. **Explain State Flow**: Document how data flows through components
4. **Keep Updated**: Re-run documentation generation after changes
5. **Link Related Items**: Cross-reference related components and hooks

## TypeScript-Specific Features

### Component Documentation

````typescript
/**
 * A button component with various styles and sizes.
 *
 * @param props - The component props
 * @returns A styled button element
 *
 * @example
 * ```tsx
 * <Button variant="primary" size="lg" onClick={handleClick}>
 *   Click me
 * </Button>
 * ```
 */
export const Button: React.FC<ButtonProps> = ({
  variant,
  size,
  children,
  onClick,
}) => {
  // ...
};
````

### Interface Documentation

```typescript
/**
 * Properties for the Workspace component.
 */
export interface WorkspaceProps {
  /** The unique identifier for the workspace */
  id: string;
  /** The display name of the workspace */
  name: string;
  /** Optional description */
  description?: string;
  /** Callback fired when workspace is selected */
  onSelect?: (id: string) => void;
}
```

### Hook Documentation

````typescript
/**
 * Custom hook for managing workspace state.
 *
 * @param workspaceId - The ID of the workspace to manage
 * @returns Workspace state and mutation functions
 *
 * @example
 * ```tsx
 * const { workspace, isLoading, updateWorkspace } = useWorkspace('ws-123');
 * ```
 */
export function useWorkspace(workspaceId: string) {
  // ...
}
````

### API Type Documentation

```typescript
/**
 * Response from the search endpoint.
 *
 * @remarks
 * This type includes pagination metadata and results array.
 */
export interface SearchResponse {
  /** Total number of matching results */
  total: number;
  /** Current page number (0-indexed) */
  page: number;
  /** Results per page */
  pageSize: number;
  /** Array of search results */
  results: SearchResult[];
}
```

## Integration with EdgeQuake WebUI

This skill is specifically tuned for EdgeQuake's React + TypeScript patterns:

- **Next.js App Router**: Documents page components and layouts
- **shadcn/ui Components**: Documents UI component usage
- **React Hooks**: Documents custom hooks for API calls
- **TypeScript Types**: Documents API types and interfaces
- **State Management**: Documents state flow patterns
- **Form Handling**: Documents form components and validation

## React Patterns

### Functional Components

```typescript
/**
 * Displays a list of workspaces with search and filter capabilities.
 *
 * @param props - Component props
 * @returns The workspace list component
 */
export const WorkspaceList: React.FC<WorkspaceListProps> = ({
  workspaces,
  onSelect
}) => {
  // Document internal state
  const [filter, setFilter] = useState('');

  // Document effects
  useEffect(() => {
    // Load workspaces
  }, []);

  return (
    // JSX
  );
};
```

### Custom Hooks

````typescript
/**
 * Hook for fetching and managing document data.
 *
 * @param documentId - The document ID to fetch
 * @returns Document data, loading state, and error state
 *
 * @example
 * ```tsx
 * function DocumentViewer({ id }: { id: string }) {
 *   const { data, isLoading, error } = useDocument(id);
 *
 *   if (isLoading) return <Spinner />;
 *   if (error) return <Error message={error.message} />;
 *
 *   return <Document {...data} />;
 * }
 * ```
 */
export function useDocument(documentId: string) {
  return useSWR<Document>(`/api/documents/${documentId}`, fetcher);
}
````

### Context Providers

````typescript
/**
 * Provides workspace context to child components.
 *
 * @param props - Provider props
 * @returns The context provider component
 *
 * @example
 * ```tsx
 * <WorkspaceProvider>
 *   <App />
 * </WorkspaceProvider>
 * ```
 */
export const WorkspaceProvider: React.FC<PropsWithChildren> = ({
  children,
}) => {
  // Document context value
  const value = useWorkspaceState();

  return (
    <WorkspaceContext.Provider value={value}>
      {children}
    </WorkspaceContext.Provider>
  );
};
````

## Type Documentation Patterns

### Utility Types

````typescript
/**
 * Makes all properties of T optional except for specified keys K.
 *
 * @typeParam T - The base type
 * @typeParam K - Keys to keep required
 *
 * @example
 * ```typescript
 * type User = { id: string; name: string; email: string };
 * type PartialUser = PartialExcept<User, 'id'>;
 * // Result: { id: string; name?: string; email?: string }
 * ```
 */
export type PartialExcept<T, K extends keyof T> = Partial<T> & Pick<T, K>;
````

### Generic Types

```typescript
/**
 * API response wrapper with loading and error states.
 *
 * @typeParam T - The type of the data payload
 */
export interface ApiResponse<T> {
  /** The response data */
  data: T | null;
  /** Loading state indicator */
  isLoading: boolean;
  /** Error object if request failed */
  error: Error | null;
}
```

## Quality Checklist

Before considering documentation complete, verify:

- [ ] All exported components have documentation
- [ ] All exported functions/hooks have documentation
- [ ] All exported types/interfaces have documentation
- [ ] Component props are documented
- [ ] Examples compile and run
- [ ] Hook usage is demonstrated
- [ ] Cross-references between related items exist
- [ ] Module-level documentation exists

## Tools and Utilities

### TypeDoc

Generate documentation from TypeScript:

```bash
npm install --save-dev typedoc
npx typedoc src/
```

### Storybook

Component documentation:

```bash
npm install --save-dev @storybook/react
npm run storybook
```

### TSDoc

Validate TSDoc comments:

```bash
npm install --save-dev @microsoft/tsdoc
```

## Examples

See the `examples/` directory for:

- Component documentation
- Hook documentation
- API type documentation
- Full app documentation
