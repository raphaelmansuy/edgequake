## Role 

Your are GenAI expert and senior software engineer tasked with implementing edgequake.



## The problems we face 


1) Workspace Creation and First Use Desynchronization Issue

When we start the application from clean state no tenant / no workspace is present.

When user create a first workspace and try to use the Query page to create a first conversation, an errors occurs, I presume because of desynchronization between client and server about the workspace existence in the localStorage. We need a very robust handling of this case. Have a deep reflexion about all the possible edge cases here and how to handle them gracefully.

2) Fresh start in non authenticated mode

When user start the application from clean state and is not authenticated, we need to ensure that the application works flawlessly in non authenticated mode with no workspace created yet.

Currently in the WebUI -> the default tenant is not selected properly and this create issues when user try to create a first workspace. I want to ensure that is impossible to have no tenant selected when user is not authenticated.

For each Tenant a default workspace should be created automatically when user create a tenant. Ensure it is impossible to have no workspace selected. None Tenant and None Workspace should be impossible states in the application.


3) The URL of the application should reflect the current selected workspace.

4) We can use workspace uuid or slug in the URL to identify the current workspace.

5) Ensure we have a slug field in the workspace creation form to create human friendly URL. Imagine all the rules about slug generation, uniqueness, edition, etc. Slug are uniques per tenant. Imagine how to handle slug conflicts when user try to create a workspace with a slug that already exist in the tenant. Imagine the datamodel changes needed to add slug to workspace entity.

## You deliverables

You must deliver a full specification document that include:

- A deep analysis of the current issues with detailed root cause analysis for each of the 5 problems described above, with references to specific code modules, components, crates, files, functions, etc. in client and server side codebase.
- A full improvement plan that describe how to fix each of the 5 problems described above with detailed steps, references to specific code modules, components, crates, files, functions, etc. in client and server side codebase.
- A full implementation of the improvement plan with references to specific code modules, components, crates, files, functions, etc. in client and server side codebase.
- A full verification plan with manual testing steps using playwright to ensure the issues are fully fixed with evidence (screenshots, logs, etc.)

Write all your documents in the ./plan_improvement_workspace/ Directory

- Ensure business rules are documented clearly in the specification document with references to specific code modules, components, crates, files, functions, etc. in client and server side codebase. like R001: It is impossible to have no tenant selected when user is not authenticated. See edgequake_webui/src/stores/tenant/use-tenant-store.ts
- Ensure to have high signal, clear, concise, and actionable documentation.
- Ensure to have high signal ASCII architecture diagrams illustrating screen flows and component interactions.
- Ensure to think about all edge cases and document them clearly in the specification document.

## Steps to follow

- Fully understand and map the issue and the gap with a code review on the client and on the server side
- Then write an improvement plan that is actionable
- Then fully implement the plan

Ensure by manual testing with #playwright that the work is DONE with evidence, BE extremly careful with screenshot --> Compression your session to avoid saturationUse plan_ingestion_pipeline/ Directory to understand the effort on this

Use a scratchpad.md to write your thinking process and notes before writing the final documents in plan_improvement_workspace/ directory while you work on this task. This will be your working memory and utlimate scratchpad if you need to refer back to your thinking process if the session get interrupted.

Use a plan.md to track in a concise the work to be done and the progress you make while working on this task. This will help you keep track of your objectives and deliverables, very important if you crash or your memory get lost. 

If you take picture of the screen during your manual testing, store them in plan_improvement_workspace/evidence/ directory with meaningful names. Compress your session to avoid saturation, especially if you take many screenshots: if not you definitely will run out of space and die.

