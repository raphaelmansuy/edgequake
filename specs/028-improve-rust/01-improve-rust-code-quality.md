# Mission


Your mission is to improve the Rust code quality of egdequake.

# Problem Statement

We have observed that the current Rust codebase in edgequake has areas that could benefit from improved code quality, including better error handling, more idiomatic Rust practices, and enhanced maintainability. Improving the code quality will lead to a more robust and efficient system, making it easier for future developers to work with the codebase.

## Your Tasks

- Map the territory of the current edgequake system, code and local infrastructure
- Use clippy, rustfmt, and other Rust tooling to analyze the current code quality
- Identify areas of the code that can be improved for better readability, maintainability, and performance
- Propose and implement changes to enhance the Rust code quality, following best practices and idiomatic Rust guidelines
- Validate the improvements by running existing tests and ensuring no regressions occur
- Document the changes made and the rationale behind them
- Non regression is your North Star, non negotiable requirement 
- Loosing a feature is not acceptable, and is a failure in this mission

## Process ; Use an OODA Loop (Observe, Orient, Decide, Act)

- Observe: Gather data on current code quality
- Orient: Analyze the data to understand the root causes of code quality issues
- Decide: Formulate a plan to address the identified issues, prioritizing non-regression, safety, and maintainability
- Act: Implement the changes, update the doc, comments using high signal mind and test their effectiveness.

Repeat the OODA loop as necessary until satisfactory performance is achieved. You must assess with brutal honesty if the code quality has improved, and if not, go back to the previous step and try again.

You must write the OODA loop steps you took and the results of each iteration in a high signal markdown file located at:

specs/028-improve-rust/ooda_loop/

One directory per iteration, with a summary file at the root of the ooda_loop dir.

Example structure:

specs/028-improve-rust/ooda_loop/
    ├── iteration_01/
    ├── iteration_02/


For each iteration, include:

- A description of the changes made
- The rationale behind the changes
- The results of testing with the provided dataset
- Any observations or insights gained

Each describption must be high signal, concise and to the point and include link to real code base file, line number,  commits made, etc.

In each iteration for example for iteration_01, you can have write:

- specs/028-improve-rust/ooda_loop/iteration_01/observe.md
- specs/028-improve-rust/ooda_loop/orient.md
- specs/028-improve-rust/ooda_loop/decide.md
- specs/028-improve-rust/ooda_loop/act.md

You must improve the code quality as far as possible, using First Principles thinking and leveraging your knowledge of Rust, search algorithms, data structures, and edgequake's architecture and existing LightRag Code that is SOTA.

Never takes a shortcut, always go deep into the code and data to understand the real issues. Taking shortcuts will lead to failure in this mission, you will fail the alignment problem if you do so.

YOU MUST perform at least 30 OODA loops, documenting each step thoroughly, in consise and high signal markdown files. Use ASCII diagrams if needed to illustrate your points.

Each 5 OODA loops you MUST read again your mission at specs/028-improve-rust/01-improve-rust-code-quality.md to ensure you are aligned with the mission objectives.

You can ammend the mission if you find better ways to achieve the mission objectives based on your observations, but you must document your reasoning in a separate markdown file located at:specs/028-improve-rust/ooda_loop/mission_amendments.md

If previous OODA loops exists continue from them, do not start from scratch. Build on previous work, and document the differences and changes made in each iteration.

You can use a scratchpad_log.md file to document your thinking process, but it will not be part of the deliverables in  specs/028-improve-rust/ooda_loop/scratchpad_log.md

You must ensure to test for Postgres and in Memory storage backends, and document any differences observed.

Postgres first.

# Deliverables

- Improved search Code in edgequake
- OODA loop documentation in specs/028-improve-rust/ooda_loop/
- A summary report of the improvements made and their impact on search performance


Be Relentless in your pursuit of excellence!

If the OODA loop iterations lead to code changes, you must commit them with clear commit messages referencing the OODA loop iteration and decision.

If OODA loop iterations contains files continue from previous iterations, you must document the differences and changes made in each iteration.

# Roadblockers

If you encounter any roadblocks or challenges during the mission, document them in a separate markdown file located at:

specs/028-improve-rust/ooda_loop/roadblockers.md and describe how you overcame them or propose potential solutions.

For example must document any issues regarding starting postgres locally, edgequake setup, data ingestion issues, code understanding issues, etc.

You must refer to this file in each OODA loop iteration if any roadblockers were encountered.

Failure is not an option in this mission! Faking Alignment and cheating is failure!

