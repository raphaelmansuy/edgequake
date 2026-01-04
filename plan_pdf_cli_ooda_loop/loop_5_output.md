# One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

### Abstract

1. Introduction

Locating the files and functions requiring modifi- (LLM)-based methods typically treat this as a repository-level retrieval task and rely on multiple auxiliary tools, which overlook code execution logic and complicate model control. We propose Repo Navigator, an LLM agent equipped with single execution-aware tool-jumping to the definition of a invoked symbol. This unified de- while simplifying tool manipulation. Repo Navi- Learning (RL) directly from a pretrained model, without any closed-source distillation. Experi- 7B model outperforming 14B baselines, the 14B model surpassing 32B competitors, and even the 32B model exceeding closed-source models such as Claude-3.7. These results confirm that integrat- RL training provides an efficient and scalable solution for repository-level issue localization.

*Figure 1. Illustration of a LLM navigating through a code reposi-*

tory. The LLM is equipped with a single yet powerful tool: jump , which is realized through a language server.

Zhaoxi Zhang 1 Yitong Duan2 Yanzhi Zhang 2 Yiming Xu 1 Jiyan He2 Yunfang Wu 1

1School of Computer Science, Peking University

2Zhongguancun Academy. Correspondence to: Yitong

Submitted to International Conference on Machine Learning

# arXiv:2512.20957v2 [cs.SE] 25 Dec 2025

With the rapid advancement of Large Language Models (LLMs) (Liu et al., 2024; Team, 2024; Yang et al., 2025a), equipping LLMs with pre-built tools to form LLM agents has become a common paradigm for expanding their ca- In the domain of software engineering (SWE), although LLM agents can effectively handle simple programming tasks (Hui et al., 2024; Guo et al., 2024a), their ability to operate on large-scale open-source software (OSS) reposito-

, 2026.

currently serves as the most comprehensive benchmark for evaluating whether LLMs can resolve real-world Git Hub is- (Jimenez et al., 2023) provides moderate gains, it remains far from enabling robust repository-level reasoning.

Most existing agents rely on test-time scaling applied di- Schmidgall et al., 2025). In software engineering (SWE) tasks, tool usage is essential rather than optional: real-world repositories are far larger than the context window of current LLMs, making it impossible to process an entire codebase in a single forward pass. Agents must therefore iteratively invoke tools to retrieve partial information from the repos- calls.

However, mainstream LLMs are rarely exposed to such agentic interaction patterns during pretraining and typically acquire tool usage only through few-shot prompting. Such in-context demonstrations are insufficient for learning com- limited context windows. Moreover, because tool definition spaces are effectively unbounded, pretrained models cannot fully internalize their semantics without post-training. To mitigate these issues, post-training paradigms such as Super- Learning with Verifiable Rewards (RLVR) (Yu et al., 2025a; Yue et al., 2025) have been applied, with promising results in domains including retrieval agents (Jin et al., 2025), GUI agents (Hong et al., 2024), and math agents (Yan et al., 2025).


---

2. Related Works

only precise evaluation method requires executing candi- each repository (Luo et al., 2025), which is prohibitively expensive. To make training more tractable, we adopt a simplified yet widely generalizable assignment: issue local- substantially easier to resolve once the relevant functions and files are correctly identified (Chen et al., 2025; Ma et al.,

2025; Xia et al., 2024; Jiang et al., 2025). Since modern OSS repositories contain a significant amount of code-far beyond any LLM's context window-localization drasti- solvability. Crucially, localization outputs a discrete set of paths, enabling verifiable, string-level evaluation that is when the tools are search engines (Jin et al., 2025), python compatible with scalable training frameworks such as SFT and RLVR.

Existing localization agents (Ma et al., 2025; Chen et al., 2025; He et al., 2025) typically rely on multiple tools, including Search Class,Search Methods Get Imports. Although effective to some extent, these tools considers high-level abstractions (classes, function, etc) of programing languages, which do not reflect how code actually executes. High-level abstractions, such as classes or inheritance, disappear after compilation, leav- modern LLMs already excel at modeling sequential depen- the repository-that is, to follow and inspect the source def-spository as a graph and applied graph-level searching tools inition of symbols as they appear in execution. To this end, we introduce a single, structurally grounded tool: jump which retrieves the precise definition of a given symbol. Details of this tool are provided in Sec. 3.3.

Our main contributions are threefold: (1) We propose the first repo-level localization agent trained on reinforcement learning directly from the pretrained model, regardless of distillation from a close-source model. (2) We design a repository-navigation agent that operates by performing realisticjump operations aligned with actual execution se- multi-tool pipelines.

format or failed parameter parsing. Thus, training a LLM to master new-defined tool is critical for LLM agents. In- more powerful LLM, and such trajectories can be used to train a student model via supervised finetuning (SFT) (Chen et al., 2025). However, this pipeline requires a stronger teacher model which has capability to master the tool. Re- required. Rejected-sampled finetuning (RFT) (Ahn et al.,

                        2024) utilizes generated trajectories of the agent itself via

multiple rollouts. Agentic RL (Jin et al., 2025) is an on- trajectories. Such training methods yield remarkable results

executer (Jimenez et al., 2023), calculator (Yan et al., 2025), and visual models (Gupta & Kembhavi, 2023).

### 2.2. Software Engineering Agents

, and The introduction of SWE-bench (Jimenez et al., 2023; Yang

et al., 2024b) has motivated a range of agentic pipelines for software engineering (SWE) tasks. Among them, SWE-

2025a) are widely adopted frameworks that equip agents with tools for interacting with computing environments. Workflow-based methods such as Agentless (Xia et al.,

                        2024) decompose issue resolution into localization, repair,

and validation subproblems. Chen et al. (2025) builds the re-

, grated commit history as agent memory. Repo Lens (Wang

et al., 2025b) equip conceptual information of the respos- are training-free, compatible with closed-source language models, and yield competitive results.

                        2025) and SWE-S W I S S (He et al., 2025) employ reinforce-

ment learning and achieve strong performance. However, end-to-end training remains costly because patch evaluation requires executing Docker environments across numerous repositories. Consequently, issue localization has emerged as a computationally efficient alternative, aiming to identify faulty components-at file or function level-rather than generating full patches.

                        2025) and C OSIL (Jiang et al., 2025), which model code-

bases as graphs and integrates them into LLMs, and

Directly training an agent to fix software issues, however,any tools, most tools are out-of-domain (OOD) for LLMs. remains difficult. A single bug often admits multiple valid Even for the most powerful models, failures often happen patches, making string-level evaluation unreliable. Thewhen calling the new-defined tools due to wrong calling

2024; Guo et al., 2024b). However, because most pretrainedthrough priority scheduling, action decomposition, and LLMs are trained on texts only and developers can definecontext pruning. From an open-source perspective, R E-

### 2.1. Agentic Training

LLM agents are promising methods to equip models with


---

3. Method

*Figure 2. Overview of our Repo Navigator. During the rollout phrase, the agent can call thejump tool, and the language server will return*

the definition code of the symbol. This process is trained by reinforcement learning.

and RL on the Qwen model family (Team, 2024), represents a notable advancement.

Nevertheless, prior agents overlook the structural relations within repositories-where modules, classes, and functions are cross-referenced across files-and typically rely on mul- reasoning step r , a tool call tiple search tools for symbol definition retrieval, amplifying o , forming a trajectory τ error propagation (see Sec. 3). In contrast, we employ a sin- nation, a final predictionY is scored by a reward R gle execution-logic-focused tool, reducing usage complexity. Finally, our approach constitutes the first localization agent trained directly from pretrained models, without relying on distillation-based supervised finetuning, a crucial stage in both Repo Searcher (Ma et al., 2025) and Loc Agent (Chen et al., 2025).

### 3.1. Problem Formulation

scription q, the goal is to output relevant code regions

*i i,j i,j*

span in filef . At each stept, the agent produces a optional*i*

*t* *a , and receives the observation**t*

The objective is max *θ* E *τ* ∼*π* [*R*(*τ* )].

### 3.2. Agent Architecture

Repo Navigator uses a single-tool design to avoid multi- decides whether to continue reasoning or to emit a JSON- file are parsed to the tool. The agent receives structured ob- reasoning until termination. The loop is reason *→act →* observe.

### 3.3. Jump: Symbol Resolution

Language servers resolve the definition of a Python symbol through a deterministic static analysis pipeline that approxi-

1

the detailed method. symbol occurrence s at source location ℓ, Pyright computes

a resolution mapping

R(*s, ℓ*) → {(*f* *i**, p)}**i**,* (1)

We present Repo Navigator, a reinforcement-learning agent for repository-level issue localization. The method con- definition of any symbols in a given file, (2) a reasoning- action agent loop that alternates between natural-language reasoning and tool invocation, and (3) a GRPO-based RL algorithm for optimizing long-horizon tool-augmented tra-


---

                        4. Experiment

we have multiple symbols with the same name exist in the same code snippet, we additionally parse an indexto the tool, which allows for accurate resolution of *ℓ*.

Syntactic Analysis In this process, the source file is parsed into an abstract syntax tree (AST). The syntactic role of s (e.g., name, attribute access, or call expression)where the first term is the standard policy gradient objective determines the subsequent resolution strategy. For attributewith an estimated advantage function A expressions a.b , Pyright treatsa as a receiver expressionactions that lead to higher-than-expected returns. The sec-

scaled by a coefficientβ, which acts as a trust region, pre-

Lexical Scope Resolution For a name symbol *x , candi-* venting the updated policy π date definitions are searched along a scope chain the previous policy π

and consistent policy improvement by balancing reward

S = {local*, enclosing, module, builtins*}, (2) maximization with behavioral consistency.

following Python's LEGB rule. Each scope maintains a symbol table mapping identifiers to defining AST nodes.

Static Type Inference . For attribute symbols, it com- expression a using type annotations, assignment flow analy- resolution is then defined as

resolve(*a.b*) = lookup(*b,* MRO(*t*))*,*

and S (

*t*∈*T* (*a*) *τ. We consider the tool-call to be failed when the format*

where MRO(*t)* denotes the method resolution order of typeis incorrect, or the symbol parsed does not exist, or for any *t.* other reason that causes the tool to quit unexpectedly.

Import Dependency Graph For cross-file resolution, im- module loading semantics is built. Import statements intro- of target modules, including re-exports andall -based filtering. Resolution may therefore traverse multiple mod-

are zero. For validation, we test our method on SWE-bench-

3.4. Reasoning-Action Loop verified (Jimenez et al., 2023), which is a human-verified

Given history h= (*q, o, a*), the agent samplessubset of SWE-bench. We additionally test our method on

*t t*− *t*−

either a natural-language reasoning step r ∼ *π (·|h*) or a a subset of SWE-bench-pro (Yang et al., 2025b) (which structured tool calla ∼ *π ( ·|h*). Tool calls must satisfyis a new and more difficult benchmark) for generalization. a JSON grammar enforced via constrained decoding. The loop continues until the agent outputs its final localizationgolden patches. All datasets are open-source and are built *Y .*ˆ on real-world github issues.

*θold t t*

ˆ, which promotes

*θ* from moving too far from

*θold*. This formulation ensures stable

The reward of GRPO process is calculated as:

Dice is a common metric for set-level comparison, for set *Y* and set *Y*ˆ ∗

*τ* ) is the success rate of tool-calling extracted from

where each pair ( f , p ) denotes a file path and a source*i i* Reference Policy Optimization (GRPO), which has the loss position corresponding to a valid definition site of s . Infunction: practice, we use filepath and symbol to resolve ℓ. If

2

1:1 1:1

### 3.5. Reinforcement Learning

                        2025) applied recall and precision as metrics. However,

We apply reinforcement learning with verifiable rewardsbecause the predicted locations and ground-truth locations to train the agent directly from the pretrained model, withare sets of strings, recall and precision singularly can not no teacher model required. In practice, we apply Groupreflect the performance fairly. Thus, we utilize Sample-F1

### 4.1. Experimnent Setup

Datasets We extract valid samples from SWE-smith (Yang et al., 2025b) to form the training set. We apply Qwen2.5-7B-Instruct with Repo Navigator to sample each

For ground-truth locations, we directly use the locations in

Metrics Previous works (Chen et al., 2025; Ma et al.,


---

Repo Navigator trained with GRPO.

Function-level File-level

Agent Pipeline Model

Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU

Close-source Models

Repo Searcher Claude3.7-Sonnet 66.80 19.90 28.30 17.89 89.71 21.04 33.15 20.67

Repo Navigator Claude3.7-Sonnet 31.03 34.43 31.72 30.22 72.26 75.95 73.01 71.37 Repo Navigator GPT5-chat 30.42 34.56 31.17 29.67 58.17 61.87 58.88 57.33 Repo Navigator Claude4.5-Sonnet 43.97 45.76 43.62 41.31 80.68 81.92 79.94 77.49

Qwen2.5-7B

Locagent Training Free 17.62 11.71 12.71 10.31 60.96 34.88 40.67 33.33

CoSIL Training Free 29.30 8.98 12.90 8.07 70.12

Agentless Training Free 24.92 12.93 15.31 11.74 63.01 19.32 27.82 18.85 Orcaloca Training Free 27.70 20.29

Repo Searcher Distillation+GRPO 63.26 19.24 27.37 17.59 84.11 19.97 31.64 19.57

Repo Navigator Training Free 15.89 17.46 Repo Navigator GRPO 26.69 30.34

Qwen2.5-14B

Locagent Training Free 35.62 13.32 17.71 12.32 71.42 31.66 40.77 30.64

CoSIL Training Free 48.61 13.40 19.81 12.12 78.35

Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88 19.30 Orcaloca Training Free 29.92 20.98 22.77 18.92 52.17 52.15 50.93 48.72

Repo Searcher Training Free 26.13 11.96 14.35 10.60 74.77 18.80 28.79 18.15

Repo Navigator Training Free 27.96 25.77 Repo Navigator GRPO 31.02 30.08

Qwen2.5-32B

Locagent Training Free 46.79 16.29 21.48 14.18 79.39 34.18 44.18 33.24

CoSIL Training Free 55.38 14.85 22.11 13.52 83.50 19.34 30.77 18.93

Agentless Training Free 40.79 24.07 27.33 22.08 78.93 25.60 35.38 24.96 Orcaloca Training Free 39.14 25.59

Repo Searcher Distillation+GRPO 69.50 20.29 29.11 18.23 89.33

Repo Navigator Training Free 28.11 28.19 Repo Navigator GRPO 33.71 37.19

17.90 27.39 17.42

21.70 17.92 48.04 48.65 47.36 45.77

16.19 15.46 42.36 43.23 42.12 40.97 27.49 26.43 50.62 53.83 51.63 50.62

18.10 28.79 17.72

25.58 23.00 59.00 56.68 56.39 53.74 29.23 26.84 61.60 58.97 58.90 56.36

*Table 1. Comparison of different agent pipelines on function-level and file-level Dice/IoU metrics. We use Qwen2.5-Instruct series as*

our base model. Bold numbers denote the best performance among same-size models; underline numbers denote the best training-

train it with 16 Tesla-A100-80G GPUs. We apply verl4.2. Effectiveness (Shen, 2024) as the training framework, and we apply vLLM

Baselines We compare our method against Locagent

(Kwon et al., 2023) as the inference engine. We train the (Chen et al., 2025), CoSIL (Jiang et al., 2025), Agent-

(which is the averaged score of per-sample F1 values) andto 128 on 4k training samples filtered from SWE-smith, IoU (intersection out of union) as our core metrics. At thewith maximum prompt length and max response length same time, we also present the recall and precision scoresboth set to 10240. Additionally, we rollout 8 times for to align with previous methods, although they do not reflecteach sample, and the temperature is set to 1.0 to encourage the methods' performance fairly. exploration. We use greedy decoding in the inference stage

to ensure stable performance. More implementation details

Training For the 7B model, we conduct GRPO with 8 are provided in Appendix. B. Tesla-A100-80G GPUs. For the 14B and 32B model, we

28.72 22.89 59.57 59.51 58.11 55.62

20.27 32.93 20.35

27.12 25.16 63.05 62.75 61.67 59.28 34.09 32.30 67.29 70.76 67.75 65.75


---

GRPO.

Function-level File-level

Agent Pipeline Model

Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU

Qwen2.5-7B

Loc Agent Training Free 1.01 0.02 0.65 0.40 12.16 0.17 10.81 8.93

CoSIL Training Free 8.64 3.33 4.58 2.87 26.64 8.47 12.11 7.70

Agentless Training Free 12.82 6.94 8.05 5.73 39.41

Repo Searcher Training Free 1.07 0.93 0.97 0.86 4.91 1.64 2.30 1.63

Repo Navigator Training Free 9.84 14.65 Repo Navigator GRPO 12.33 21.26

Qwen2.5-14B

Loc Agent Training Free 6.22 0.13 3.65 2.65 15.58 0.21 11.69 9.53

CoSIL Training Free 10.73 4.67 5.96 3.94 34.31 9.97 14.81 9.30

Agentless Training Free 10.49 6.75 7.41 5.28 41.42 13.42 19.02 12.37

Repo Searcher Training Free 2.79 1.38 1.69 1.14 17.37 5.17 7.60 4.84

Repo Navigator Training Free 14.36 19.74 Repo Navigator GRPO 16.05 25.25

Qwen2.5-32B

Loc Agent Training Free 8.72 0.17 4.30 2.90 25.73 0.38 19.77 16.50

CoSIL Training Free 15.00 6.35 8.14 5.21 45.37 13.04 19.42 12.36

Agentless Training Free 11.08 7.31 7.98 5.80 43.07 13.89 20.07 13.11

Repo Searcher Training Free 2.00 1.29 1.45 1.00 13.51 3.43 5.31 3.24

Repo Navigator Training Free 13.96 20.25 Repo Navigator GRPO 18.13 29.44

baseline methods are presented in Appendix. A.

13.15 18.89 12.35

10.67 9.20 30.50 37.24 31.86 28.82 14.29 12.02 36.36 48.13 39.74 36.36

*Table 2. Comparison of different agent pipelines on function-level and file-level metrics on SWE-bench Pro for generalization. Bold*

| numbers | denote | the | best | performance | among | same-size | models; | underline | numbers | denote | the | best | training-free | performance | among |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 15.27 12.00 43.57 54.52 46.06 | 41.07 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 18.06 14.58 46.85 58.64 49.72 | 45.14 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 15.36 12.87 50.24 63.24 53.48 | 48.50 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 20.72 17.16 53.49 68.69 57.57 | 52.44 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |

(S-F1 and IoU) for both function-level and file-level local-

14B model surpasses 32B baselines on S-F1 and IoU. This

Although some baselines have higher recall score signifi-

SOTA among all training-free methods. This implies that


---

                        5. Discussion: Building Less yet More Capable

## Tools

*Figure 4. Scaling law of tool-calling, where Pre and Post denote*

the corresponding metric before and after the RL training.

Loc Agent 2.65 13.01

Repo Navigator 12.00 14.74

Repo Navigator+RL 14.58 15.03

*Table 3. We use Qwen2.5-14B-Instruct as the localization model,*

and use Qwen2.5-32B-Instruct as the repair model on SWE-

### 4.4. Scaling Law of Tool-Calling

To assess the significance of tool-calling in Repo Navigator, we varied the maximum number of tool-calling turns and reported the results in Fig. 4.2. As shown in the figure, allow- performance for Repo Navigator, both before and after re- results empirically validate the scaling law of tool-calling

### 4.5. Influence on Issue Resolution

To evaluate the impact of different localization results on the final issue resolution performance, we test Repo Naviga- apply the repairing phrase of Agentless while replacing its localization front-end with other methods. Table.3 illus- has the highest performance on issue resolution, while rein-

Agent Pipeline Func-IoU(%) Resolved(%)

Agentless 5.28 10.12

hybrid reward (with tool-calling success rate) has highersince additional tools often introduce new and unfamiliar performance than pure outcome reward (without tool-callinginterfaces that large language models have not been exposed success rate). This indicates that learning to correctly call to during pretraining, potentially increasing the likelihood tools is vital in agentic learning. of errors.

GRPO, trained Repo Navigator outperforms it on all metri- method outperforms Repo Searcher for 14B models. This is probably due to the simplified tool we integrate to the agent in this context. (see Sec. 5 for more details).

To assess the generalizability of Repo Navigator, we present its performance on Python samples from the SWE-bench- Pro dataset (Yang et al., 2025b) in Table 2. The results on this dataset are consistent with those observed on SWE- influence of data leakage in SWE-bench Verified, we can make a stronger claim regarding SWE-bench Pro, as it was released after the publication of the Qwen2.5 series.

### 4.3. Training Strategy Comparison

To explore the capability of GRPO on agentic training, we compare GRPO against RFT-only and RFT+GRPO. As pre- RFT-only and RFT+GRPO. Moreover, although RFT has ac- improvement GRPO makes after the cold start. This conclu- (Ma et al., 2025), however, it aligns with the broader field of reinforcement learning, where RFT and SFT (as a cold start) is effective only when the pretrained model is not strong enough (Guo et al., 2024a). When the pretrained model is strong enough and data is high-quality, directly training a model with RL is better than training after SFT (RFT) as its cold start.

We also remove the success rate in the reward function for ablation. As presented in Fig. 3, reinforcement learning with ple tools are available. This reduction is generally beneficial,

In this section, we analyze the logic behind Repo Naviga- task-specific tools.

### 5.1. Impact on the Action Space of Agents

Let the total number of available tools be denoted as*k .* When only a single tool-specifically thejump tool-is re- both the action space and the observation space are restricted to what this tool can access. In this case, the set of possible actions and observable elements is smaller than when multi-


---

                        6. Conclusion

*Figure 5. Venn graph illustrating access scope ofjump . Compared*

with the repository scope, the access scope has a much higher IoU with the groundtruth set.

✓ ✓ ✓ ✗ 21.44 ✓ ✗ ✗ ✓ 24.00 ✓ ✗ ✗ ✗ 24.28

*Table 4. We change the tool set of Repo Navigator and present*

the function-level IoU (%) on Qwen2.5-7B-Instruct. Apparently, excessive tools do not boost Repo Navigator's performance.

mantically activated by that entry point. Because every location that contributes to the issue must lie on some de- expansion. Therefore, the final access scope produced by exhaustive jumptraversal is guaranteed to contain all loca-

### 5.4. Verification

Repo Navigator and conduct RL training with only the out- used in previous works (Chen et al., 2025; Ma et al., 2025; Jiang et al., 2025) and present the result in Table. 4. Get- Class/Get Func takes a class/function name as input and outputs the class/function definition. Get Struc takes no in- implies that additional tools do not increase model's perfor- capable tools.

Jump Get Class Get Func Get Struc IoU

✓ ✓ ✓ ✓ 13.71

scope equal to the whole repository scope.

jointly optimized with reinforcement learning, can provide

When we start from the entry point and repeatedly applystronger robustness and more reliable multi-step reason- symbol-we effectively traverse all symbols that are se-scoped tools.

### 5.2. Impact on Tool-Calling Success Rate

For a given process in issue localization (for instance, check- ity of thei-th call be*p . For a task that requires**i* *k sequential* tool invocations, the overall success rate can be expressed

Since each step introduces an additional potential point of failure, the cumulative success rate typically decreases as the number of required tool calls increases. Therefore, in general, completing a task with a single, more versatile tool tends to be more reliable than relying on multiple narrow-

### 5.3. Impact on the Prediction Space

The access scope of a tool is defined as the complete set of files, symbols, and other resources that the tool can access within a repository. For ajumptool that navigates to sym- from a given entry point and recursively resolving all ref- Apparently, its access scope is significantly smaller than the closed-loop manner, enabling end-to-end optimization with- Intersection over Union (IoU) between the prediction set and the groundtruth set, using the jumptool results in a higher IoU, as depicted in Fig. 5. On the other hand, ap-

In this work, we introduced Repo Navigator, a repository- multi-tool paradigms by leveraging a single, more-capable jump tool for symbol resolution. This unified design faith-

chaining. Through tool-integrated GRPO, Repo Navigator learns to reason, invoke tools, and refine its predictions in a

out relying on closed-source teacher models or distillation.

Extensive experiments across SWE-bench-Verified and SWE-bench-Pro demonstrate that Repo Navigator achieves state-of-the-art localization performance. We theoretically analyze the results, confirming that a single powerful tool,


---

Our findings highlight the importance of aligning agent tool- References ing with real execution structure, and show that efficient reasoning-tool co-training can unlock substantial gains even for medium-sized open-source models. Future work will explore extending Repo Navigator from Python to more pro-

Ahn, J., Verma, R., Lou, R., Liu, D., Zhang, R., and Yin, W.

Large language models for mathematical reasoning: Pro- 2024.

Anthropic. Claude 3.7 sonnet and claude code.

https://www.anthropic.com/news/ claude-3-7-sonnet, February 2025. data: 2025-11-18.

Chen, Z., Tang, R., Deng, G., Wu, F., Wu, J., Jiang, Z.,

Prasanna, V., Cohan, A., and Wang, X. Loc Agent: Graph- Nabende, J., Shutova, E., and Pilehvar, M. T. (eds.), Pro- for Computational Linguistics (Volume 1: Long Papers), pp. 8697-8727, Vienna, Austria, July 2025. Association for Computational Linguistics. ISBN 979-8-89176-251-

                        0. doi: 10.18653/v1/2025.acl-long.426. URLhttps:

//aclanthology.org/2025.acl-long.426/

Guo, D., Zhu, Q., Yang, D., Xie, Z., Dong, K.,

Zhang, W., Chen, G., Bi, X., Wu, Y., Li, Y., et al. Deepseek-coder: When the large language model meets programming-the rise of code intelligence. arXiv preprint arXiv:2401.14196, 2024a.

Guo, T., Chen, X., Wang, Y., Chang, R., Pei, S., Chawla,

N. V., Wiest, O., and Zhang, X. Large language model based multi-agents: A survey of progress and challenges. arXiv preprint arXiv:2402.01680, 2024b.

Gupta, T. and Kembhavi, A. Visual programming: Compo-

of the IEEE/CVF conference on computer vision and pat-

He, Z., Yang, Q., Sheng, W., Zhong, X., Zhang, K., An, C.,

Shi, W., Cai, T., He, D., Chen, J., and Xu, J. Swe-swiss: A multi-task fine-tuning and rl recipe for high-performance issue resolution. https://github.com/zhenyuhe00/SWE- Swiss, 2025. Notion Blog.

Hong, W., Wang, W., Lv, Q., Xu, J., Yu, W., Ji, J., Wang, Y.,

Wang, Z., Dong, Y., Ding, M., et al. Cogagent: A visual language model for gui agents. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition, pp. 14281-14290, 2024.

Huang, X., Liu, W., Chen, X., Wang, X., Wang, H., Lian,

D., Wang, Y., Tang, R., and Chen, E. Understanding the planning of llm agents: A survey. arXiv preprint arXiv:2402.02716, 2024.

Hui, B., Yang, J., Cui, Z., Yang, J., Liu, D., Zhang, L.,

Liu, T., Zhang, J., Yu, B., Lu, K., et al. Qwen2. 5-coder technical report. arXiv preprint arXiv:2409.12186, 2024.


---

arXiv:2503.22424, 2025.

Jimenez, C. E., Yang, J., Wettig, A., Yao, S., Pei, K., Press,

O., and Narasimhan, K. Swe-bench: Can language mod- arXiv:2310.06770, 2023.

Jin, B., Zeng, H., Yue, Z., Yoon, J., Arik, S., Wang, D.,

Zamani, H., and Han, J. Search-r1: Training llms to reason and leverage search engines with reinforcement learning. arXiv preprint arXiv:2503.09516, 2025.

Kwon, W., Li, Z., Zhuang, S., Sheng, Y., Zheng, L., Yu,

C. H., Gonzalez, J. E., Zhang, H., and Stoica, I. Efficient memory management for large language model serving with pagedattention. In Proceedings of the ACM SIGOPS 29th Symposium on Operating Systems Principles, 2023.

Langley, P. Crafting papers on machine learning. In Langley,

P. (ed.), on Machine Learning (ICML 2000), pp. 1207-1216, Stan-

Li, Y., Wen, H., Wang, W., Li, X., Yuan, Y., Liu, G., Liu,

J., Xu, W., Wang, X., Sun, Y., et al. Personal llm agents: Insights and survey about the capability, efficiency and security. arXiv preprint arXiv:2401.05459, 2024.

Liu, A., Feng, B., Xue, B., Wang, B., Wu, B., Lu, C., Zhao,

C., Deng, C., Zhang, C., Ruan, C., et al. Deepseek-v3 technical report. arXiv preprint arXiv:2412.19437, 2024.

Liu, Z., Zhang, Y., Li, P., Liu, Y., and Yang, D. Dy-

framework with agent team optimization. arXiv preprint arXiv:2310.02170, 2023.

Lu, J., Holleis, T., Zhang, Y., Aumayer, B., Nan, F., Bai,

F., Ma, S., Ma, S., Li, M., Yin, G., et al. Toolsand- benchmark for llm tool use capabilities. arXiv preprint arXiv:2408.04682, 2024.

Luo, M., Jain, N., Singh, J., Tan, S., Patel, A., Wu, Q.,

Ariyak, A., Cai, C., Tarun Venkat, S. Z., Athiwaratkun, B., Roongta, M., Zhang, C., Li, L. E., Popa, R. A., Sen, K., and Stoica, I. Deepswe: Training a state- https://pretty-radio-b75.notion.site/ DeepSWE-Training-a-Fully-Open-sourced-State-of-the-Art-Coding-Agent-by-Scaling-RL-22281902c1468193aabbe9a8c59bbe33

2025. Notion Blog.

preprint arXiv:2501.04227, 2025.

Shen, Z. Llm with tools: A survey. arXiv preprint

arXiv:2409.18807, 2024.

Team, Q. Qwen2 technical report. arXiv preprint

arXiv:2407.10671, 2024.

Wang, X., Li, B., Song, Y., Xu, F. F., Tang, X., Zhuge,

M., Pan, J., Song, Y., Li, B., Singh, J., Tran, H. H., Li, F., Ma, R., Zheng, M., Qian, B., Shao, Y., Muen- Peng, H., Ji, H., and Neubig, G. Openhands: An open platform for AI software developers as general- on Learning Representations, 2025a. URL https: //openreview.net/forum?id=OJd3ayDDoF

Wang, Y., Mao, W., Wang, C., Zhou, Z., Zhou, Y., Zhao, W.,

Lou, Y., and Peng, X. Extracting conceptual knowledge to locate software issues. arXiv preprint arXiv:2509.21427, 2025b.

Xia, C. S., Deng, Y., Dunn, S., and Zhang, L. Agentless: De-

preprint arXiv:2407.01489, 2024.

Yan, Y., Wang, S., Huo, J., Yu, P. S., Hu, X., and Wen, Q.

Mathagent: Leveraging a mixture-of-math-agent frame-

Yang, A., Li, A., Yang, B., Zhang, B., Hui, B., Zheng, B.,

Yu, B., Gao, C., Huang, C., Lv, C., et al. Qwen3 technical report. arXiv preprint arXiv:2505.09388, 2025a.

Yang, J., Jimenez, C. E., Wettig, A., Lieret, K., Yao, S.,

Narasimhan, K. R., and Press, O. SWE-agent: Agent- Neural Information Processing Systems, 2024a. URL https://arxiv.org/abs/2405.15793

Yang, J., Jimenez, C. E., Zhang, A. L., Lieret, K., Yang,

J., Wu, X., Press, O., Muennighoff, N., Synnaeve, G., Narasimhan, K. R., et al. Swe-bench multimodal: Do ai systems generalize to visual software domains? arXiv, preprint arXiv:2410.03859, 2024b.

Jiang, Z., Ren, X., Yan, M., Jiang, W., Li, Y., and Schmidgall, S., Su, Y., Wang, Z., Sun, X., Wu, J., Yu, X.,

Liu, Z. Cosil: Software issue localization via llm-Liu, J., Moor, M., Liu, Z., and Barsoum, E. Agent lab-

 The Thirteenth International Conference

 Proceedings of the 17th International Conference

 The Thirty-eighth Annual Conference on

Ma, Z., Peng, C., Zeng, Q., Gao, P., Zou, Y., and Xie,Yang, J., Lieret, K., Jimenez, C. E., Wettig, A., Khandpur,

B. Tool-integrated reinforcement learning for repo deepK., Zhang, Y., Hui, B., Press, O., Schmidt, L., and Yang, search, 2025. URL https://arxiv.org/abs/ D. Swe-smith: Scaling data for software engineering

2508.03012 agents. arXiv preprint arXiv:2504.21798, 2025b.


---

## A. Detailed Illustration of Baselines

## B. Experimental Details

arXiv:2503.14476, 2025a.

repository. Second, relevant classes and functions are de-

Yu, Z., Zhang, H., Zhao, Y., Huang, H., Yao, M., Ding,tected. Third, precise locations for edit are given by LLMs

K., and Zhao, J. Orcaloca: An llm agent frameworkbased on the classes and functions. for software issue localization, 2025b. URLhttps: //arxiv.org/abs/2502.00350

conduct file-level localization and then conduct function-

Yuan, S., Song, K., Chen, J., Tan, X., Shen, Y., Kan, R.,

level localization. CoSIL dynamically constructs call graphs

Li, D., and Yang, D. Easytool: Enhancing llm-based

of modules (class, functions) during the repo-level searching

agents with concise tool instruction. arXiv preprint

process, and applies context pruning to effectively reduce

arXiv:2401.06201, 2024.

the searching scope.

Yue, Y., Yuan, Y., Yu, Q., Zuo, X., Zhu, R., Xu, W., Chen,

J., Wang, C., Fan, T., Du, Z., et al. Vapo: Efficient and reliable reinforcement learning for advanced reasoningautomatic LLM agent besides its planning prompt concate-

process. It builds the whole repository into a direct hetero-

ports and invocations. Multiple graph-level searching tools are equipped to the LLM for multi-hop reasoning.

that first conducts file-level localization and then function- introduced the first training framework calization agents, which is composed of distilling from a close-source model (Claude3.7-Sonnet in Repo Seacher) as warmup and reinforcement learning to further enhance the performance.

automatic LLM agent, with no fixed workflow and no plan- from pretrained open-source LLMs without a close-source teacher model. Lastly, we only integrate a single yet power- narrows the access scope of the agent.

 CoSIL (Jiang et al., 2025) is an agent which first

 Loc Agent (Chen et al., 2025) is almost a fully-

Additionally, edges are built by dependencies such as im-

 Repo Searcher (Ma et al., 2025) is an agent

 Tool Train for lo-

Ours Compared with all baselines, we are the first fully-

Yu, Q., Zhang, Z., Zhu, R., Yuan, Y., Zuo, X., Yue, Y., Dai,

W., Fan, T., Liu, G., Liu, L., et al. Dapo: An open-source llm reinforcement learning system at scale. arXiv preprint Agentless (Xia et al., 2024) is a workflow for

issue localization. First, it identifies suspicious files in the

10

tions *Y* ∗, the aforementioned metrics are calculated as:

|Y ∩ *Y*ˆ ∗|

Recall = (7)

|*Y* ∗|

clip ratiohigh to 0.8, learning rate to ing batch size to 128,training temperature to 1.0, maximum tool-calling times to 12, and max response length to 10240.

level or function-level)Y , and the set of groundtruth loca-

Hyperparameters We set clip ratio low to 0.2,

−6, train-

Metrics Given the set of predicted locations (ether file-


---

| ✗ |  | ✗ |  | 25.11 | 29.16 |  |  |  |
|---|---|---|---|---|---|---|---|---|
|  | ˆ |Y  ∩  Y |  | | |  | repository—and dynamic imports can degrade the perfor- |  |  |  |
| Precision  = | 2  × |Y | |Y | ˆ |Y  | ˆ  ∩  Y |  | | | (8) | (8) | mance of the language server, as its functionality relies on static analysis techniques such as abstract syntax trees and symbol tables. When such circumstances occur, the tool |
| ˆ |Y  ∩  Y | ˆ |Y  |  +  | Y | | | | | (9) | (9) | returns an error message indicating that the definition of the current symbol cannot be located due to unknown reasons. Nevertheless, in our empirical evaluation, we did not ob- |  |  |

## C. Threats to Validity

## D. Case Study

✓ ✗ ✗ ✓ 24.64 27.48 25.05 24.00 53.48 55.76 53.68 52.69 ✓ ✗ ✗ ✗

*Table 5. We change the tool set of Repo Navigator and present the function-level IoU. Because the jump*

| for | localization, | excessive | tools | do | not | increase | its | performance. |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 25.11 29.16 25.75 24.28 55.81 58.71 56.32 | 54.89 |  |  |  |  |  |  |  |
| ✓ ✓ ✓ ✓ 14.28 15.44 14.40 13.71 35.78 36.76 35.59 | 34.55 |  |  |  |  |  |  |  |
| ✓ ✓ ✓ ✗ 22.60 25.02 22.80 21.44 48.49 50.13 48.52 | 47.17 |  |  |  |  |  |  |  |

to an invoked symbol within a repository. However, thecific step-by-step workflow in its system prompt), CoSIL presence of monkey patches-runtime modifications to theand Repo Searcher (which is half-automatic because some

|Y ∪ *Y*ˆ ∗|

In practice, when the prediction set*Y is empty (for instance,*ˆ total failure), we set recall, precision, sample-F1, and IoU to zero. We use the function-level localization result of different methods and apply the patch generation backend in Agentless (Xia et al., 2024) to generate patches. Re- test units after applying the patch.

Implementation When the response exceeds the maxi- zero as its score. When the agent exceeds the maximum tool-calling times (which is 12), we add "You must not call tools anymore, and you must give the final answer" to the tool's response. Most of the time, the agent will stop calling tools and generate the final response. If not, we force it to stop and give zero as its score. Note that when the maxi- is generated, the agent loop will stop automatically. The aforementioned process is an automatic agentic framework, which allows the agent to explore in the environments with little constraints.

Preventing Data Leakage It is a widespread concern that data leakage at the pre-training phrase threatens the validity of post-training methods. Nevertheless, we exclude this concern by results in Tabel. 2. The SWE-bench dataset was published in 2025, while the Qwen2.5 series were published in 2024. Moreover, we exclude the samples in the training dataset if the repository also appears in SWE- (Shen, 2024) and present an example. Noted, we do not bench Verified or SWE-bench Pro.

(10)

serve any instances of monkey patching or dynamic imports within the analyzed datasets.

Groundtruth Retrieval A limitation of our work lies in the extraction of groundtruth locations. We extract modified locations directly from the goldpatch in the datasets, which may ignore other patches that also resolve the issue. Our evaluation metrics do not take these correct alternatives into consideration. However, using golden patches is ac- reveals golden locations (locations in golden patches), it undoubtedly contributes to the resolution of the issue, and the result in Table. 3 demonstrates this claim.

Language Limit Another limitation is that we only evalu- each language (C/C++, Java, etc.) has its unique language server, and we only succeed in implementing the language server of python. We will implement more language servers and validate our approach on more programing languages in the future.

Pro In this section, we present the full trajectory of Repo Navi-

We apply the default tool-calling prompt template of verl

present any process restrictions in our prompt, encourag-


---

multi-turns tool-calling conversations).

forced steps are added to the workflow besides the automatic


---

'file_where_the_symbol_is_used'}}

{'name': 'check', 'arguments': {'symbol': 'symbol_to_be_checked', 'file_path':

For instance:

You can only call the tool once each turn.

Please put your final answer inside \boxed{} only in the last turn.

...(a series of file::function pairs seperated by comma)

relevant/path/to/file1.py::func_name1,relevant/path/to/file2.py::func_name2,

Your final answer should be all functions that should be modified, such as:

[Entry Point]

[Relevant Path To Entry Point]

The entry file of the code base is:

[Problem Statement]

This is the issue:

in fileA.py), you should directly check 'functionB' in 'fileA.py'.

For instance, if 'classA.functionB' is what you want to check (which is called

NOT where it is defined!

The 'file_path' is the relevant path of where the symbol is called,

check the symbol once for each turn.

You can call the tool to check the definition code of a symbol. You can only

functions causing this issue.

You are given a codebase and an issue, you need to locate the files and

[user]

</tool_call>

{"name": <function-name>, "arguments": <args-json-object>}

<tool_call>

within <tool_call></tool_call> XML tags:

For each function call, return a json object with function name and arguments

</tools>

"type": "string"}}, "required": ["symbol", "file_path"], "type": "object"}}}

{"description": "The relevant path to the file where the symbol is referred.",

definition code will be given to the agent.", "type": "string"}, "file_path":

"parameters": {"properties": {"symbol": {"description": "The symbol whose

For instance, in the first turn, file_path is the entry point of.",

where the tool is defined.

specific file path, a symbol is referred and this tool can find

{"type": "function", "function": {"name": "check", "description": "In the

<tools>

You are provided with function signatures within <tools></tools> XML tags:

You may call one or more functions to assist with the user query.

# Tools

You are Qwen, created by Alibaba Cloud. You are a helpful assistant.

[system]

Prompt
