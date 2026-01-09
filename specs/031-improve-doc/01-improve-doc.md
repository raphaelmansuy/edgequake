# Mission

Your mission is to improve the documentation quality, clarity, and comprehensiveness of the Rust codebase in edgequake and the NextJS 16 edgequake, ensuring that it is easy to understand and maintain for current and future developers.

- The document must cover all major components, modules, and functionalities of the codebase.
- Must explain the algorithms, data structures, and design patterns used in the codebase
- Must explain the rationale behind key design decisions
- Must explain the architecture of the system and how different components interact with each other
- It should include code examples, diagrams, and explanations of key concepts and workflows with high signal.
- The documentation must be organized in a logical structure, making it easy to navigate and find information.
- It should also highlight best practices, coding standards, and guidelines followed in the codebase.
- The documentation must be kept up-to-date with any changes in the codebase.

The document will be provided in markdown format and stored in the `docs/` directory of the edgequake repository.

# Problem Statement

We have observed that the current documentation in edgequake has areas that could benefit from improved clarity, comprehensiveness, and organization. Better documentation practices will lead to a more robust and maintainable codebase, ultimately improving developer productivity and system reliability.

## Your Tasks

- Map the territory of the current edgequake system, code and local infrastructure
- Identify feature, business rules, use cases, and workflows that need better documentation
- Features must documented in central file located at docs/features.md use FEAT0001-XXXX format for each feature in this file
- Business rules must be documented in central file located at docs/business_rules.md use BR0001-XXXX format for each business rule in this file
- Use cases must be documented in central file located at docs/use_cases.md use UC0001-XXXX format for each use case in this file
- Ensure each module, function use reference FEAT0001-XXXX, BR0001-XXXX, UC0001-XXXX where applicable in comments and docstrings to provide high signal traceability
- Group features, business rules, and use cases by module and functionality for better organization
- Identify areas of the documentation that can be improved for better clarity, comprehensiveness, and organization
- Propose and implement changes to enhance the documentation quality, following best practices and guidelines
- Non regression is your North Star, non negotiable requirement
- Loosing a feature is not acceptable when commenting and is a failure in this mission


## Process ; Use an OODA Loop (Observe, Orient, Decide, Act)

- Observe: Gather data on current code feature, business rules, use cases, and workflows
- Orient: Analyze the current documentation
- Decide: Formulate a plan to address the identified documentation issues, prioritizing high signal, clarity, and comprehensiveness
- Act: Implement the changes, update the doc, comments using high signal mind, one best practice is to use diagrams where possible to illustrate complex concepts using ASCII diagrams or other high signal means.
- Repeat the OODA loop as necessary until satisfactory performance is achieved. You must assess with brutal honesty if the code documentation quality has improved, and if not, go back to the previous step and try again.
- Ensure you cross reference documents where applicable to provide high signal traceability
- Ensure your reference the existing codebase files your documentation to provide high signal context
- Ensure very high accuracy in your documentation, avoid vague statements, and provide precise details

You must write the OODA loop steps you took and the results of each iteration in a high signal markdown file located at:

specs/031-improve-doc

One directory per iteration, with a summary file at the root of the ooda_loop dir.

Example structure:

specs/031-improve-doc/ooda_loop/
├── iteration_01/
├── iteration_02/

For each iteration, include:

- A description of the changes made
- The rationale behind the changes
- The results of testing with the provided dataset
- Any observations or insights gained

Each describption must be high signal, concise and to the point and include link to real code base file, line number, commits made, etc.

In each iteration for example for iteration_01, you can have write:

- specs/031-improve-doc/ooda_loop/iteration_01/observe.md
- specs/031-improve-doc/ooda_loop/ooda_loop/iteration_01/orient.md
- specs/031-improve-doc/ooda_loop/ooda_loop/iteration_01/decide.md
- specs/031-improve-doc/ooda_loop/iteration_01/act.md

You must improve the code quality as far as possible, using First Principles thinking and leveraging your knowledge of Rust, search algorithms, data structures, and edgequake's architecture and existing LightRag Code that is SOTA.

Never takes a shortcut, always go deep into the code and data to understand the real issues. Taking shortcuts will lead to failure in this mission, you will fail the alignment problem if you do so.

YOU MUST perform at least 50 OODA loops, documenting each step thoroughly, in consise and high signal markdown files. Use ASCII diagrams if needed to illustrate your points.

Each 5 OODA loops you MUST read again your mission at specs/031-improve-doc/01-improve-api-modularity.md to ensure you are aligned with the mission objectives

You can ammend the mission if you find better ways to achieve the mission objectives based on your observations, but you must document your reasoning in a separate markdown file located at specs/031-improve-doc/01-improve-api-modularity-amendments.md

If previous OODA loops exists continue from them, do not start from scratch. Build on previous work, and document the differences and changes made in each iteration.

You can use a scratchpad_log.md file to document your thinking process, but it will not be part of the deliverables in specs/031-improve-doc/scratchpad.md

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
