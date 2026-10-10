---
title: TypeScript SDK quickstart
description: Install edgequake-sdk, upload a document, wait for indexing, and run a first query from Node or TypeScript.
---

# TypeScript SDK quickstart

This guide uploads one document, waits until it is indexed, and asks a question. You need Node.js 18+ and a running EdgeQuake server on port 8080.

## 1. Install

```bash
npm install edgequake-sdk@0.1.0
```

## 2. Upload and wait for indexing

```ts
import { EdgeQuake } from "edgequake-sdk";

const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });

const up = await client.documents.upload({
  content: "Marie Curie won two Nobel Prizes.",
  title: "Curie",
});
if (!up.task_id) {
  throw new Error(`Duplicate upload of ${up.duplicate_of}`);
}

let status = "pending";
while (status === "pending" || status === "processing") {
  const task = await client.tasks.get(up.task_id);
  status = task.status;
  await new Promise((r) => setTimeout(r, 1000));
}
if (status !== "indexed") {
  throw new Error(`Ingestion ended with status ${status}`);
}
```

## 3. Query

```ts
const res = await client.query.execute({
  query: "How many Nobel Prizes did Marie Curie win?",
  mode: "mix",
});
console.log(res.answer);
```

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
    Upload["documents.upload"] --> Dup{"task_id set?"}
    Dup -->|"no, duplicate"| Stop["Throw with duplicate_of"]
    Dup -->|"yes"| Poll["tasks.get until final status"]
    Poll --> Final{"status indexed?"}
    Final -->|"no: failed or cancelled"| Fail["Throw with status"]
    Final -->|"yes"| Query["query.execute with mode mix"]
    Query --> Answer["Print answer"]
```

Upload and polling come before the query. A duplicate or a failed task stops the script with a clear error.

Next: [TypeScript README](README.md) for the resource list, and [Document upload](../../api-reference/document-upload-quick-reference.md).
