## Role 

Your are GenAI expert and senior software engineer tasked with implementing the update of the edgequake_webui/ after the implementation of the new ingestion pipeline in edgequake/.

This ingestion pipelines add advanced GenAI-powered entity extraction, summarization, lineage tracking, and progress/cost monitoring features to the Knowledge Graph system. And WebUI needs to be updated to leverage these new capabilities.

## Objective

Your objective is to update the edgequake_webui/ to fully utilize and showcase the new features of the ingestion pipeline in edgequake/.

First map the territory by reading how edgequake/ ingestion-pipeline is designed and implemented and then read the edgequake_webui/ codebase to understand how it currently interacts with the ingestion pipeline and what changes are needed.

## Your objectives are:

Your mission is to design a full specification for a SLICK moder update of the the screen flows, API interactions, and data visualizations in edgequake_webui/ to leverage the new ingestion pipeline features.

- Document lineage tracking and provenance visualization for ingested documents and entities.
- Integrate real-time progress and cost monitoring into the ingestion job UI. (Web Sockets)
- Update API interactions to support new ingestion pipeline endpoints.
- Review all the existing ingestion-related / documents UI components and identify areas for enhancement.

Mandatory objectives:

- Have a full understanding of the ingestion pipeline design goals
etc.
- Ensure the WebUI design is aligned with the ingestion pipeline capabilities.
- Ensure lineage tracking is clearly represented in the UI.
- Ensure real-time progress ingestion / extraction is intuitive and informative.
- Ensure cost monitoring is transparent and actionable for users.
- Ensure all API interactions are robust and efficient.
- Ensure the overall user experience is seamless and engaging.
- Ensure every screen flow is documented with wireframes/mockups.
- Ensure the specification is clear, concise, high signal, and actionable.
- Ensure the specification is accessible to both technical and non-technical stakeholders.
- Ensure Typography, Color theory, and UX principles are applied to the design.
- Ensure Information Architecture best practices are followed.
- Ensure Accessibility standards are met.
- Ensure SLICK and minimalist Responsive Design principles are applied.
- Ensure reflexion what is fixed vs what is flexible in the implementation in each screen or component.
- Light and Dark mode considerations.
- Ensure all technical details are covered for implementation.

## Deliverables

A full specification document that includes:

- Architecture diagrams (ASCII) illustrating screen flows and component interactions.
- Wireframes or mockups for each updated screen flow or component.
- API interaction diagrams showing data flow between WebUI and ingestion pipeline.
- Error handling, monitoring, and alerting strategies.
- A comprehensive README.md file that outlines the ingestion pipeline design, implementation details, and usage instructions.
- A set of unit and e2e tests to validate the ingestion pipeline functionality and reliability.
- A full implementation plan that descrive how to implement the designed ingestion pipeline web ui and improvemnts of documents display in edgequake/ codebase with reference to specific modules, crates, and components.
- All documentation should be clear, concise, high signal, and actionable and accessible to both technical and non-technical stakeholders.


Use rules numbers, feature numbers to improve clarity in the documentation such as R001, F001 etc.

Directory to write all your concises documents : ./plan_ingestion_pipeline/

## Process

Maintains a scratchpad file ./plan_ingestion_pipeline/scratchpad.md where you jot down your thoughts, ideas, and any relevant information you come across during your research. Write as often as possible to keep track of your progress, it your ultimate memory aid if the session is interrupted and your memory is lost.

The documents already present are the first phases of the backend ingestion pipeline implementation plan. Use them as reference to understand the ingestion pipeline design and implementation.

Write and update your plan in ./plan_ingestion_pipeline/plan.md as you make progress. The plan.md is your main deliverable, ensure it is well structured, clear, and comprehensive. It helps you keep track of your objectives and deliverables.