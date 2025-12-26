# State Sharing with useCopilotReadable

Expose app state to the LLM so it can provide context-aware answers.

## Concept

**Readable State** is data from your app that you share with the copilot. The model sees it and can reason about it.

```typescript
useCopilotReadable({
  description: "What is this data?",
  value: dataObject // Must be JSON-serializable
});
```

## Simple Example: User List

```typescript
"use client";

import { CopilotPopup } from "@copilotkit/react-ui";
import { useCopilotReadable } from "@copilotkit/react-core";

const USERS = [
  { id: "1", name: "Alice", email: "alice@example.com", role: "Admin" },
  { id: "2", name: "Bob", email: "bob@example.com", role: "User" },
  { id: "3", name: "Charlie", email: "charlie@example.com", role: "User" }
];

export default function UserListPage() {
  // Share the user list with the model
  useCopilotReadable({
    description: "List of users in the system",
    value: USERS
  });

  return (
    <div>
      <h2>Users</h2>
      <ul>
        {USERS.map(user => (
          <li key={user.id}>{user.name} ({user.email}) - {user.role}</li>
        ))}
      </ul>
      <CopilotPopup instructions="Answer questions about the user list." />
    </div>
  );
}
```

**User can ask:**
- "How many users are there?"
- "Who is the admin?"
- "What is Alice's email?"

**Model can answer** because it sees the `USERS` data.

## Dynamic State

```typescript
"use client";

import { useState } from "react";
import { useCopilotReadable } from "@copilotkit/react-core";
import { CopilotPopup } from "@copilotkit/react-ui";

export default function DashboardPage() {
  const [count, setCount] = useState(0);
  const [isLoading, setIsLoading] = useState(false);

  // Share current state
  useCopilotReadable({
    description: "Current dashboard state",
    value: {
      counter: count,
      isLoading,
      status: isLoading ? "loading" : "ready"
    }
  });

  return (
    <div>
      <h2>Counter: {count}</h2>
      <button onClick={() => setCount(count + 1)}>Increment</button>
      {isLoading && <p>Loading...</p>}
      <CopilotPopup instructions="Monitor and comment on the counter state." />
    </div>
  );
}
```

## Multiple State Sources

Share multiple pieces of state:

```typescript
"use client";

import { useCopilotReadable } from "@copilotkit/react-core";

const USER = { id: "123", name: "Alice", email: "alice@example.com" };
const TASKS = [
  { id: "1", title: "Review PR", done: false },
  { id: "2", title: "Update docs", done: true }
];

export default function DashboardPage() {
  // Share user info
  useCopilotReadable({
    description: "Current user information",
    value: USER
  });

  // Share tasks
  useCopilotReadable({
    description: "User's task list",
    value: {
      tasks: TASKS,
      completedCount: TASKS.filter(t => t.done).length,
      pendingCount: TASKS.filter(t => !t.done).length
    }
  });

  return (
    <div>
      <p>Welcome, {USER.name}</p>
      <p>You have {TASKS.filter(t => !t.done).length} pending tasks.</p>
    </div>
  );
}
```

## Best Practices

### ✅ Keep Data Minimal

```typescript
// GOOD: Only essential data
useCopilotReadable({
  description: "Current page",
  value: {
    pageName: "Dashboard",
    itemCount: 42
  }
});
```

```typescript
// BAD: Too much data (slows down model)
useCopilotReadable({
  description: "All app data",
  value: entireAppState // Don't do this
});
```

### ✅ Make Descriptions Clear

```typescript
// GOOD
useCopilotReadable({
  description: "List of active users with their roles",
  value: activeUsers
});

// BAD
useCopilotReadable({
  description: "data",
  value: users
});
```

### ✅ Ensure Data is JSON-Serializable

```typescript
// GOOD: Plain objects and arrays
useCopilotReadable({
  value: { name: "Alice", age: 30, active: true }
});

// BAD: Functions, Dates, Map, Set
useCopilotReadable({
  value: {
    createdAt: new Date(), // ❌ Not serializable
    handler: () => {}, // ❌ Function
    cache: new Map() // ❌ Not serializable
  }
});

// FIXED: Convert to serializable types
useCopilotReadable({
  value: {
    createdAt: new Date().toISOString(), // ✅ String
    hasHandler: true, // ✅ Boolean
    cacheSize: cache.size // ✅ Number
  }
});
```

### ✅ Update State Reactively

```typescript
"use client";

import { useEffect, useState } from "react";
import { useCopilotReadable } from "@copilotkit/react-core";

export default function LiveDataPage() {
  const [price, setPrice] = useState(0);

  // useCopilotReadable always sees latest price
  useCopilotReadable({
    description: "Current stock price",
    value: { price, lastUpdate: new Date().toISOString() }
  });

  useEffect(() => {
    // Simulate live updates
    const interval = setInterval(() => {
      setPrice(Math.random() * 100);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  return <div>Price: ${price.toFixed(2)}</div>;
}
```

## Advanced: Conditional State

```typescript
"use client";

import { useCopilotReadable } from "@copilotkit/react-core";

export default function AdminPage() {
  const isAdmin = true; // Would be checked from auth

  // Only share admin data if user is admin
  if (isAdmin) {
    useCopilotReadable({
      description: "Admin panel state (email, passwords, etc)",
      value: sensitiveData
    });
  }

  return <div>Admin Panel</div>;
}
```

## Example: E-commerce Cart

```typescript
"use client";

import { useState } from "react";
import { useCopilotReadable } from "@copilotkit/react-core";
import { CopilotPopup } from "@copilotkit/react-ui";

interface CartItem {
  id: string;
  name: string;
  price: number;
  quantity: number;
}

export default function CartPage() {
  const [items, setItems] = useState<CartItem[]>([
    { id: "1", name: "Laptop", price: 999, quantity: 1 },
    { id: "2", name: "Mouse", price: 29, quantity: 2 }
  ]);

  const total = items.reduce((sum, item) => sum + item.price * item.quantity, 0);

  useCopilotReadable({
    description: "Shopping cart contents and totals",
    value: {
      items: items.map(item => ({
        name: item.name,
        price: item.price,
        quantity: item.quantity,
        subtotal: item.price * item.quantity
      })),
      itemCount: items.length,
      totalPrice: total,
      currency: "USD"
    }
  });

  return (
    <div>
      <h2>Cart ({items.length} items)</h2>
      <ul>
        {items.map(item => (
          <li key={item.id}>
            {item.name} × {item.quantity} = ${item.price * item.quantity}
          </li>
        ))}
      </ul>
      <h3>Total: ${total.toFixed(2)}</h3>
      <CopilotPopup instructions="Help with the shopping cart. You can see the items and prices." />
    </div>
  );
}
```

## Testing

```typescript
// __tests__/readable-state.test.ts
import { renderHook } from "@testing-library/react";
import { useCopilotReadable } from "@copilotkit/react-core";

describe("useCopilotReadable", () => {
  it("should expose state to copilot", () => {
    const testData = { count: 5, name: "Test" };
    const { result } = renderHook(() =>
      useCopilotReadable({
        description: "Test state",
        value: testData
      })
    );
    // State is shared with copilot
  });
});
```

## Troubleshooting

| Issue | Solution |
| --- | --- |
| Model doesn't know about my state | Make sure `useCopilotReadable` is called in a `"use client"` component |
| Data shows as `[object Object]` | Ensure data is JSON-serializable (no Functions, Dates, Map, Set) |
| State updates aren't reflected | State is reactive - ensure React state updates trigger re-render |
| Too much data slowing model | Reduce the amount of data shared; focus on essential fields only |

## Next Steps

- Add actions with `useCopilotAction` (see [actions-tools.md](actions-tools.md))
- Combine state + actions for full automation
- See [production-security.md](production-security.md) for validation
