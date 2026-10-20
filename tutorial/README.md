# Click through EdgeQuake

This is a picture tour of the EdgeQuake website. Every chapter is one screen, written in plain English. The published version lives at **[https://edgequake.com/tutorial/](https://edgequake.com/tutorial/)**. Original pictures were taken on the public demo at [https://demo.edgequake.com/](https://demo.edgequake.com/); Members & invites shots are from the v0.36.0 UI.

Open the demo, sign in with the username and password you were given, and follow the chapters in order. You can also jump to the screen you care about.

The counts in the pictures (15 documents, 13,200 entities, and so on) are what the demo showed on the day the pictures were taken. Your own install will show your own numbers.

## Words used in this tour

A **document** is a file you add, such as a PDF.

An **entity** is a thing the software picked out of that file: a person, a company, a product, a place, or an idea.

A **relationship** is a line between two entities, such as "this person works at this company".

A **chunk** is a short piece of a document. The software cuts each file into chunks before it reads them.

A **knowledge graph** is the map of those entities and the lines between them. The sidebar calls this map **Knowledge Graph**.

A **workspace** is one collection of documents and the map built from them. The demo collection is named **Default Workspace**.

A **tenant** is the organization that owns one or more workspaces. The demo organization is named **Default**.

A **model** is the language program that reads documents and writes answers. You will see names such as `gpt-5.6-terra` and `mistral-small-latest` on the screens.

## Chapters

1. [What this is](01-what-this-is.md) — what the website is for, before you click anything.
2. [Sign in and getting around](02-sign-in-and-getting-around.md) — the sign-in card, the sidebar, and the organization switcher.
3. [Dashboard](03-dashboard.md) — the home counts and the three shortcuts.
4. [Documents](04-documents.md) — the file list and the drop zone.
5. [One document](05-one-document.md) — the preview, the extracted text, and where each fact came from.
6. [Ask a question](06-ask-a-question.md) — the question box, the six ways to search, and an answer with sources.
7. [The map](07-the-map.md) — the knowledge graph, and what you see when you pick one dot.
8. [Extra knowledge](08-extra-knowledge.md) — glossaries you add by hand.
9. [Workspaces](09-workspaces.md) — which model reads the files, and the buttons that rebuild or delete.
10. [Pipeline](10-pipeline.md) — the queue that turns files into the map.
11. [Costs](11-costs.md) — what the model calls have cost.
12. [API Explorer](12-api-explorer.md) — the same product, as web requests a program can send.
13. [Settings](13-settings.md) — theme, language, connections, and the switches that change pictures and answers.
14. [Members and invites](https://edgequake.com/tutorial/14-members-and-invites/) — invite teammates, roles, and one-time sign-in links (v0.36.0).

## How the pages link

Each chapter starts with **Contents**, **Previous**, and **Next**. The first time a chapter mentions another screen, that mention is a link. Each chapter ends with **Keep going**, which points at the next chapter and one related chapter.

If you want the command-line recipes (upload with curl, pick a model, run Docker), those live in [docs/tutorials](../docs/tutorials/index.md). This book is only the website.
