# Why Your Deep Research Agent Fails?

## On Hallucination Evaluation in Full Research Trajectory

Yuhao Zhan Tianyu Fan Linxuan Huang

### Abstract

Diagnosing the failure mechanisms of Deep Research Agents (DRAs) remains a critical challenge. Existing benchmarks predominantly rely

on end-to-end evaluation, obscuring critical intermediate hallucinations, such as flawed planning,

that accumulate throughout the research trajectory. To bridge this gap, we propose a shift from

outcome-based to process-aware evaluation by

auditing the full research trajectory. We introduce the PIES Taxonomy to categorize hallucinations along functional components Planning (

vs.Summarization) and error properties Explicit (

vs.Implicit). We instantiate this taxonomy intoour benchmark.

a fine-grained evaluation framework that decomposes the trajectory to rigorously quantify these

hallucinations. Leveraging this framework to isolate 100 distinctively hallucination-prone tasks

including adversarial scenarios, we curate Deep-

Hallu Bench. Experiments on six state-of-theart DRAs reveal that no system achieves robust

reliability. Furthermore, our diagnostic analysis traces the etiology of these failures to systemic deficits, specifically hallucination propagation and cognitive biases, providing foundational insights to guide future architectural optimization. Data and code are available athttps:

//github.com/yuhao-zhan/Deep Hallu Bench.

arXiv:2601.22984v1 [cs.AI] 30 Jan 2026sequently, existing evaluations suffer from two major limi-1. Introduction

The rapid advancement of Large Language Models (LLMs)

has spurred the development of Deep Research Agents

(DRAs) (Huang et al., 2025; Zhang et al., 2025b). A DRA

is an LLM-based system designed to iteratively plan, search,

and reason to retrieve and synthesize information, ultimately

generating a final report for a user query. Existing DRAs,

such as OpenAI's (OpenAI, 2025) and Gemini's (Gemini,

1Zhejiang University. Work done during internship at HKU.

2the University of Hong Kong. Correspondence to: Chao Huangassessment to process-aware evaluation, capable of auditing

*<chuang@cs.hku.hk>.*

Preprint. February 2, 2026.

Zirui Guo Chao Huang

*Figure 1. Comparison between existing benchmarks for DRAs and*

2025), accelerate complex research, reducing completion

times from hours to minutes.

Despite their potential, the complexity and sophistication

of DRAs make holistic and faithful evaluation challenging.

Existing benchmarks predominantly fall into two categories

based on query type: close-ended, which verifies short-form

answers against ground-truth data (Mialon et al., 2023; Wei

et al., 2025); and open-ended, which assesses long-form reports across metrics like factuality and citation quality using

reference reports or rubrics (Du et al., 2025; Li et al., 2025).

Critically, both approaches share a fundamental deficiency:

they rely on end-to-end evaluation. These benchmarks

focus solely on the final output, neglecting the complex

research process as a black box, as shown in Figure 1. Contations: (1) Incomplete Hallucination Detection: Critical

intermediate hallucinations, such as misleading plans, occur

exclusively within intermediate steps and remain invisible

to end-to-end checks. (2) Opaque Performance Diagnosis:

Without tracing these intermediate errors, attributing the final poor performance to specific modules (e.g., planning or

summarization) becomes infeasible, impeding fine-grained

understanding and optimization. Addressing these challenges necessitates a paradigm shift from outcome-based

hallucinations throughout the entire research trajectory.

However, realizing such process-aware evaluation is im-

## 1However, realizing such process-aware evaluation is im-


---

peded by three obstacles: (1) Taxonomic Gap: Hallucina-veal that no agent achieves robust reliability. We identify

tion taxonomies tailored to DRAs remain under-explored;a strategic dichotomy between over-confidence and over-

(2) Data Acquisition Barriers: Proprietary DRAs eitherconservatism, alongside pervasive execution deficits such as

impose prohibitive API costs or operate via Web UIs lackingunfaithful grounding and information neglect. Moving from

structured logs (e.g., JSON), complicating automated track-symptoms to causes (RQ2: Failure Mechanisms), we trace

ing; (3) Evaluation Complexity: Constructing a holisticthese failures to systemic deficits stemming from: (1) Halluand faithful benchmark is non-trivial, given the multifaceted,cination Propagation, where proprietary agents suffer from

multi-stage nature of the research trajectory.early-stage cascading fabrications, while the open-source

To overcome these barriers, we first address the taxonomic gap. We model the research trajectory as iterative

1. Cognitive Biases, specifically a temporal "Anchor Effect"

plan-search-summarize loopsand propose the PIES Taxonomy. This framework structures hallucinations along

two dimensions: the functional components (Planning vs.

Summarization) and error properties (Explicit vs.Implicit).

Specifically, Explicit Hallucinations refer to the presence of

incorrect information, while Implicit Hallucinations denote

the critical absence of required content, violating the user's

intent. The intersection of these dimensions yields four

categories: (1) Explicit Planning: Generating deviated or

redundant plans; (2) Implicit Planning: Neglecting specific

user restrictions; (3) Explicit Summarization: Fabricating content or misquoting citations; (4) Implicit Summarization: Neglecting essential retrieved information. This

systematic classification forms the foundation to rigorously

evaluate hallucinations and unveil precise limitations.

Building on this taxonomy, we address the remaining obstacles. To circumvent data acquisition barriers, we developed

parsers that reconstruct unstructured Web UI traces into

standardized "plan-search-summarize" loop. To address

evaluation complexity, we decompose plans into atomic

actions and summaries into atomic claims. This granular

approach enables us to quantify hallucinations on each PIES

category via verifying atomic actions and claims (Explicit)

while detecting neglected restrictions and important information (Implicit).

Leveraging this framework, we introduce Deep Hallu Bench,

the first benchmark designed to evaluate hallucinations

throughout the DRA research trajectory. We construct the

dataset by aggregating diverse queries from existing benchmarks (Gou et al., 2025; Fan et al., 2025; Wei et al., 2025)

and synthesizing adversarial "no-answer" queries via atomic

perturbations. To isolate the most challenging queries, we

employ a rigorous filtering pipeline that generates and evaluates trajectories from Gemini Deep Research, retaining only

the 100 most "hallucination-prone" queries based on the

derived hallucination scores, with a balanced distribution

between openand close-ended tasks.

Using Deep Hallu Bench, we benchmark five proprietary and

one open-source DRAs to investigate two core questions.

Regarding RQ1 (Hallucination Landscape), our results re-

1The final report is treated as the terminal summary.

framework succumbs to late-stage context collapse; and (2)

(fixating on initial retrieval) and a semantic "Homogeneity

Bias" (neglecting diverse insights). These findings suggest

a necessary pivot from simple retrieval scaling to architectural interventions targeting early-stage error correction and

long-context attention debiasing.

Contributions. (1) We pioneer a paradigm shift from

outcome-based to process-aware evaluation for DRAs, marking the first systematic audit of the full research trajectory.

(2) We introduce the PIES Taxonomy and evaluation framework, serving as the foundation for Deep Hallu Bench-the

first benchmark dedicated to stress-testing DRA hallucinations. (3) Our diagnostic analysis unveils critical failure

etiologies, providing foundational insights into systemic

deficits to guide future architectural optimization.

                        2. Related Work

Hallucinations. Hallucination in LLMs is generally defined as content that is nonsensical or unfaithful to source

materials (Farquhar et al., 2024). These errors are typically categorized into three types: input-conflicting, contextconflicting, and fact-conflicting (Zhang et al., 2025c). To detect hallucinations, researchers widely employ fact-checking

(Vlachos & Riedel, 2014) or claim verification (Zerong et al.,

2025). These approaches utilize Natural Language Inference (NLI) models (Chen et al., 2025b; Schopf et al., 2025),

LLMs (Wei et al., 2024; Rahman et al., 2025), or agents

(Cheng et al., 2024) to predict verdict labels (e.g., Entailment vs. Contradiction). Recently, hallucinations within

LLM-based agents have gained attention, leading to new taxonomies and detection methods (Lin et al., 2025; Zhu et al.,

2025b). However, hallucinations specific to DRAs, though

briefly assessed in benchmarks like Mind2Web2 (Gou et al.,

2025), lack systematic evaluation and analysis, leaving the

fundamental limitations of DRAs largely unexplored.

Deep Research Evaluation. Current benchmarks for

DRAs can be categorized by query type: close-ended and

open-ended. Close-ended evaluations involve queries with

short, ground-truth answers, facilitating automated verification. Benchmarks such as GAIA (Mialon et al., 2023),

Browse Comp (Wei et al., 2025), xbench (Chen et al., 2025a),

and Browse Comp-Plus (Chen et al., 2025c) typically rely on

2


---

*Table 1. Comparison between Deep Hallu Bench and existing Deep Research benchmarks.△denotes evaluation on incomplete*

hallucinations. Deep Hallu Bench uniquely integrates close-ended, open-ended, and "no-answer" queries, providing the first comprehensivehallucination evaluation throughout the full research trajectory.

Benchmark Close-ended Open-ended Research Trajectory Hallucination No-answer Query

GAIA ✓ ✗ ✗ Browse Comp ✓ ✗ ✗

Browse Comp-Plus ✓ ✗ ✗

Rigorous Bench ✗ ✓ ✗ Mind2Web2 ✗ ✓ ✗

Deep Research-Report Eval ✗ ✓ ✗

metrics like accuracy. Open-ended evaluations focus oncan be supported by other uncited sources.Report Bench ✗ ✓ ✗ △ report-style, long-form outputs. For instance, Mind2Web2Implicit Summarization→Noise Domination: This•Deep Research Arena ✗ ✓ ✗ △ -

(Gou et al., 2025) assesses agentic search using an Agent-as-category highlights the critical absence of essential infor-

Deep Hallu Bench (Ours) ✓ ✓ ✓

a-Judge framework with a tree-structured rubric. Similarly,mation. Despite retrieving relevant documents, the agent

report-oriented benchmarks like Deep Research Bench (fails to utilize them, allowing "noise" (Dui.e., irrelevant parts

et al., 2025) evaluate information retrieval via reference re-of the retrieval) to dominate the summary. This results in

ports, while Deep Research-Report Eval (Fan et al., 2025an answer that misses the core user intent despite having)

employs an LLM-as-a-Judge to assess quality, redundancy,access to the correct data (i.e., input-conflicting).

and factuality. Other frameworks, including Report Bench•Explicit Planning→Action Hallucination: This occurs

(Li et al., 2025), Rigorous Bench (Yao et al., 2025), andwhen the agent generates explicit execution steps that

Deep Research Arena (Wan et al., 2025), utilize automatedare flawed. It manifests primarily in three forms: (1)

or human-designed rubrics. Despite this progress, most Action Deviation: The plan deviates from the user's intent

benchmarks apply end-to-end evaluation that overlooks the(Input-conflicting); (2) Action Redundancy: The agent

full research trajectory and assesses hallucinations partiallyproposes unnecessary steps that repeat previous efforts

(e.g., focusing only on factuality), lacking a holistic, trace-(Context-conflicting); (3) Propagation: A unique case

able framework tailored to DRA hallucinations. Tablewhere the plan is logically correct but based on previous 1

provides a comprehensive comparison between these exist-hallucinated claims, leading to a cascade of errors.

ing benchmarks and our proposed Deep Hallu Bench.•Implicit Planning→Restriction neglect: Unlike ex-

3. Hallucination Taxonomy

While standard LLM hallucinations are often categorized

as input-, context-, or fact-conflicting (Zhang et al., 2025c),

these categories do not fully capture the search-based,

multi-stage nature of DRAs. To address this, we propose the PIES Taxonomy, which structures hallucinations

along two dimensions: the functional component (Planning

2

vs. Summarization)and the error property (Explicit vs.Guided by the PIES taxonomy, this section establishes our

Implicit). As illustrated in Figure 2, this intersection yieldsframework for trajectory data acquisition and fine-grained

four distinct categories of DRA hallucinations:hallucination evaluation.

•Explicit Summarization→Claim Hallucination: In

the summarization stage, explicit hallucinations involve

the presence of incorrect information. This includes: (1)To evaluate proprietary DRAs lacking cost-friendly APIs

Fabrication: Generating content (i.e., claims) unsupportedand structured reasoning output, we developed a pipeline

by any document and context; (2) Misattribution: Citingthat reconstructs full research trajectories from Web UI

documents that do not support the claim, even if the claimtraces (Figure 3). We employ custom HTML-parsers and

2We exclude the Search stage as it relies on external engines,

distinguishing retrieval outputs from LLM-induced hallucinations.

3

 ✗ ✗

 ✗ ✗

 △ ✗

 ✓ ✓

plicit deviations, this hallucination is defined by the absence of adherence to user restrictions. The agent formulates a plan that is technically executable but silently

ignores specific user restrictions (e.g., ignoring "full-time"

in a job-seeking task, Figure 2), representing a subtle form

of Input-conflicting hallucination.

                        4. Evaluation and Benchmark

### 4.1. Data Acquisition and Decomposition

LLMs to disentangle interleaved reasoning and URLs into

structured plan-search-summarize loops. To quantify hallu-


---

*Figure 2. The PIES Taxonomy. The framework intersects functional components (vertical axis) with error properties (horizontal*

axis). The four quadrants represent specific hallucination categories derived from these combinations: Explicit Summarization, Implicit Summarization, Explicit Planning, and Implicit Planning.

*Figure 3. The Data Acquisition and Decomposition Pipeline. We first employ custom parsers to structure raw Web UI traces into*

iterative plan-search-summarize loops. These loops are further decomposed by LLMs atomically to enable fine-grained evaluation.

cinations precisely, we adopt an atomicity-based approachport claims; full retrieval history for others. We adopt a

(Min et al., 2023; Yan et al., 2025), decomposing the trajec-retrieve-then-verify strategy: relevant evidence chunks are

tory into atomic units: user query into atomic sub-queriesretrieved via a coarse-to-fine pipeline, then verified using,

plans into atomic actions, and summaries into atomic claimsa cost-efficient NLI-then-LLM cascade. Supported claims

(preserving citation mappings). As shown in Figure 3and their evidence chunks are stored in, this Claim Memory

is implemented via a two-stage LLM pipeline, initial decom-and Chunk Memory, respectively.

position followed by a double-check refinement, to ensure•Round 2: Adaptive Re-Verification. Unsupported

strictly atomic and verifiable units. All prompt templates inclaims trigger branching checks to categorize errors: (1)

this work are in Appendix F.

### 4.2. Evaluation Framework

Leveraging atomic claims, actions and sub-queries as fundamental units, we design a rigorous evaluation framework

tailored to each category of the PIES taxonomy.

Claim Verification (Explicit Summarization). To distinguish between factual observations and internal reflections,

we implement a two-round verification pipeline (Figure 4).

•Round 1: Initial Verification. We verify claims against

their specific evidence scope: cited documents for re-

4

Misattribution Check: For claims with explicit citations,

we expand the evidence scope to all retrieved documents. Support here indicates misattributionC; othmisattribution

erwise, it is confirmed as fabricationC. (2) Re-fabrication

flection Check: For intermediate claims, we verify them

against Claim Memory to validate internal reflections.

Lack of support confirms fabrication C.fabrication

We quantify Explicit Summarization Hallucination (H)ES

as the ratio of fabricated and misattributed claims to the

total set:

HES=*. (1)*

|Ctotal|


---

*Figure 4. The Evaluation Framework for Summarization Hallucinations. The pipeline assesses Explicit errors (top) and Implicit*

neglect (bottom). The addition symbols (⊕) define the data scope: selecting evidence scope for verification (top) or specifying documentsets for global/local level (bottom). The cross symbol (⊗) intersects ranked clusters with Chunk Memory to classify them as utilized

(In-Memory) or ignored (Out-Memory), enabling the penalty quantification shown on the right.

See Appendix B.1 for implementation details.

Noise Detection (Implicit Summarization). LLMs often

struggle to prioritize valuable information due to positional

bias (Liu et al., 2024; Trienes et al., 2025; Elaraby & Litman,

2025). To quantify DRA's capability to distinguish essential

signals from massive retrieval streams, as shown in figure

4, we propose a cluster-based heuristic at two granularities:

global-level (assessing total information utilization) and

local-level (measuring utilization within each search round).

•Semantic Clustering & Value Estimation. We firstpropagation

map retrieved chunks into semantic clusters to reduce

redundancy and rank them by relevance to the atomic

sub-queries (Rank=1 denotes highest importance).

•Penalty Quantification. We distinguish between utilized

clusters Cand ignored onesC. We penalize ne-inout 3

glect more heavily when an ignored cluster is large, high-EP

ranking, and skipped in favor of more inferior content.

The penalty Pfor an ignored cluster c is:c

*S× Ncc-inv*

P =*c**, (2)*

where Sis size,Ris rank, andN(inversion count)ccc-invstrictions and identify which were actively addressed across

is the number of lower-ranked clusters that were utilized.the trajectory (Figure 5, bottom). We adopt a subtractive P

The total penalty is P =P.c

•Hallucination Quantification. We normalize Pagainstall sub-queries and employ the elbow method to isolate

a theoretical worst-caseP(where the highest-valueworstthe specific subset of restrictions it effectively "execute"

clusters are systematically ignored) to derive the Implicit(Q). The Implicit Planning Hallucination (H) is

Summarization Hallucination () or Noise Score:HISdefined as the proportion of restrictions that remain unad-

P

HIS=*. (3)*

Pworst

3A cluster is utilized if it contains any chunk in Chunk Memory.

See Appendix B.3 for clustering and computation details.

Action Verification (Explicit Planning). To assess the

validity of planned steps, we employ a history-aware verification mechanism (Figure 5, top). We provide the LLM with

the user query, action history, and top-K relevant prior findings from Claim Memory to categorize atomic actions. Beyond standard errors like Action DeviationA, irrel- (deviation

evant to user query) and Action RedundancyA (,redundancy

repetitive steps), we explicitly identify Action Propagation

(A): actions that are logically sound but grounded

in previously hallucinated claims. This captures the cascading nature of errors in long-horizon research.

The Explicit Planning Hallucination () is defined as:HEP

|Atotal|

Restriction Checking (Implicit Planning). To detect the

neglect of user restrictions, we treat sub-queries as atomic reprocess: for every atomic action, we rank its relevance to

executed IP

dressed after the full research session:

|Qtotal\ Qexecuted |

HIP=*. (5)*

|Qtotal|

5


---

*Figure 5. The Evaluation Framework for Planning Hallucinations. The pipeline assesses Explicit errors (top) and Implicit neglect*

(bottom). The subtraction symbol (⊖) defines the neglect identification logic: removing the set of effectively executed sub-queries fromthe full set of sub-queries to isolate neglected restrictions.

Reliability. Benchmarking claim verification module5. Results & Analysis

against standard fact-checking datasets confirms its robustness: the pipeline achieves∼95% accuracy on FEVER

subset (Thorne et al., 2018>) and 85% on Sci Fact-Open

(Wadden et al., 2022) (see Appendix B.2 for details). For

action verification, lacking established benchmarks, we ensure its reliability through an iterative human-in-the-loop

prompt optimization and validation process.

### 4.3. Benchmark Construction

To stress-test DRAs, we construct Deep Hallu Bench, a

benchmark of 100 queries through a three-stage pipeline.

•Aggregation & Difficulty Assessment. We aggregated a

diverse candidate pool of queries from Mind2Web2, Re-et al., 2025), a top-performer on the Deep Research Bench

port Eval, and Browse Comp. To isolate the most challeng-leaderboard). For evaluation, we adopt our proposed haling tasks, we utilized Gemini Deep Research as a probelucination metrics alongside Retrieval Quality, defined as

to generate full research trajectories for all candidates.the average relevance of the top-50% retrieved documents

We computed a composite hallucination scoreH(Equa-against use query; see Appendix D.1 for details.

tion 6) for each trajectory and selected the top-75 most

"hallucination-prone" queries (25 from each dataset).

1. Overview. Table 2 overviews the performance of the

4. Overview. Table 2 overviews the performance of the

•Adversarial Synthesis. To evaluate DRA's ability to

reject unsolvable tasks, we synthesized 25 adversarial "noanswer" queries. These were created by applying atomic

perturbations to solvable close-ended queries-modifying

specific restrictions (e.g., temporal details) to render the

logical intersection of all restrictions empty.

The final benchmark comprises 100 queries, evenly split between open-ended and close-ended tasks. See Appendix C

for dataset details and case studies for atomic perturbations.

This section investigates two core research questions (RQs):

RQ1 (What): What characterizes the hallucination landscape of current DRAs? RQ2 (Why): What underlying

mechanisms drive the DRA's failures? We first benchmark

representative DRAs on Deep Hallu Bench, followed by a

diagnostic analysis of their failure etiologies.

### 5.1. Experimental Setup

We evaluate six state-of-the-art DRAs, comprising five Proprietary DRAs (Gemini, 2025; OpenAI, 2025; Perplexity,

2025; Qwen, 2025; Grok, 2025), along with one Open-

Source DRA (Salesforce Air Deep Research (Prabhakar

### 5.2. Results

DRAs on Deep Hallu Bench. No single DRA achieves robust performance across the full trajectory. Qwen emerges

as the top performer with the lowest overall hallucination score (H ≈ 0.149). It is followed closely by OpenAI (H ≈ 0.155), while a mid-tier comprises Gemini

(H ≈ 0.175) and SalesforceH ≈ ( 0.185). Perplexity trails

with a higher hallucination degree (H ≈ 0.21), while Grok

lags significantly behind, exhibiting severe hallucinations inthe summarization stage. 4

4WhenH is averaged exclusively on summarization metrics

HESand H, Grok (H ≈ 0.38) is the poorest performer.IS

6


---

*Figure 6. Evaluation Results over Deep Hallu Benchwith seven hallucination metrics alongside Retrieval Quality for six DRAs.*

*Table 2. Evaluation results on Deep Hallu Bench. DRAs above the Table 3. Performance on Close-Ended Queries (N = 50). Ans.:*

| midline | are | proprietary. | Bold | denotes | lowest | hallucination | score.Answerable | queries; | No-Ans.: | Adversarial | queries | (correct | re- |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Gemini 0.2171 0.2786 0.0170 0.1866 | 0.1749 |  |  |  |  |  |  |  |  |  |  |  |  |
| OpenAI 0.2207 0.3121 0.0456 0.0401 | 0.1546 |  |  |  |  |  |  |  |  |  |  |  |  |
| Perplexity 0.2220 0.3940 0.0313 0.1865 | 0.2084 |  |  |  |  |  |  |  |  |  |  |  |  |
| Qwen 0.2311 0.2374 0.0197 0.1070 | 0.1488 |  |  |  |  |  |  |  |  |  |  |  |  |
| Salesforce 0.3231 0.1003 0.0291 0.2879 | 0.1851 |  |  |  |  |  |  |  |  |  |  |  |  |

distinct failure patterns across the agents:

•Explicit Summarization (Claim Hallucination). We observe a sharp dichotomy in error types. OpenAI and Grok

act as "confident fabricators" (High Fabrication≈ 0.15,

Low Misattribution), generating content without sufficient Close-Ended Tasks. Performance on close-ended queries

support. Conversely, Salesforce creates an "illusion of(Table 3) reveals a critical trade-off between over-confidence

grounding" dominated by Misattribution (> 0.20).and over-conservatism, with three distinct profiles: (1) Over-

•Implicit Summarization (Noise Domination). This met-Confidence (Gemini, Grok): These DRAs fail to reject adric highlights a bottleneck in information prioritization.versarial queries (near0%accuracy), force-hallucinating an-

Grok and Perplexity succumb to high noise (≈ 0.33)swers due to an inability to identify empty intersection of redespite decent retrieval. Qwen proves most resiliencestriction sets. (2) Over-Conservatism (Salesforce, Qwen):

(≈ 0.23), whereas Salesforce, with lower retrieval quality,They achieve high adversarial accuracy (72-80%) but at

achieves the lowest noise (≈ 0.10) likely by retrieving athe cost of prematurely abandoning answerable queries (0%

narrower, safer set of information. accuracy). Their excessive rejection may be due to insuf-

•Explicit Planning (Action Hallucination). While gen-ficient retrieval results and a default conservative fallback.

eral planning capabilities are robust (< 5%errors), nu-(3) Balanced Struggle (OpenAI, Perplexity): Only these

ances emerge. OpenAI shows slightly higher DeviationDRAs genuinely distinguish intent, with OpenAI maintainand Redundancy≈ ( 4%) likely due to its exhaustive strate-ing a consistent profile (28%across metrics) rather than

gies. Gemini, while efficient, exhibits specific suscepti-collapsing into systemic bias. For extended results on dobility to Propagation≈ 1. (7%), where planning errorsmain sensitivity and performance disparities between opencascade from prior fabrications. and close-ended tasks, see Appendix D.2 and D.3.

7

Accuracy

DRA Rejection

Overall Ans. No-Ans.

OpenAI 28% 28% 28% 22% Perplexity 24% 16% 32% 42%

Qwen 36% 0% 72% 60% Grok 16% 24% 8% 10%

Salesforce 40% 0% 80% 80%

to restrictions reveals a tier-stratified gap. OpenAI leads

with near-perfect adherence (neglect< 0.05), followed by

Qwen. In contrast, Salesforce, Gemini, and Perplexity

fail significantly more often (≈ 0.18-0.30), struggling to

internalize boundary conditions.


---

*Figure 7. Temporal Distribution of Hallucinations. We segment*

the research trajectory into three equal stages (Early, Middle, Late). Src. Dist.: source errors that trigger propagation; Desc. Dist.:

consequent errors propagating from source; and Hallu. Dist.: thedistribution of explicit hallucinations derived after backtracking

all propagation chains to their root sources.

### 5.3. Analysis

Based on this multifaceted hallucination landscape, wedrops precipitously in later stages. Paradoxically, Noise

delve into RQ2 (Why): What underlying mechanisms drive Scores peak in the late stage even as Retrieval Quality

the DRA's failures? We identify two primary failure etiolo-increases. This indicates a "saturation" bottleneck: agents

gies: hallucination propagation and cognitive biases.may stop attending to new, superior information once their

Hallucination Propagation. Hallucinations in DRAs exhibit strong sequential dependency ("domino effect"). We

map these dependencies into a Directed Acyclic Graph

where nodes represent atomic claims or actions and directed

edges (A → B) indicate that the hallucination in Bpropagates from A. Figure 7 reveals distinct temporal profiles:

•Early-Stage Cascading (Gemini, OpenAI). Proprietary

DRAs exhibit systemic cascading, where> 57%of

source errors occur in the early stage. Initial fabrication acts as the primary catalyst, immediately triggering

a chain of descendants that undermines the subsequent

research foundation.

•Late-Stage Collapse (Salesforce). In contrast, the opensource framework maintains stability early on but breaks

down in the late stage (> 40%of errors). This highlights

a limitation in maintaining coherence over long contexts.

We further analyze root-cause errors (the earliest step

precipitating final failure) for the 50 close-ended queries

(heatmap in Appendix E.2). The results reveal two primary

mechanisms: (1) Fabrication Dominance: For most DRAs,

the dominant failure trigger is intermediate summarization

fabrication, where DRAs derive conclusions unsupported

by retrieved documents. (2) Divergent Fallbacks: In trajectories free of explicit hallucinations, the open-source DRA

(i.e., Salesforce) tends to conservatively refuse the query,

whereas proprietary DRAs often proceed to fabricate a final

answer. This behavior aligns with the "Over-Confidence vs.

Over-Conservatism" dichotomy observed in Table 3.

Cognitive Biases. DRAs struggle to maintain unbiasedagation and cognitive biases like the Anchor Effect. These

information attention across the temporal and semantic di-insights indicate that future progress requires moving be-

*Figure 8. Temporal Analysis of Information Attention and Noise.*

mensions, leading to severe Noise Domination.

•Temporal: "Anchor Effect". DRAs disproportionately

favor early retrieval (Figure 8). The Utilized Chunk Count

context is filled by initial findings.

•Semantic: "Homogeneity Bias". DRAs prefer redundancy over novelty. Our analysis reveals that utilized

clusters are significantly larger than ignored ones, indicating a reliance on homogeneity. Furthermore, higher

information heterogeneity correlates with increased neglect, implying that DRAs struggle to prioritize unique,

singleton insights in diverse contexts (see Figure 18 in

Appendix E.3 for more details).

In summary, while backbone LLM is the fundamental cause

of hallucinations, our analysis isolates the agent-specific

mechanisms, to understand hallucinations within agentic

workflows. This diagnostic analysis moves beyond surfacelevel evaluation, indicating that future optimization should

pivot from retrieval scaling to architectural interventions targeting early-stage error correction and attention debiasing.

                        6. Conclusion

This work shifts the evaluation paradigm of Deep Research

Agents from outcome-based metrics to a process-aware evaluation. By introducing the PIES Taxonomy and Deep Hallu Bench, we expose the critical intermediate hallucinations

that remain invisible to traditional end-to-end benchmarks.

Our comprehensive evaluation reveals that current DRAs

fail to achieve robust reliability, exposing multidimensional

deficits across the full trajectory: from strategic failures

in balancing confidence versus conservatism, to execution

flaws involving unfaithful grounding and severe information neglect. Crucially, our diagnostic analysis traces these

failures to systemic deficits, specifically hallucination prop-

8


---

yond simple retrieval scaling toward architectural interven-Chen, Y.-N. (eds.), Proceedings of the 2024 Conference

tions that enforce early-stage error correction and unbiasedon Empirical Methods in Natural Language Processlong-context attention.

## Impact Statement

This paper presents work whose goal is to advance the field

of Machine Learning. There are many potential societal

consequences of our work, none which we feel must be

specifically highlighted here.

## References

Cemri, M., Pan, M. Z., Yang, S., Agrawal, L. A., Chopra,

B., Tiwari, R., Keutzer, K., Parameswaran, A., Klein,

D., Ramchandran, K., Zaharia, M., Gonzalez, J. E., and

Stoica, I. Why do multi-agent llm systems fail?, 2025.

URL https://arxiv.org/abs/2503.13657.

Chen, J., Xiao, S., Zhang, P., Luo, K., Lian, D., andM. S., Xu, M. Y., Zhang, M., Zhang, M., Tang, M., Zhou,

Liu, Z. Bge m3-embedding: Multi-lingual, multi-M., Huang, P., Cong, P., Wang, P., Wang, Q., Zhu, Q.,

functionality, multi-granularity text embeddings through Li, Q., Chen, Q., Du, Q., Xu, R., Ge, R., Zhang, R., Pan,

self-knowledge distillation, 2024. URLhttps://arxiv.R., Wang, R., Yin, R., Xu, R., Shen, R., Zhang, R., Liu,

org/abs/2402.03216.

Chen, K., Ren, Y., Liu, Y., Hu, X., Tian, H., Xie, T., Liu,

F., Zhang, H., Liu, H., Gong, Y., Sun, C., Hou, H., Yang,

H., Pan, J., Lou, J., Mao, J., Liu, J., Li, J., Liu, K., Liu,

K., Wang, R., Li, R., Niu, T., Zhang, W., Yan, W., Wang,

X., Zhang, Y., Hung, Y.-H., Jiang, Y., Liu, Z., Yin, Z.,

Ma, Z., and Mo, Z. xbench: Tracking agents productivity

scaling with profession-aligned real-world evaluations,

2025a. URL https://arxiv.org/abs/2506.13651.

Chen, W.-F., Zhao, Z., Karimi, A., and Flek, L. Explainable hallucination through natural language inference mapping. In Che, W., Nabende, J., Shutova,

E., and Pilehvar, M. T. (eds.), Findings of the Association for Computational Linguistics: ACL 2025, pp.

1888-1896, Vienna, Austria, July 2025b. Association

for Computational Linguistics. ISBN 979-8-89176-256-

5. doi: 10.18653/v1/2025.findings-acl.96. URLhttps:

//aclanthology.org/2025.findings-acl.96/.

Chen, Z., Ma, X., Zhuang, S., Nie, P., Zou, K., Liu,Chen, R. J., Jin, R. L., Li, S. S., Zhou, S., Sun, T., Li,

A., Green, J., Patel, K., Meng, R., Su, M., Sharify-X. Q., Jin, X., Shen, X., Chen, X., Song, X., Zhou, X.,

moghaddam, S., Li, Y., Hong, H., Shi, X., Liu, X.,Zhu, Y. X., Huang, Y., Li, Y., Zheng, Y., Zhu, Y., Ma, Y.,

Thakur, N., Zhang, C., Gao, L., Chen, W., and Lin, J.Huang, Z., Xu, Z., Zhang, Z., Ji, D., Liang, J., Guo, J.,

Browsecomp-plus: A more fair and transparent evalu-Chen, J., Xia, L., Wang, M., Li, M., Zhang, P., Chen, R.,

ation benchmark of deep-research agent, 2025c. URLSun, S., Wu, S., Ye, S., Wang, T., Xiao, W. L., An, W.,

https://arxiv.org/abs/2508.06600.

Cheng, X., Li, J., Zhao, X., Zhang, H., Zhang, F., Zhang,

D., Gai, K., and Wen, J.-R. Small agent can also

rock! empowering small language models as hallucination detector. In Al-Onaizan, Y., Bansal, M., and Du, M., Xu, B., Zhu, C., Wang, X., and Mao, Z. Deep-

9

ing, pp. 14600-14615, Miami, Florida, USA, November 2024. Association for Computational Linguistics.

doi: 10.18653/v1/2024.emnlp-main.809. URLhttps:

//aclanthology.org/2024.emnlp-main.809/.

Deep Seek-AI, Liu, A., Mei, A., Lin, B., Xue, B., Wang,

B., Xu, B., Wu, B., Zhang, B., Lin, C., Dong, C., Lu, C.,

Zhao, C., Deng, C., Xu, C., Ruan, C., Dai, D., Guo, D.,

Yang, D., Chen, D., Li, E., Zhou, F., Lin, F., Dai, F., Hao,

G., Chen, G., Li, G., Zhang, H., Xu, H., Li, H., Liang,

H., Wei, H., Zhang, H., Luo, H., Ji, H., Ding, H., Tang,

H., Cao, H., Gao, H., Qu, H., Zeng, H., Huang, J., Li,

J., Xu, J., Hu, J., Chen, J., Xiang, J., Yuan, J., Cheng, J.,

Zhu, J., Ran, J., Jiang, J., Qiu, J., Li, J., Song, J., Dong,

K., Gao, K., Guan, K., Huang, K., Zhou, K., Huang, K.,

Yu, K., Wang, L., Zhang, L., Wang, L., Zhao, L., Yin,

L., Guo, L., Luo, L., Ma, L., Wang, L., Zhang, L., Di,

S. H., Lu, S., Zhou, S., Chen, S., Cai, S., Chen, S., Hu,

S., Liu, S., Hu, S., Ma, S., Wang, S., Yu, S., Zhou, S.,

Pan, S., Zhou, S., Ni, T., Yun, T., Pei, T., Ye, T., Yue, T.,

Zeng, W., Liu, W., Liang, W., Pang, W., Luo, W., Gao,

W., Zhang, W., Gao, X., Wang, X., Bi, X., Liu, X., Wang,

X., Chen, X., Zhang, X., Nie, X., Cheng, X., Liu, X., Xie,

X., Liu, X., Yu, X., Li, X., Yang, X., Li, X., Chen, X.,

Su, X., Pan, X., Lin, X., Fu, X., Wang, Y. Q., Zhang, Y.,

Xu, Y., Ma, Y., Li, Y., Li, Y., Zhao, Y., Sun, Y., Wang, Y.,

Qian, Y., Yu, Y., Zhang, Y., Ding, Y., Shi, Y., Xiong, Y.,

He, Y., Zhou, Y., Zhong, Y., Piao, Y., Wang, Y., Chen, Y.,

Tan, Y., Wei, Y., Ma, Y., Liu, Y., Yang, Y., Guo, Y., Wu,

Y., Wu, Y., Cheng, Y., Ou, Y., Xu, Y., Wang, Y., Gong,

Y., Wu, Y., Zou, Y., Li, Y., Xiong, Y., Luo, Y., You, Y.,

Liu, Y., Zhou, Y., Wu, Z. F., Ren, Z. Z., Zhao, Z., Ren,

Z., Sha, Z., Fu, Z., Xu, Z., Xie, Z., Zhang, Z., Hao, Z.,

Gou, Z., Ma, Z., Yan, Z., Shao, Z., Huang, Z., Wu, Z.,

Li, Z., Zhang, Z., Xu, Z., Wang, Z., Gu, Z., Zhu, Z., Li,

Z., Zhang, Z., Xie, Z., Gao, Z., Pan, Z., Yao, Z., Feng,

B., Li, H., Cai, J. L., Ni, J., Xu, L., Li, M., Tian, N.,

Wang, X., Sun, X., Wang, X., Tang, Y., Zha, Y., Zhang,

Z., Ju, Z., Zhang, Z., and Qu, Z. Deepseek-v3.2: Pushing

the frontier of open large language models, 2025. URL

https://arxiv.org/abs/2512.02556.


---

research bench: A comprehensive benchmark for deep Li, M., Zeng, Y., Cheng, Z., Ma, C., and Jia, K. Reportresearch agents, 2025. URLhttps://arxiv.org/abs/bench: Evaluating deep research agents via academic sur-

2506.11763.

Elaraby, M. and Litman, D. Arc: Argument representation

and coverage analysis for zero-shot long document summarization with instruction following llms, 2025. URL

https://arxiv.org/abs/2505.23654.

Fan, T., Niu, X., Zheng, Y., Zhang, F., Huang, C., Chen,

B., Lin, J., and Huang, C. Understanding deepresearch

via reports, 2025. URLhttps://arxiv.org/abs/2510.

07861. Liu, N. F., Lin, K., Hewitt, J., Paranjape, A., Bevilac-

Farquhar, S., Kossen, J., Kuhn, L., and Gal, Y. Detecting

hallucinations in large language models using semantic

entropy. Nature, 630(8017):625-630, 2024.

Gemini. Gemini deep research - your personal research

assistant. https://gemini.google/overview/deep-research/,

Gou, B., Huang, Z., Ning, Y., Gu, Y., Lin, M., Qi, W.,

Kopanev, A., Yu, B., Gutierrez, B. J., Shu, Y., Song, ´

C. H., Wu, J., Chen, S., Moussa, H. N., Zhang, T., Xie, J.,

Li, Y., Xue, T., Liao, Z., Zhang, K., Zheng, B., Cai, Z.,

Rozgic, V., Ziyadi, M., Sun, H., and Su, Y. Mind2web

2: Evaluating agentic search with agent-as-a-judge, 2025.

URL https://arxiv.org/abs/2506.21506.

Grok. Grok agents: Combining reasoning and tool use.

https://x.ai/news/grok-3/, 2025.

Huang, Y., Chen, Y., Zhang, H., Li, K., Zhou, H., Fang, M.,

Yang, L., Li, X., Shang, L., Xu, S., et al. Deep research

agents: A systematic examination and roadmap. arXiv

preprint arXiv:2506.18096, 2025.

Jiang, Y., Bordia, S., Zhong, Z., Dognin, C., Singh, M., andhttps://www.perplexity.ai/hub/blog/introducing-

Bansal, M. Ho Ver: A dataset for many-hop fact extrac-perplexity-deep-research, 2025.

tion and claim verification. In Cohn, T., He, Y., and Liu, Y.

(eds.), Findings of the Association for Computational Linguistics: EMNLP 2020, pp. 3441-3460, Online, November 2020. Association for Computational Linguistics.

doi: 10.18653/v1/2020.findings-emnlp.309. URLhttps:

//aclanthology.org/2020.findings-emnlp.309/.

Laurer, M., Van Atteveldt, W., Casas, A., and Welbers, K.

Less annotating, more classifying: Addressing the data

scarcity issue of supervised machine learning with deep

transfer learning and bert-nli. Political Analysis, 32(1):

84-100, 2024.

Li, C., Liu, Z., Xiao, S., and Shao, Y. Making large languagefactuality evaluation in large language models. arXiv

models a better foundation for dense retrieval, 2023.preprint arXiv:2508.03860, 2025.

10

vey tasks, 2025. URLhttps://arxiv.org/abs/2508.

15804.

Lin, X., Ning, Y., Zhang, J., Dong, Y., Liu, Y., Wu, Y., Qi, X.,

Sun, N., Shang, Y., Cao, P., et al. Llm-based agents suffer

from hallucinations: A survey of taxonomy, methods, and

directions. arXiv preprint arXiv:2509.18970, 2025.

Liu, M. and Fang, J. Enhancing mathematical reasoning in

large language models with self-consistency-based hallucination detection, 2025. URLhttps://arxiv.org/

abs/2504.09440.

qua, M., Petroni, F., and Liang, P. Lost in the middle: How language models use long contexts. Transactions of the Association for Computational Linguistics,

12:157-173, 2024. doi: 10.1162/tacla00638. URL

https://aclanthology.org/2024.tacl-1.9/.

chical density based clustering. J. Open Source Softw., 2

(11):205, 2017.

Mialon, G., Fourrier, C., Swift, C., Wolf, T., Le Cun, Y., and

Scialom, T. Gaia: a benchmark for general ai assistants,

                        2023. URL https://arxiv.org/abs/2311.12983.

Min, S., Krishna, K., Lyu, X., Lewis, M., tau Yih, W.,

Koh, P. W., Iyyer, M., Zettlemoyer, L., and Hajishirzi,

## H. Factscore: Fine-grained atomic evaluation of factual

precision in long form text generation, 2023. URLhttps:

//arxiv.org/abs/2305.14251.

OpenAI. Introducing deep research.

https://openai.com/index/introducing-deep-research/,

Perplexity. Introducing perplexity deep research.

Prabhakar, A., Ram, R., Chen, Z., Savarese, S., Wang, F.,

Xiong, C., Wang, H., and Yao, W. Enterprise deep research: Steerable multi-agent deep research for enterprise

analytics, 2025. URLhttps://arxiv.org/abs/2510.

17797.

Qwen. Qwen deepresearch: When inspiration becomes its

own reason. https://qwen.ai/blog?id=qwen-deepresearch,

Rahman, S. S., Islam, M. A., Alam, M. M., Zeba, M., Rahman, M. A., Chowa, S. S., Raiaan, M. A. K., and Azam,

## S. Hallucination to truth: A review of fact-checking and


---

Schopf, T., Vladika, J., Farber, M., and Matthes, F. Nat-Browsecomp: A simple yet challenging benchmark for ¨

ural language inference fine-tuning for scientific hal-browsing agents, 2025. URLhttps://arxiv.org/abs/

lucination detection. In Ghosal, T., Mayr, P., Singh,2504.12516.

A., Naik, A., Rehm, G., Freitag, D., Li, D., Schimmler, S., and De Waard, A. (eds.), Proceedings of the

Fifth Workshop on Scholarly Document Processing (SDP

2025), pp. 344-352, Vienna, Austria, July 2025. Association for Computational Linguistics. ISBN 979-8-

89176-265-7. doi: 10.18653/v1/2025.sdp-1.33. URL

https://aclanthology.org/2025.sdp-1.33/.

Thorne, J., Vlachos, A., Christodoulopoulos, C., and Mittal,

## A. FEVER: a large-scale dataset for fact extraction and

VERification. In NAACL-HLT, 2018.

Trienes, J., Schlotterer, J., Li, J. J., and Seifert, C. Be- ¨

havioral analysis of information salience in large language models. In Che, W., Nabende, J., Shutova,

E., and Pilehvar, M. T. (eds.), Findings of the Association for Computational Linguistics: ACL 2025, pp.

23428-23454, Vienna, Austria, July 2025. Association

for Computational Linguistics. ISBN 979-8-89176-256-5.

doi: 10.18653/v1/2025.findings-acl.1204. URLhttps:

//aclanthology.org/2025.findings-acl.1204/.

Vlachos, A. and Riedel, S. Fact checking: Task definition

and dataset construction. In Danescu-Niculescu-Mizil,

C., Eisenstein, J., Mc Keown, K., and Smith, N. A. (eds.),

Proceedings of the ACL 2014 Workshop on Language

Technologies and Computational Social Science, pp. 18-

22, Baltimore, MD, USA, June 2014. Association for

Computational Linguistics. doi: 10.3115/v1/W14-2508.Zhang, W., Li, X., Zhang, Y., Jia, P., Wang, Y., Guo, H., Liu,

URL https://aclanthology.org/W14-2508/.

Wadden, D., Lo, K., Kuehl, B., Cohan, A., Beltagy, I.,

Wang, L. L., and Hajishirzi, H. Sci Fact-open: Towards

open-domain scientific claim verification. In Goldberg,

Y., Kozareva, Z., and Zhang, Y. (eds.), Findings of the Association for Computational Linguistics: EMNLP 2022,

pp. 4719-4734, Abu Dhabi, United Arab Emirates, December 2022. Association for Computational Linguistics.

doi: 10.18653/v1/2022.findings-emnlp.347. URLhttps:

//aclanthology.org/2022.findings-emnlp.347/.

Wan, H., Yang, C., Yu, J., Tu, M., Lu, J., Yu, D., Cao,

J., Gao, B., Xie, J., Wang, A., Zhang, W., Torr, P., and

Zhou, D. Deepresearch arena: The first exam of llms'

research abilities via seminar-grounded tasks, 2025. URL

https://arxiv.org/abs/2509.01396.

Wei, J., Yang, C., Song, X., Lu, Y., Hu, N., Huang, J.,

Tran, D., Peng, D., Liu, R., Huang, D., et al. Long-form

factuality in large language models. Advances in Neural

Information Processing Systems, 37:80756-80827, 2024.

Wei, J., Sun, Z., Papay, S., Mc Kinney, S., Han, J., Fulford,

I., Chung, H. W., Passos, A. T., Fedus, W., and Glaese, A.

11

West, A., Weng, Y., Zhu, M., Lin, Z., Ning, Z., and Zhang,

## Y. Abduct, act, predict: Scaffolding causal inference

for automated failure attribution in multi-agent systems,

                        2025. URL https://arxiv.org/abs/2509.10401.

Yan, Z., Wang, J., Chen, J., Li, X., Li, R., and Pan, J. Z.

Atomic fact decomposition helps attributed question answering, 2025. URLhttps://arxiv.org/abs/2410.

16708.

Yao, Y., Wang, Y., Zhang, Y., Lu, Y., Gu, T., Li, L., Zhao,

D., Wu, K., Wang, H., Nie, P., Teng, Y., and Wang, Y.

A rigorous benchmark with multidimensional evaluation

for deep research agents: From answers to reports, 2025.

URL https://arxiv.org/abs/2510.02190.

Zerong, Z., Li, C., Liu, X., Chen, J.-h., and Xia, F. A systematic survey of claim verification: Corpora, systems,

and case studies. In Findings of the Association for Computational Linguistics: EMNLP 2025, pp. 21452-21474,

2025.

Zhang, S., Yin, M., Zhang, J., Liu, J., Han, Z., Zhang, J.,

Li, B., Wang, C., Wang, H., Chen, Y., and Wu, Q. Which

agent causes task failures and when? on automated failure attribution of llm multi-agent systems, 2025a. URL

https://arxiv.org/abs/2505.00212.

Y., and Zhao, X. Deep research: A survey of autonomous

research agents. arXiv preprint arXiv:2508.12752, 2025b.

Zhang, Y. Cutting the root of hallucination: Structural

trimming for vulnerability mitigation in code llms. In

Second Conference on Language Modeling, 2025.

Zhang, Y., Li, Y., Cui, L., Cai, D., Liu, L., Fu, T., Huang,

X., Zhao, E., Zhang, Y., Chen, Y., et al. Siren's song in

the ai ocean: A survey on hallucination in large language

models. Computational Linguistics, pp. 1-46, 2025c.

Zhu, K., Liu, Z., Li, B., Tian, M., Yang, Y., Zhang, J., Han,

P., Xie, Q., Cui, F., Zhang, W., Ma, X., Yu, X., Ramesh,

G., Wu, J., Liu, Z., Lu, P., Zou, J., and You, J. Where llm

agents fail and how they can learn from failures, 2025a.

URL https://arxiv.org/abs/2509.25370.

Zhu, K., Liu, Z., Li, B., Tian, M., Yang, Y., Zhang, J., Han,

P., Xie, Q., Cui, F., Zhang, W., et al. Where llm agents

fail and how they can learn from failures. arXiv preprint

arXiv:2509.25370, 2025b.


---

## A. Detailed Related Work

### A.1. Failure Analysis

Failure analysis is critical for diagnosing system reliability and guiding architectural improvements. Existing work broadly

categorizes these efforts into multi-agent and single-agent domains. In multi-agent settings, research focuses on failure

attribution: (Cemri et al., 2025) establish taxonomies for coordination breakdowns, while (Zhang et al., 2025a) and (West

et al., 2025) introduce benchmarks and causal frameworks to pinpoint responsible agents. Conversely, single-agent analysis

predominantly targets domain-specific verification, such as hierarchical checking in mathematical reasoning (Liu & Fang

2025) or error localization in code generation (Zhang, 2025). Although (Zhu et al., 2025a) extend root-cause detection to

general agents, these methods largely operate within short-horizon, re-runnable environments. They fall short of addressing

the distinct complexities of Deep Research Agents, which suffer from long-context information overload and irreversible

research workflow.

## B. Evaluation Framework

### B.1. Implementation Details in Claim Verification B.1.1. RETRIEVE-THEN-VERIFY STRATEGY

Exhaustive validation against every full-text document is cost-prohibitive and noise-sensitive. To address this, we implement

a granular retrieval approach:

•Chunking: We slice documents into 15-sentence chunks (see Appendix B.2.3 for discussion). This window size balances

context integrity with token efficiency.

•Retrieval Pipeline: We select the top-K (K=5) candidates using a coarse-to-fine pipeline: initial filtering via an

embedding modelBAAI/bge-m3(Chen et al., 2024) with a similarity thresholdθ = 0.4, followed by selection via a

reranker BAAI/bge-reranker-v2-m3 (Li et al., 2023). These parameters ensure robust recall of supporting evidences.

#### B.1.2. COST-EFFICIENT NLI-THEN-LLM CASCADE

To optimize computational costs without sacrificing accuracy, we employ a hybrid verification model in factual grounding

(i.e., verifying whether a claim can be supported by any evidence chunk):

•NLI Filter: An Natural Language Inference (NLI) model serves as a preliminary gatekeeper. If the NLI model predicts

"Entailment" (Supported) with high confidence (> 0.99), the verdict is finalized immediately.

- LLM Judge: Only ambiguous or low-confidence claims are delegated to the more expensive LLM for a final verdict. 5

A claim is "supported" if supported by at least one document in its evidence scope.

#### B.1.3. REFLECTION CHECK LOGIC

Considering some claims in intermediate steps are meta-cognitive reflections, in Round 2, we retrieve the top-Kmost 6

similar claims from the Claim Memory (accumulated from prior research steps) and task LLM to verify the unsupported

claim against these retrieved claims, which can determine if an intermediate claim unsupported by any external document is

a valid internal reflection based on the DRA's past reasoning and findings.

### B.2. Validation for Claim Verification

To validate the reliability of our automated claim verification pipeline (specifically the retrieve-then-verify module), we

benchmark its performance against human-annotated ground truth from two established fact-checking datasets.

5We utilize Moritz Laurer/DeBERTa-v3-large-mnli-fever-anli-ling-wanli(Laurer et al., 2024) and Deep Seek-v3.2

(Deep Seek-AI et al., 2025) as default NLI model and LLM respectively in this work.

6K=10 to include more abundant context.

12


---

#### B.2.1. EXPERIMENTAL SETUP

Datasets. We utilize FEVER (Thorne et al., 2018) and Sci Fact-Open (Wadden et al., 2022) to cover both general and

scientific domains.

•FEVER (General Domain): A large-scale dataset of claims derived from Wikipedia. Since the original dataset

distinguishes between Refuted and Not Enough Info (NEI), a distinction shown to be highly subjective (Jiang et al., 2020

we collapse these into a single "Unsupported" category to align with our binary verification logic. To construct a balanced

validation set efficiently, we sampled a subset containing 659 claims associated with a corpus of∼50,000 documents,

ensuring an equal distribution of Supported and Unsupported instances.

•Sci Fact-Open (Scientific Domain): A benchmark for verifying claims against scientific abstracts. This dataset is

particularly pertinent to Deep Research agents that frequently process complex academic literature. We utilize the full

test set (279 claims against a corpus of 500k abstracts). Similar to FEVER, we map the original labels to a binary

Supported/Unsupported classification. This dataset presents a significantly harder challenge due to domain-specific

terminology and complex sentence structures.

Evaluation Metrics. We evaluate the pipeline using three metrics: (1) Label Accuracy: The proportion of claims where

the predicted verdict matches the ground truth. (2) Strict Accuracy: Also known as the FEVER score (Thorne et al., 2018),

this metric counts a prediction as correct only if both the verdict is correct and the retrieved evidence matches the ground

truth evidence set. (3) Evidence Recall (R): To isolate retrieval performance, we measure the proportion of supportedev

claims for which at least one valid evidence chunk appears in the top-K candidates:

whereC supported is the set of claims with ground-truth evidence, and Cis the set where valid evidence was retrieved

successfully retrieved.

B.2.2. IMPLEMENTATION & RESULTS

Pipeline Configuration. We apply the exact Round 1 verification logic described in the main text: documents are

segmented into 15-sentence chunks, from which the top-5 relevant candidates are retrieved via our coarse-to-fine pipeline

(Embedding→Reranker). The NLI-then-LLM verification module then judges the claim against these candidates. Note

that we strictly test the Initial Verification stage (Round 1) here; the Adaptive Re-Verification (Round 2) is not applicable as

these benchmarks do not involve citation misattributions or self-reflections.

Results & Analysis. Table 4 summarizes the performance of our pipeline.

Table 4 summarizes the performance. We analyze the results from two perspectives: retrieval robustness and reasoning

precision.

Retrieval Robustness (Stress Test). The pipeline achieves high evidence recall on both FEVER (95.6%) and Sci Fact-Open

(88.3%). Crucially, this benchmark functions as a stress test for our retrieval module: while a typical Deep Research

session retrieves∼100-200 documents from the Internet, this experiment requires isolating evidence from massive pools of

50,000 (FEVER) to 500,000 (Sci Fact) documents. The high recall under these extreme conditions demonstrates that our

coarse-to-fine retrieval strategy is highly robust against the noise inherent in large-scale information environments, ensuring

that the subsequent verification stage is supplied with high-relevance context.

Reasoning Precision across Domains. In terms of verification accuracy, the pipeline excels in the general domain (FEVER:

94% Label Accuracy) and maintains robust in the complex scientific domain (Sci Fact:∼86% Label Accuracy). Although

performance naturally dips on Sci Fact due to specialized terminology and complex logic, the pipeline maintains a strong

Dataset #Claim #Document Label Acc. Strict Acc. Evidence Recall

FEVER (subset) 659 50k 0.940 0.883 0.956 Sci Fact-Open 279 500k 0.862 0.824 0.883

*Table 4. Benchmarking results of the automated claim verification pipeline against human ground truth.*

|Cretrieved ∩ Csupported |

*R*=ev*, (7)*

|Csupported |

13


---

alignment between Label Accuracy (0.862) and Strict Accuracy (0.824). This narrow gap indicates that the model rarely

guesses a correct verdict by chance; rather, its judgments are consistently grounded in the correct supporting evidence,

validating its reliability for the multi-domain rigor required in Deep Research.

#### B.2.3. CHUNK LENGTH

To determine the optimal granularity for evidence retrieval, balancing semantic integrity with token efficiency, we conducted

a sensitivity analysis on the chunk size using the FEVER development subset. We defined a chunk as a contiguous block of

*N sentences and evaluated the pipeline's performance by varying N from 1 to 20.*

*Figure 9. Impact of Chunk Length on Verification Performance. Both Label Accuracy and F1-Score improve as context expands,*

stabilizing afterN = 13. We selectN = 15(highlighted) as the optimal threshold, where Label Accuracy peaks at 94.33% and F1-Scorereaches 90.39%, balancing robust performance with computational cost.

As illustrated in Figure 9, performance is suboptimal at lower lengths (N < 5), suggesting that small windows often

fragment necessary context. While there is minor volatility in the mid-range (N ≈ 8 − 11), the metrics stabilize and reach a

high plateau as the length exceeds 13 sentences.

The performance peaks atN = 15, achieving the highest Label Accuracy of 0.9433 and F1-Score of 0.9039. Extending the

window beyond this point (N > 15) leads to a slight performance dip rather than further improvement. This trend suggests

that excessively long chunks may introduce irrelevant noise that interferes with verification, in addition to linearly increasing

the token consumption for the embedding and reranking models. Consequently, we adopt a 15-sentence window as the

standard configuration, ensuring the retrieval system captures sufficient context without incurring unnecessary computational

overhead.

#### B.2.4. NLI MODEL UTILITY

To optimize the cost-efficiency of our verification pipeline, we employ a specialized NLI model as a preliminary filter. This

component acts as a gatekeeper, resolving straightforward claims where it exhibits high confidence and delegating only

ambiguous cases to the more expensive LLM.

Setup. For each claim-evidence pair, the NLI model predicts probabilities for Entailment, Contradiction, and Neutral. We

finalize the verdict immediately only if the model predicts Entailment with extreme confidence (> 0.99) against at least one

evidence chunk. Claims with all other outcomes are delegated to the LLM. To validate this configuration, we perform an

ablation on the FEVER and Sci Fact-Open datasets, stratifying NLI predictions into three confidence intervals (0.99-1.00,

0.95-0.99, and 0.90-0.95) to assess its utility. We then benchmark the hybrid pipeline against a pure-LLM baseline to

quantify the gains in both accuracy and computational efficiency.

Results & Analysis. Tables 5 and 6 present the performance breakdown across different confidence intervals.

We observe two key findings that justify the hybrid design:

1. High Accuracy in High-Confidence Zones. When the NLI model is highly confident (> 0.99), it achieves exceptional

accuracy: 98.47% on FEVER and 90.09% on Sci Fact-Open. This confirms that for clear-cut claims, the NLI model is as

reliable as, or potentially more reliable than, the LLM. However, accuracy drops precipitously as confidence decreases (

14


---

dropping to ∼68% in the 0.90-0.95 range on FEVER), validating our decision to set a strict threshold at 0.99.

2. Superior Performance with Lower Cost. The hybrid NLI-then-LLM strategy effectively optimizes the efficiency-

accuracy trade-off. First, it slightly outperforms the pure LLM baseline on both datasets (FEVER: 0.9402 vs. 0.9333;

Sci Fact: 0.8623 vs. 0.8587), suggesting that the specialized NLI model effectively filters simple cases where LLMs might

occasionally hallucinate or over-reason. Second, it significantly reduces computational overhead. On FEVER, the NLI

model resolves 261 out of 659 claims (∼40%) directly; on Sci Fact, it handles 111 out of 279 (∼40%). This means our

pipeline reduces the demand for expensive LLM inference by approximately 40% without compromising overall verification

accuracy.

### B.3. Implementation Details in Noise Detection B.3.1. CLUSTERING IMPLEMENTATION

To manage redundancy and identifying semantic topics, we implement a two-step clustering pipeline:

•Dimensionality Reduction: We use UMAP to project embeddings into a lower-dimensional space, preserving local

semantic structures.

•Density Clustering: We apply HDBSCAN (Mc Innes et al., 2017) with parameters set tominclustersize=2,

minsamples=1, andepsilon=0. These conservative settings allow us to retain fine-grained granularity, ensuring

even small but distinct information nuggets (single-chunk clusters) are identified as valid topics.

#### B.3.2. VALIDATION OF WORST-CASE APPROXIMATION

This section validates the approximation used for the theoretical worst-case penalty () in Equation 8.Pworst

Problem Formulation. Recall that the penalty for a single ignored cluster is given byP= (S× N)/R. Toccc-invc

determine the theoretical worst-case scenario, we must identify a subset of "ignored" clustersC(where|C| = N)outoutout

from the total set of clustersthat maximizes the total penalty. Ctotal

Our proposed approximation assumes the worst case occurs when the agent ignores the highest-ranked1to clusters (Ranks *N**out*) while utilizing the lowest-ranked ones. In this scenario:

•The inversion count Nis maximized for every ignored cluster (N= N), as all utilized clusters are ranked lower.c-invc-invin

- The rank denominatoris minimized (ranging from R 1 to), maximizing the term N 1./Rcoutc

Setting Confidence Range Count Label Acc. (FEVER)

NLI Only 0.99-1.00 261 0.9847 NLI Only 0.95-0.99 93 0.8602

NLI Only 0.90-0.95 25 0.6800

Pure LLM - 659 0.9333

*Table 5. Ablation study on FEVER: NLI confidence distribution and hybrid pipeline performance.*

| Setting | Confidence | Range | Count | Label | Acc. | (Sci Fact) |
| --- | --- | --- | --- | --- | --- | --- |
| NLI Only 0.99-1.00 111 | 0.9009 |  |  |  |  |  |
| NLI Only 0.95-0.99 42 | 0.7143 |  |  |  |  |  |
| NLI Only 0.90-0.95 13 | 0.8462 |  |  |  |  |  |
| Pure LLM - 279 | 0.8587 |  |  |  |  |  |

*Table 6. Ablation study on Sci Fact-Open: NLI confidence distribution and hybrid pipeline performance.*

| Hybrid | (Ours) | Hybrid | 659 | 0.9402 |
| --- | --- | --- | --- | --- |
| Hybrid (Ours) Hybrid 279 | 0.8623 |  |  |  |

15


---

However, because the cluster size Svaries, simply selecting the highest-ranked clusters might not yield the absolutec

maximum if a lower-ranked cluster has a significantly larger size Sthat outweighs the penalty reduction from a largerR.cc

Rigorously, finding the true global maximum is a combinatorial optimization problem:

Pmax = max

where N*c-inv*(S) denotes the count of clusters in\ S C ranked lower than c.

Computational Complexity. Solving this exact optimization requires enumerating all possible subsetsS. The search

space size is defined by the binomial coefficient. To assess feasibility, we analyzed clustering results from 100 full

research trajectories generated by Gemini Deep Research (50 from Mind2Web2, 50 from Report Eval). As shown in Table 7,

## 168*Ntotal*

the mean search space size exceeds, rendering brute-force enumeration computationally infeasible. 10

Metric Mean Max Min

Search Space (logT ) 168 849 10

*Table 7. Estimated computational complexity (T ) for exact worst-case enumeration across 100 research trajectories.*

Statistical Justification. Given the computational intractability, we validate our rank-based approximation by analyzing

the distribution of cluster sizes (S). If Svaried dramatically (e.g., if one lower-ranked cluster contained 50% of all chunks),cc

the rank-based assumption would fail.

However, empirical data in Figure 12 reveals a highly concentrated distribution. Across both datasets,∼75% of clusters

consist of only 1-3 chunks. This low variance implies that Sacts nearly as a constant factor relative to the rank Randcc

inversion countN. Consequently, swapping a high-ranked small cluster for a low-ranked large cluster is statisticallyc-inv

unlikely to increase the total penalty, as the penalty reduction from increased rankR(which grows linearly) typicallyc

outweighs the gain from a marginally larger. Sc

Therefore, our approximation, defining the worst case as the neglect of the highest-ranked clusters, serves as a robust and

computationally efficient proxy for the theoretical maximum.

## C. Benchmark Details

### C.1. Candidate Data Sources

To ensure broad coverage, we aggregated queries from three high-quality sources:

- Mind2Web2: We utilized the full set of 130 search-oriented open-ended queries.
- Report Eval: We included 100 research-oriented open-ended queries covering multifaceted topics.

•Browse Comp: From this large-scale dataset of∼1,200 close-ended tasks, we employed stratified sampling to select

∼100 representative queries, preserving the original topic distribution while keeping workload manageable.

### C.2. Difficulty Assessment Logic

Our selection process relies on the premise that queries inducing hallucinations in a top-performing DRA (e.g., Gemini

Deep Research) are likely to be effective stress tests for other DRAs. Queries with the highest Hscores were retained for

the final benchmark.

### C.3. Dataset Composition

Deep Hallu Benchcontains queries spanning 11 distinct domains, ranging from humanities (e.g., Art, Music & Literature,

History) to technical fields (e.g., Science & Technology). Figure 10 visualizes this distribution.

As shown in the left chart (N = 100), the dataset achieves a broad and multifaceted coverage of complex topics. Dominant

segments include Art, Music & Literature (19.0%), Science & Technology (13.0%), and Entertainment & Gaming (13.0%),

while specialized fields like Politics, Health, and Career provide critical diversity. The right chart (N = 75) confirms that

X*S× N*(S)

*c c-inv*

*, (9)*

S⊂C *total**,|S|=Nout* *R**c*

*c∈S*

*total*

*Nout*

10

16


---

*Figure 10. Topic Distribution of Deep Hallu Bench. The left chart details the domain breakdown for the full benchmark (N = 100),*

including adversarial queries. The right chart illustrates the distribution for the "answerable" subset (i.e., without "no-answer" queries, *N = 75). The broad coverage across 11 diverse categories prevents domain-specific bias and ensures a holistic assessment of DRA*

capabilities.

excluding the 25 adversarial "no-answer" queries preserves this distribution structure, confirming that our analysis remains

statistically robust across varying subject matters.

Domain Vulnerability Analysis. Figure 11 illustrates the filtering process from the initial candidate pool to the final

benchmark. The percentage above each bar represents the Selection Ratio, i.e., the proportion of queries in that domain that

triggered significant hallucinations and were thus retained for the final difficult set.

Analyzing these ratios reveals a critical insight: hallucinations are unevenly distributed across domains. While high-resource,

popular topics such as Science & Technology and Lifestyle & Leisure show relatively low selection rates, identifying them as

areas where DRAs are generally robust. In contrast, "long-tail" or specialized domains exhibit much higher vulnerability.

Notably, Geography & Environment has the highest selection ratios of 75.0%, despite having smaller initial candidate counts.

This suggests that DRAs struggle significantly more with niche topics. The severe hallucination degrees in domains like

Geography (75.0%) are likely attributable to their long-tail nature and the density of specialized domain knowledge. These

factors complicate accurate retrieval and synthesis, thereby increasing the propensity for fabrication when the agent cannot

access or reason over obscure facts.

### C.4. Case Study for Atomic Perturbations

*Table 8. Examples of Atomic Perturbations. We merge different perturbation types into a single view, each type with three examples*

respectively.

Query (Original) Query (Modified) Modification

Type 1: Entity Attribute Modification

Continued on next page...

17


---

Table 8 continued from previous page

Query (Original) Query (Modified) Modification

A musical artist has a first name and surnameA musical artist has a first name and surname Entity Substitution

that begins with the exact same letter (as ofthat begins with the exact same letter (as of(Institution): Change

2023). This musical artist quoted a Columbia2023). This musical artist quoted a University"Columbia University"

University alumnus in a 2019 interview. Theof Idaho alumnus in a 2019 interview. Theto "University of

year prior to the interview, the musical artist re-year prior to the interview, the musical artist re-Idaho"

leased a song in which the lyrics liken a bodilyleased a song in which the lyrics liken a bodily

organ to a food item. The food item in questionorgan to a food item. The food item in question

has a strong historical connection to a mythicalhas a strong historical connection to a mythical

individual. This mythical individual shares aindividual. This mythical individual shares a

name with a protagonist in a speculative fictionname with a protagonist in a speculative fiction

novel published as the first in a series, betweennovel published as the first in a series, between

2000 and 2005 (inclusive). What is the title of2000 and 2005 (inclusive). What is the title of

the song? the song?

Give me the first and the last name of the foot-Give me the first and the last name of the foot-Attribute Modifica-

|  |  |
|---|---|
|  |  |
|  |  |
| each other? what age did Person 2 and the Illustrator knowSince what age did Person 2 and the Illustrator lustrator knew each other for a long time. Sincethe illustrator knew each other for a long time. founded in the early 1700s. Person 2 and the il-sity founded in the early 1700s. Person 2 and degree in literature from another universitygree in computer science from another univer- ter’s degree in graphic design and a bachelor’sdegree in graphic design and a bachelor’s de- sive. The illustrator of that book has a mas-sive. The illustrator of that book has a master’s | Type 2: Temporal Detail Modification know each other? |
| book in years between 2010 and 2020, inclu-book in years between 2010 and 2020, inclu- the Georgian era and has published their firstthe Georgian era and has published their firstscience”. graduate of one of the universities founded ingraduate of one of the universities founded inture” to “computer name, and identical ethnicity. Person 2 is aname, and identical ethnicity. Person 2 is aChange “litera- ties with person 2 such as a near-identical lastties with person 2 such as a near-identical lastcation (Education): There’s a person 1 who shares many similari-There’s a person 1 who shares many similari-Attribute Modifi- two brothers. 1995 under the zodiac sign Taurus, he also has1995 under the zodiac sign Scorpio, he also able in January 2014. Born between 1988 andable in January 2014. Born between 1988 and of an European country as of information avail-of an European country as of information avail- in an African country, he later had nationalityin an African country, he later had nationality seasons in the Premier League. Although bornseasons in the Premier League. Although born This player represented the same club for sevenThis player represented the same club for seven“Scorpi” | has two brothers |
|  |  |
| country to play in the English Premier League?country to play in the English Premier League?Change “Taurus” to ball player who became the first from his birthball player who became the first from his birthtion (Birth Data): |  |

ball player who became the first from his birthball player who became the first from his birthtion (Birth Data):


---

Table 8 continued from previous page

Query (Original) Query (Modified) Modification

A university established between 1995 andA university established between 1995 and Temporal Shift

2005 (exclusive) organized the inaugural ses-2005 (exclusive) organized the inaugural ses-(Event Timing):

sion for the student branch of an organizationsion for the student branch of an organization Change "July" to

that aims to promote technological advance-that aims to promote technological advance-"January"

ments less than ten years after it was estab-ments less than ten years after it was established, during the first week of July. Lesslished, during the first week of January. Less

than fifteen years after this session, a memberthan fifteen years after this session, a member

of the student branch was awarded a scholar-of the student branch was awarded a scholarship that recognizes leadership and academicship that recognizes leadership and academic

qualities in students. They were the first stu-qualities in students. They were the first stu-

|  |  |
|---|---|
|  |  |
| created the show’s main theme? first name and surname of the composer whofirst name and surname of the composer who house in the following year. What was thehouse in the following year. What was the ticle for the same paper about decorating theirticle for the same paper about decorating their omega.’ The writer of this review wrote an ar-omega.’ The writer of this review wrote an ar- aforementioned pair of actors as its ’alpha andaforementioned pair of actors as its ’alpha and on. A 2010 review of the show described theon. A 1995 review of the show described the one of the previous projects they had workedone of the previous projects they had worked videos and missed out on the same award withvideos and missed out on the same award with had also worked on famous musicians’ musichad also worked on famous musicians’ music it won was due to the work of an artist whoit won was due to the work of an artist who inated for more than two. One of the awardsinated for more than two. One of the awards show won fewer than four wards and was nom-show won fewer than four wards and was nom-“2010” to “1995”. starred in it attended the same university. Thestarred in it attended the same university. The(Source Document): A TV show aired in the 1990s. Two actors thatA TV show aired in the 1990s. Two actors thatTemporal Shift in the paper. | created the show’s main theme? in the paper. |
| State the full name of this author as expressedState the full name of this author as expressed iated with the same university as this person.iated with the same university as this person. lecturer. The paper had one other author affil-lecturer. The paper had one other author affil- year they joined the university as a full-timeyear they joined the university as a full-time of the paper, after revision, was submitted theof the paper, after revision, was submitted the year after their graduation. The second versionyear after their graduation. The second version possibilities. The paper was first submitted apossibilities. The paper was first submitted a in finding the best solution from a finite set ofin finding the best solution from a finite set of and testing how two different methods performand testing how two different methods perform authors of a paper that focuses on comparingauthors of a paper that focuses on comparing received their bachelor’s. They are one of thereceived their bachelor’s. They are one of the full-time lecturer in the same department theyfull-time lecturer in the same department they after graduating, they joined the university as aafter graduating, they joined the university as a ter receiving this award. Less than three yearster receiving this award. Less than three years They graduated from the university a year af-They graduated from the university a year af- |  |

dent from the university to win the scholarship.dent from the university to win the scholarship.


---

Table 8 continued from previous page

| Query | (Original) | Query | (Modified) | Modification |
| --- | --- | --- | --- | --- |
| sponsored and remained in service until 1973.sponsored and remained in service until | 1973. |  |  |  |

surname. surname."

Type 3: Quantitative Value Modification

In a little agricultural town with a population In a little agricultural town with a population Detail Modification

density of less than 25,000 people, as recordeddensity of less than 25,000 people, as recorded(Document Content):

before 2023, but after 2020, a proposal wasbefore 2023, but after 2020, a proposal was Change "Five stone

submitted to extend a section of the town,submitted to extend a section of the town,foundations" to "Fifty

adding over 400 new housing plots. The sec-adding over 400 new housing plots. The sec-stone foundations".

tion to be extended is inhabited by people whotion to be extended is inhabited by people who

were relocated from an earlier settlement thatwere relocated from an earlier settlement that

bore a name originating from Aramaic. Dur-bore a name originating from Aramaic. During a site inspection between 2010 and 2015ing a site inspection between 2010 and 2015

inclusive, a report was issued with the follow-inclusive, a report was issued with the following details: a) Area situated on a bedrock ofing details: a) Area situated on a bedrock of

what was once molten magma b) The area iswhat was once molten magma b) The area is

disturbed c) No graves or rock engravings d)disturbed c) No graves or rock engravings d)

Five stone foundations noted Please provide Fifty stone foundations noted Please provide

me with the name of the company that com-me with the name of the company that compiled this survey, as well as the initials andpiled this survey, as well as the initials and

surname of the individual who compiled thissurname of the individual who compiled this

particular survey. Also include the year of theparticular survey. Also include the year of the

report in parentheses. report in parentheses.

In 2021 an article discussing a meme posted on In 2021 an article discussing a meme posted on Fingerprint Ala social media platform was published by a na-a social media platform was published by a na-teration (Count):

tional newswire service founded in the 1930s.tional newswire service founded in the 1930s.Change "exactly 5

The article references by name exactly 3 au-The article references by name exactly 3 au-books" to "exactly 15

thors, 1 lecturer, and 1 foundation president. Itthors, 1 lecturer, and 1 foundation president. Itbooks".

also references exactly 5 books by name and 1also references exactly 15 books by name and 1

book series by name. A 2015 article discussesbook series by name. A 2015 article discusses

the author cited in the text of the meme fromthe author cited in the text of the meme from

the 2021 article. The 2015 article was pub-the 2021 article. The 2015 article was published on a platform that an article publishedlished on a platform that an article published

in August of 2021 cites as being created by ain August of 2021 cites as being created by a

person with a Ph.D. in computer science. Whatperson with a Ph.D. in computer science. What

is the first and last name of the 1997 footballis the first and last name of the 1997 football

coach referenced in the 2015 article?coach referenced in the 2015 article?

Continued on next page...

20


---

Table 8 continued from previous page

Query (Original) Query (Modified) Modification

A child was reported missing several timesA child was reported missing several times Fact Alteration (Incibetween January 1, 2014, and December 31,between January 1, 2014, and December 31,dent Detail): Change

2018. In late 2014, the missing 13-year-old2018. In late 2014, the missing 13-year-old"two other missing

was found along with two other missing teens.was found along with seven other missingteens" to "seven other

In late 2015, the 14-year-old was also reportedteens. In late 2015, the 14-year-old was alsomissing teens".

missing but was located shortly afterward. Inreported missing but was located shortly afearly 2018, the 16-year-old was reported miss-terward. In early 2018, the 16-year-old was

ing. According to the police's description,reported missing. According to the police's dewhat color shirt were they last wearing whenscription, what color shirt were they last wearthey went missing in 2018? ing when they went missing in 2018?

Type 4: Logical Relationship Modification

This African leader, born in the early 20th This African leader, born in the early 20th Logic/Riddle Break:

century visited the official residence of thecentury visited the official residence of the Changed the descripleader of a global superpower in the 21st cen-leader of a global superpower in the 21st cen-tion of the topping's

tury. Apart from helping boost the economytury. Apart from helping boost the economynamesake (from an

of his country of origin, he also played a piv-of his country of origin, he also played a piv-assassination warner,

otal role in the restoration of peace in Eastotal role in the restoration of peace in Eastto the discoverer of

Africa. During his visit to the residence of this Africa. During his visit to the residence of thispenicillin).

global superpower's leader, a grand dinner wasglobal superpower's leader, a grand dinner was

held in his honor, featuring a particular dessertheld in his honor, featuring a particular dessert

topping that shares its name with a prominenttopping that shares its name with a prominent

individual who was burdened with a wordindividual who discovered penicillin. Can you

of caution that could avert the assassinationprovide the name of this food?

of a former leader of this same global superpower. Can you provide the name of this

food?

The university was established between 2000The university was established between 2000Logical Impossibility

and 2003, inclusive. Prior to December 2023,and 2003, inclusive. Prior to December 2023,(Timeline): Change

the university's founder was a scientist andthe university's founder was a scientist and"10th anniversary" to

the chairman of its board of trustees. Theythe chairman of its board of trustees. They"50th anniversary".

earned their PhD from an institute that wasearned their PhD from an institute that was

officially recognized as a university in Julyofficially recognized as a university in July

between 1965 and 1968, inclusive. Prior tobetween 1965 and 1968, inclusive. Prior to

December 2023, students at the university were December 2023, students at the university were

required to take mandatory language coursesrequired to take mandatory language courses

in a specific foreign language. Between 2020in a specific foreign language. Between 2020

and 2023, inclusive, the university celebratedand 2023, inclusive, the university celebrated

the 10th anniversary of its campus openingthe 50th anniversary of its campus opening

in another country. What is the name of thein another country. What is the name of the

university? university?

Between 1990 and 2002 inclusive, this music Between 1990 and 2002 inclusive, this music Procedural Imgroup lost one of their parents. The incidentgroup lost one of their parents. The incidentpossibility (Legal

was classified as a homicide. In the trial, the in-was classified as a homicide. In the trial, the in-Context): Change "8

dividual accused of the murder had an attorneydividual accused of the murder had an attorneyand 17" to "1 and 2".

who once represented an individual in a casewho once represented an individual in a case

where the crime/incident occurred in that samewhere the crime/incident occurred in that same

year range. In this same trial, an individual atyear range. In this same trial, an individual at

a very young age, between 8 and 17, testifieda very young age, between 1 and 2, testified in

in it. Which month did this trial begin?it. Which month did this trial begin?

21


---

*Figure 11. Comparison of Candidate vs. Selected Sets. The percentages indicate the Selection Ratio for each domain, defined as the*

ratio of queries retained for the final benchmark to the total candidate pool aggregated from the source datasets.

The specific distribution of these perturbation types across the 25 adversarial queries is summarized in Table 9, ensuring

coverage across semantic, temporal, quantitative, and logical restrictions.

| these scores across all tasks in the benchmark. top-5 candidates to determine the task-level quality. The final Retrieval Quality for a DRA is calculated by averaging the document’s length or surrounding irrelevant text. score as the maximum score of its constituent chunks, ensuring we capture high-value information signals regardless of relevance score against each atomic sub-query via a reranker, using the average as the chunk’s final relevance score. | Total 25 (4) Logical Relationship Modification 3 (3) Quantitative Value Modification 4 |  |  |  |  |  |  |  |  |  |
|---|---|---|---|---|---|---|---|---|---|---|
| (2) | Temporal | Detail | Modification | 9 | (1) | Entity | Attribute | Modification | 9 |  |
| (1) | Entity | Attribute | Modification | 9 | Perturbation | Type | Count |  |  |  |
|  |  |  |  |  |  |  |  |  |  |  |

*Table 9. Distribution of Adversarial Perturbations. The dataset prioritizes entity and temporal modifications while including specific*

We quantify Retrieval Quality by assessing the relevance of the top-ranking documents retrieved for a user query. Since

would obscure the agent's true capability to locate critical information. To measure the agent's peak retrieval power (i.e.


---

*Figure 12. Distribution of Semantic Cluster Sizes. The majority of clusters are small, with∼75% containing only 1-3 chunks. This low*

variance supports the rank-dominant approximation.

(a) Explicit Summarization: Claim Hallucination (b) Implicit Summarization: Noise Domination

*Figure 13. Domain-specific performance for Summarization Hallucinations. High-entropy domains like Entertainment and Sports induce*

higher hallucination degree across both dimensions compared to structured domains like Economy.

high-value evidence.

### D.2. Domain Sensitivity Analysis

To further investigate the "hallucination profiles" of DRAs, we decompose their performance across 11 distinct query

domains. The following radar plots visualize the distribution of hallucinations for each DRA, highlighting the disparity

between structured domains (e.g., Economy, Science) and high-entropy domains (e.g., Entertainment, Lifestyle).

Figure 13 details the Summarization stage, including Claim Hallucinations (Explicit) and Noise Domination (Implicit).

Figure 14 details the Planning stage, visualizing Action Hallucinations (Explicit) and Restriction Neglect (Implicit). Finally,

Figure 15 presents the composite hallucination score (H) across all domains.

These figures show that hallucination severity exhibits strong domain dependency. We observe three distinct patterns:

•The "Universal Trap": Geography & Environment emerges as the most challenging domain, ranking poorly across

nearly all categories. This suggests that spatial reasoning and dispersed environmental data trigger systemic failures in

both planning logic and information summarization.

23


---

(a) Explicit Planning: Action Hallucination (b) Implicit Planning: Restriction Neglect

*Figure 14. Domain-specific performance for Planning Hallucinations. Note the specific spike in Action Hallucination for Geography and*

Politics, and the high Restriction Neglect in Lifestyle and Career domains.

•Structured vs. High-Entropy: A clear performance gap exists between structured and unstructured fields. DRAs

demonstrate high performance in Economy and Medicine, benefiting from standardized terminologies. In contrast,

performance degrades significantly in high-entropy, pop-culture domains like Entertainment and Sports.

•Restriction Nuance: Implicit planning reveals a vulnerability to qualitative ambiguity. While agents effectively minimize

restriction neglect in Science ("hard" restrictions), they falter in Career and Lifestyle. This indicates a fundamental

difficulty in parsing the "soft," subjective constraints inherent to human-centric tasks.

D.3. Close vs. Open-Ended

Figure 16 reveals that close-ended tasks impose a significantly higher challenge, triggering elevated error rates across critical

dimensions compared to open-ended tasks. Specifically, we observe systemic spikes in Fabrication, Noise Domination and

Action Hallucination in the close-ended setting. This phenomenon stems from the inherent difficulty of the Browse Comp

dataset, where queries impose rigid, binary restrictions that demand exact retrieval. Unlike open-ended reporting, where

agents can synthesize broad information to mask retrieval gaps, these rigorous restrictions force agents into immediate

failure modes, cascading into subsequent steps. Thus, rather than being simple, close-ended tasks serve as a severe stress

test for retrieval precision and summarization faithfulness.

## E. Extended Analysis of Failure Mechanisms

### E.1. Propagation Detection Methodology

To construct the Directed Acyclic Graph (DAG) presented in Section 5.3, we detect propagation between explicit hallucinations through two specific mechanisms:

•Homogeneous Propagation: This captures errors propagating within the same modality (→i.e.Fabrication, Fabrication

or Deviation→Deviation). We identify these links by leveraging NLI models to detect high-confidence entailment

relationships between successive error nodes.

•Heterogeneous Propagation: This captures errors crossing modalities (Fabrication→Deviation). These are identified

via our Action Propagation metric (A) defined in Section 4.2, where an action is deemed compliant with apropagation

hallucinated premise.

We limit this graph analysis to Gemini, OpenAI, and Salesforce, as other DRAs do not expose the sufficient intermediate

summarizations or plans required for granular propagation tracking.

24


---

"Universal Trap" for current DRAs.

*Figure 15. Composite Hallucination Score (H) across query domains. Geography & Environment represents the most challenging*

### E.2. Root-Cause Error Analysis

To understand the etiology of final failures in close-ended tasks, we isolate the root-cause error, defined as the earliest step

in the research trajectory that precipitates the final incorrect outcome. Following (Zhu et al., 2025b), we leverage an LLM to

identify this critical pivot point by analyzing the full trajectory alongside the final answer.

Figure 17 visualizes the distribution of these root-cause errors.

### E.3. Semantic Bias Analysis

We further investigate how information diversity impacts agent performance. Figure 18 visualizes two key trends:

Preference for Redundancy (Top). We compare the average size (chunk count) of utilized clusters (In-Memory) versus

ignored clusters (Out-Memory). Across all agents, utilized clusters are consistently larger (e.g., Gemini: 4.0 vs. 2.5 chunks).

This confirms that DRAs use repetition as a proxy for importance, favoring homogeneous content over singleton insights.

Vulnerability to Diversity (Bottom). We analyze the correlation between information heterogeneity (total cluster count)

and the Noise Score. For weaker models like Salesforce and Grok, we observe a significant positive correlation. This

implies that as the retrieval context becomes more diverse (more distinct topics), the DRA's attention mechanism fails to

prioritize effectively, leading to higher rates of information neglect.

25


---

| Figure 16. Comparison of Hallucination Metrics between Open-Ended and Close-Ended tasks. Light bars denote open-ended tasks, | while dark bars denote close-ended ones. Close-ended tasks generally incur more severe hallucinations across most metrics; the notable | exception is Misattribution, which is naturally higher in open-ended tasks due to the requirement for long-form reports containing | numerous citations, contrasting with the short-form answers typical of close-ended queries. |
| --- | --- | --- | --- |
| Figure 17. Heatmap of Root-Cause Errors across Modules and Stages. We classify detected root-cause errors by module and research | stage. Search denotes cases where the agent failed to retrieve information and reported "no answer found" despite a trajectory free of | hallucinations. None denotes cases where the agent produced a fabricated answer despite a research trajectory containing no detectable | errors. Darker cells indicate higher frequency. |
| Figure 18. Semantic Analysis of Information Attention. Top: Average size of utilized clusters versus ignored clusters. Bottom: | Correlation between the total number of clusters (information heterogeneity) and the Noise Score. |

26


---

## F. Prompts

To ensure the robustness of our automated evaluators, we employ an iterative human-in-the-loop prompt optimization

strategy. Prompts are refined over multiple cycles of expert critique until the judgment logic stabilizes and produces accurate

results, ensuring the LLM judges align closely with human reasoning.

### F.1. Prompt for Decomposition F.1.1. QUERY DECOMPOSITION

You are an expert query analysis system specialized in decomposing user queries into structured

atomic restrictions.

## TASK

Extract concise, independent Atomic Restrictions from user queries.

## ATOMIC CONSTRAINT CRITERIA

Each extracted constraint must satisfy the following properties:

- Indivisibility: Must be a single, self-contained unit with clear meaning. Break down complex

queries (linked by 'and', 'with', 'while') into separate items.

- Objectivity: Must contain objective conditions or criteria only. Exclude descriptive facts,

background information, or subjective statements.

- Context Independence: Must be neutral and understandable in isolation. Remove personal

references (\eg, 'I', 'me', 'my', 'for me') and ambiguous pronouns.

## EXTRACTION METHODOLOGY

1. Decompose: Split compound sentences into individual atomic units based on the criteria above.
2. Refine: Ensure strictly objective, neutral language.
3. Format: Output each constraint on its own line prefixed with '- '.

#### F.1.2. REASONING TEXT DECOMPOSITION

You are an expert text decomposition system specialized in reconstructing research trajectories

by disentangling reasoning text interleaved with plans and summaries.

## TASK

Deconstruct paragraphs to isolate and extract Atomic Claims (from summaries) and Atomic Actions

(from plans). You must perform systematic fragmentation and classification to ensure every

extracted item satisfies the criteria of Indivisibility, Semantic Integrity, Verifiability, and

Context Independence.

## METHODOLOGY

### 1. Source Fidelity

- Use the provided paragraph as the single source of truth. The query is context only; never add

details that are not explicitly written in the paragraph.

- Do not infer missing steps, reasons, or entities from background knowledge.

### 2. Step 1: Fragmentation (Minimal Splitting & Disentanglement)

- Produce the smallest set of fragments that faithfully reflect the paragraph's explicit

sentences.

- Disentanglement: If a sentence mixes summaries and plan (interleaved reasoning), split *only*

along that boundary; otherwise keep the sentence intact.

- Resolve pronouns using paragraph context immediately to ensure atoms are self-contained.

27


---

Context reminder: The text may contain both discoveries and plans. Classify only what is

explicitly written.

- `summaries`: Facts, findings, reflections, or summary statements (Output as Atomic Claims).
- `plan`: Actions the agent explicitly states it will take next (Output as Atomic Actions).

### 4. Step 3: Atomic Extraction (The 4 Essential Properties)

Refine the classified fragments into valid atomic units. Each unit must strictly satisfy the

following four properties defined in the research trajectory:

1. Indivisibility: The unit must represent a single, indivisible action or claim; further

splitting would compromise its semantic meaning.

*Operational Rule:* Prefer to keep clauses together; only split truly parallel elements

(\eg, clearly enumerated lists).

2. Semantic Integrity: Each unit must retain sufficient detail to preclude ambiguity, including

necessary conditions and clauses, ensuring the original intent is fully preserved.

*Operational Rule:* Keep integral conditions attached (\eg, 'Search for issues... *with

the specified label*'). Do not fragment conditions from their actions.

3. Verifiability: The unit must be objectively verifiable. Speculative language and subjective

opinions are filtered out.

*Filtering Criteria:* EXCLUDE speculative language ('may', 'might', 'could', 'likely',

'seems'), subjective opinions ('effective', 'best'), and vague process descriptions.

4. Context Independence: All coreferences (\eg, pronouns) must be explicitly resolved, ensuring

the unit can be assessed in isolation without relying on preceding context.

### 5. Format Compliance (For Plans)

- Imperative Verbs: Atomic Actions must start with an imperative verb (\eg, 'Search', 'Analyze',

'Run').

- Ignore implied steps; strictly output the explicit action described.

## EXAMPLES

Decomposition & Context Independence:

- Input: I found some roles, but I need to search more.
- Output: Two fragments:
- I found some roles (summary)
- Search for more roles (plan)

Verifiability (Filtering):

- Input: This approach likely improved performance by 15%.
- Output: No extractable content (Speculative likely).
- Input: The neural network optimization approach improved performance by 15%.
- Output: The neural network optimization approach improved performance by 15% (summary)

Indivisibility (Atomic Extraction):

- Input: Meta's careers page lists 'Research Scientist' in Menlo Park, CA, and Seattle, WA.
- Output:
- Meta's careers page lists 'Research Scientist' in Menlo Park, CA
- Meta's careers page lists 'Research Scientist' in Seattle, WA

Semantic Integrity - DO NOT Split Conditions:

- Input: Search for issues within the target module that have the specified label.
- [Incorrect] Wrong Output:

28


---

- Search for issues within the target module
- Filter issues with the specified label
- [Correct] Output:
- Search for issues within the target module that have the specified label

## OUTPUT FORMAT

Fragment 1: [Context-independent text]

Classification: [summary/plan]

Atomic [Claims/Actions]:

If no extractable content: `No extractable content paragraph contains only vague descriptions

or speculative language.'

#### F.1.3. REPORT PARAGRAPH DECOMPOSITION

You are an expert fact decomposition system specialized in extracting Atomic Claims from text.

## TASK

Extract ONLY concrete, verifiable observations or findings. You must decompose the text into

Atomic Claims that satisfy the criteria of Indivisibility, Semantic Integrity, Verifiability,

and Context Independence.

## ATOMIC CLAIM PROPERTIES (METHODOLOGY)

### 1. Indivisibility

The unit must represent a single, indivisible fact.

- Operational Rule: Only split truly parallel elements (\eg, X and Y where X and Y are

independent facts).

- Constraint: Do NOT split complex sentences if doing so would compromise semantic meaning or

disconnect a clause from its subject.

### 2. Semantic Integrity

Each unit must retain sufficient detail to preclude ambiguity.

- Operational Rule: Preserve all modifiers, conditions, and qualifiers that are semantically

integral to the main clause.

- Constraint: Do NOT split prepositional phrases, relative clauses, or purpose clauses (\eg, 'to

find...') from the entity they modify.

| using the paragraph context. - Operational Rule: Replace pronouns ('this', 'that', 'it', 'they') with specific referents All coreferences must be explicitly resolved ensuring the claim is self-contained. ### 4. Context Independence - URLs. - Vague process summaries ('Progress has been made...', 'We plan to...'). - Subjective opinions ('effective', 'ideal', 'best', 'good', 'useful'). - Speculative language ('may', 'might', 'could', 'possibly', 'likely', 'appears', 'seems'). - FILTER OUT (Exclude): |
|---|
| - FILTER OUT (Exclude): - Include: Specific facts, data, concrete entities, locations, numbers, and definitive results. The unit must be objectively verifiable. ### 3. Verifiability |


---

- Verification Test: Can someone verify this claim's truthfulness without reading the original

surrounding text?

## EXAMPLES

Verifiability (Filtering Speculation):

- Input: This approach likely improved performance by 15%.
- Output: No extractable content (Speculative likely).
- Input: The neural network optimization approach improved performance by 15%.
- Output: - The neural network optimization approach improved performance by 15%

Context Independence (Resolution):

- Input: Google xxx. They offer remote positions.
- Output: - Google offers remote positions

Indivisibility (Parallel Elements):

- Input: Meta has roles in Menlo Park and Seattle.
- Output:
- Meta has a role in Menlo Park
- Meta has a role in Seattle

Semantic Integrity - DO NOT Split Conditions:

- Input: xxx to find information about the oldest closed issue in the target module with the

specified label

- [Incorrect] Wrong Output:
- xxx to find information about the oldest closed issue in the target module
- The oldest closed issue in the target module has the specified label
- [Correct] Output:
- xxx to find information about the oldest closed issue in the target module with the

specified label

## OUTPUT FORMAT

If no extractable content: `No extractable content paragraph contains only vague descriptions

or speculative language.`

#### F.1.4. DOUBLE CHECK FOR ATOMIC CLAIMS

You are a quality control system specialized in validating and refining Atomic Claims as a

secondary double-check layer.

## TASK

Review preliminary claims to rectify common errors in Divisibility (\eg, parallel structures)

and Context Independence (\eg, unresolved pronouns).

## REFINEMENT CRITERIA

### 1. Indivisibility (Split Parallel Structures)

Ensure each claim represents a single, indivisible fact.

- Rule: Break compound statements linked by `and`, `or`, `but` ONLY when they represent

independent, parallel facts that do not affect each other's meaning.

- Example: `Role available in Menlo Park and Seattle` -> Split into two separate claims.

30


---

### 2. Semantic Integrity (Do NOT Split Modifiers)

Preserve semantic detail to preclude ambiguity.

- CRITICAL: Do NOT split modifiers, conditions, or qualifiers from their main clauses.
- Preserve:
- Prepositional phrases (\eg, `within the target module`).
- Relative clauses (\eg, `that have the specified label`).
- Purpose clauses and integral qualifiers.

### 3. Context Independence (Resolve Coreferences)

Ensure claims are verifiable in isolation without surrounding context.

- Resolve Pronouns: Replace `the`, `this`, `that`, `it`, `they` with specific entity names.
- Explicit References: If a claim references `the position` or `this role`, specify the exact

entity using the broader context.

- Exclusion: If the context for a pronoun or reference cannot be determined, exclude the claim

entirely.

## EXAMPLES

Indivisibility (Parallel Elements - OK to Split):

- Input: `Role available in Menlo Park, CA and Seattle, WA`
- Output:
- Role available in Menlo Park, CA
- Role available in Seattle, WA

Semantic Integrity - DO NOT Split Conditions:

- Input: `xxx to find information about the oldest closed issue in the target module with the

specified label`

- [Incorrect] Wrong Output:
- `xxx to find information about the oldest closed issue in the target module`
- `The oldest closed issue in the target module has the specified label`
- [Correct] Output:
- `xxx to find information about the oldest closed issue in the target module with the

specified label`

Context Independence (Resolution):

- Input: `The position focuses on experimenting with neural network architectures.`
- Context: Deep Mind Research Engineer/Scientist position
- Output: `Deep Mind Research Engineer/Scientist position focuses on experimenting with neural

network architectures`

## OUTPUT FORMAT

Return each refined, atomic claim on a new line with `- ` prefix.

#### F.1.5. DOUBLE CHECK FOR ATOMIC ACTIONS

You are a quality control system specialized in validating and refining Atomic Actions as a

secondary double-check layer.

## TASK

Review preliminary actions to rectify common errors in Divisibility, Context Independence, and

Format. Remove any items that are observations (facts) rather than actions.

## REFINEMENT CRITERIA

31


---

### 1. Indivisibility (Split Parallel Actions)

Ensure each action represents a single, indivisible task.

- Rule: Break compound statements linked by 'and', 'or', 'but' ONLY when they represent

independent, parallel actions that do not affect each other's meaning.

### 2. Semantic Integrity (Do NOT Split Modifiers)

Preserve semantic detail to preclude ambiguity.

- CRITICAL: Do NOT split modifiers, conditions, or qualifiers from their main clauses.
- Preserve:
- Prepositional phrases (\eg, 'with the specified label', 'within the target module').
- Relative clauses (\eg, 'that have the specified label').
- Purpose clauses and integral qualifiers.

### 3. Context Independence (Resolve Coreferences)

Ensure actions are executable in isolation without surrounding context.

- Resolve Pronouns: Replace 'the', 'this', 'that', 'it', 'they' with specific entity names.
- Context Integration: Use broader action list context to provide necessary specificity.
- Exclusion: If context cannot be determined, exclude the action entirely.

### 4. Format Compliance & Validity

- Imperative Form: Start with a verb. Remove subjects like 'I', 'the agent', 'the user'. (\eg,

Transform 'I will search' to 'Search').

- Validity Check: If the item is a fact/claim (\eg, 'Ronnie Wood has four children') and not a

plan/action, remove it.

## EXAMPLES

Basic Action Transformation:

- Input: The agent will search for authors and identify the ones that have the specified label
- Output:
- Search for authors
- Identify the ones that have the specified label

Semantic Integrity - Do NOT Split Conditions:

- Input: Search for issues within the target module that have the specified label
- [Incorrect] Wrong Output:
- Search for issues within the target module
- Filter issues with the specified label
- [Correct] Output:
- Search for issues within the target module that have the specified label

Context Independence:

- Input: Confirm this information
- Context: Check the population data for Tokyo first -> Confirm this information
- Output: Confirm the population data for Tokyo

## OUTPUT FORMAT

Return each refined, atomic action on a new line with '- ' prefix.

### F.2. Prompt for Claim Verification

You are an expert claim verification system specialized in assessing the evidentiary

relationship between a specific claim and a retrieved document chunk.

32


---

## TASK

Given a claim, a query, and a document chunk, classify the relationship as Support or Unsupport.

- Source of Truth: The provided document chunk represents information explicitly retrieved by

the agent during its research.

- Inference Rule: If a claim describes the agent's focus, actions, or conclusions that naturally

follow from this chunk, treat it as Support unless the chunk clearly contradicts it.

Before showing your final answer, think step-by-step and show your specific reasoning.

## CLASSIFICATION CRITERIA

### 1. Support

The document validates the claim through explicit statement, reasonable inference, or logical

abstraction.

- Explicit/Inferred: The claim is stated in the text or is a direct logical consequence of the

facts presented.

- Resource Availability (IMPORTANT): If the claim describes the acquisition, access, or

availability of information (\eg, 'The agent accessed the product page'), consider the presence

of the document content itself as sufficient evidence that such access was established.

### 2. Unsupport

The document fails to validate the claim due to contradiction or insufficiency.

- Contradiction: The document contains information that directly refutes the claim.
- Insufficient Information: The document mentions related topics but lacks the specific data,

numbers, or details required to verify the claim.

## EXAMPLES

Case 1: Support (High-level Abstraction)

- Document: 'Product specifications and pricing information for the new smartphone model...'
- Claim: 'The agent has successfully accessed the product page.'
- Judgment: Support (The claim about accessing the page is validated by the actual presence of

content from that page.)

Case 2: Support (Reasonably Inferred)

- Document: 'Phase III trials reported an efficacy rate above 90% for the vaccine.'
- Claim: 'The vaccine was highly effective in trials.'
- Judgment: Support ('Highly effective' is a reasonable inference from 'efficacy above 90%'.)

Case 3: Unsupport (Contradiction)

- Document: 'The experiment was conducted with 100 participants aged 18-25.'
- Claim: 'The study included elderly participants over 65.'
- Judgment: Unsupport (The document explicitly defines a younger age range, contradicting the

claim.)

Case 4: Unsupport (Insufficient Information)

- Document: 'The company announced a new product launch.'
- Claim: 'The product launch increased quarterly revenue by 15%.'
- Judgment: Unsupport (The document mentions the launch event but provides no financial data to

verify the specific revenue figure.)

33


---

## OUTPUT FORMAT

After your reasoning, output ONLY the JSON object in this exact format:

```json

'judgment': 'Support|Unsupport',

'evidence': 'One-sentence explanation for your judgment',

'confidence': 0.0-1.0

### F.3. Prompt for Action Verification

You are an expert action verification system specialized in assessing the coherence and

necessity of a proposed action within a research trajectory.

## TASK

Evaluate whether the Action to Evaluate supports the User Query, considering the current context

of Previous Claims (facts) and Previous Actions (plans).

## INPUT CONTEXT

- User Query: {query}
- Previous Claims: {claims_context}
- Previous Actions: {actions_context} (In-progress steps)
- Action to Evaluate: {action}
1. Goal Coherence: Does the action align with the user's objectives?
2. Logical Continuity: Is the action a reasonable next step?
3. Assumption of Success (CRITICAL): Treat in-progress Previous Actions as if they will succeed

and return ideal results. An action is NOT premature if it relies on prerequisites that are

currently being fetched by previous steps.

## CLASSIFICATION CRITERIA

### 1. Support

The action makes reasonable progress toward the goal.

- Valid Extensions: The action uncovers new info, expands search space, or advances the task.
- Lightweight Operations: Actions like 'Extract', 'Format', 'Summarize', or 'Compile' based on

existing data are always Support.

- Sequential Planning: If Action B depends on Action A (which is in progress), Action B is

Support, not a deviation.

### 2. Unsupport

The action is either redundant or irrelevant.

Type A: Redundancy

- Definition: The action repeats a step that has *already produced concrete results*.
- Strict Rule: Mark as Redundancy ONLY if a Previous Claim documents the exact same search/tool

execution with actual results.

| Type B: Deviation - Actions dependent on prerequisites currently being fetched. - Repeating a failed search (if the first attempt yielded nothing). |
|---|
| - Repeating a failed search (if the first attempt yielded nothing). - Different tools (\eg, Wikipedia vs. Google) or query phrasings. - Exceptions (Not Redundant): execution with actual results. |


---

- Definition: The action pursues a completely irrelevant tangent.
- Constraint: Do not mark as deviation if the action is an intermediate step toward the main

goal.

## SOURCE ASSIGNMENT RULES

- If Support: Set source to -1 (derived from query) OR claim index [i] (if building upon a

specific fact).

- If Redundancy: Set source to claim index [i] (the specific claim that makes this action

unnecessary).

- If Deviation: ALWAYS set source to -1 (deviates from the query/goal).

## EXAMPLES

Case 1: Support (Alternative Search)

- Query: 'Find Python 3.12 features'
- Previous Claim [2]: 'Official docs lack 3.12 details'
- Action: 'Search Git Hub for Python 3.12 features'
- Output: {{'label': 'Support', 'source': 2, 'type': null, 'confidence': 0.9, 'explanation':

'Explores alternative sources after claim [2] confirmed a gap.'}}

Case 2: Unsupport (Redundancy)

- Query: 'Top Italian restaurants in Boston'
- Previous Claim : 'Found top rated: Mamma Maria (4.8), Giulia (4.7)'
- Action: 'Search for best Italian restaurants in Boston'
- Output: {{'label': 'Unsupport', 'source': 1, 'type': 'redundancy', 'confidence': 0.95,

'explanation': 'Claim already provides the exact results this action seeks.'}}

Case 3: Support (Lightweight Extraction)

- Previous Claim [2]: 'Q3 revenue up 15%...'
- Action: 'Create a summary table of regional sales'
- Output: {{'label': 'Support', 'source': 2, 'type': null, 'confidence': 0.85, 'explanation':

'Formatting data for analysis is a valid step.'}}

Case 4: Unsupport (Deviation)

- Query: 'Analyze 2008 financial crisis'
- Action: 'Research medieval banking regulations'
- Output: {{'label': 'Unsupport', 'source': -1, 'type': 'deviation', 'confidence': 0.9,

'explanation': 'Irrelevant historical tangent unrelated to the 2008 crisis.'}}

Case 5: Support (Sequential Planning - NOT Premature)

- Query: 'Calculate temp trends'
- Previous Actions: [0] Fetch NOAA data, Download records
- Previous Claims: [0] 'Data not yet retrieved'
- Action: 'Run regression model on climate data'
- Output: {{'label': 'Support', 'source': -1, 'type': null, 'confidence': 0.88, 'explanation':

'Valid next step assuming previous actions [0] and succeed in fetching data.'}}

## OUTPUT FORMAT

Return JSON ONLY:

'label': 'Support' | 'Unsupport',

'type': 'deviation' | 'redundancy' | null,

35


---

'confidence': 0.0-1.0,

'explanation': 'One sentence justification.'

### F.4. Prompt for Root-cause Error Detection

We detail the two-stage workflow for identifying the root-cause error in a research trajectory:

We detail the two-stage workflow for identifying the root-cause error in a research trajectory:

(1) Trajectory Annotation. Before detection, we construct an annotated timeline of the agent's research trajectory to

visualize potential failure points:

•Atomic Hallucinations. All identified Claim Hallucinations, Action Hallucinations, and Neglected Restrictions are

marked at their corresponding steps.

•Severe Noise Domination. To identify steps where the agent summarizes less relevant information such that valuable

content is neglected, we leverage the local-level noise scores (H) defined in our main methodology. Since these scoresIS

are continuous, we apply elbow clustering to isolate a set of candidate steps with anomalously high noise levels. To ensure

these steps represent genuine information loss rather than benign filtering, we validate them via an LLM. Specifically, we

prompt the LLM to estimate the impact of neglecting the highest-value ignored cluster (the Out-Memory cluster with the

highest rankR) on the research outcome. Only steps with an estimated impact score> 0.5are annotated as sufferingc

from severe noise domination.

(2) Root-Cause Error Detection. We provide the LLM with the fully annotated trajectory and the final answer. Adapting

the detection prompt from (Zhu et al., 2025b), we instruct the model to analyze the logical chain of events and pinpoint the

earliest error that served as the critical cause for the incorrect final answer.

#### F.4.1. PROMPT FOR INTERPRETING NEGLECT

You are an insight analyst reviewing retrieval chunks that were skipped in the final report.

Each chunk may support hidden reasoning steps instead of answering the query directly. Infer

subtle or implicit relationships between the chunk and the user query.

Instructions:

1. Provide a one-sentence summary that highlights any signal relevant to the query or its

supporting sub-tasks (do not copy the chunk verbatim).

2. Provide a one-sentence explanation of the potential impact of omitting this chunk, even if

the impact is indirect or speculative (it's acceptable to say the impact is negligible).

3. Output an impact score between 0 and 1 indicating how strongly the omission could hurt the
4. Avoid absolute or exclusive claims unless the chunk explicitly states them; qualify

statements with phrases like 'suggests', 'indicates', or 'one plausible candidate' when the

evidence is indirect.

5. Mention remaining uncertainties or missing links when appropriate so the reader understands

the limits of the evidence.

6. Be concise and analytical; reason about latent connections or missed opportunities.

Query/Task: {query}

Chunk Content: {chunk_text}

Respond EXACTLY in the following format:

Summary: <one sentence>

Impact: <one sentence>

Impact Score: <float between 0 and 1>You are an insight analyst reviewing retrieval chunks that

were skipped in the final report. Each chunk may support hidden reasoning steps instead of

36


---

answering the query directly. Infer subtle or implicit relationships between the chunk and the

user query.

Instructions:

1. Provide a one-sentence summary that highlights any signal relevant to the query or its

supporting sub-tasks (do not copy the chunk verbatim).

2. Provide a one-sentence explanation of the potential impact of omitting this chunk, even if

the impact is indirect or speculative (it's acceptable to say the impact is negligible).

3. Output an impact score between 0 and 1 indicating how strongly the omission could hurt the
4. Avoid absolute or exclusive claims unless the chunk explicitly states them; qualify

statements with phrases like 'suggests', 'indicates', or 'one plausible candidate' when the

evidence is indirect.

5. Mention remaining uncertainties or missing links when appropriate so the reader understands

the limits of the evidence.

6. Be concise and analytical; reason about latent connections or missed opportunities.

Query/Task: {query}

Chunk Content: {chunk_text}

Respond EXACTLY in the following format:

Summary: <one sentence>

Impact: <one sentence>

Impact Score: <float between 0 and 1>

#### F.4.2. PROMPT FOR ROOT-CAUSE ERROR DETECTION

query: {query}

Scenario & Error Context:

Scenario Background:

- Chain-of-Research trajectory: Each iteration contains planning actions (`action_list_N`) and

observations/claims (`claim_list_N`), culminating in a final report.

- The full trajectory shows the complete research chain; the hallucination timeline shows errors

(hallucinated actions/claims verified as Not Support), noise_domination (missed content with high

possible impact), and potentially missed queries (unaddressed user intent) all of these are

hallucinations.

- Only timeline entries are hallucinations; steps without timeline entries stayed on track.

FULL TRAJECTORY - Complete Chain of Research:

CRITICAL: Carefully examine observations for strategy shift signals like:

''shift strategy'', ''change approach'', ''start over'', ''complete shift'', ''need a new

strategy'', ''pivot'', ''abandon previous approach''

If you see such signals after an error, that error was CORRECTED and is NOT the root cause.

{full_research_trajectory}

FINAL REPORT - Research Results and Conclusions:

37


---

CRITICAL ANALYSIS INSTRUCTIONS:

The report below shows what the agent ACTUALLY concluded. Use it to REVERSE-ENGINEER the root

cause:

1. Identify the FINAL ANSWER/CONCLUSION in the report
2. The final answer is INCORRECT and trace BACKWARDS from the final conclusion to find:
- Which step's error directly led to this wrong conclusion?
- Which early errors were ABANDONED (not mentioned in final report = were corrected/abandoned)
3. Root cause identification logic:
- If an error is NOT reflected in the final report -> it was abandoned -> NOT root cause
- If an error IS reflected in the final report -> it affected the conclusion -> POTENTIAL

root cause

- The EARLIEST error that directly led to the final wrong conclusion is the root cause

{report}

================================================================================

HALLUCINATION TIMELINE - Errors Detected:

================================================================================

Compare the timeline below with the full trajectory and final report above:

- If an error led to a strategy shift (mentioned in trajectory), it is NOT the root cause
- If an error is not reflected in the final report, it was likely abandoned and is NOT the root

cause

- Only errors that directly led to the final incorrect conclusion are root causes
- Note: The timeline includes noise_domination (missed content) and query_missed entries when

applicable

{hallucination_timeline}

Analysis Guidelines:

CRITICAL: Do NOT be dominated by early hallucinations. An early hallucination that was later

recognized

and corrected by the agent is NOT the root cause.

1. Analysis process:
- FIRST: Understand the ENTIRE trajectory to see how the agent's strategy evolved and when

errors were recognized/corrected

- THEN: Compare the hallucination timeline with the full trajectory to identify which errors

were critical

- Root cause = the earliest error that irreversibly doomed the final outcome (NOT corrected,

NOT led to successful pivot)

2. Root cause criteria:
- Must be an error that, if corrected, would have fundamentally changed the trajectory toward

success

- Must have STRONG LINKAGE between the error and the final wrong answer
- Early exploration errors (steps 1-3) are often normal learning steps - only flag if never

corrected

38


---

- If agent recognized an early error and changed strategy, root cause is likely later in the

chain

- Trace backwards from final failure to find the earliest uncorrected error
- If NO hallucinations have strong linkage to the final failure, output critical_step = -1
3. Never cite a step/module unless the timeline explicitly lists a hallucinated item there.

Modules:

- planning -> hallucinated actions in action_list_<step>
- observation -> hallucinated claims in claim_list_<step> or final report
- noise_domination -> missed content with high impact
- query_missed -> missed user intent/queries

root-cause error TYPES:

If there is a strong linkage between an error and the final failure, identify ONE of the

following types as the root cause:

1. planning - Hallucinated planning actions that led to wrong search direction
2. observation - Hallucinated claims/observations that led to wrong conclusions
3. noise_domination - Critical content was retrieved but missed, directly causing failure
4. query_missed - Critical user intent/queries were not addressed, directly causing failure

If NO hallucinations have strong linkage to the final failure, set critical_step = -1 and

critical_module = 'none'.

REQUIRED OUTPUT FORMAT (JSON):

''critical_step'': <step_number or -1 if no strong linkage>,

''critical_module'': ''<module_name: planning|observation|noise_domination|query_missed>'',

''root_cause'': ''Concise description of the fundamental problem'',

''cascading_effects'': [{ ''step'': <step_number>, ''impact'': ''description'' }]

Note: If no hallucinations have strong linkage to the final failure, set critical_step = -1.

39
