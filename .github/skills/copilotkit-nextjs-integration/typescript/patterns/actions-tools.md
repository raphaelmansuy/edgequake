# Actions & Tools with useCopilotAction

Let the copilot trigger functions that mutate your app state or call backend APIs.

## Concept

**Actions** (or **Tools**) are functions the model can execute based on user requests.

```typescript
useCopilotAction({
  name: "actionName",
  description: "What this action does",
  parameters: [...], // Function parameters
  handler: async (params) => {
    // Your code to execute
    return "Result message";
  }
});
```

## Simple Example: Mark Task Done

```typescript
"use client";

import { useState } from "react";
import { useCopilotAction, useCopilotReadable } from "@copilotkit/react-core";
import { CopilotPopup } from "@copilotkit/react-ui";

interface Task {
  id: string;
  title: string;
  done: boolean;
}

export default function TasksPage() {
  const [tasks, setTasks] = useState<Task[]>([
    { id: "1", title: "Review PR", done: false },
    { id: "2", title: "Write tests", done: false },
    { id: "3", title: "Deploy", done: false }
  ]);

  // Share task list
  useCopilotReadable({
    description: "Current task list",
    value: {
      tasks,
      pendingCount: tasks.filter(t => !t.done).length
    }
  });

  // Action: Mark task as done
  useCopilotAction({
    name: "markTaskDone",
    description: "Mark a task as completed",
    parameters: [
      {
        name: "taskId",
        type: "string",
        required: true,
        description: "The ID of the task to mark as done"
      }
    ],
    handler: async (params: { taskId: string }) => {
      setTasks(tasks =>
        tasks.map(t =>
          t.id === params.taskId ? { ...t, done: true } : t
        )
      );
      const task = tasks.find(t => t.id === params.taskId);
      return `Marked "${task?.title}" as done`;
    }
  });

  return (
    <div>
      <h2>Tasks</h2>
      {tasks.map(task => (
        <div key={task.id}>
          <input type="checkbox" checked={task.done} disabled />
          <span style={{ textDecoration: task.done ? "line-through" : "none" }}>
            {task.title}
          </span>
        </div>
      ))}
      <CopilotPopup instructions="Help manage tasks. You can mark tasks as done." />
    </div>
  );
}
```

**User can say:** "Mark the first task as done" or "Complete the review PR task"

## Multiple Parameters

```typescript
useCopilotAction({
  name: "sendEmail",
  description: "Send an email to a user",
  parameters: [
    {
      name: "to",
      type: "string",
      required: true,
      description: "Recipient email address"
    },
    {
      name: "subject",
      type: "string",
      required: true,
      description: "Email subject"
    },
    {
      name: "body",
      type: "string",
      required: true,
      description: "Email body/message"
    }
  ],
  handler: async (params: { to: string; subject: string; body: string }) => {
    const response = await fetch("/api/send-email", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params)
    });

    if (!response.ok) {
      throw new Error("Failed to send email");
    }

    return `Email sent to ${params.to}`;
  }
});
```

## Complex Action: Update User

```typescript
useCopilotAction({
  name: "updateUser",
  description: "Update a user's information",
  parameters: [
    { name: "userId", type: "string", required: true },
    { name: "name", type: "string", required: false },
    { name: "email", type: "string", required: false }
  ],
  handler: async (params: {
    userId: string;
    name?: string;
    email?: string;
  }) => {
    // Validate
    if (!params.userId) throw new Error("userId is required");

    // Call backend
    const response = await fetch(`/api/users/${params.userId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        ...(params.name && { name: params.name }),
        ...(params.email && { email: params.email })
      })
    });

    if (!response.ok) {
      throw new Error(`Failed to update user: ${response.statusText}`);
    }

    const updated = await response.json();
    return `Updated user ${updated.name}`;
  }
});
```

## Action with State Update

```typescript
"use client";

import { useState } from "react";
import { useCopilotAction } from "@copilotkit/react-core";

interface User {
  id: string;
  name: string;
  status: "online" | "offline";
}

export default function UsersPage() {
  const [users, setUsers] = useState<User[]>([
    { id: "1", name: "Alice", status: "online" },
    { id: "2", name: "Bob", status: "offline" }
  ]);

  useCopilotAction({
    name: "toggleUserStatus",
    description: "Toggle a user's online status",
    parameters: [
      { name: "userId", type: "string", required: true }
    ],
    handler: async (params: { userId: string }) => {
      setUsers(users =>
        users.map(u =>
          u.id === params.userId
            ? { ...u, status: u.status === "online" ? "offline" : "online" }
            : u
        )
      );

      const user = users.find(u => u.id === params.userId);
      const newStatus = user?.status === "online" ? "offline" : "online";
      return `${user?.name} is now ${newStatus}`;
    }
  });

  return (
    <div>
      {users.map(user => (
        <div key={user.id}>
          {user.name} - {user.status}
        </div>
      ))}
    </div>
  );
}
```

## Form Action

```typescript
useCopilotAction({
  name: "submitForm",
  description: "Submit a contact form",
  parameters: [
    { name: "name", type: "string", required: true },
    { name: "email", type: "string", required: true },
    { name: "message", type: "string", required: true }
  ],
  handler: async (params: {
    name: string;
    email: string;
    message: string;
  }) => {
    // Validation
    if (!params.email.includes("@")) {
      throw new Error("Invalid email address");
    }

    // Submit
    const response = await fetch("/api/contact", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params)
    });

    if (!response.ok) {
      throw new Error("Failed to submit form");
    }

    return "Form submitted successfully";
  }
});
```

## Best Practices

### ✅ Clear Parameter Descriptions

```typescript
// GOOD
parameters: [
  {
    name: "userId",
    type: "string",
    required: true,
    description: "The unique identifier of the user to update"
  }
]

// BAD
parameters: [
  { name: "id", type: "string", required: true }
]
```

### ✅ Validate Parameters

```typescript
handler: async (params: { email: string; quantity: number }) => {
  // Validate email format
  if (!params.email.includes("@")) {
    throw new Error("Invalid email format");
  }

  // Validate quantity
  if (params.quantity < 1 || params.quantity > 100) {
    throw new Error("Quantity must be between 1 and 100");
  }

  // Execute
  return "Done";
}
```

### ✅ Provide Meaningful Return Messages

```typescript
// GOOD: Clear what happened
return `Created order #${orderId} for ${customerName}`;

// BAD: Vague
return "Done";
```

### ✅ Handle Errors Gracefully

```typescript
handler: async (params) => {
  try {
    const response = await fetch("/api/something", {
      method: "POST",
      body: JSON.stringify(params)
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.statusText}`);
    }

    return "Success";
  } catch (error) {
    // Re-throw with user-friendly message
    throw new Error(`Failed to process: ${error.message}`);
  }
}
```

### ✅ Use Consistent Naming

```typescript
// GOOD: verb + noun
useCopilotAction({ name: "createTask", ... })
useCopilotAction({ name: "deleteUser", ... })
useCopilotAction({ name: "sendNotification", ... })

// BAD: unclear
useCopilotAction({ name: "action1", ... })
useCopilotAction({ name: "doStuff", ... })
```

## Common Patterns

### Optional Parameters

```typescript
parameters: [
  { name: "userId", type: "string", required: true },
  { name: "notifyUser", type: "boolean", required: false } // Optional
],
handler: async (params: { userId: string; notifyUser?: boolean }) => {
  // Handle optional param
  if (params.notifyUser !== false) {
    await sendNotification(params.userId);
  }
  return "Done";
}
```

### Number Parameters

```typescript
parameters: [
  { name: "quantity", type: "number", required: true },
  { name: "discountPercent", type: "number", required: false }
],
handler: async (params: { quantity: number; discountPercent?: number }) => {
  const discount = params.discountPercent || 0;
  const finalPrice = calculatePrice(params.quantity, discount);
  return `Price: $${finalPrice}`;
}
```

### Boolean Flags

```typescript
parameters: [
  { name: "itemId", type: "string", required: true },
  { name: "archive", type: "boolean", required: true }
],
handler: async (params: { itemId: string; archive: boolean }) => {
  if (params.archive) {
    await archiveItem(params.itemId);
  } else {
    await unarchiveItem(params.itemId);
  }
  return params.archive ? "Archived" : "Unarchived";
}
```

## Complete Example: Task Manager

```typescript
"use client";

import { useState } from "react";
import { useCopilotAction, useCopilotReadable } from "@copilotkit/react-core";
import { CopilotPopup } from "@copilotkit/react-ui";

interface Task {
  id: string;
  title: string;
  priority: "low" | "medium" | "high";
  done: boolean;
}

export default function TaskManagerPage() {
  const [tasks, setTasks] = useState<Task[]>([
    { id: "1", title: "Review PR", priority: "high", done: false },
    { id: "2", title: "Write tests", priority: "medium", done: false }
  ]);

  useCopilotReadable({
    description: "Task list with priority levels",
    value: {
      tasks,
      highPriority: tasks.filter(t => t.priority === "high" && !t.done),
      completed: tasks.filter(t => t.done).length
    }
  });

  // Add a new task
  useCopilotAction({
    name: "addTask",
    description: "Add a new task to the list",
    parameters: [
      { name: "title", type: "string", required: true },
      { name: "priority", type: "string", required: false }
    ],
    handler: async (params: { title: string; priority?: string }) => {
      const newTask: Task = {
        id: Date.now().toString(),
        title: params.title,
        priority: (params.priority as any) || "medium",
        done: false
      };
      setTasks(tasks => [...tasks, newTask]);
      return `Added task: "${params.title}"`;
    }
  });

  // Mark task as done
  useCopilotAction({
    name: "completeTask",
    description: "Mark a task as completed",
    parameters: [
      { name: "taskId", type: "string", required: true }
    ],
    handler: async (params: { taskId: string }) => {
      setTasks(tasks =>
        tasks.map(t =>
          t.id === params.taskId ? { ...t, done: true } : t
        )
      );
      return "Task marked as done";
    }
  });

  return (
    <div>
      <h2>Tasks</h2>
      {tasks.map(task => (
        <div key={task.id} style={{ opacity: task.done ? 0.5 : 1 }}>
          [{task.priority}] {task.title} {task.done && "✓"}
        </div>
      ))}
      <CopilotPopup instructions="Help manage tasks. You can add tasks and mark them as done." />
    </div>
  );
}
```

## Testing

```typescript
import { renderHook, act } from "@testing-library/react";
import { useCopilotAction } from "@copilotkit/react-core";

describe("useCopilotAction", () => {
  it("should call handler with correct parameters", async () => {
    const mockHandler = jest.fn();

    renderHook(() =>
      useCopilotAction({
        name: "testAction",
        parameters: [{ name: "param1", type: "string", required: true }],
        handler: mockHandler
      })
    );

    await act(async () => {
      await mockHandler({ param1: "value" });
    });

    expect(mockHandler).toHaveBeenCalledWith({ param1: "value" });
  });
});
```

## Next Steps

- Combine state + actions for full workflows
- Add parameter validation from state (see [production-security.md](production-security.md))
- Build human-in-the-loop approval flows
