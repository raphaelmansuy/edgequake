## Page 1

## Why Your Deep Research Agent Fails?On Hallucination Evaluation in Full Research Trajectory

Yuhao Zhan Tianyu FanLinxuan Huang

### Abstract

Diagnosing the failure mechanisms of Deep Re-search Agents (DRAs) remains a critical challenge. Existing benchmarks predominantly rely on end-to-end evaluation, obscuring critical intermediate hallucinations, such as flawed planning, that accumulate throughout the research trajectory. To bridge this gap, we propose a shift from outcome-based to process-aware evaluation by auditing the full research trajectory. We introduce the PIES Taxonomy to categorize hallucinations along functional components Planning (

vs.Summarization) and error properties Explicit ( vs.Implicit). We instantiate this taxonomy into 

including adversarial scenarios, we curate Deep- 

//github.com/yuhao-zhan/DeepHalluBench.

1. Introduction

The rapid advancement of Large Language Models (LLMs) has spurred the development of Deep Research Agents (DRAs) (Huang et al., 2025; Zhang et al., 2025b). A DRA is an LLM-based system designed to iteratively plan, search, and reason to retrieve and synthesize information, ultimately generating a final report for a user query. Existing DRAs, such as OpenAI's (OpenAI, 2025) and Gemini's ( Gemini,

Zirui GuoChao Huang

> Figure 1. Comparison between existing benchmarks for DRAs and our benchmark.
>

2025), accelerate complex research, reducing completion times from hours to minutes. Despite their potential, the complexity and sophistication of DRAs make holistic and faithful evaluation challenging. Existing benchmarks predominantly fall into two categories based on query type: close-ended, which verifies short-form answers against ground-truth data (Mialon et al., 2023; Wei 

reference reports or rubrics (Du et al., 2025; Li et al., 2025). Critically, both approaches share a fundamental deficiency: they rely on end-to-end evaluation. These benchmarks focus solely on the final output, neglecting the complex 

intermediate hallucinations, such as misleading plans, occur exclusively within intermediate steps and remain invisible to end-to-end checks. (2) Opaque Performance Diagnosis: 

summarization) becomes infeasible, impeding fine-grained 

assessment to process-aware evaluation, capable of auditing

1Zhejiang University. Work done during internship at HKU.

2the University of Hong Kong. Correspondence to: Chao Huang *<chuang@cs.hku.hk>.* Preprint. February 2, 2026.

hallucinations throughout the entire research trajectory. However, realizing such process-aware evaluation is im-

## 1However, realizing such process-aware evaluation is im-


---

## Page 2

peded by three obstacles: (1) Taxonomic Gap: Hallucination taxonomies tailored to DRAs remain under-explored; (2) Data Acquisition Barriers: Proprietary DRAs either impose prohibitive API costs or operate via Web UIs lacking structured logs (e.g., JSON), complicating automated tracking; (3) Evaluation Complexity: Constructing a holistic

and faithful benchmark is non-trivial, given the multifaceted, multi-stage nature of the research trajectory. To overcome these barriers, we first address the nomic gap. We model the research trajectory as iterative 

two dimensions: the functional components (Planning vs. Summarization) and error properties (Explicit vs.Implicit). Specifically, Explicit Hallucinations refer to the incorrect information, while Implicit Hallucinations the critical absence of required content, violating the user's intent. The intersection of these dimensions yields four categories: (1) Explicit Planning: Generating deviated or redundant plans; (2) Implicit Planning: Neglecting specific 

systematic classification forms the foundation to rigorously evaluate hallucinations and unveil precise limitations. 

parsers that reconstruct unstructured Web UI traces into standardized "plan-search-summarize" loop. To address evaluation complexity, we decompose plans into atomic actions and summaries into atomic claims. This granular approach enables us to quantify hallucinations on each PIES category via verifying atomic actions and claims (Explicit) 

Leveraging this framework, we introduce DeepHalluBench the first benchmark designed to evaluate hallucinations throughout the DRA research trajectory. We construct the 

and synthesizing adversarial "no-answer" queries via atomic perturbations. To isolate the most challenging queries, we 

the 100 most "hallucination-prone" queries based on the derived hallucination scores, with a balanced distribution between openand close-ended tasks. Using DeepHalluBench, we benchmark five proprietary and one open-source DRAs to investigate two core questions.

veal that no agent achieves robust reliability. We identify 

unfaithful grounding and information neglect. Moving from symptoms to causes (RQ2: Failure Mechanisms), we trace 

early-stage cascading fabrications, while the open-source framework succumbs to late-stage context collapse; and (2) taxo- Cognitive Biases, specifically a temporal "Anchor Effect" (fixating on initial retrieval) and a semantic "Homogeneity Bias" (neglecting diverse insights). These findings suggest 

long-context attention debiasing. presence of denote Contributions. (1) We pioneer a paradigm shift from 

etiologies, providing foundational insights into systemic deficits to guide future architectural optimization.

                        2. Related Work

Hallucinations. Hallucination in LLMs is generally defined as content that is nonsensical or unfaithful to source materials (Farquhar et al., 2024). These errors are typically categorized into three types: input-conflicting, contextconflicting, and fact-conflicting (Zhang et al., 2025c). To detect hallucinations, researchers widely employ fact-checking (Vlachos & Riedel, 2014) or claim verification (Zerong et al.,

2025). These approaches utilize Natural Language Inference (NLI) models (Chen et al., 2025b; Schopf et al., 2025),, LLMs (Wei et al., 2024; Rahman et al., 2025), or agents (Cheng et al., 2024) to predict verdict labels (e.g., Entailment vs. Contradiction). Recently, hallucinations within

LLM-based agents have gained attention, leading to new tax-, 2025) onomies and detection methods (Lin et al., 2025; Zhu et al.,

2025b). However, hallucinations specific to DRAs, though briefly assessed in benchmarks like Mind2Web2 (Gou et al., 2025), lack systematic evaluation and analysis, leaving the fundamental limitations of DRAs largely unexplored. Deep Research Evaluation. Current benchmarks for DRAs can be categorized by query type: close-ended and open-ended. Close-ended evaluations involve queries with short, ground-truth answers, facilitating automated verifi-

Regarding RQ1 (Hallucination Landscape), our results re-

1The final report is treated as the terminal summary.

cation. Benchmarks such as GAIA (Mialon et al., 2023), BrowseComp (Wei et al., 2025), xbench (Chen et al., 2025a), and BrowseComp-Plus (Chen et al., 2025c) typically rely on 2


---

## Page 3

> Table 1. Comparison between DeepHalluBench and existing Deep Research benchmarks. hallucinations. DeepHalluBench uniquely integrates close-ended, open-ended, and "no-answer" queries, providing the first comprehensive
>

hallucination evaluation throughout the full research trajectory.

Benchmark Close-ended Open-ended Research Trajectory Hallucination No-answer Query GAIA ✓ BrowseComp ✓ BrowseComp-Plus ✓ Rigorous Bench ✗ Mind2Web2 ✗ DeepResearch-ReportEval ✗ ✓ metrics like accuracy. Open-ended evaluations ReportBench ✗

 ✗ focus oncan be supported by other uncited sources.

report-style, long-form outputs. For instance, Mind2Web2DeepResearch Arena ✗ (Gou et al., 2025) assesses agentic search using an Agent-as- DeepHalluBench (Ours) ✓ ✓ ✓ a-Judge framework with a tree-structured rubric. Similarly, report-oriented benchmarks like DeepResearch Bench ( 

employs an LLM-as-a-Judge to assess quality, redundancy, and factuality. Other frameworks, including ReportBench (Li et al., 2025), Rigorous Bench (Yao et al., 2025), andwhen the agent generates explicit execution steps that DeepResearch Arena (Wan et al., 2025), utilize automated or human-designed rubrics. Despite this progress, most benchmarks apply end-to-end evaluation that overlooks the full research trajectory and assesses hallucinations partially 

3. Hallucination Taxonomy

While standard LLM hallucinations are often categorized as input-, context-, or fact-conflicting (Zhang et al. these categories do not fully capture the search-based, 

along two dimensions: the functional component ( Planning vs. Summarization)and the error property (Explicit vs. Implicit). As illustrated in Figure 2, this intersection yields four distinct categories of DRA hallucinations: •Explicit Summarization→Claim Hallucination: In the summarization stage, explicit hallucinations involve the presence of incorrect information. This includes: (1) Fabrication: Generating content (i.e., claims) unsupported by any document and context; (2) Misattribution: Citingthat reconstructs full research trajectories from Web UI documents that do not support the claim, even if the claim

△denotes evaluation on incomplete

•Implicit Summarization→Noise Domination: This △ 

fails to utilize them, allowing "noise" (Dui.e., irrelevant parts of the retrieval) to dominate the summary. This results in , 2025an answer that misses the core user intent despite having) access to the correct data (i.e., input-conflicting). •Explicit Planning→Action Hallucination: This occurs are flawed. It manifests primarily in three forms: (1) Action Deviation: The plan deviates from the user's intent (Input-conflicting); (2) Action Redundancy: The agent proposes unnecessary steps that repeat previous efforts (Context-conflicting); (3) Propagation: A unique case where the plan is logically correct but based on previous 1 hallucinated claims, leading to a cascade of errors. 

ignores specific user restrictions (e.g., ignoring "full-time" in a job-seeking task, Figure 2), representing a subtle form, 2025c), of Input-conflicting hallucination.

                        4. Evaluation and Benchmark

Guided by the PIES taxonomy, this section establishes our framework for trajectory data acquisition and fine-grained hallucination evaluation.

### 4.1. Data Acquisition and Decomposition

To evaluate proprietary DRAs lacking cost-friendly APIs and structured reasoning output, we developed a pipeline traces (Figure 3). We employ custom HTML-parsers and

2We exclude the Search stage as it relies on external engines, distinguishing retrieval outputs from LLM-induced hallucinations.

3

LLMs to disentangle interleaved reasoning and URLs into structured plan-search-summarize loops. To quantify hallu-


---

## Page 4

> Figure 2. The PIES Taxonomy. The framework intersects functional components (vertical axis) with error properties (horizontal axis). The four quadrants represent specific hallucination categories derived from these combinations: Explicit Summarization, Implicit
>

Summarization, Explicit Planning, and Implicit Planning.

> Figure 3. The Data Acquisition and Decomposition Pipeline. iterative plan-search-summarize loops. These loops are further decomposed by LLMs
>

cinations precisely, we adopt an atomicity-based approach 

plans into atomic actions, and summaries into atomic claimsa cost-efficient NLI-then-LLM cascade. Supported claims (preserving citation mappings). As shown in Figure 

strictly atomic and verifiable units. All prompt templates in this work are in Appendix F.

### 4.2. Evaluation Framework

Leveraging atomic claims, actions and sub-queries as fundamental units, we design a rigorous evaluation framework tailored to each category of the PIES taxonomy. Claim Verification (Explicit Summarization). To distinguish between factual observations and internal reflections,

 We first employ custom parsers to structure raw Web UI traces into

 atomically to enable fine-grained evaluation.

port claims; full retrieval history for others. We adopt a retrieve-then-verify strategy: relevant evidence chunks are retrieved via a coarse-to-fine pipeline, then verified using, 3and their evidence chunks are stored in, this Claim Memory and Chunk Memory, respectively. •Round 2: Adaptive Re-Verification. Unsupported claims trigger branching checks to categorize errors: (1) Misattribution Check: For claims with explicit citations, we expand the evidence scope to all retrieved documents. Support here indicates misattributionC; othmisattribution

erwise, it is confirmed as fabricationC. (2) Re-fabrication flection Check: For intermediate claims, we verify them against Claim Memory to validate internal reflections. Lack of support confirms fabrication C.fabrication We quantify Explicit Summarization Hallucination (H)ES as the ratio of fabricated and misattributed claims to the

total set:

we implement a two-round verification pipeline (Figure •Round 1: Initial Verification. We verify claims against their specific evidence scope: cited documents for re-

4

HES=

*. (1)*

|Ctotal|


---

## Page 5

> Figure 4. The Evaluation Framework for Summarization Hallucinations. neglect (bottom). The addition symbols (⊕) define the data scope: selecting evidence scope for verification (top) or specifying document
>

sets for global/local level (bottom). The cross symbol (⊗) intersects ranked clusters with (In-Memory) or ignored (Out-Memory), enabling the penalty quantification shown on the right. See Appendix B.1 for implementation details. Noise Detection (Implicit Summarization). LLMs often struggle to prioritize valuable information due to positional bias (Liu et al., 2024; Trienes et al., 2025; Elaraby & Litman

2025). To quantify DRA's capability to distinguish essential signals from massive retrieval streams, as shown in figure 4, we propose a cluster-based heuristic at two granularities: global-level (assessing total information utilization) and local-level (measuring utilization within each search round). •Semantic Clustering & Value Estimation. We first map retrieved chunks into semantic clusters to reduce redundancy and rank them by relevance to the atomic sub-queries (Rank=1 denotes highest importance). •Penalty Quantification. We distinguish between utilized clustersC and ignored onesC . We penalize ne-inout 3 

The penalty Pfor an ignored cluster c is:c

*S× Ncc-inv , (2)*

where Sis size,Ris rank, andN (inversion count)ccc-inv is the number of lower-ranked clusters that were utilized. P The total penalty is P =P.c •Hallucination Quantification. We normalize Pagainst a theoretical worst-caseP(where the highest-valueworst clusters are systematically ignored) to derive the Summarization Hallucination () or Noise Score:HIS

P

 The pipeline assesses Explicit errors (top) and Implicit Chunk Memory to classify them as utilized

See Appendix B.3 for clustering and computation details. Action Verification (Explicit Planning). To assess the 

evant to user query) and Action RedundancyA ( ,redundancy repetitive steps), we explicitly identify Action Propagation (Apropagation): actions that are logically sound but grounded 

The Explicit Planning Hallucination () is defined as:HEP

HEP=

 |Atotal|

Restriction Checking (Implicit Planning). To detect the 

the trajectory (Figure 5, bottom). We adopt a subtractive process: for every atomic action, we rank its relevance to all sub-queries and employ the elbow method to isolate the specific subset of restrictions it effectively "execute" (Q executed Implicit). The Implicit Planning Hallucination (H) is IP 

HIS=*. (3)* Pworst

3A cluster is utilized if it contains any chunk in Chunk Memory.

|Qtotal\ Qexecuted | HIP=

*. (5)*

|Qtotal|

5


---

## Page 6

> Figure 5. The Evaluation Framework for Planning Hallucinations.
>

(bottom). The subtraction symbol (⊖) defines the neglect identification logic: removing the set of effectively executed sub-queries from the full set of sub-queries to isolate neglected restrictions. Reliability. Benchmarking claim verification module 

subset (Thorne et al., 2018>) and 85% on SciFact-Open (Wadden et al., 2022) (see Appendix B.2 for details). For 

prompt optimization and validation process.

### 4.3. Benchmark Construction

To stress-test DRAs, we construct DeepHalluBench benchmark of 100 queries through a three-stage pipeline. •Aggregation & Difficulty Assessment. We aggregated a diverse candidate pool of queries from Mind2Web2 

to generate full research trajectories for all candidates. 

"hallucination-prone" queries (25 from each dataset).

 The pipeline assesses Explicit errors (top) and Implicit neglect

This section investigates two core research questions (RQs): 

mechanisms drive the DRA's failures? We first benchmark representative DRAs on DeepHalluBench, followed by a diagnostic analysis of their failure etiologies.

### 5.1. Experimental Setup

We evaluate six state-of-the-art DRAs, comprising five Pro-, a prietary DRAs (Gemini, 2025; OpenAI, 2025; Perplexity, 2025; Qwen, 2025; Grok, 2025), along with one Open- Source DRA (Salesforce Air Deep Research (Prabhakar 

the average relevance of the top-50% retrieved documents against use query; see Appendix D.1 for details.

### 5.2. Results

Overview. Table 2 overviews the performance of the 

(H ≈ 0.175) and SalesforceH ≈ ( 0.185). Perplexity trails with a higher hallucination degree (H ≈ 0.21), while Grok lags significantly behind, exhibiting severe hallucinations in the summarization stage. 4

1. Overview. Table 2 overviews the performance of the

The final benchmark comprises 100 queries, evenly split between open-ended and close-ended tasks. See Appendix for dataset details and case studies for atomic perturbations.

WhenHis averaged exclusively on summarization metrics HESand H, Grok (H ≈ 0.38) is the poorest performer.IS

 C

6

4. Overview. Table 2 overviews the performance of the

•Adversarial Synthesis. To evaluate DRA's ability to 

perturbations to solvable close-ended queries-modifying specific restrictions (e.g., temporal details) to render the logical intersection of all restrictions empty.


---

## Page 7

> Figure 6. Evaluation Results over DeepHalluBenchwith seven hallucination metrics alongside Retrieval Quality for six DRAs.
>

> Table 2. Evaluation results on DeepHalluBench. DRAs above the midline are proprietary. Bold denotes lowest hallucination score.
>

| Note | that | Grok | lacks | planning | metrics | (HandH) | because | EP | IP |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Gemini 0.2171 0.2786 0.0170 0.1866 | 0.1749 |  |  |  |  |  |  |  |  |
| OpenAI 0.2207 0.3121 0.0456 0.0401 | 0.1546 |  |  |  |  |  |  |  |  |
| Perplexity 0.2220 0.3940 0.0313 0.1865 | 0.2084 |  |  |  |  |  |  |  |  |
| Qwen 0.2311 0.2374 0.0197 | 0.1070 |  |  |  |  |  |  |  |  |

### 0.1488OpenAI 28% 28% 28% 22%

Salesforce 0.3231 0.1003 0.0291 0.2879 0.1851

Hallucination Landscape across Categories. Dissecting performance along the PIES taxonomy (Figure 6) reveals distinct failure patterns across the agents: 

act as "confident fabricators" (High Fabrication≈ 0.15, Low Misattribution), generating content without sufficient support. Conversely, Salesforce creates an "illusion of grounding" dominated by Misattribution (> 0.20).and over-conservatism, with three distinct profiles: (1) Over- •Implicit Summarization (Noise Domination). This met-Confidence (Gemini, Grok): These DRAs fail to reject adric highlights a bottleneck in information prioritization. Grok and Perplexity succumb to high noise (≈ 0.33) despite decent retrieval. Qwen proves most resilience (≈ 0.23), whereas Salesforce, with lower retrieval quality, achieves the lowest noise (≈ 0.10) likely by retrieving a narrower, safer set of information. •Explicit Planning (Action Hallucination). While general planning capabilities are robust (< 5%errors), nuances emerge. OpenAI shows slightly higher Deviation

and Redundancy≈ ( 4%) likely due to its exhaustive strate- collapsing into systemic bias. For extended results on domain sensitivity and performance disparities between openand close-ended tasks, see Appendix D.2 and D.3. 7 versarial queries (near0%accuracy), force-hallucinating answers due to an inability to identify empty intersection of re-striction sets. (2) Over-Conservatism (Salesforce, Qwen):

> Table 3. Performance on Close-Ended Queries (N = 50). Ans.:
>

Answerable queries; No-Ans.: Adversarial queries (correct re-sponse is rejection). Rejection: Proportion of queries where the DRA reported no answer. Bold denotes highest metric.

Accuracy

Rejection

Overall Ans. No-Ans.

Perplexity 24% 16% 32% 42% Qwen 36% 0% 72% 60% Grok 16% 24% 8% 10% Salesforce 40% 0% 80% 80% •Implicit Planning (Restriction Neglect). Adherence to restrictions reveals a tier-stratified gap. OpenAI leads with near-perfect adherence (neglect< 0.05), followed by Qwen. In contrast, Salesforce, Gemini, and Perplexity fail significantly more often (≈ 0.18-0.30), struggling to internalize boundary conditions. Close-Ended Tasks. Performance on close-ended queries (Table 3) reveals a critical trade-off between over-confidence

gies. Gemini, while efficient, exhibits specific susceptibility to Propagation≈ 1. (7%), where planning errors cascade from prior fabrications.

They achieve high adversarial accuracy (72-80%) but at the cost of prematurely abandoning answerable queries (0% 

(3) Balanced Struggle (OpenAI, Perplexity): Only these 


---

## Page 8

> Figure 7. Temporal Distribution of Hallucinations. We segment the research trajectory into three equal stages (Early, Middle, Late).
>

Src. Dist.: source errors that trigger propagation; Desc. Dist.: consequent errors propagating from source; and Hallu. Dist. distribution of explicit hallucinations derived after backtracking all propagation chains to their root sources.

### 5.3. Analysis

Based on this multifaceted hallucination landscape, we delve into RQ2 (Why): What underlying mechanisms drive 

map these dependencies into a Directed Acyclic Graph where nodes represent atomic claims or actions and directed 

•Early-Stage Cascading (Gemini, OpenAI). Proprietary DRAs exhibit systemic cascading, where> 57% of 

a chain of descendants that undermines the subsequent research foundation. 

down in the late stage (> 40%of errors). This highlights a limitation in maintaining coherence over long contexts. We further analyze root-cause errors (the earliest step precipitating final failure) for the 50 close-ended queries (heatmap in Appendix E.2). The results reveal two primary mechanisms: (1) Fabrication Dominance: For most DRAs, the dominant failure trigger is intermediate summarization fabrication, where DRAs derive conclusions unsupported 

(i.e., Salesforce) tends to conservatively refuse the query, whereas proprietary DRAs often proceed to fabricate a final answer. This behavior aligns with the "Over-Confidence vs. Over-Conservatism" dichotomy observed in Table 3.

> Figure 8. Temporal Analysis of Information Attention and Noise.
>

mensions, leading to severe Noise Domination. •Temporal: "Anchor Effect". DRAs disproportionately favor early retrieval (Figure 8). The Utilized Chunk Count drops precipitously in later stages. Paradoxically, Noise Scores peak in the late stage even as Retrieval Quality increases. This indicates a "saturation" bottleneck: agents may stop attending to new, superior information once their context is filled by initial findings. 

singleton insights in diverse contexts (see Figure 18 in Appendix E.3 for more details). In summary, while backbone LLM is the fundamental cause of hallucinations, our analysis isolates the agent-specific mechanisms, to understand hallucinations within agentic 

                        6. Conclusion

This work shifts the evaluation paradigm of Deep Research 

that remain invisible to traditional end-to-end benchmarks. Our comprehensive evaluation reveals that current DRAs fail to achieve robust reliability, exposing multidimensional deficits across the full trajectory: from strategic failures in balancing confidence versus conservatism, to execution 

Cognitive Biases. DRAs struggle to maintain unbiased information attention across the temporal and semantic di-

failures to systemic deficits, specifically hallucination propagation and cognitive biases like the Anchor Effect. These insights indicate that future progress requires moving be- 8


---

## Page 9

yond simple retrieval scaling toward architectural interventions that enforce early-stage error correction and unbiased long-context attention.

## Impact Statement

This paper presents work whose goal is to advance the field of Machine Learning. There are many potential societal consequences of our work, none which we feel must be specifically highlighted here.

## References

Cemri, M., Pan, M. Z., Yang, S., Agrawal, L. A., Chopra, B., Tiwari, R., Keutzer, K., Parameswaran, A., Klein, D., Ramchandran, K., Zaharia, M., Gonzalez, J. E., and Stoica, I. Why do multi-agent llm systems fail?, 2025. URL https://arxiv.org/abs/2503.13657. Chen, J., Xiao, S., Zhang, P., Luo, K., Lian, D., and 

self-knowledge distillation, 2024. URLhttps://arxiv. org/abs/2402.03216. Chen, K., Ren, Y., Liu, Y., Hu, X., Tian, H., Xie, T., Liu, F., Zhang, H., Liu, H., Gong, Y., Sun, C., Hou, H., Yang, H., Pan, J., Lou, J., Mao, J., Liu, J., Li, J., Liu, K., Liu, K., Wang, R., Li, R., Niu, T., Zhang, W., Yan, W., Wang, X., Zhang, Y., Hung, Y.-H., Jiang, Y., Liu, Z., Yin, Z., Ma, Z., and Mo, Z. xbench: Tracking agents productivity scaling with profession-aligned real-world evaluations,

2025a. URL https://arxiv.org/abs/2506.13651. 

1888-1896, Vienna, Austria, July 2025b. Association for Computational Linguistics. ISBN 979-8-89176-256-

5. doi: 10.18653/v1/2025.findings-acl.96. URLhttps:

//aclanthology.org/2025.findings-acl.96/. Chen, Z., Ma, X., Zhuang, S., Nie, P., Zou, K., Liu, 

Thakur, N., Zhang, C., Gao, L., Chen, W., and Lin, J. 

https://arxiv.org/abs/2508.06600.

Cheng, X., Li, J., Zhao, X., Zhang, H., Zhang, F., Zhang,

Chen, Y.-N. (eds.), Proceedings of the 2024 Conference 

doi: 10.18653/v1/2024.emnlp-main.809. URLhttps: //aclanthology.org/2024.emnlp-main.809/. DeepSeek-AI, Liu, A., Mei, A., Lin, B., Xue, B., Wang, B., Xu, B., Wu, B., Zhang, B., Lin, C., Dong, C., Lu, C., Zhao, C., Deng, C., Xu, C., Ruan, C., Dai, D., Guo, D., Yang, D., Chen, D., Li, E., Zhou, F., Lin, F., Dai, F., Hao, G., Chen, G., Li, G., Zhang, H., Xu, H., Li, H., Liang, H., Wei, H., Zhang, H., Luo, H., Ji, H., Ding, H., Tang, H., Cao, H., Gao, H., Qu, H., Zeng, H., Huang, J., Li, J., Xu, J., Hu, J., Chen, J., Xiang, J., Yuan, J., Cheng, J., Zhu, J., Ran, J., Jiang, J., Qiu, J., Li, J., Song, J., Dong, K., Gao, K., Guan, K., Huang, K., Zhou, K., Huang, K., Yu, K., Wang, L., Zhang, L., Wang, L., Zhao, L., Yin, L., Guo, L., Luo, L., Ma, L., Wang, L., Zhang, L., Di, M. S., Xu, M. Y., Zhang, M., Zhang, M., Tang, M., Zhou, M., Huang, P., Cong, P., Wang, P., Wang, Q., Zhu, Q., Li, Q., Chen, Q., Du, Q., Xu, R., Ge, R., Zhang, R., Pan, R., Wang, R., Yin, R., Xu, R., Shen, R., Zhang, R., Liu, S. H., Lu, S., Zhou, S., Chen, S., Cai, S., Chen, S., Hu, S., Liu, S., Hu, S., Ma, S., Wang, S., Yu, S., Zhou, S., Pan, S., Zhou, S., Ni, T., Yun, T., Pei, T., Ye, T., Yue, T., Zeng, W., Liu, W., Liang, W., Pang, W., Luo, W., Gao, W., Zhang, W., Gao, X., Wang, X., Bi, X., Liu, X., Wang, X., Chen, X., Zhang, X., Nie, X., Cheng, X., Liu, X., Xie, X., Liu, X., Yu, X., Li, X., Yang, X., Li, X., Chen, X., Su, X., Pan, X., Lin, X., Fu, X., Wang, Y. Q., Zhang, Y., Xu, Y., Ma, Y., Li, Y., Li, Y., Zhao, Y., Sun, Y., Wang, Y., Qian, Y., Yu, Y., Zhang, Y., Ding, Y., Shi, Y., Xiong, Y., He, Y., Zhou, Y., Zhong, Y., Piao, Y., Wang, Y., Chen, Y., Tan, Y., Wei, Y., Ma, Y., Liu, Y., Yang, Y., Guo, Y., Wu, Y., Wu, Y., Cheng, Y., Ou, Y., Xu, Y., Wang, Y., Gong, Y., Wu, Y., Zou, Y., Li, Y., Xiong, Y., Luo, Y., You, Y., , pp.Liu, Y., Zhou, Y., Wu, Z. F., Ren, Z. Z., Zhao, Z., Ren, Z., Sha, Z., Fu, Z., Xu, Z., Xie, Z., Zhang, Z., Hao, Z., Gou, Z., Ma, Z., Yan, Z., Shao, Z., Huang, Z., Wu, Z., Li, Z., Zhang, Z., Xu, Z., Wang, Z., Gu, Z., Zhu, Z., Li, Z., Zhang, Z., Xie, Z., Gao, Z., Pan, Z., Yao, Z., Feng, B., Li, H., Cai, J. L., Ni, J., Xu, L., Li, M., Tian, N., Chen, R. J., Jin, R. L., Li, S. S., Zhou, S., Sun, T., Li, X. Q., Jin, X., Shen, X., Chen, X., Song, X., Zhou, X., Zhu, Y. X., Huang, Y., Li, Y., Zheng, Y., Zhu, Y., Ma, Y., Huang, Z., Xu, Z., Zhang, Z., Ji, D., Liang, J., Guo, J., Chen, J., Xia, L., Wang, M., Li, M., Zhang, P., Chen, R., Sun, S., Wu, S., Ye, S., Wang, T., Xiao, W. L., An, W., Wang, X., Sun, X., Wang, X., Tang, Y., Zha, Y., Zhang, Z., Ju, Z., Zhang, Z., and Qu, Z. Deepseek-v3.2: Pushing the frontier of open large language models, 2025. URL

D., Gai, K., and Wen, J.-R. Small agent can also 

9. Du, M., Xu, B., Zhu, C., Wang, X., and Mao, Z. Deephttps://arxiv.org/abs/2512.02556.


---

## Page 10

research bench: A comprehensive benchmark for deep research agents, 2025. URLhttps://arxiv.org/abs/ 2506.11763. Elaraby, M. and Litman, D. Arc: Argument representation 

https://arxiv.org/abs/2505.23654.

Fan, T., Niu, X., Zheng, Y., Zhang, F., Huang, C., Chen, B., Lin, J., and Huang, C. Understanding deepresearch via reports, 2025. URLhttps://arxiv.org/abs/2510.

07861. Liu, N. F., Lin, K., Hewitt, J., Paranjape, A., Bevilac- Farquhar, S., Kossen, J., Kuhn, L., and Gal, Y. Detecting hallucinations in large language models using semantic entropy. Nature, 630(8017):625-630, 2024. Gemini. Gemini deep research - your personal research

assistant. https://gemini.google/overview/deep-research/

Gou, B., Huang, Z., Ning, Y., Gu, Y., Lin, M., Qi, W., Kopanev, A., Yu, B., Gutierrez, B. J., Shu, Y., Song, ´ C. H., Wu, J., Chen, S., Moussa, H. N., Zhang, T., Xie, J., Li, Y., Xue, T., Liao, Z., Zhang, K., Zheng, B., Cai, Z., Rozgic, V., Ziyadi, M., Sun, H., and Su, Y. Mind2web 2: Evaluating agentic search with agent-as-a-judge, 2025. URL https://arxiv.org/abs/2506.21506. Grok. Grok agents: Combining reasoning and tool use.

https://x.ai/news/grok-3/, 2025. Huang, Y., Chen, Y., Zhang, H., Li, K., Zhou, H., Fang, M., Yang, L., Li, X., Shang, L., Xu, S., et al. Deep research agents: A systematic examination and roadmap. arXiv

preprint arXiv:2506.18096, 2025. Jiang, Y., Bordia, S., Zhong, Z., Dognin, C., Singh, M., and 

doi: 10.18653/v1/2020.findings-emnlp.309. URLhttps: //aclanthology.org/2020.findings-emnlp.309/. Laurer, M., Van Atteveldt, W., Casas, A., and Welbers, K. Less annotating, more classifying: Addressing the data scarcity issue of supervised machine learning with deep transfer learning and bert-nli. Political Analysis 84-100, 2024.

Li, M., Zeng, Y., Cheng, Z., Ma, C., and Jia, K. Reportbench: Evaluating deep research agents via academic survey tasks, 2025. URLhttps://arxiv.org/abs/2508. 15804. Lin, X., Ning, Y., Zhang, J., Dong, Y., Liu, Y., Wu, Y., Qi, X., Sun, N., Shang, Y., Cao, P., et al. Llm-based agents suffer from hallucinations: A survey of taxonomy, methods, and directions. arXiv preprint arXiv:2509.18970, 2025. Liu, M. and Fang, J. Enhancing mathematical reasoning in large language models with self-consistency-based hallucination detection, 2025. URLhttps://arxiv.org/

qua, M., Petroni, F., and Liang, P. Lost in the middle: How language models use long contexts. Transactions of the Association for Computational Linguistics,

12:157-173, 2024. doi: 10.1162/tacla00638. URL

https://aclanthology.org/2024.tacl-1.9/.,

chical density based clustering. J. Open Source Softw., 2 (11):205, 2017. Mialon, G., Fourrier, C., Swift, C., Wolf, T., LeCun, Y., and Scialom, T. Gaia: a benchmark for general ai assistants,

                        2023. URL https://arxiv.org/abs/2311.12983.

Min, S., Krishna, K., Lyu, X., Lewis, M., tau Yih, W., Koh, P. W., Iyyer, M., Zettlemoyer, L., and Hajishirzi,

## H. Factscore: Fine-grained atomic evaluation of factual

precision in long form text generation, 2023. URLhttps: //arxiv.org/abs/2305.14251. OpenAI. Introducing deep research.

https://openai.com/index/introducing-deep-research/,

Perplexity. Introducing perplexity deep research. 

Prabhakar, A., Ram, R., Chen, Z., Savarese, S., Wang, F., 

analytics, 2025. URLhttps://arxiv.org/abs/2510. 17797. Qwen. Qwen deepresearch: When inspiration becomes its

own reason. https://qwen.ai/blog?id=qwen-deepresearch,

Rahman, S. S., Islam, M. A., Alam, M. M., Zeba, M., Rah-, 32(1): man, M. A., Chowa, S. S., Raiaan, M. A. K., and Azam,

Li, C., Liu, Z., Xiao, S., and Shao, Y. Making large language models a better foundation for dense retrieval, 2023.

10

## S. Hallucination to truth: A review of fact-checking and

factuality evaluation in large language models. arXiv

preprint arXiv:2508.03860, 2025.


---

## Page 11

Schopf, T., Vladika, J., Farber, M., and Matthes, F. Nat- ¨ 

Fifth Workshop on Scholarly Document Processing (SDP

2025), pp. 344-352, Vienna, Austria, July 2025. Association for Computational Linguistics. ISBN 979-8for automated failure attribution in multi-agent systems,

89176-265-7. doi: 10.18653/v1/2025.sdp-1.33. URL

https://aclanthology.org/2025.sdp-1.33/.

Thorne, J., Vlachos, A., Christodoulopoulos, C., and Mittal,

## A. FEVER: a large-scale dataset for fact extraction and

VERification. In NAACL-HLT, 2018. Trienes, J., Schlotterer, J., Li, J. J., and Seifert, C. Be- ¨ 

23428-23454, Vienna, Austria, July 2025. Association for Computational Linguistics. ISBN 979-8-89176-256-5.

doi: 10.18653/v1/2025.findings-acl.1204. URLhttps: //aclanthology.org/2025.findings-acl.1204/. Vlachos, A. and Riedel, S. Fact checking: Task definition and dataset construction. In Danescu-Niculescu-Mizil, C., Eisenstein, J., McKeown, K., and Smith, N. A. (eds.), Proceedings of the ACL 2014 Workshop on Language Technologies and Computational Social Science, pp. 18-

22, Baltimore, MD, USA, June 2014. Association for Computational Linguistics. doi: 10.3115/v1/W14-2508. URL https://aclanthology.org/W14-2508/. Wadden, D., Lo, K., Kuehl, B., Cohan, A., Beltagy, I., Wang, L. L., and Hajishirzi, H. SciFact-open: Towards open-domain scientific claim verification. In Goldberg, 

doi: 10.18653/v1/2022.findings-emnlp.347. URLhttps: //aclanthology.org/2022.findings-emnlp.347/. Wan, H., Yang, C., Yu, J., Tu, M., Lu, J., Yu, D., Cao, J., Gao, B., Xie, J., Wang, A., Zhang, W., Torr, P., and Zhou, D. Deepresearch arena: The first exam of llms' research abilities via seminar-grounded tasks, 2025. URL

https://arxiv.org/abs/2509.01396.

Wei, J., Yang, C., Song, X., Lu, Y., Hu, N., Huang, J., Tran, D., Peng, D., Liu, R., Huang, D., et al. Long-form factuality in large language models. Advances in Neural Information Processing Systems, 37:80756-80827, 2024.

Browsecomp: A simple yet challenging benchmark for browsing agents, 2025. URLhttps://arxiv.org/abs/ 2504.12516. West, A., Weng, Y., Zhu, M., Lin, Z., Ning, Z., and Zhang,

## Y. Abduct, act, predict: Scaffolding causal inference

                        2025. URL https://arxiv.org/abs/2509.10401.

Yan, Z., Wang, J., Chen, J., Li, X., Li, R., and Pan, J. Z. 

16708. Yao, Y., Wang, Y., Zhang, Y., Lu, Y., Gu, T., Li, L., Zhao, D., Wu, K., Wang, H., Nie, P., Teng, Y., and Wang, Y. A rigorous benchmark with multidimensional evaluation for deep research agents: From answers to reports, 2025. URL https://arxiv.org/abs/2510.02190. 

2025. Zhang, S., Yin, M., Zhang, J., Liu, J., Han, Z., Zhang, J., Li, B., Wang, C., Wang, H., Chen, Y., and Wu, Q. Which 

https://arxiv.org/abs/2505.00212.

Zhang, W., Li, X., Zhang, Y., Jia, P., Wang, Y., Guo, H., Liu, Y., and Zhao, X. Deep research: A survey of autonomous research agents. arXiv preprint arXiv:2508.12752, 2025b. Zhang, Y. Cutting the root of hallucination: Structural trimming for vulnerability mitigation in code llms. In Second Conference on Language Modeling, 2025. Zhang, Y., Li, Y., Cui, L., Cai, D., Liu, L., Fu, T., Huang, X., Zhao, E., Zhang, Y., Chen, Y., et al. Siren's song in the ai ocean: A survey on hallucination in large language models. Computational Linguistics, pp. 1-46, 2025c. Zhu, K., Liu, Z., Li, B., Tian, M., Yang, Y., Zhang, J., Han, P., Xie, Q., Cui, F., Zhang, W., Ma, X., Yu, X., Ramesh, G., Wu, J., Liu, Z., Lu, P., Zou, J., and You, J. Where llm agents fail and how they can learn from failures, 2025a. URL https://arxiv.org/abs/2509.25370. Zhu, K., Liu, Z., Li, B., Tian, M., Yang, Y., Zhang, J., Han, P., Xie, Q., Cui, F., Zhang, W., et al. Where llm agents fail and how they can learn from failures. arXiv preprint

arXiv:2509.25370, 2025b.

Wei, J., Sun, Z., Papay, S., McKinney, S., Han, J., Fulford, I., Chung, H. W., Passos, A. T., Fedus, W., and Glaese, A.

11


---

## Page 12

## A. Detailed Related Work

### A.1. Failure Analysis

Failure analysis is critical for diagnosing system reliability and guiding architectural improvements. Existing work broadly categorizes these efforts into multi-agent and single-agent domains. In attribution: (Cemri et al., 2025) establish taxonomies for coordination breakdowns, while ( et al., 2025) introduce benchmarks and causal frameworks to pinpoint responsible agents. Conversely, predominantly targets domain-specific verification, such as hierarchical checking in mathematical reasoning (

2025. or error localization in code generation (

general agents, these methods largely operate within short-horizon, re-runnable environments. They fall short of addressing the distinct complexities of Deep Research Agents, which suffer from long-context information overload and irreversible research workflow.

## B. Evaluation Framework

### B.1. Implementation Details in Claim Verification B.1.1. RETRIEVE-THEN-VERIFY STRATEGY

Exhaustive validation against every full-text document is cost-prohibitive and noise-sensitive. To address this, we implement a granular retrieval approach: •Chunking: We slice documents into 15-sentence chunks (see Appendix context integrity with token efficiency. •Retrieval Pipeline: We select the top-K (K=5) candidates using a coarse-to-fine pipeline: initial filtering via an embedding modelBAAI/bge-m3(Chen et al., 2024) with a similarity thresholdθ = 0.4, followed by selection via a reranker BAAI/bge-reranker-v2-m3 (Li et al., 2023). These parameters ensure robust recall of supporting evidences.

#### B.1.2. COST-EFFICIENT NLI-THEN-LLM CASCADE

To optimize computational costs without sacrificing accuracy, we employ a hybrid verification model in factual grounding (i.e., verifying whether a claim can be supported by any evidence chunk): •NLI Filter: An Natural Language Inference (NLI) model serves as a preliminary gatekeeper. If the NLI model predicts "Entailment" (Supported) with high confidence (

- LLM Judge: Only ambiguous or low-confidence claims are delegated to the more expensive LLM for a final verdict

A claim is "supported" if supported by at least one document in its evidence scope.

#### B.1.3. REFLECTION CHECK LOGIC

Considering some claims in intermediate steps are meta-cognitive reflections, in Round 2, we retrieve the top-K similar claims from the Claim Memory (accumulated from prior research steps) and task LLM to verify the unsupported claim against these retrieved claims, which can determine if an intermediate claim unsupported by any external document is a valid internal reflection based on the DRA's past reasoning and findings.

### B.2. Validation for Claim Verification

To validate the reliability of our automated claim verification pipeline (specifically the benchmark its performance against human-annotated ground truth from two established fact-checking datasets.

5We utilize MoritzLaurer/DeBERTa-v3-large-mnli-fever-anli-ling-wanli (DeepSeek-AI et al., 2025) as default NLI model and LLM respectively in this work.

6K=10 to include more abundant context.

 multi-agent settings, research focuses on failure

Zhang et al., 2025a) and (West single-agent analysis

Zhang, 2025). Although (Zhu et al., 2025a) extend root-cause detection to

 B.2.3 for discussion). This window size balances

*> 0.99), the verdict is finalized immediately.*

5.

6most

12

 retrieve-then-verify module), we

(Laurer et al., 2024) and DeepSeek-v3.2


---

## Page 13

#### B.2.1. EXPERIMENTAL SETUP

Datasets. We utilize FEVER (Thorne et al., 2018) and SciFact-Open (Wadden et al., 2022) to cover both general and scientific domains. •FEVER (General Domain): A large-scale dataset of claims derived from Wikipedia. Since the original dataset distinguishes between Refuted and NotEnoughInfo (NEI) we collapse these into a single "Unsupported" category to align with our binary verification logic. To construct a balanced validation set efficiently, we sampled a subset containing 659 claims associated with a corpus of ensuring an equal distribution of Supported and Unsupported instances. •SciFact-Open (Scientific Domain): A benchmark for verifying claims against scientific abstracts. This dataset is particularly pertinent to Deep Research agents that frequently process complex academic literature. We utilize the full test set (279 claims against a corpus of 500k abstracts). Similar to FEVER, we map the original labels to a binary Supported/Unsupported classification. This dataset presents a significantly harder challenge due to domain-specific terminology and complex sentence structures. Evaluation Metrics. We evaluate the pipeline using three metrics: (1) the predicted verdict matches the ground truth. (2) this metric counts a prediction as correct only if both the verdict is correct and the retrieved evidence matches the ground truth evidence set. (3) Evidence Recall (R): To isolate retrieval performance, we measure the proportion of supportedev claims for which at least one valid evidence chunk appears in the top-K candidates:

|C retrieved

|C supported

whereC supported is the set of claims with ground-truth evidence, and successfully retrieved. B.2.2. IMPLEMENTATION & RESULTS Pipeline Configuration. We apply the exact Round 1 verification logic described in the main text: documents are segmented into 15-sentence chunks, from which the top-5 relevant candidates are retrieved via our coarse-to-fine pipeline (Embedding→Reranker). The NLI-then-LLM verification module then judges the claim against these candidates. Note that we strictly test the Initial Verification stage (Round 1) here; the Adaptive Re-Verification (Round 2) is not applicable as these benchmarks do not involve citation misattributions or self-reflections. Results & Analysis. Table 4 summarizes the performance of our pipeline.

Dataset #Claim #Document Label Acc. Strict Acc. Evidence Recall FEVER (subset) 659 50k 0.940 0.883 0.956 SciFact-Open 279 500k 0.862 0.824 0.883

> Table 4. Benchmarking results of the automated claim verification pipeline against human ground truth.
>

, a distinction shown to be highly subjective (Jiang et al., 2020),

∼50,000 documents,

 Label Accuracy: The proportion of claims where

 Strict Accuracy: Also known as the FEVER score (Thorne et al., 2018),

∩ Csupported |

*, (7) C*retrieved is the set where valid evidence was

Reasoning Precision across Domains. In terms of verification accuracy, the pipeline excels in the general domain (FEVER: 94% Label Accuracy) and maintains robust in the complex scientific domain (SciFact: performance naturally dips on SciFact due to specialized terminology and complex logic, the pipeline maintains a strong

13

∼86% Label Accuracy). Although

Table 4 summarizes the performance. We analyze the results from two perspectives: retrieval robustness and reasoning precision. Retrieval Robustness (Stress Test). The pipeline achieves high evidence recall on both FEVER (95.6%) and SciFact-Open (88.3%). Crucially, this benchmark functions as a session retrieves∼100-200 documents from the Internet, this experiment requires isolating evidence from massive pools of

50,000 (FEVER) to 500,000 (SciFact) documents. The high recall under these extreme conditions demonstrates that our coarse-to-fine retrieval strategy is highly robust against the noise inherent in large-scale information environments, ensuring that the subsequent verification stage is supplied with high-relevance context.

 stress test for our retrieval module: while a typical Deep Research


---

## Page 14

alignment between Label Accuracy (0.862) and Strict Accuracy (0.824). This narrow gap indicates that the model rarely guesses a correct verdict by chance; rather, its judgments are consistently grounded in the correct supporting evidence, validating its reliability for the multi-domain rigor required in Deep Research.

#### B.2.3. CHUNK LENGTH

To determine the optimal granularity for evidence retrieval, balancing semantic integrity with token efficiency, we conducted a sensitivity analysis on the chunk size using the FEVER development subset. We defined a chunk as a contiguous block of *N sentences and evaluated the pipeline's performance by varying*

> Figure 9. Impact of Chunk Length on Verification Performance. stabilizing afterN = 13. We selectN = 15(highlighted) as the optimal threshold, where Label Accuracy peaks at 94.33% and F1-Score
>

reaches 90.39%, balancing robust performance with computational cost. As illustrated in Figure 9, performance is suboptimal at lower lengths ( fragment necessary context. While there is minor volatility in the mid-range ( high plateau as the length exceeds 13 sentences. The performance peaks atN = 15, achieving the highest Label Accuracy of 0.9433 and F1-Score of 0.9039. Extending the window beyond this point (N > 15) leads to a slight performance dip rather than further improvement. This trend suggests that excessively long chunks may introduce irrelevant noise that interferes with verification, in addition to linearly increasing the token consumption for the embedding and reranking models. Consequently, we adopt a 15-sentence window as the standard configuration, ensuring the retrieval system captures sufficient context without incurring unnecessary computational overhead.

#### B.2.4. NLI MODEL UTILITY

To optimize the cost-efficiency of our verification pipeline, we employ a specialized NLI model as a preliminary filter. This component acts as a gatekeeper, resolving straightforward claims where it exhibits high confidence and delegating only ambiguous cases to the more expensive LLM. Setup. For each claim-evidence pair, the NLI model predicts probabilities for finalize the verdict immediately only if the model predicts evidence chunk. Claims with all other outcomes are delegated to the LLM. To validate this configuration, we perform an ablation on the FEVER and SciFact-Open datasets, stratifying NLI predictions into three confidence intervals (0.99-1.00,

0.95-0.99, and 0.90-0.95) to assess its utility. We then benchmark the hybrid pipeline against a pure-LLM baseline to quantify the gains in both accuracy and computational efficiency. Results & Analysis. Tables 5 and 6 present the performance breakdown across different confidence intervals. We observe two key findings that justify the hybrid design:

 *N from 1 to 20.*

 Both Label Accuracy and F1-Score improve as context expands,

*N < 5), suggesting that small windows often N ≈ 8 − 11), the metrics stabilize and reach a*

1. High Accuracy in High-Confidence Zones. When the NLI model is highly confident (

accuracy: 98.47% on FEVER and 90.09% on SciFact-Open. This confirms that for clear-cut claims, the NLI model is as reliable as, or potentially more reliable than, the LLM. However, accuracy drops precipitously as confidence decreases (

14

*> 0.99), it achieves exceptional* Entailment, Contradiction, and Neutral. We

 Entailment with extreme confidence (> 0.99) against at least one


---

## Page 15

Setting Confidence Range Count Label Acc. (FEVER) NLI Only 0.99-1.00 261 0.9847 NLI Only 0.95-0.99 93 0.8602 NLI Only 0.90-0.95 25 0.6800 Pure LLM - 659 0.9333 Hybrid (Ours) Hybrid 659

> Table 5. Ablation study on FEVER: NLI confidence distribution and hybrid pipeline performance.
>

| Setting | Confidence | Range | Count | Label | Acc. | (SciFact) |
| --- | --- | --- | --- | --- | --- | --- |
| NLI Only 0.99-1.00 111 | 0.9009 |  |  |  |  |  |
| NLI Only 0.95-0.99 42 | 0.7143 |  |  |  |  |  |
| NLI Only 0.90-0.95 13 | 0.8462 |  |  |  |  |  |
| Pure LLM - 279 | 0.8587 |  |  |  |  |  |

> Table 6. Ablation study on SciFact-Open: NLI confidence distribution and hybrid pipeline performance.
>

dropping to ∼68% in the 0.90-0.95 range on FEVER), validating our decision to set a strict threshold at 0.99.

2. Superior Performance with Lower Cost. The hybrid NLI-then-LLM strategy effectively optimizes the efficiency- worst) in Equation 8.P

accuracy trade-off. First, it slightly outperforms the pure LLM baseline on both datasets (FEVER: 0.9402 vs. 0.9333; SciFact: 0.8623 vs. 0.8587), suggesting that the specialized NLI model effectively filters simple cases where LLMs might occasionally hallucinate or over-reason. Second, it significantly reduces computational overhead. On FEVER, the NLI model resolves 261 out of 659 claims (∼40%) directly; on SciFact, it handles 111 out of 279 ( pipeline reduces the demand for expensive LLM inference by approximately 40% without compromising overall verification accuracy.

### B.3. Implementation Details in Noise Detection B.3.1. CLUSTERING IMPLEMENTATION

To manage redundancy and identifying semantic topics, we implement a two-step clustering pipeline: •Dimensionality Reduction: We use UMAP to project embeddings into a lower-dimensional space, preserving local semantic structures. •Density Clustering: We apply HDBSCAN (McInnes et al. minsamples=1, andepsilon=0. These conservative settings allow us to retain fine-grained granularity, ensuring even small but distinct information nuggets (single-chunk clusters) are identified as valid topics.

#### B.3.2. VALIDATION OF WORST-CASE APPROXIMATION

This section validates the approximation used for the theoretical worst-case penalty (

 0.9402

 0.8623

∼40%). This means our

, 2017) with parameters set tominclustersize=2,

*Nout*) while utilizing the lowest-ranked ones. In this scenario: •The inversion count Nis maximized for every ignored cluster ( *c-inv*

- The rank denominatoris minimized (ranging from R c

15

*Nc-inv* = N ), as all utilized clusters are ranked lower.*in*

 1 to), maximizing the term N 1./R*out*

Pworst ≈ N*in*×

Problem Formulation. Recall that the penalty for a single ignored cluster is given by determine the theoretical worst-case scenario, we must identify a subset of "ignored" clusters from the total set of clustersthat maximizes the total penalty. Ctotal Our proposed approximation assumes the worst case occurs when the agent ignores the

*Nout*X *. (8)*

 highest-ranked1to clusters (Ranks


---

## Page 16

However, because the cluster size Svaries, simply selecting the highest-ranked clusters might not yield the absolutec maximum if a lower-ranked cluster has a significantly larger size Rigorously, finding the true global maximum is a combinatorial optimization problem:

Pmax = max

S⊂C

where N*c-inv*(S) denotes the count of clusters in\ S *total*

Computational Complexity. Solving this exact optimization requires enumerating all possible subsets space size is defined by the binomial coefficient. To assess feasibility, we analyzed clustering results from 100 full *Ntotal*

*Nout*

research trajectories generated by Gemini Deep Research (50 from Mind2Web2, 50 from ReportEval). As shown in Table the mean search space size exceeds, rendering brute-force enumeration computationally infeasible. 10

Metric Mean Max Min Search Space (logT ) 168 849 10

> Table 7. Estimated computational complexity (
>

Statistical Justification. Given the computational intractability, we validate our rank-based approximation by analyzing the distribution of cluster sizes (S). IfSvaried dramatically ( *c c*

the rank-based assumption would fail. However, empirical data in Figure 12 reveals a highly concentrated distribution. Across both datasets, consist of only 1-3 chunks. This low variance implies that inversion countN. Consequently, swapping a high-ranked small cluster for a low-ranked large cluster is statisticallyc-inv unlikely to increase the total penalty, as the penalty reduction from increased rank outweighs the gain from a marginally larger. Sc Therefore, our approximation, defining the worst case as the neglect of the highest-ranked clusters, serves as a robust and computationally efficient proxy for the theoretical maximum.

## C. Benchmark Details

### C.1. Candidate Data Sources

To ensure broad coverage, we aggregated queries from three high-quality sources:

- Mind2Web2: We utilized the full set of 130 search-oriented open-ended queries.
- ReportEval: We included 100 research-oriented open-ended queries covering multifaceted topics.

•BrowseComp: From this large-scale dataset of∼1,200 close-ended tasks, we employed stratified sampling to select ∼100 representative queries, preserving the original topic distribution while keeping workload manageable.

### C.2. Difficulty Assessment Logic

Our selection process relies on the premise that queries inducing hallucinations in a top-performing DRA ( Deep Research) are likely to be effective stress tests for other DRAs. Queries with the highest the final benchmark.

### C.3. Dataset Composition

DeepHalluBenchcontains queries spanning 11 distinct domains, ranging from humanities ( History) to technical fields (e.g., Science & Technology

*Sthat outweighs the penalty reduction from a largerR.cc* X*S× N*(S)

*c c-inv*

*, (9)*

*total,|S|=Nout Rc*

*c∈S*

## S. The search

T ) for exact worst-case enumeration across 100 research trajectories.

e.g., if one lower-ranked cluster contained 50% of all chunks),

∼75% of clusters

*Sacts nearly as a constant factor relative to the rank Randcc*

*R (which grows linearly) typicallyc*

168

As shown in the left chart (N = 100), the dataset achieves a broad and multifaceted coverage of complex topics. Dominant segments include Art, Music & Literature (19.0%), while specialized fields like Politics, Health

10. T ) for exact worst-case enumeration across 100 research trajectories.

 Science & Technology (13.0%), and Entertainment & Gaming (13.0%), , and Career provide critical diversity. The right chart (N = 75) confirms that 16

Hscores were retained for

e.g., Art, Music & Literature,

). Figure 10 visualizes this distribution.


---

## Page 17

> Figure 10. Topic Distribution of DeepHalluBench. The left chart details the domain breakdown for the full benchmark ( including adversarial queries. The right chart illustrates the distribution for the "answerable" subset (
>

*N = 75). The broad coverage across 11 diverse categories prevents domain-specific bias and ensures a holistic assessment of DRA* capabilities.

excluding the 25 adversarial "no-answer" queries preserves this distribution structure, confirming that our analysis remains statistically robust across varying subject matters. Domain Vulnerability Analysis. Figure 11 illustrates the filtering process from the initial candidate pool to the final benchmark. The percentage above each bar represents the triggered significant hallucinations and were thus retained for the final difficult set. Analyzing these ratios reveals a critical insight: hallucinations are popular topics such as Science & Technology and Lifestyle & Leisure show relatively low selection rates, identifying them as areas where DRAs are generally robust. In contrast, "long-tail" or specialized domains exhibit much higher vulnerability. Notably, Geography & Environment has the highest selection ratios of 75.0%, despite having smaller initial candidate counts. This suggests that DRAs struggle significantly more with niche topics. The severe hallucination degrees in domains like Geography (75.0%) are likely attributable to their long-tail nature and the density of specialized domain knowledge. These factors complicate accurate retrieval and synthesis, thereby increasing the propensity for fabrication when the agent cannot access or reason over obscure facts.

### C.4. Case Study for Atomic Perturbations

> Table 8. Examples of Atomic Perturbations. We merge different perturbation types into a single view, each type with three examples respectively.
>

Query (Original) Query (Modified) Modification

Type 1: Entity Attribute Modification

*N = 100),* i.e., without "no-answer" queries,

 Selection Ratio, i.e., the proportion of queries in that domain that

 unevenly distributed across domains. While high-resource,

17

Continued on next page...


---

## Page 18

Table 8 continued from previous page

Query (Original) Query (Modified) Modification A musical artist has a first name and surnameA musical artist has a first name and surname that begins with the exact same letter (as ofthat begins with the exact same letter (as of

2023). This musical artist quoted a Columbia2023). This musical artist quoted a 

organ to a food item. The food item in questionorgan to a food item. The food item in question has a strong historical connection to a mythicalhas a strong historical connection to a mythical individual. This mythical individual shares aindividual. This mythical individual shares a name with a protagonist in a speculative fictionname with a protagonist in a speculative fiction novel published as the first in a series, betweennovel published as the first in a series, between

2000 and 2005 (inclusive). What is the title of2000 and 2005 (inclusive). What is the title of the song? 

country to play in the English Premier League?country to play in the English Premier League? This player represented the same club for seven This player represented the same club for seven seasons in the Premier League. Although bornseasons in the Premier League. Although born in an African country, he later had nationalityin an African country, he later had nationality 

1995 under the zodiac sign Taurus, he also has1995 under the zodiac sign two brothers.

has two brothers

There's a person 1 who shares many similari-There's a person 1 who shares many similarities with person 2 such as a near-identical lastties with person 2 such as a near-identical last name, and identical ethnicity. Person 2 is aname, and identical ethnicity. Person 2 is a graduate of one of the universities founded ingraduate of one of the universities founded in the Georgian era and has published their firstthe Georgian era and has published their first book in years between 2010 and 2020, inclu-book in years between 2010 and 2020, inclusive. The illustrator of that book has a mas-sive. The illustrator of that book has a master's

ter's degree in graphic design and a bachelor'sdegree in graphic design and a bachelor's dedegree in literature from another universitygree in founded in the early 1700s. Person 2 and the il-sity founded in the early 1700s. Person 2 and lustrator knew each other for a long time. Sincethe illustrator knew each other for a long time. what age did Person 2 and the Illustrator know Since what age did Person 2 and the Illustrator each other?

Type 2: Temporal Detail Modification

Entity Substitution (Institution): Change

Idaho"

Attribute Modification (Birth Data): Change "Taurus" to "Scorpi"

 Scorpio, he also

Attribute Modification (Education): Change "literature" to "computer science".

 computer science from another univer-

University alumnus in a 2019 interview. Theof Idaho

 University"Columbia University"

 alumnus in a 2019 interview. Theto "University of

18

Continued on next page...


---

## Page 19

Table 8 continued from previous page

Query (Original) Query (Modified) Modification A university established between 1995 andA university established between 1995 and 

than fifteen years after this session, a memberthan fifteen years after this session, a member 

after graduating, they joined the university as aafter graduating, they joined the university as a full-time lecturer in the same department theyfull-time lecturer in the same department they received their bachelor's. They are one of thereceived their bachelor's. They are one of the authors of a paper that focuses on comparingauthors of a paper that focuses on comparing and testing how two different methods performand testing how two different methods perform in finding the best solution from a finite set ofin finding the best solution from a finite set of possibilities. The paper was first submitted apossibilities. The paper was first submitted a year after their graduation. The second versionyear after their graduation. The second version of the paper, after revision, was submitted theof the paper, after revision, was submitted the year they joined the university as a full-timeyear they joined the university as a full-time 

State the full name of this author as expressed State the full name of this author as expressed in the paper. A TV show aired in the 1990s. Two actors thatA TV show aired in the 1990s. Two actors that starred in it attended the same university. Thestarred in it attended the same university. The 

it won was due to the work of an artist whoit won was due to the work of an artist who had also worked on famous musicians' musichad also worked on famous musicians' music videos and missed out on the same award withvideos and missed out on the same award with one of the previous projects they had workedone of the previous projects they had worked on. A 2010 review of the show described theon. A 1995 review of the show described the aforementioned pair of actors as its 'alpha andaforementioned pair of actors as its 'alpha and 

house in the following year. What was thehouse in the following year. What was the first name and surname of the composer whofirst name and surname of the composer who created the show's main theme? created the show's main theme?

Temporal Shift (Event Timing): Change "July" to "January" January. Less

Temporal Shift (Source Document): "2010" to "1995".

19

Continued on next page...


---

## Page 20

Table 8 continued from previous page

| Query | (Original) | Query | (Modified) | Modification |
| --- | --- | --- | --- | --- |
| sponsored and remained in service until 1973.sponsored and remained in service until | 1973. |  |  |  |

surname.

Type 3: Quantitative Value Modification

In a little agricultural town with a population In a little agricultural town with a population density of less than 25,000 people, as recordeddensity of less than 25,000 people, as recorded before 2023, but after 2020, a proposal wasbefore 2023, but after 2020, a proposal was submitted to extend a section of the town,submitted to extend a section of the town, 

were relocated from an earlier settlement thatwere relocated from an earlier settlement that 

what was once molten magma b) The area iswhat was once molten magma b) The area is disturbed c) No graves or rock engravings d)disturbed c) No graves or rock engravings d) Five stone foundations noted Please provide Fifty stone foundations noted Please provide 

surname of the individual who compiled thissurname of the individual who compiled this particular survey. Also include the year of theparticular survey. Also include the year of the report in parentheses. In 2021 an article discussing a meme posted on In 2021 an article discussing a meme posted on 

also references exactly 5 books by name and 1also references book series by name. A 2015 article discussesbook series by name. A 2015 article discusses the author cited in the text of the meme fromthe author cited in the text of the meme from 

in August of 2021 cites as being created by ain August of 2021 cites as being created by a person with a Ph.D. in computer science. Whatperson with a Ph.D. in computer science. What is the first and last name of the 1997 footballis the first and last name of the 1997 football

Fact Alteration "Easter Sunday" to "Halloween".

Detail Modification (Document Content): Change "Five stone foundations" to "Fifty stone foundations".

Fingerprint Alteration (Count): Change "exactly 5 books" to "exactly 15 books".

 exactly 15 books by name and 1

coach referenced in the 2015 article?coach referenced in the 2015 article?

20

Continued on next page...


---

## Page 21

Table 8 continued from previous page

Query (Original) Query (Modified) Modification A child was reported missing several timesA child was reported missing several times between January 1, 2014, and December 31,between January 1, 2014, and December 31,

2018. In late 2014, the missing 13-year-old2018. In late 2014, the missing 13-year-old

was found along with two other missing teens.was found along with In late 2015, the 14-year-old was also reportedteens. In late 2015, the 14-year-old was also missing but was located shortly afterward. Inreported missing but was located shortly afearly 2018, the 16-year-old was reported miss-terward. In early 2018, the 16-year-old was ing. According to the police's description,reported missing. According to the police's dewhat color shirt were they last wearing whenscription, what color shirt were they last wear- seven other missingteens" to "seven other

they went missing in 2018?

Type 4: Logical Relationship Modification

This African leader, born in the early 20thThis African leader, born in the early 20th century visited the official residence of thecentury visited the official residence of the 

Africa. During his visit to the residence of this Africa. During his visit to the residence of this global superpower's leader, a grand dinner wasglobal superpower's leader, a grand dinner was held in his honor, featuring a particular dessertheld in his honor, featuring a particular dessert topping that shares its name with a prominenttopping that shares its name with a prominent individual who was burdened with a wordindividual who of caution that could avert the assassinationprovide the name of this food? 

food? The university was established between 2000The university was established between 2000 and 2003, inclusive. Prior to December 2023,and 2003, inclusive. Prior to December 2023, the university's founder was a scientist andthe university's founder was a scientist and the chairman of its board of trustees. Theythe chairman of its board of trustees. They earned their PhD from an institute that wasearned their PhD from an institute that was officially recognized as a university in Julyofficially recognized as a university in July between 1965 and 1968, inclusive. Prior tobetween 1965 and 1968, inclusive. Prior to December 2023, students at the university were December 2023, students at the university were required to take mandatory language coursesrequired to take mandatory language courses in a specific foreign language. Between 2020in a specific foreign language. Between 2020 and 2023, inclusive, the university celebratedand 2023, inclusive, the university celebrated the 10th anniversary of its campus openingthe 50th anniversary in another country. What is the name of thein another country. What is the name of the university? Between 1990 and 2002 inclusive, this music Between 1990 and 2002 inclusive, this music group lost one of their parents. The incidentgroup lost one of their parents. The incident 

who once represented an individual in a casewho once represented an individual in a case where the crime/incident occurred in that samewhere the crime/incident occurred in that same year range. In this same trial, an individual atyear range. In this same trial, an individual at

Fact Alteration (Incident Detail): Change "two other missing

missing teens".

Logic/Riddle Break: 

namesake (from an assassination warner, to the discoverer of penicillin).

 discovered penicillin. Can you

Logical Impossibility (Timeline): Change "10th anniversary" to "50th anniversary".

a very young age, between 8 and 17, testifieda very young age, between in it. Which month did this trial begin?it. Which month did this trial begin?

21

 1 and 2, testified in of its campus opening

Procedural Impossibility (Legal Context): Change "8 and 17" to "1 and 2".


---

## Page 22

> Figure 11. Comparison of Candidate vs. Selected Sets. The percentages indicate the Selection Ratio for each domain, defined as the ratio of queries retained for the final benchmark to the total candidate pool aggregated from the source datasets.
>

The specific distribution of these perturbation types across the 25 adversarial queries is summarized in Table coverage across semantic, temporal, quantitative, and logical restrictions.

> Table 9. Distribution of Adversarial Perturbations. The dataset prioritizes entity and temporal modifications while including specific logical and quantitative challenges.
>

| Perturbation | Type | Count |
| --- | --- | --- |
| (1) Entity Attribute Modification | 9 |  |
| (2) Temporal Detail Modification | 9 |  |
| (3) Quantitative Value Modification | 4 |  |
| (4) Logical Relationship Modification | 3 |  |
| Total | 25 |  |

## D. Detailed Results

### D.1. Retrieval Quality

We quantify Retrieval Quality by assessing the relevance of the top-ranking documents retrieved for a user query. Since DRAs often retrieve a large volume of noisy documents alongside relevant ones, a simple average across the full retrieval set would obscure the agent's true capability to locate critical information. To measure the agent's peak retrieval power ( upper limit), we isolate the top-5 most relevant documents and utilize their average relevance as the metric for the task. The specific implementation details are as follows: •Chunk-Level Scoring. We first evaluate semantic relevance at the granular level. For every chunk, we calculate its relevance score against each atomic sub-query via a reranker, using the average as the chunk's final relevance score. •Document-Level Aggregation. We adopt a "max-relevance" strategy to score full documents. We define a document's score as the maximum score of its constituent chunks, ensuring we capture high-value information signals regardless of the document's length or surrounding irrelevant text. •Quality Quantification. Finally, we rank all retrieved documents by their scores and compute the mean relevance of the top-5 candidates to determine the task-level quality. The final Retrieval Quality for a DRA is calculated by averaging these scores across all tasks in the benchmark. In summary, by isolating the top-5 candidates, this metric provides a noise-robust estimate of the agent's capacity to discover

22


---

## Page 23

> Figure 12. Distribution of Semantic Cluster Sizes. The majority of clusters are small, with∼75% containing only 1-3 chunks. This low variance supports the rank-dominant approximation.
>

(a) Explicit Summarization: Claim Hallucination

> Figure 13. Domain-specific performance for Summarization Hallucinations. High-entropy domains like Entertainment and Sports induce higher hallucination degree across both dimensions compared to structured domains like Economy.
>

high-value evidence.

### D.2. Domain Sensitivity Analysis

To further investigate the "hallucination profiles" of DRAs, we decompose their performance across 11 distinct query domains. The following radar plots visualize the distribution of hallucinations for each DRA, highlighting the disparity between structured domains (e.g., Economy, Science) and high-entropy domains ( Figure 13 details the Summarization stage, including Claim Hallucinations (Explicit) and Noise Domination (Implicit). Figure 14 details the Planning stage, visualizing Action Hallucinations (Explicit) and Restriction Neglect (Implicit). Finally, Figure 15 presents the composite hallucination score ( These figures show that hallucination severity exhibits strong domain dependency. We observe three distinct patterns:

(b) Implicit Summarization: Noise Domination

•The "Universal Trap": Geography & Environment emerges as the most challenging domain, ranking poorly across nearly all categories. This suggests that spatial reasoning and dispersed environmental data trigger systemic failures in both planning logic and information summarization.

23

e.g., Entertainment, Lifestyle).

H) across all domains.


---

## Page 24

(a) Explicit Planning: Action Hallucination

> Figure 14. Domain-specific performance for Planning Hallucinations. Note the specific spike in Action Hallucination for Geography and
>

Politics, and the high Restriction Neglect in Lifestyle and Career domains.

•Structured vs. High-Entropy: A clear performance gap exists between structured and unstructured fields. DRAs demonstrate high performance in Economy and Medicine, benefiting from standardized terminologies. In contrast, performance degrades significantly in high-entropy, pop-culture domains like •Restriction Nuance: Implicit planning reveals a vulnerability to qualitative ambiguity. While agents effectively minimize restriction neglect in Science ("hard" restrictions), they falter in difficulty in parsing the "soft," subjective constraints inherent to human-centric tasks. D.3. Close vs. Open-Ended Figure 16 reveals that close-ended tasks impose a significantly higher challenge, triggering elevated error rates across critical dimensions compared to open-ended tasks. Specifically, we observe systemic spikes in Action Hallucination in the close-ended setting. This phenomenon stems from the inherent difficulty of the BrowseComp dataset, where queries impose rigid, binary restrictions that demand exact retrieval. Unlike open-ended reporting, where agents can synthesize broad information to mask retrieval gaps, these rigorous restrictions force agents into immediate failure modes, cascading into subsequent steps. Thus, rather than being simple, close-ended tasks serve as a severe stress test for retrieval precision and summarization faithfulness.

## E. Extended Analysis of Failure Mechanisms

### E.1. Propagation Detection Methodology

To construct the Directed Acyclic Graph (DAG) presented in Section 5.3, we detect propagation between explicit hallucina- tions through two specific mechanisms: •Homogeneous Propagation: This captures errors propagating within the same modality ( or Deviation→Deviation). We identify these links by leveraging NLI models to detect high-confidence entailment relationships between successive error nodes. •Heterogeneous Propagation: This captures errors crossing modalities (Fabrication via our Action Propagation metric (A propagation) defined in Section 4.2, where an action is deemed compliant with a

(b) Implicit Planning: Restriction Neglect

 Entertainment and Sports. Career and Lifestyle. This indicates a fundamental

 Fabrication, Noise Domination and

hallucinated premise. We limit this graph analysis to Gemini, OpenAI, and Salesforce, as other DRAs do not expose the sufficient intermediate summarizations or plans required for granular propagation tracking.

24

→i.e.Fabrication, Fabrication

→Deviation). These are identified


---

## Page 25

> Figure 15. Composite Hallucination Score (H) across query domains. Geography & Environment represents the most challenging
>

"Universal Trap" for current DRAs.

### E.2. Root-Cause Error Analysis

To understand the etiology of final failures in close-ended tasks, we isolate the in the research trajectory that precipitates the final incorrect outcome. Following ( identify this critical pivot point by analyzing the full trajectory alongside the final answer. Figure 17 visualizes the distribution of these root-cause errors.

### E.3. Semantic Bias Analysis

We further investigate how information diversity impacts agent performance. Figure Preference for Redundancy (Top). We compare the average size (chunk count) of utilized clusters ( ignored clusters (Out-Memory). Across all agents, utilized clusters are consistently larger ( This confirms that DRAs use repetition as a proxy for importance, favoring homogeneous content over singleton insights. Vulnerability to Diversity (Bottom). We analyze the correlation between information heterogeneity (total cluster count) and the Noise Score. For weaker models like Salesforce implies that as the retrieval context becomes more diverse (more distinct topics), the DRA's attention mechanism fails to prioritize effectively, leading to higher rates of information neglect.

 root-cause error, defined as the earliest step Zhu et al., 2025b), we leverage an LLM to

 18 visualizes two key trends:

In-Memory) versus e.g., Gemini: 4.0 vs. 2.5 chunks).

 and Grok, we observe a significant positive correlation. This

25


---

## Page 26

> Figure 16. Comparison of Hallucination Metrics between Open-Ended and Close-Ended tasks. while dark bars denote close-ended ones. Close-ended tasks generally incur more severe hallucinations across most metrics; the notable
>

exception is Misattribution, which is naturally higher in open-ended tasks due to the requirement for long-form reports containing numerous citations, contrasting with the short-form answers typical of close-ended queries.

> Figure 17. Heatmap of Root-Cause Errors across Modules and Stages. We classify detected root-cause errors by module and research stage. Search denotes cases where the agent failed to retrieve information and reported "no answer found" despite a trajectory free of
>

hallucinations. None denotes cases where the agent produced a fabricated answer despite a research trajectory containing no detectable errors. Darker cells indicate higher frequency.

 Light bars denote open-ended tasks,

> Figure 18. Semantic Analysis of Information Attention.
>

Correlation between the total number of clusters (information heterogeneity) and the Noise Score.

26

 Top: Average size of utilized clusters versus ignored clusters. Bottom:


---

## Page 27

## F. Prompts

To ensure the robustness of our automated evaluators, we employ an iterative strategy. Prompts are refined over multiple cycles of expert critique until the judgment logic stabilizes and produces accurate results, ensuring the LLM judges align closely with human reasoning.

### F.1. Prompt for Decomposition F.1.1. QUERY DECOMPOSITION

You are an expert query analysis system specialized in decomposing user queries into structured atomic restrictions. ## TASK Extract concise, independent Atomic Restrictions from user queries. ## ATOMIC CONSTRAINT CRITERIA Each extracted constraint must satisfy the following properties:

- Indivisibility: Must be a single, self-contained unit with clear meaning. Break down complex

queries (linked by 'and', 'with', 'while') into separate items.

- Objectivity: Must contain objective conditions or criteria only. Exclude descriptive facts,

background information, or subjective statements.

- Context Independence: Must be neutral and understandable in isolation. Remove personal

references (\eg, 'I', 'me', 'my', 'for me') and ambiguous pronouns. ## EXTRACTION METHODOLOGY

1. Decompose: Split compound sentences into individual atomic units based on the criteria above.
2. Refine: Ensure strictly objective, neutral language.
3. Format: Output each constraint on its own line prefixed with

#### F.1.2. REASONING TEXT DECOMPOSITION

You are an expert text decomposition system specialized in reconstructing research trajectories by disentangling reasoning text interleaved with plans and summaries. ## TASK Deconstruct paragraphs to isolate and extract Atomic Claims (from summaries) and Atomic Actions (from plans). You must perform systematic fragmentation and classification to ensure every extracted item satisfies the criteria of Indivisibility, Semantic Integrity, Verifiability, and Context Independence. ## METHODOLOGY ### 1. Source Fidelity

- Use the provided paragraph as the single source of truth. The query is context only; never add

details that are not explicitly written in the paragraph.

- Do not infer missing steps, reasons, or entities from background knowledge.

### 2. Step 1: Fragmentation (Minimal Splitting & Disentanglement)

- Produce the smallest set of fragments that faithfully reflect the paragraph

sentences.

- Disentanglement: If a sentence mixes summaries and plan (interleaved reasoning), split *only*

 human-in-the-loop prompt optimization

along that boundary; otherwise keep the sentence intact.

- Resolve pronouns using paragraph context immediately to ensure atoms are self-contained.

27

's explicit


---

## Page 28

Context reminder: The text may contain both discoveries and plans. Classify only what is explicitly written.

- `summaries`: Facts, findings, reflections, or summary statements (Output as Atomic Claims).
- `plan`: Actions the agent explicitly states it will take next (Output as Atomic Actions).

### 4. Step 3: Atomic Extraction (The 4 Essential Properties) Refine the classified fragments into valid atomic units. Each unit must strictly satisfy the following four properties defined in the research trajectory:

1. Indivisibility: The unit must represent a single, indivisible action or claim; further

splitting would compromise its semantic meaning.

*Operational Rule:* Prefer to keep clauses together; only split truly parallel elements

(\eg, clearly enumerated lists).

2. Semantic Integrity: Each unit must retain sufficient detail to preclude ambiguity, including

necessary conditions and clauses, ensuring the original intent is fully preserved.

*Operational Rule:* Keep integral conditions attached (\eg,

the specified label*'). Do not fragment conditions from their actions.

3. Verifiability: The unit must be objectively verifiable. Speculative language and subjective

opinions are filtered out.

*Filtering Criteria:* EXCLUDE speculative language (

'seems'), subjective opinions ('effective', 'best'), and vague process descriptions.

4. Context Independence: All coreferences (\eg, pronouns) must be explicitly resolved, ensuring

the unit can be assessed in isolation without relying on preceding context. ### 5. Format Compliance (For Plans)

- Imperative Verbs: Atomic Actions must start with an imperative verb (\eg,

'Run').

- Ignore implied steps; strictly output the explicit action described.

## EXAMPLES Decomposition & Context Independence:

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

- Input: Meta's careers page lists 'Research Scientist
- Output:
- Meta's careers page lists 'Research Scientist
- Meta's careers page lists 'Research Scientist

 'Search for issues... *with

'may', 'might', 'could', 'likely',

 'Search', 'Analyze',

Semantic Integrity - DO NOT Split Conditions:

- Input: Search for issues within the target module that have the specified label.
- [Incorrect] Wrong Output:

28

' in Menlo Park, CA, and Seattle, WA.


---

## Page 29

- Search for issues within the target module
- Filter issues with the specified label
- [Correct] Output:
- Search for issues within the target module that have the specified label

## OUTPUT FORMAT Fragment 1: [Context-independent text] Classification: [summary/plan] Atomic [Claims/Actions]:

If no extractable content: `No extractable content paragraph contains only vague descriptions or speculative language.'

#### F.1.3. REPORT PARAGRAPH DECOMPOSITION

You are an expert fact decomposition system specialized in extracting Atomic Claims from text. ## TASK Extract ONLY concrete, verifiable observations or findings. You must decompose the text into Atomic Claims that satisfy the criteria of Indivisibility, Semantic Integrity, Verifiability, and Context Independence. ## ATOMIC CLAIM PROPERTIES (METHODOLOGY) ### 1. Indivisibility The unit must represent a single, indivisible fact.

- Operational Rule: Only split truly parallel elements (\eg, X and Y where X and Y are

independent facts).

- Constraint: Do NOT split complex sentences if doing so would compromise semantic meaning or

disconnect a clause from its subject. ### 2. Semantic Integrity Each unit must retain sufficient detail to preclude ambiguity.

- Operational Rule: Preserve all modifiers, conditions, and qualifiers that are semantically

integral to the main clause.

- Constraint: Do NOT split prepositional phrases, relative clauses, or purpose clauses (\eg,

find...') from the entity they modify. ### 3. Verifiability The unit must be objectively verifiable.

- Include: Specific facts, data, concrete entities, locations, numbers, and definitive results.
- FILTER OUT (Exclude):
- Speculative language ('may', 'might', 'could', 'possibly', 'likely', 'appears', 'seems').
- Subjective opinions ('effective', 'ideal ', 'best', 'good', 'useful').
- Vague process summaries ('Progress has been made...
- URLs.

### 4. Context Independence All coreferences must be explicitly resolved ensuring the claim is self-contained.

- Operational Rule: Replace pronouns ('this ', 'that', 'it', 'they') with specific referents

using the paragraph context.

29

', 'We plan to...').


---

## Page 30

- Verification Test: Can someone verify this claim

surrounding text? ## EXAMPLES Verifiability (Filtering Speculation):

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

specified label ## OUTPUT FORMAT

If no extractable content: `No extractable content paragraph contains only vague descriptions or speculative language.`

#### F.1.4. DOUBLE CHECK FOR ATOMIC CLAIMS

You are a quality control system specialized in validating and refining Atomic Claims as a secondary double-check layer. ## TASK Review preliminary claims to rectify common errors in Divisibility (\eg, parallel structures) and Context Independence (\eg, unresolved pronouns). ## REFINEMENT CRITERIA ### 1. Indivisibility (Split Parallel Structures) Ensure each claim represents a single, indivisible fact.

's truthfulness without reading the original

- Rule: Break compound statements linked by `and`, `or`, `but` ONLY when they represent

independent, parallel facts that do not affect each other

- Example: `Role available in Menlo Park and Seattle

30

's meaning. ` -> Split into two separate claims.


---

## Page 31

### 2. Semantic Integrity (Do NOT Split Modifiers) Preserve semantic detail to preclude ambiguity.

- CRITICAL: Do NOT split modifiers, conditions, or qualifiers from their main clauses.
- Preserve:
- Prepositional phrases (\eg, `within the target module
- Relative clauses (\eg, `that have the specified label
- Purpose clauses and integral qualifiers.

### 3. Context Independence (Resolve Coreferences) Ensure claims are verifiable in isolation without surrounding context.

- Resolve Pronouns: Replace `the`, `this`, `that`, `it`, `they` with specific entity names.
- Explicit References: If a claim references `the position` or `this role`, specify the exact

entity using the broader context.

- Exclusion: If the context for a pronoun or reference cannot be determined, exclude the claim

entirely. ## EXAMPLES Indivisibility (Parallel Elements - OK to Split):

- Input: `Role available in Menlo Park, CA and Seattle, WA
- Output:
- Role available in Menlo Park, CA
- Role available in Seattle, WA

Semantic Integrity - DO NOT Split Conditions:

- Input: `xxx to find information about the oldest closed issue in the target module with the

specified label`

- [Incorrect] Wrong Output:
- `xxx to find information about the oldest closed issue in the target module
- `The oldest closed issue in the target module has the specified label
- [Correct] Output:
- `xxx to find information about the oldest closed issue in the target module with the

specified label` Context Independence (Resolution):

- Input: `The position focuses on experimenting with neural network architectures.
- Context: DeepMind Research Engineer/Scientist position
- Output: `DeepMind Research Engineer/Scientist position focuses on experimenting with neural

network architectures` ## OUTPUT FORMAT Return each refined, atomic claim on a new line with

Format. Remove any items that are observations (facts) rather than actions. ## REFINEMENT CRITERIA

31

#### F.1.5. DOUBLE CHECK FOR ATOMIC ACTIONS

You are a quality control system specialized in validating and refining Atomic Actions as a secondary double-check layer. ## TASK Review preliminary actions to rectify common errors in Divisibility, Context Independence, and

 `- ` prefix.


---

## Page 32

### 1. Indivisibility (Split Parallel Actions) Ensure each action represents a single, indivisible task.

- Rule: Break compound statements linked by 'and', 'or', 'but' ONLY when they represent

independent, parallel actions that do not affect each other ### 2. Semantic Integrity (Do NOT Split Modifiers) Preserve semantic detail to preclude ambiguity.

- CRITICAL: Do NOT split modifiers, conditions, or qualifiers from their main clauses.
- Preserve:
- Prepositional phrases (\eg, 'with the specified label
- Relative clauses (\eg, 'that have the specified label
- Purpose clauses and integral qualifiers.

### 3. Context Independence (Resolve Coreferences) Ensure actions are executable in isolation without surrounding context.

- Resolve Pronouns: Replace 'the', 'this', 'that', 'it', 'they' with specific entity names.
- Context Integration: Use broader action list context to provide necessary specificity.
- Exclusion: If context cannot be determined, exclude the action entirely.

### 4. Format Compliance & Validity

- Imperative Form: Start with a verb. Remove subjects like

Transform 'I will search' to 'Search').

- Validity Check: If the item is a fact/claim (\eg,

plan/action, remove it. ## EXAMPLES Basic Action Transformation:

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

## OUTPUT FORMAT Return each refined, atomic action on a new line with

's meaning.

', 'within the target module').

 'I', 'the agent', 'the user'. (\eg, 'Ronnie Wood has four children') and not a

You are an expert claim verification system specialized in assessing the evidentiary relationship between a specific claim and a retrieved document chunk.

32

### F.2. Prompt for Claim Verification

 '- ' prefix.


---

## Page 33

## TASK Given a claim, a query, and a document chunk, classify the relationship as Support or Unsupport.

- Source of Truth: The provided document chunk represents information explicitly retrieved by

the agent during its research.

- Inference Rule: If a claim describes the agent

follow from this chunk, treat it as Support unless the chunk clearly contradicts it. Before showing your final answer, think step-by-step and show your specific reasoning. ## CLASSIFICATION CRITERIA ### 1. Support The document validates the claim through explicit statement, reasonable inference, or logical abstraction.

- Explicit/Inferred: The claim is stated in the text or is a direct logical consequence of the

facts presented.

- Resource Availability (IMPORTANT): If the claim describes the acquisition, access, or

availability of information (\eg, 'The agent accessed the product page of the document content itself as sufficient evidence that such access was established. ### 2. Unsupport The document fails to validate the claim due to contradiction or insufficiency.

- Contradiction: The document contains information that directly refutes the claim.
- Insufficient Information: The document mentions related topics but lacks the specific data,

numbers, or details required to verify the claim. ## EXAMPLES Case 1: Support (High-level Abstraction)

- Document: 'Product specifications and pricing information for the new smartphone model...
- Claim: 'The agent has successfully accessed the product page.
- Judgment: Support (The claim about accessing the page is validated by the actual presence of

content from that page.) Case 2: Support (Reasonably Inferred)

- Document: 'Phase III trials reported an efficacy rate above 90% for the vaccine.
- Claim: 'The vaccine was highly effective in trials.
- Judgment: Support ('Highly effective' is a reasonable inference from

Case 3: Unsupport (Contradiction)

- Document: 'The experiment was conducted with 100 participants aged 18-25.
- Claim: 'The study included elderly participants over 65.
- Judgment: Unsupport (The document explicitly defines a younger age range, contradicting the

claim.) Case 4: Unsupport (Insufficient Information)

- Document: 'The company announced a new product launch.
- Claim: 'The product launch increased quarterly revenue by 15%.

's focus, actions, or conclusions that naturally

'), consider the presence

 'efficacy above 90%'.)

- Judgment: Unsupport (The document mentions the launch event but provides no financial data to

verify the specific revenue figure.)

33


---

## Page 34

## OUTPUT FORMAT After your reasoning, output ONLY the JSON object in this exact format: ```json 'judgment': 'Support|Unsupport', 'evidence': 'One-sentence explanation for your judgment 'confidence': 0.0-1.0

### F.3. Prompt for Action Verification

You are an expert action verification system specialized in assessing the coherence and necessity of a proposed action within a research trajectory. ## TASK Evaluate whether the Action to Evaluate supports the User Query, considering the current context of Previous Claims (facts) and Previous Actions (plans). ## INPUT CONTEXT

- User Query: {query}
- Previous Claims: {claims_context}
- Previous Actions: {actions_context} (In-progress steps)
- Action to Evaluate: {action}
1. Goal Coherence: Does the action align with the user
2. Logical Continuity: Is the action a reasonable next step?
3. Assumption of Success (CRITICAL): Treat in-progress Previous Actions as if they will succeed

and return ideal results. An action is NOT premature if it relies on prerequisites that are currently being fetched by previous steps. ## CLASSIFICATION CRITERIA ### 1. Support The action makes reasonable progress toward the goal.

- Valid Extensions: The action uncovers new info, expands search space, or advances the task.
- Lightweight Operations: Actions like 'Extract

existing data are always Support.

- Sequential Planning: If Action B depends on Action A (which is in progress), Action B is

Support, not a deviation. ### 2. Unsupport The action is either redundant or irrelevant. Type A: Redundancy

- Definition: The action repeats a step that has *already produced concrete results*.
- Strict Rule: Mark as Redundancy ONLY if a Previous Claim documents the exact same search/tool

execution with actual results.

- Exceptions (Not Redundant):
- Different tools (\eg, Wikipedia vs. Google) or query phrasings.
- Repeating a failed search (if the first attempt yielded nothing).

's objectives?

', 'Format', 'Summarize', or 'Compile' based on

- Actions dependent on prerequisites currently being fetched.

Type B: Deviation

34


---

## Page 35

- Definition: The action pursues a completely irrelevant tangent.
- Constraint: Do not mark as deviation if the action is an intermediate step toward the main

goal. ## SOURCE ASSIGNMENT RULES

- If Support: Set source to -1 (derived from query) OR claim index [i] (if building upon a

specific fact).

- If Redundancy: Set source to claim index [i] (the specific claim that makes this action

unnecessary).

- If Deviation: ALWAYS set source to -1 (deviates from the query/goal).

## EXAMPLES Case 1: Support (Alternative Search)

- Query: 'Find Python 3.12 features'
- Previous Claim [2]: 'Official docs lack 3.12 details
- Action: 'Search GitHub for Python 3.12 features
- Output: {{'label': 'Support', 'source': 2, 'type': null, 'confidence': 0.9, 'explanation':

'Explores alternative sources after claim [2] confirmed a gap. Case 2: Unsupport (Redundancy)

- Query: 'Top Italian restaurants in Boston '
- Previous Claim : 'Found top rated: Mamma Maria (4.8), Giulia (4.7)
- Action: 'Search for best Italian restaurants in Boston
- Output: {{'label': 'Unsupport', 'source': 1, 'type': 'redundancy', 'confidence': 0.95,

'explanation': 'Claim already provides the exact results this action seeks. Case 3: Support (Lightweight Extraction)

- Previous Claim [2]: 'Q3 revenue up 15%... '
- Action: 'Create a summary table of regional sales
- Output: {{'label': 'Support', 'source': 2, 'type': null, 'confidence': 0.85, 'explanation':

'Formatting data for analysis is a valid step. '}} Case 4: Unsupport (Deviation)

- Query: 'Analyze 2008 financial crisis'
- Action: 'Research medieval banking regulations
- Output: {{'label': 'Unsupport', 'source': -1, 'type': 'deviation', 'confidence': 0.9,

'explanation': 'Irrelevant historical tangent unrelated to the 2008 crisis. Case 5: Support (Sequential Planning - NOT Premature)

- Query: 'Calculate temp trends'
- Previous Actions: [0] Fetch NOAA data, Download records
- Previous Claims: [0] 'Data not yet retrieved '
- Action: 'Run regression model on climate data
- Output: {{'label': 'Support', 'source': -1, 'type': null, 'confidence': 0.88, 'explanation':

'Valid next step assuming previous actions [0] and succeed in fetching data. ## OUTPUT FORMAT Return JSON ONLY: 'label': 'Support' | 'Unsupport', 'type': 'deviation' | 'redundancy' | null,

35


---

## Page 36

'confidence': 0.0-1.0, 'explanation': 'One sentence justification.

### F.4. Prompt for Root-cause Error Detection

We detail the two-stage workflow for identifying the root-cause error in a research trajectory: We detail the two-stage workflow for identifying the root-cause error in a research trajectory: (1) Trajectory Annotation. Before detection, we construct an annotated timeline of the agent's research trajectory to visualize potential failure points: •Atomic Hallucinations. All identified Claim Hallucinations marked at their corresponding steps. •Severe Noise Domination. To identify steps where the agent summarizes less relevant information such that valuable content is neglected, we leverage the local-level noise scores ( are continuous, we apply elbow clustering to isolate a set of candidate steps with anomalously high noise levels. To ensure these steps represent genuine information loss rather than benign filtering, we validate them via an LLM. Specifically, we prompt the LLM to estimate the impact of neglecting the highest-value ignored cluster (the Out-Memory cluster with the highest rankR) on the research outcome. Only steps with an estimated impact score *c*

from severe noise domination. (2) Root-Cause Error Detection. We provide the LLM with the fully annotated trajectory and the final answer. Adapting the detection prompt from (Zhu et al., 2025b), we instruct the model to analyze the logical chain of events and pinpoint the earliest error that served as the critical cause for the incorrect final answer.

#### F.4.1. PROMPT FOR INTERPRETING NEGLECT

You are an insight analyst reviewing retrieval chunks that were skipped in the final report. Each chunk may support hidden reasoning steps instead of answering the query directly. Infer subtle or implicit relationships between the chunk and the user query. Instructions:

1. Provide a one-sentence summary that highlights any signal relevant to the query or its

supporting sub-tasks (do not copy the chunk verbatim).

2. Provide a one-sentence explanation of the potential impact of omitting this chunk, even if

the impact is indirect or speculative (it' s acceptable to say the impact is negligible).

3. Output an impact score between 0 and 1 indicating how strongly the omission could hurt the
4. Avoid absolute or exclusive claims unless the chunk explicitly states them; qualify

statements with phrases like 'suggests', ' indicates', or 'one plausible candidate' when the evidence is indirect.

5. Mention remaining uncertainties or missing links when appropriate so the reader understands

the limits of the evidence.

6. Be concise and analytical; reason about latent connections or missed opportunities.

Query/Task: {query} Chunk Content: {chunk_text} Respond EXACTLY in the following format: Summary: <one sentence>

, Action Hallucinations, and Neglected Restrictions are

H ) defined in our main methodology. Since these scoresIS

*> 0.5are annotated as suffering*

Impact: <one sentence> ImpactScore: <float between 0 and 1>You are an insight analyst reviewing retrieval chunks that were skipped in the final report. Each chunk may support hidden reasoning steps instead of

36


---

## Page 37

answering the query directly. Infer subtle or implicit relationships between the chunk and the user query. Instructions:

1. Provide a one-sentence summary that highlights any signal relevant to the query or its

supporting sub-tasks (do not copy the chunk verbatim).

2. Provide a one-sentence explanation of the potential impact of omitting this chunk, even if

the impact is indirect or speculative (it' s acceptable to say the impact is negligible).

3. Output an impact score between 0 and 1 indicating how strongly the omission could hurt the
4. Avoid absolute or exclusive claims unless the chunk explicitly states them; qualify

statements with phrases like 'suggests', ' indicates', or 'one plausible candidate' when the evidence is indirect.

5. Mention remaining uncertainties or missing links when appropriate so the reader understands

the limits of the evidence.

6. Be concise and analytical; reason about latent connections or missed opportunities.

Query/Task: {query} Chunk Content: {chunk_text} Respond EXACTLY in the following format: Summary: <one sentence> Impact: <one sentence> ImpactScore: <float between 0 and 1>

#### F.4.2. PROMPT FOR ROOT-CAUSE ERROR DETECTION

query: {query} Scenario & Error Context: Scenario Background:

- Chain-of-Research trajectory: Each iteration contains planning actions (

observations/claims (`claim_list_N`), culminating in a final report.

- The full trajectory shows the complete research chain; the hallucination timeline shows errors

(hallucinated actions/claims verified as NotSupport), noise_domination (missed content with high possible impact), and potentially missed queries (unaddressed user intent) all of these are hallucinations.

- Only timeline entries are hallucinations; steps without timeline entries stayed on track.

FULL TRAJECTORY - Complete Chain of Research:

CRITICAL: Carefully examine observations for strategy shift signals like: ''shift strategy'', ''change approach'', '' start over'', ''complete shift'', ''need a new strategy'', ''pivot'', ''abandon previous approach If you see such signals after an error, that error was CORRECTED and is NOT the root cause. {full_research_trajectory}

`action_list_N`) and

FINAL REPORT - Research Results and Conclusions:

37


---

## Page 38

CRITICAL ANALYSIS INSTRUCTIONS: The report below shows what the agent ACTUALLY concluded. Use it to REVERSE-ENGINEER the root

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

{report} ================================================================================ HALLUCINATION TIMELINE - Errors Detected: ================================================================================ Compare the timeline below with the full trajectory and final report above:

- If an error led to a strategy shift (mentioned in trajectory), it is NOT the root cause
- If an error is not reflected in the final report, it was likely abandoned and is NOT the root

cause

- Only errors that directly led to the final incorrect conclusion are root causes
- Note: The timeline includes noise_domination (missed content) and query_missed entries when

applicable

{hallucination_timeline} Analysis Guidelines: CRITICAL: Do NOT be dominated by early hallucinations. An early hallucination that was later recognized and corrected by the agent is NOT the root cause.

1. Analysis process:
- FIRST: Understand the ENTIRE trajectory to see how the agent

errors were recognized/corrected

- THEN: Compare the hallucination timeline with the full trajectory to identify which errors

were critical

- Root cause = the earliest error that irreversibly doomed the final outcome (NOT corrected,

NOT led to successful pivot)

2. Root cause criteria:
- Must be an error that, if corrected, would have fundamentally changed the trajectory toward

success

- Must have STRONG LINKAGE between the error and the final wrong answer

's strategy evolved and when

- Early exploration errors (steps 1-3) are often normal learning steps - only flag if never

corrected

38


---

## Page 39

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

root-cause error TYPES: If there is a strong linkage between an error and the final failure, identify ONE of the following types as the root cause:

1. planning - Hallucinated planning actions that led to wrong search direction
2. observation - Hallucinated claims/observations that led to wrong conclusions
3. noise_domination - Critical content was retrieved but missed, directly causing failure
4. query_missed - Critical user intent/queries were not addressed, directly causing failure

If NO hallucinations have strong linkage to the final failure, set critical_step = -1 and critical_module = 'none'. REQUIRED OUTPUT FORMAT (JSON): ''critical_step'': <step_number or -1 if no strong linkage>, ''critical_module'': ''<module_name: planning|observation|noise_domination|query_missed> ''root_cause'': ''Concise description of the fundamental problem ''cascading_effects'': [{ ''step'': <step_number>,

Note: If no hallucinations have strong linkage to the final failure, set critical_step = -1.

 ''impact'': ''description'' }]

39
