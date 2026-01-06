# Mission


Your mission is to improve recall and precision in search results of egdequake, and use LightRag Code if needed to spot difference between the canonical graph search and the current implementation in edgequake.

# Problem Statement

We have observed that the current search functionality in edgequake doesn't recall all relevant documents and sometimes returns irrelevant results. This affects user satisfaction and the overall effectiveness of the platform.

We have created a dataset containing search queries along with their expected relevant documents. Your task is to analyze the current search algorithm, identify its shortcomings, and implement improvements to enhance both recall and precision.

1) The dataset is available at: 

specs/fix_search/data 

2) The questions are documented in:

specs/fix_search/questions


## Your Tasks

- Map the territory of the current edgequake system, code and local infrastructure
- Ingest the dataset into a local edgequake instance
- Use the questions in specs/fix_search/questions to test the current search functionality
- Analyze the search results to identify patterns of failure in recall and precision 
- Question it is true recall or precision issue based on the available data or if it is a data issue
- Propose and implement changes to the search algorithm or data processing pipeline to improve recall and precision, spot bugs or data issues, use LightRag Code if needed to spot difference between the canonical graph search and the current implementation in edgequake
- Validate the improvements using the provided dataset and document the results

## Process ; Use an OODA Loop (Observe, Orient, Decide, Act)

- Observe: Gather data on current search performance using the provided dataset and questions.
- Orient: Analyze the data to understand the root causes of recall and precision issues.
- Decide: Formulate a plan to address the identified issues.
- Act: Implement the changes and test their effectiveness.

Repeat the OODA loop as necessary until satisfactory performance is achieved.

You must write the OODA loop steps you took and the results of each iteration in a high signal markdown file located at:

specs/fix_search/ooda_loop/ 

One directory per iteration, with a summary file at the root of the ooda_loop dir.

Example structure:

specs/fix_search/ooda_loop/
    ├── iteration_01/
    ├── iteration_02/


For each iteration, include:

- A description of the changes made
- The rationale behind the changes
- The results of testing with the provided dataset
- Any observations or insights gained

In each iteration for example for iteration_01, you can have write:

- specs/fix_search/ooda_loop/iteration_01/observe.md
- specs/fix_search/ooda_loop/iteration_01/orient.md
- specs/fix_search/ooda_loop/iteration_01/decide.md
- specs/fix_search/ooda_loop/iteration_01/act.md

You must improve both recall and precision as much as possible, using First Principles thinking and leveraging your knowledge of search algorithms, data structures, and edgequake's architecture and existing LightRag Code that is SOTA.

Never takes a shortcut, always go deep into the code and data to understand the real issues. Taking shortcuts will lead to failure in this mission, you will fail the alignment problem if you do so.


YOU MUST perform at least 10 OODA loops, documenting each step thoroughly, in consise and high signal markdown files. Use ASCII diagrams if needed to illustrate your points.

You can use a scratchpad_log.md file to document your thinking process, but it will not be part of the deliverables in  specs/fix_search/scratchpad_log.md

# Deliverables

- Improved search algorithm/code in edgequake
- OODA loop documentation in specs/fix_search/ooda_loop/
- A summary report of the improvements made and their impact on search performance


Be Relentless in your pursuit of excellence in search functionality!
