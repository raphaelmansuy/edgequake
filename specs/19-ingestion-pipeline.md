## Role 

Your are GenAI expert and senior data engineer. You will help to create and maintain GenAI ingestion pipeline for Knowledge Graph system.

## Objective

Your objective is to design, implement, and optimize a robust ingestion pipelines that efficiently processes and integrates diverse data sources into a scalable Knowledge Graph. The pipeline should leverage GenAI capabilities to enhance data extraction, transformation, and loading (ETL) processes.

First map the territory by reading how edgequake/ ingestion-pipeline is designed and implemented. Fully document using ASCII diagrams the architecture, data flow, components, and interactions involved in the ingestion pipeline. Document the data models, schemas, and formats used throughout the pipeline. Documet how the GenAI components are integrated into the pipeline and their specific roles. Document how the lineage and provenance of ingested data is tracked and managed. Document the error handling, monitoring, and alerting mechanisms in place for the ingestion pipeline. Document the scalability and performance optimization strategies employed in the pipeline. Specifically focus on how GenAI is used to enhance the ingestion process: keyword extraction, entity recognition, relationship mapping, data enrichment etc, summarization, embedding generation etc.

You have access to lightrag/ legacy python implementation of ingestion pipeline. You will compare and contrast the two implementations and document the differences, pros and cons of each approach.


## Your objectives are:

Your mission is to design a full specification for a SOTA GenAI-powered ingestion pipeline for a Knowledge Graph system for edgequake/.

Mandatory objectives:

- Have a full data model that captures documents, chunks, lineage information about chunking (start/end line number), extracted entities, relationships, keywords etc.
- Design a modular architecture that allows easy integration of new data sources and GenAI models.
- Implement robust error handling and monitoring to ensure data integrity and pipeline reliability.
- Optimize for scalability to handle large volumes of data efficiently, use map reduce techniques where applicable.
- Ensure the ingestion is as fast as possible while maintaining high quality of extracted information with respect of LLM rate limits and cost.
- Ensure data provenance and lineage tracking throughout the ingestion process.
- Ensure cost of ingestion (tokens, API calls etc) is tracked in the data model.
- Ensure the strategy employed for chunking is well documented and configurable and tracked in the lineage information.
- Ensure the ingestion pipeline can handle multi-tenant data with namespace isolation at the API and storage level.
- Ensure metadata management for documents and graph entities (page numbers, source documents etc.) is well defined and implemented.
- Design the ingestion pipeline to support multi LLM providers with easy switching/configuration.
- Ensure the ingestion pipeline supports predefined Ontologic schema per workspace in the future.
- Design the ingestion pipeline to preserve lineage information for chunking (start/end line number).
- Design the ingestion pipeline to support suppression of documents and define what happens to the graph in such cases.
- Design the ingestion pipeline to support merging, deleting, and editing entities.
- Ensure the ingestion pipeline supports by-chunk keywords, relations, entities extracted.
- Design the ingestion pipeline and data model to support citations and chunk query retrieval.
- Design the ingestion pipeline to support evaluation suite for RAG systems (RAGAS, mlflow etc.)
- Design the ingestion pipeline to support a Metadata Layer schema in the graph: document extraction chunk linked to concepts and entities/edges.
- Design the ingestion pipeline to support speed up techniques (MapReduce, quota management).
- Design the ingestion pipeline to track and manage the cost of ingestion (tokens, API calls etc) in the data model.
- Design the ingestion pipeline to support multi-namespace queries in the future.
- Ensure Workspace and multi-tenancy is well managed in the ingestion pipeline.
- Ensure we have a correct that model to report the progress of ingestion (number of documents, chunks, entities, relationships ingested etc).
- Ensure we have updated API endpoints to monitor and manage the ingestion pipeline with a full visibility on the progress, errors, cost etc.

## Deliverables

A full specification document that includes:

- Architecture diagrams (ASCII) illustrating the ingestion pipeline components and data flow.
- Detailed data models and schemas used in the ingestion process.
- Description of GenAI integration points and their roles in the pipeline.
- Error handling, monitoring, and alerting strategies.
- Scalability and performance optimization techniques.
- Comparison document between edgequake/ Rust implementation and lightrag/ Python implementation of the ingestion pipeline, highlighting differences, pros, and cons.
- Recommendations for future improvements and enhancements to the ingestion pipeline.
- A comprehensive README.md file that outlines the ingestion pipeline design, implementation details, and usage instructions.
- A set of unit and integration tests to validate the ingestion pipeline functionality and reliability.
- Documentation on how to deploy and maintain the ingestion pipeline in a production environment.
- A full implementation plan that descrive how to implement the designed ingestion pipeline in edgequake/ codebase with reference to specific modules, crates, and components.
- All documentation should be clear, concise, high signal, and actionable and accessible to both technical and non-technical stakeholders.


Use rules numbers, feature numbers to improve clarity in the documentation such as R001, F001 etc.

Directory to write all your concises documents : ./plan_ingestion_pipeline/

## Process

Maintains a scratchpad file ./plan_ingestion_pipeline/scratchpad.md where you jot down your thoughts, ideas, and any relevant information you come across during your research. Write as often as possible to keep track of your progress, it your ultimate memory aid if the session is interrupted and your memory is lost.

Write and update your plan in ./plan_ingestion_pipeline/plan.md as you make progress. The plan.md is your main deliverable, ensure it is well structured, clear, and comprehensive. It helps you keep track of your objectives and deliverables.