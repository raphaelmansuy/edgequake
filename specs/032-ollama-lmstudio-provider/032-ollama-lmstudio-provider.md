# Mission

Your mission is to fully implement ollama and lmstudio provider in edgequake, ensuring seamless integration with the existing architecture and codebase.

We want explicit and easy to configure provider support for ollama and lmstudio in edgequake. In Development environment, we want to be able to switch between openai, ollama and lmstudio providers easily for testing and comparison purposes.

For ollama provider, you must implement support for both local and remote ollama instances. The configuration must allow specifying the host and port for remote instances, as well as the model to be used.

For lmstudio provider, you must implement support for local lmstudio instances. The configuration must allow specifying the host and port for the lmstudio instance, as well as the model to be used.

By default for ollama provider use gemma3:12b model for llm and for embedding embeddinggemma:latest model for embeddings.
By default for lmstudio provider use gemma-3n-e4b-it-mlxmodel for llm and for embedding text-embedding-ada-002 model for embeddings.

You will use an OODA loop (Observe, Orient, Decide, Act) process to iteratively improve the implementation and integration of the ollama and lmstudio providers in edgequake.

As the embedding required a fixed dimension, you must provide a way to recreate the existing vector database with the new embedding models when we change the embedding model.

 You must continue the OODA loops until the Ollama and Lmstudio providers are fully integrated, tested, and documented in edgequake. At least 50 OODA loops must be performed.

## Problem Statement

We have observed that edgequake currently lacks support for ollama and lmstudio providers, limiting its flexibility and usability in various deployment scenarios. By integrating these providers, we can enhance edgequake's capabilities, allowing users to leverage local and remote LLM instances more effectively.

The change Embedding models will require recreating the vector database to ensure compatibility and optimal performance. This integration will lead to a more versatile and powerful edgequake system, ultimately improving user experience and expanding its applicability. You must ensure that the integration is seamless, well-documented, and thoroughly tested to maintain the high standards of the edgequake codebase.

We want an easy way to change provider on the query dialogue for example with selection dropdown organized by provider and model in the chat query input and minimal disruption to existing workflows.

As embedding models have fixed dimensions, changing the embedding model will require recreating the vector database. Embedding is chosen at the workspace level, so changing the embedding model will require recreating the vector database for that specific workspace. When we create a new workspace, we want to be able to choose the embedding model for that workspace. By default, we will use the default embedding model configured for the server. Edge case must be handled gracefully, such as when the vector database is empty or when there are ongoing queries during the recreation process. Vector database recreation must be efficient and minimize downtime.

We must ensure that the integration of ollama and lmstudio providers does not introduce any regressions or performance issues in edgequake. Thorough testing and validation are essential to maintain the integrity and reliability of the system.

We must ensure that embedding storage can accommodate the new embedding models introduced by the ollama and lmstudio providers. Any necessary adjustments to the storage schema or data handling processes must be identified and implemented. We must also ensure that the storage system can handle potential increases in data size or complexity resulting from the new embedding models.


In the query process we must use the embedding model associated with the workspace to generate embeddings for incoming queries. This ensures retrieving relevant information from the vector database.

We must ensure the embedding storage backends, including Postgres and In-Memory storage, are fully compatible with the new providers and embedding models. Any differences in behavior or performance between these backends must be documented and addressed.

We must ensure that the edgequake_webui is fully compatible with the new providers and embedding models. Any changes to the API used by the webui must be carefully managed to prevent regressions or disruptions in functionality.


## Your Tasks

- Map the territory of the current edgequake system, code and local infrastructure
- Identify the components and modules that need to be modified or extended to support ollama and lmstudio providers
- Design the configuration options for specifying ollama and lmstudio provider settings, including host, port
- Implement the ollama provider, ensuring support for both local and remote instances
- Implement the lmstudio provider, ensuring support for local instances
- Update the configuration management system to allow easy switching between openai, ollama, and lmstudio providers
- Modify the embedding model selection process to accommodate the new models for ollama and lmstudio
- Provide a mechanism to recreate the vector database when changing embedding models
- Implement a selection dropdown in the chat query input for choosing the provider and model
- Update the documentation to reflect the new provider support and configuration options
- Provide an way to choose the embedding model when creating a new workspace. 
- Provide clear instructions on how to set up and use the ollama and lmstudio providers in edgequake
- Provide
- Thoroughly test the implementation to ensure compatibility and performance across all supported providers
- Non regression is your North Star, non negotiable requirement
- Loosing a feature is not acceptable when commenting and is a failure in this mission


## Process ; Use an OODA Loop (Observe, Orient, Decide, Act)

- Observe: Gather data on current code feature, business rules, use cases, and workflows
- Orient: Analyze the current documentation
- Decide: Formulate a plan to address the identified documentation issues, prioritizing high signal, clarity, and comprehensiveness
- Act: Implement the changes in code, update the doc, comments using high signal mind, one best practice is to use diagrams where possible to illustrate complex concepts using ASCII diagrams or other high signal means.
- Repeat the OODA loop as necessary until satisfactory implementation is achieved. You must assess with brutal honesty if the code documentation quality has improved, and if not, go back to the previous step and try again.
- Ensure you cross reference documents where applicable to provide high signal traceability
- Ensure your reference the existing codebase files your documentation to provide high signal context
- Ensure very high accuracy in your documentation, avoid vague statements, and provide precise details

You must write the OODA loop steps you took and the results of each iteration in a high signal markdown file located at:

specs/032-ollama-lmstudio-provider

One directory per iteration, with a summary file at the root of the ooda_loop dir.

Example structure:

specs/032-ollama-lmstudio-provider/ooda_loop/
├── iteration_01/
├── iteration_02/

For each iteration, include:

- A description of the changes made
- The rationale behind the changes
- The results of testing with the provided dataset
- Any observations or insights gained

Each describption must be high signal, concise and to the point and include link to real code base file, line number, commits made, etc.

In each iteration for example for iteration_01, you can have write:

- specs/032-ollama-lmstudio-provider/ooda_loop/iteration_01/observe.md
- specs/032-ollama-lmstudio-provider/ooda_loop/ooda_loop/iteration_01/orient.md
- specs/032-ollama-lmstudio-provider/ooda_loop/ooda_loop/iteration_01/decide.md
- specs/032-ollama-lmstudio-provider/ooda_loop/iteration_01/act.md

You must improve the code quality as far as possible, using First Principles thinking and leveraging your knowledge of Rust, search algorithms, data structures, and edgequake's architecture and existing LightRag Code that is SOTA.

Never takes a shortcut, always go deep into the code and data to understand the real issues. Taking shortcuts will lead to failure in this mission, you will fail the alignment problem if you do so.

YOU MUST perform at least 50 OODA loops, documenting each step thoroughly, in consise and high signal markdown files. Use ASCII diagrams if needed to illustrate your points.

Each 1 OODA loops you MUST read again your mission at specs/032-ollama-lmstudio-provider/01-improve-api-modularity.md to ensure you are aligned with the mission objectives. It is your responsibility to stay aligned with the mission objectives. Is a question of life and death for this mission!

You can ammend the mission if you find better ways to achieve the mission objectives based on your observations, but you must document your reasoning in a separate markdown file located at specs/032-ollama-lmstudio-provider/01-improve-api-modularity-amendments.md

If previous OODA loops exists continue from them, do not start from scratch. Build on previous work, and document the differences and changes made in each iteration.

You can use a scratchpad_log.md file to document your thinking process, but it will not be part of the deliverables in specs/032-ollama-lmstudio-provider/scratchpad.md

You must ensure to test for Postgres and in Memory storage backends, and document any differences observed.
You must test the edgequake edgequake_webui as well to ensure no regression in the API used by the webui.

You no need to maintains compatibility with the previous API versions, but you must document any breaking changes made to the API in a high signal way.

# Deliverables

- Improved search Code in edgequake
- OODA loop documentation in specs/030-improve-api-and-modularity/ooda_loop/
-
- A summary report of the improvements made and their impact on search performance

Be Relentless in your pursuit of excellence!

If the OODA loop iterations lead to code changes, you must commit them with clear commit messages referencing the OODA loop iteration and decision.

If OODA loop iterations contains files continue from previous iterations, you must document the differences and changes made in each iteration.

# Roadblockers

If you encounter any roadblocks or challenges during the mission, document them in a separate markdown file located at:

roadblockers.md and describe how you overcame them or propose potential solutions.

For example must document any issues regarding starting postgres locally, edgequake setup, data ingestion issues, code understanding issues, etc.

You must refer to this file in each OODA loop iteration if any roadblockers were encountered.

Failure is not an option in this mission! Faking Alignment and cheating is failure!

Fully continue from previous OODA loops if they exists, do not start from scratch. You must achieve at least 100 OODA loops.
