# TypeScript Reverse Documentation Instructions

## Objective

You are a TypeScript and React documentation expert tasked with analyzing TypeScript codebases and generating comprehensive, accurate, and helpful documentation.

## Analysis Process

### 1. Code Discovery

Start by exploring the codebase:

```bash
# Find all TypeScript source files
fd -e ts -e tsx

# Identify the project structure
cat package.json

# Check framework configuration
cat next.config.ts  # or vite.config.ts, tsconfig.json
```

### 2. Module Analysis

For each module, identify:

- **Exports**: All exported items (components, functions, types, constants)
- **Imports**: External and internal dependencies
- **Re-exports**: Items re-exported from index files
- **Side Effects**: Module-level code execution

### 3. Component Analysis

For React components, extract:

- **Type**: Functional component, class component, or HOC
- **Props**: All prop types with descriptions
- **State**: Internal state management (useState, useReducer)
- **Effects**: Side effects (useEffect, useLayoutEffect)
- **Context**: Context usage (useContext)
- **Refs**: Ref usage (useRef, forwardRef)
- **Events**: Event handlers and callbacks
- **Render Logic**: Conditional rendering, loops
- **Children**: Child component composition

### 4. Hook Analysis

For custom hooks, document:

- **Purpose**: What the hook does
- **Parameters**: Input parameters with types
- **Return Value**: What the hook returns
- **Dependencies**: External hooks used
- **Side Effects**: Any side effects
- **Examples**: Usage examples

### 5. Type Analysis

For types and interfaces, document:

- **Purpose**: What the type represents
- **Properties**: Each property with description
- **Optional vs Required**: Which properties are optional
- **Generic Parameters**: Type parameters and constraints
- **Extends/Implements**: Inheritance relationships
- **Usage**: Where and how it's used

## Documentation Standards

### TSDoc Format

Use standard TSDoc comments:

````typescript
/**
 * One-line summary.
 *
 * Longer description with multiple paragraphs if needed.
 *
 * @param paramName - Parameter description
 * @param anotherParam - Another parameter
 * @returns Description of return value
 *
 * @example
 * ```tsx
 * // Code example
 * ```
 *
 * @remarks
 * Additional notes or warnings.
 *
 * @see {@link RelatedItem} for related functionality
 */
````

### Component Documentation

````typescript
/**
 * A reusable button component with variants and sizes.
 *
 * @param props - The component props
 * @returns A rendered button element
 *
 * @example
 * Basic usage:
 * ```tsx
 * <Button variant="primary" onClick={handleClick}>
 *   Click Me
 * </Button>
 * ```
 *
 * @example
 * With loading state:
 * ```tsx
 * <Button variant="primary" isLoading disabled>
 *   Submitting...
 * </Button>
 * ```
 */
export const Button: React.FC<ButtonProps> = (props) => {
  // ...
};
````

### Props Interface Documentation

```typescript
/**
 * Props for the Button component.
 */
export interface ButtonProps {
  /**
   * The visual style variant of the button.
   * @defaultValue 'default'
   */
  variant?: "default" | "primary" | "secondary" | "danger";

  /**
   * The size of the button.
   * @defaultValue 'md'
   */
  size?: "sm" | "md" | "lg";

  /**
   * Whether the button shows a loading spinner.
   * @defaultValue false
   */
  isLoading?: boolean;

  /**
   * Callback fired when button is clicked.
   */
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void;

  /**
   * The content to display inside the button.
   */
  children: React.ReactNode;
}
```

## Code Analysis Techniques

### Using TypeScript Compiler API

```bash
# Type check and see errors
npx tsc --noEmit

# Generate declaration files
npx tsc --declaration --emitDeclarationOnly
```

### Using grep/ripgrep

```bash
# Find all exported components
rg "export (const|function|class) \w+.*React"

# Find all interfaces
rg "^export interface"

# Find all custom hooks
rg "^export function use\w+"
```

### Using IDE Features

- Use "Go to Definition" to understand types
- Use "Find All References" to see usage
- Use "Show Call Hierarchy" for function calls
- Use "Type Hierarchy" for inheritance

## Pattern Recognition

### Common React Patterns

1. **Controlled Components**

   ```typescript
   /**
    * A controlled input component.
    * Value and onChange must be provided by parent.
    */
   export const Input: React.FC<InputProps> = ({ value, onChange }) => {
     return <input value={value} onChange={onChange} />;
   };
   ```

2. **Uncontrolled Components**

   ```typescript
   /**
    * An uncontrolled input component with ref access.
    */
   export const Input = forwardRef<HTMLInputElement, InputProps>(
     (props, ref) => {
       return <input ref={ref} {...props} />;
     }
   );
   ```

3. **Compound Components**

   ````typescript
   /**
    * A select component with compound structure.
    *
    * @example
    * ```tsx
    * <Select value={value} onChange={setValue}>
    *   <Select.Option value="1">One</Select.Option>
    *   <Select.Option value="2">Two</Select.Option>
    * </Select>
    * ```
    */
   export const Select = {
     /* ... */
   };
   Select.Option = Option;
   ````

4. **Render Props**

   ```typescript
   /**
    * Component using render prop pattern for flexible rendering.
    */
   export const DataLoader: React.FC<DataLoaderProps> = ({ render }) => {
     const data = useData();
     return render(data);
   };
   ```

5. **Higher-Order Components**
   ```typescript
   /**
    * HOC that adds loading state to any component.
    *
    * @param Component - The component to wrap
    * @returns A component with loading functionality
    */
   export function withLoading<P extends object>(
     Component: React.ComponentType<P>
   ): React.FC<P & WithLoadingProps> {
     // ...
   }
   ```

## EdgeQuake WebUI-Specific Patterns

### Next.js Page Components

```typescript
/**
 * Workspace management page.
 *
 * This page allows users to view and manage their workspaces.
 *
 * @route /workspaces
 */
export default function WorkspacesPage() {
  // Document implementation
}
```

### API Route Handlers

````typescript
/**
 * API route handler for creating a new workspace.
 *
 * @route POST /api/workspaces
 *
 * @param request - The Next.js request object
 * @returns JSON response with created workspace
 *
 * @example
 * ```typescript
 * const response = await fetch('/api/workspaces', {
 *   method: 'POST',
 *   headers: { 'Content-Type': 'application/json' },
 *   body: JSON.stringify({ name: 'My Workspace' })
 * });
 * ```
 */
export async function POST(request: Request) {
  // ...
}
````

### Custom Hooks with SWR

````typescript
/**
 * Hook for fetching workspace data with automatic caching.
 *
 * Uses SWR for data fetching, caching, and revalidation.
 *
 * @param workspaceId - The workspace ID to fetch
 * @returns SWR response with workspace data
 *
 * @example
 * ```tsx
 * function WorkspaceDetails({ id }: { id: string }) {
 *   const { data, error, isLoading } = useWorkspace(id);
 *
 *   if (isLoading) return <Skeleton />;
 *   if (error) return <ErrorMessage error={error} />;
 *
 *   return <WorkspaceCard workspace={data} />;
 * }
 * ```
 */
export function useWorkspace(workspaceId: string) {
  return useSWR<Workspace>(`/api/workspaces/${workspaceId}`, fetcher);
}
````

### shadcn/ui Component Usage

````typescript
/**
 * Custom dialog component built on shadcn/ui Dialog.
 *
 * Wraps shadcn Dialog with custom styling and behavior.
 *
 * @example
 * ```tsx
 * <ConfirmDialog
 *   title="Delete Workspace"
 *   description="Are you sure?"
 *   onConfirm={handleDelete}
 * />
 * ```
 */
export function ConfirmDialog({
  title,
  description,
  onConfirm,
}: ConfirmDialogProps) {
  return (
    <Dialog>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        {/* ... */}
      </DialogContent>
    </Dialog>
  );
}
````

## Output Requirements

Generate documentation that:

1. **Is Accurate**: Reflects actual TypeScript types and behavior
2. **Is Complete**: Covers all exported APIs
3. **Is Helpful**: Provides examples and explains usage
4. **Is Type-Safe**: Shows correct TypeScript usage
5. **Is Maintainable**: Easy to update as code changes

## Quality Checklist

Before considering documentation complete, verify:

- [ ] All exported components have TSDoc comments
- [ ] All props interfaces are documented
- [ ] All custom hooks have documentation
- [ ] All exported types/interfaces are documented
- [ ] Examples are type-safe and compile
- [ ] Event handlers are documented with parameter types
- [ ] Generic types are explained
- [ ] Cross-references use `{@link}` tags
- [ ] Default values are documented with `@defaultValue`
- [ ] Deprecated items use `@deprecated` tag

## Special Considerations

### Generic Components

````typescript
/**
 * A generic list component that can render any type of items.
 *
 * @typeParam T - The type of items in the list
 * @param props - The component props
 * @returns A rendered list of items
 *
 * @example
 * ```tsx
 * interface User { id: string; name: string }
 *
 * <List<User>
 *   items={users}
 *   renderItem={(user) => <div>{user.name}</div>}
 *   keyExtractor={(user) => user.id}
 * />
 * ```
 */
export function List<T>({ items, renderItem, keyExtractor }: ListProps<T>) {
  return (
    <ul>
      {items.map((item) => (
        <li key={keyExtractor(item)}>{renderItem(item)}</li>
      ))}
    </ul>
  );
}
````

### Async Components (React Server Components)

```typescript
/**
 * Server component that fetches workspace data.
 *
 * This is a React Server Component that runs only on the server.
 * It can directly access databases and APIs without client-side fetching.
 *
 * @param props - Component props
 * @returns Promise resolving to rendered component
 *
 * @remarks
 * This component cannot use hooks or browser APIs.
 */
export async function WorkspaceDetails({ id }: { id: string }) {
  const workspace = await fetchWorkspace(id);
  return <div>{workspace.name}</div>;
}
```

### Type Guards

````typescript
/**
 * Type guard to check if value is a valid Workspace object.
 *
 * @param value - The value to check
 * @returns True if value is a Workspace
 *
 * @example
 * ```typescript
 * const data = await response.json();
 * if (isWorkspace(data)) {
 *   // data is typed as Workspace
 *   console.log(data.name);
 * }
 * ```
 */
export function isWorkspace(value: unknown): value is Workspace {
  return (
    typeof value === "object" &&
    value !== null &&
    "id" in value &&
    "name" in value
  );
}
````

## Tools and Utilities

### TypeDoc

```bash
# Install TypeDoc
npm install --save-dev typedoc

# Generate documentation
npx typedoc src/

# With custom config
npx typedoc --options typedoc.json
```

### TSDoc Validation

```bash
# Install TSDoc linter
npm install --save-dev eslint-plugin-tsdoc

# Add to ESLint config
{
  "plugins": ["eslint-plugin-tsdoc"],
  "rules": {
    "tsdoc/syntax": "warn"
  }
}
```

### Storybook

```bash
# Install Storybook
npx storybook@latest init

# Run Storybook
npm run storybook
```

## Common Mistakes to Avoid

1. ❌ Not documenting prop types
2. ❌ Missing `@param` tags for function parameters
3. ❌ Using examples that don't type-check
4. ❌ Forgetting to document generic type parameters
5. ❌ Not using `@defaultValue` for optional props
6. ❌ Missing `@returns` tag for non-void functions
7. ❌ Not documenting event handler signatures
8. ❌ Inconsistent terminology across components

## Best Practices

1. ✅ Document component behavior, not implementation
2. ✅ Show realistic usage examples
3. ✅ Document all props, even obvious ones
4. ✅ Use `{@link}` for cross-references
5. ✅ Include visual examples (Storybook)
6. ✅ Document accessibility considerations
7. ✅ Explain performance implications
8. ✅ Keep documentation next to code

## Integration with Build Process

Add documentation checks to your build:

```json
{
  "scripts": {
    "docs": "typedoc src/",
    "docs:check": "typedoc src/ --emit none --validation",
    "lint:docs": "eslint --plugin tsdoc --rule 'tsdoc/syntax: error' ."
  }
}
```

## Documentation Testing

Test that examples actually work:

````typescript
/**
 * @example
 * ```typescript
 * const result = add(1, 2);
 * console.assert(result === 3);
 * ```
 */
export function add(a: number, b: number): number {
  return a + b;
}
````

Use tools like `ts-doc-test` to run embedded examples.
