---
title: TypeScript SDK quickstart
description: Install edgequake-sdk, upload a document, and run a first query from Node or TypeScript.
---

# TypeScript SDK quickstart

You need Node 18+ and a running EdgeQuake server.

```bash
npm install edgequake-sdk@0.1.0
```

```ts
import { EdgeQuake } from "edgequake-sdk";

const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });

const up = await client.documents.upload({
  content: "Marie Curie won two Nobel Prizes.",
  title: "Curie",
});

let status = "pending";
while (status === "pending" || status === "processing") {
  const task = await client.tasks.get(up.task_id!);
  status = task.status;
  await new Promise((r) => setTimeout(r, 1000));
}

const res = await client.query.execute({
  query: "How many Nobel Prizes?",
  mode: "mix",
});
console.log(res.answer);
```

Next: [TypeScript README](README.md), [API reference](../../api-reference/index.md).
