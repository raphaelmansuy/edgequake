## Page 1

### One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

Zhaoxi Zhang Yitong Duan Yanzhi Zhang

## Abstract

Locating the files and functions requiring modification in large open-source software (OSS) repositories is challenging due to their scale and structural complexity. Existing large language model (LLM)-based methods typically treat this as a repository-level retrieval task and rely on multiple auxiliary tools, which overlook code execution logic and complicate model control. We propose Repo Navigator, an LLM agent equipped with a single execution-aware tool-jumping to the definition of a invoked symbol. This unified design reflects the actual flow of code execution while simplifying tool manipulation. Repo Navigator is trained end-to-end via Reinforcement Learning (RL) directly from a pretrained model, without any closed-source distillation. Experiments demonstrate that RL-trained Repo Navigator achieves state-of-the-art performance, with the 7B model outperforming 14B baselines, the 14B model surpassing 32B competitors, and even the 32B model exceeding closed-source models such as Claude-3.7. These results confirm that integrating a single, structurally grounded tool with RL training provides an efficient and scalable solution for repository-level issue localization.

Yiming Xu Jiyan He Yunfang Wu

1 2 2 1 2 1

vised Finetuning (SFT) (Ma et al., 2025) and Reinforcement

1School of Computer Science, Peking University

Learning with Verifiable Rewards (RLVR) (Yu et al., 2025a;

2Zhongguancun Academy. Correspondence to: Yitong

Yue et al., 2025) have been applied, with promising results

Duan<duanyitong@zgci.ac.cn>, Yunfang Wu<wuyf@pku.edu. cn>. in domains including retrieval agents (Jin et al., 2025), GUI

agents (Hong et al., 2024), and math agents (Yan et al.,

Submitted to International Conference on Machine Learning, 2026. 2025).

1

# arXiv:2512.20957v2 [cs.SE] 25 Dec 2025

1. Introduction

With the rapid advancement of Large Language Models (LLMs) (Liu et al., 2024; Team, 2024; Yang et al., 2025a), equipping LLMs with pre-built tools to form LLM agents has become a common paradigm for expanding their capabilities (Shen, 2024; Yuan et al., 2024; Lu et al., 2024). In the domain of software engineering (SWE), although LLM agents can effectively handle simple programming tasks (Hui et al., 2024; Guo et al., 2024a), their ability to

operate on large-scale open-source software (OSS) repositories remains limited. SWE-BENCH (Jimenez et al., 2023)

*Figure 1.Illustration of a LLM navigating through a code reposi-*

tory. The LLM is equipped with a single yet powerful tool:jump, which is realized through a language server. currently serves as the most comprehensive benchmark for evaluating whether LLMs can resolve real-world Git Hub isitory directly due to context limits. While SWE-AGENT sues. All pretrained LLMs can not process the whole reposrectly to pretrained LLMs (Liu et al., 2023; Chen et al., 2025;

(Jimenez et al., 2023) provides moderate gains, it remains far from enabling robust repository-level reasoning. Most existing agents rely on test-time scaling applied diin a single forward pass. Agents must therefore iteratively

Schmidgall et al., 2025). In software engineering (SWE) tasks, tool usage is essential rather than optional: real-world repositories are far larger than the context window of current LLMs, making it impossible to process an entire codebase invoke tools to retrieve partial information from the repository and interleave natural-language reasoning with tool calls. However, mainstream LLMs are rarely exposed to such agentic interaction patterns during pretraining and typically acquire tool usage only through few-shot prompting. Such in-context demonstrations are insufficient for learning complex multi-step tool-chaining behaviors, especially under limited context windows. Moreover, because tool definition spaces are effectively unbounded, pretrained models cannot fully internalize their semantics without post-training. To mitigate these issues, post-training paradigms such as Super-


---

## Page 2

Directly training an agent to fix software issues, however, remains difficult. A single bug often admits multiple valid patches, making string-level evaluation unreliable. The only precise evaluation method requires executing candidate patches inside a dedicated Docker environment for each repository (Luo et al., 2025), which is prohibitively expensive. To make training more tractable, we adopt a simplified yet widely generalizable assignment: issue localization. Prior work shows that a software issue becomes substantially easier to resolve once the relevant functions and files are correctly identified (Chen et al., 2025; Ma et al.,

2025; Xia et al., 2024; Jiang et al., 2025). Since modern OSS repositories contain a significant amount of code-far beyond any LLM's context window-localization drastically reduces the search space and improves downstream solvability. Crucially, localization outputs a discrete set of paths, enabling verifiable, string-level evaluation that is compatible with scalable training frameworks such as SFT and RLVR. Existing localization agents (Ma et al., 2025; Chen et al., 2025; He et al., 2025) typically rely on multiple tools, including Search Class ,Search Methods Get Imports . Although effective to some extent, these tools considers high-level abstractions (classes, function, etc) of programing languages, which do not reflect how code actually executes. High-level abstractions, such as classes or inheritance, disappear after compilation, leaving only sequential execution andjumpoperations. Since modern LLMs already excel at modeling sequential depeninition of symbols as they appear in execution. To this end, dencies, we focus on enhancing their ability tojump the repository-that is, to follow and inspect the source defwe introduce a single, structurally grounded tool: which retrieves the precise definition of a given symbol. Details of this tool are provided in Sec. 3.3. Our main contributions are threefold: (1) We propose the first repo-level localization agent trained on reinforcement learning directly from the pretrained model, regardless of distillation from a close-source model. (2) We design a repository-navigation agent that operates by performing realisticjumpoperations aligned with actual execution secantly improves efficiency and controllability compared to mantics. (3) We demonstrate that one unified tool signifimulti-tool pipelines.

2. Related Works

2.1. Agentic Training

any tools, most tools are out-of-domain (OOD) for LLMs. Even for the most powerful models, failures often happen when calling the new-defined tools due to wrong calling format or failed parameter parsing. Thus, training a LLM to master new-defined tool is critical for LLM agents. Intrain a student model via supervised finetuning (SFT) (Chen tuitively, the tool-calling trajectories can be generated by a more powerful LLM, and such trajectories can be used to et al., 2025). However, this pipeline requires a stronger teacher model which has capability to master the tool. Remultiple rollouts. Agentic RL (Jin et al., 2025) is an onpolicy RLVR methods requiring only the result for verifiying

cently, more methods have emerged with no teacher-model required. Rejected-sampled finetuning (RFT) (Ahn et al.,

2024) utilizes generated trajectories of the agent itself via

trajectories. Such training methods yield remarkable results when the tools are search engines (Jin et al., 2025), python executer (Jimenez et al., 2023), calculator (Yan et al., 2025), and visual models (Gupta & Kembhavi, 2023).

2.2. Software Engineering Agents

, and The introduction of SWE-bench (Jimenez et al., 2023; Yang

et al., 2024b) has motivated a range of agentic pipelines for software engineering (SWE) tasks. Among them, SWE- AGENT (Yang et al., 2024a) and OPENHANDS (Wang et al., 2025a) are widely adopted frameworks that equip agents with tools for interacting with computing environments. Workflow-based methods such as Agentless (Xia et al.,

2024) decompose issue resolution into localization, repair,

across and validation subproblems. Chen et al. (2025) builds the respository as a graph and applied graph-level searching tools

for localization, and Wang et al. (2025a) furthermore inteet al., 2025b) equip conceptual information of the respository to enable repo-level understanding. These pipelines

jump, grated commit history as agent memory. RepoLens (Wang

are training-free, compatible with closed-source language models, and yield competitive results. To enable task-specific training, DEEPSWE (Luo et al.,

2025) and SWE-SWISS (He et al., 2025) employ reinforce-

ment learning and achieve strong performance. However, end-to-end training remains costly because patch evaluation requires executing Docker environments across numerous repositories. Consequently, issue localization has emerged as a computationally efficient alternative, aiming to identify faulty components-at file or function level-rather than generating full patches. Recent localization agents include LOCAGENT (Chen et al.,

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

2025) and COSIL (Jiang et al., 2025), which model code-

LLM agents are promising methods to equip models withbases as graphs and integrates them into LLMs, and complex tools while reasoning (Li et al., 2024; Huang et al.,ORCALOCA (Yu et al., 2025b), which enhances efficiency

2024; Guo et al., 2024b). However, because most pretrainedthrough priority scheduling, action decomposition, and LLMs are trained on texts only and developers can definecontext pruning. From an open-source perspective, RE-

2


---

## Page 3

3

R(s, ℓ) → {(f i, pi)}, (1)

a resolution mapping

the detailed method. symbol occurrencesat source locationℓ, Pyright computes jectories. Below we provide the formal problem setting andmates Python's runtime name-binding semantics. Given a algorithm for optimizing long-horizon tool-augmented tra-through a deterministic static analysis pipeline that approxireasoning and tool invocation, and (3) a GRPO-based RLLanguage servers resolve the definition of a Python symbol

action agent loop that alternates between natural-language3.3. Jump: Symbol Resolution definition of any symbols in a given file, (2) a reasoning- sists of three components: (1) a unified tool to retrieve theobserve. for repository-level issue localization. The method con-reasoning until termination. The loop is reason →act→

We present RepoNavigator, a reinforcement-learning agent servations (code snippets or error messages), then continues

3. Method file are parsed to the tool. The agent receives structured ob-

formatted tool call, while a symbol and its corresponding

decides whether to continue reasoning or to emit a JSONet al., 2025). tool orchestration overhead. At each step the policyπθ

both Repo Searcher (Ma et al., 2025) and Loc Agent (Chen Repo Navigator uses a single-tool design to avoid multitrained directly from pretrained models, without relying on3.2. Agent Architecture distillation-based supervised finetuning, a crucial stage in Finally, our approach constitutes the first localization agent

gle execution-logic-focused tool, reducing usage complexity.The objective is max θEτ ∼π [R(τ )]. error propagation (see Sec. 3). In contrast, we employ a sin-nation, a final predictionYˆis scored by a rewardR(Y , Yˆ ∗). tiple search tools for symbol definition retrieval, amplifyingo, forming a trajectoryτ = {(r , a, o)}T . After termiare cross-referenced across files-and typically rely on mul-reasoning steprt, a tool callat, and receives the observation

within repositories-where modules, classes, and functionsspan in filefi. At each stept, the agent produces a optional Nevertheless, prior agents overlook the structural relationsY∗= {(f , g)}, whereg denotes a function or code

i i,j i,j

scriptionq, the goal is to output relevant code regions

a notable advancement. and RL on the Qwen model family (Team, 2024), represents POSEARCHER (Ma et al., 2025), trained with distillation 3.1. Problem Formulation

the definition code of the symbol. This process is trained by reinforcement learning.

*Figure 2.Overview of our Repo Navigator. During the rollout phrase, the agent can call thejumptool, and the language server will return*

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents


---

## Page 4

where each pair(fi, pi)denotes a file path and a source position corresponding to a valid definition site ofs. In function: practice, we usefile pathandsymbol to resolveℓ. If we have multiple symbols with the same name exist in the same code snippet, we additionally parse anindexto the tool, which allows for accurate resolution of ℓ. Syntactic Analysis In this process, the source file is parsed into an abstract syntax tree (AST). The syntactic role ofs(e.g., name, attribute access, or call expression)where the first term is the standard policy gradient objective determines the subsequent resolution strategy. For attributewith an estimated advantage function expressionsa.b, Pyright treatsaas a receiver expressionactions that lead to higher-than-expected returns. The secwhose type must be inferred prior to member lookup.ond term is a Kullback-Leibler (KL) divergence penalty,

scaled by a coefficient

Lexical Scope Resolution For a name symbol x, candi-venting the updated policy date definitions are searched along a scope chain the previous policy

and consistent policy improvement by balancing reward

S = {local, enclosing, module, builtins}, (2) maximization with behavioral consistency.

following Python's LEGB rule. Each scope maintains a symbol table mapping identifiers to defining AST nodes. Static Type Inference . For attribute symbols, it computes a (possibly union-valued) typeT (a) for the receiver expressionausing type annotations, assignment flow analysis, function return types, and stub files (.pyi). Member resolution is then defined as

resolve(a.b) = lookup(b, MRO(t)),

t∈T (a) τ

whereMRO(t) denotes the method resolution order of typeis incorrect, or the symbol parsed does not exist, or for any t. other reason that causes the tool to quit unexpectedly. Import Dependency Graph For cross-file resolution, import dependency graph that statically emulates Python's module loading semantics is built. Import statements introduce bindings that map local symbols to exported symbols of target modules, including re-exports andall -based filtering. Resolution may therefore traverse multiple modules before reaching a concrete definition. data for 16 times. A sample is abandoned if all 16 scores

are zero. For validation, we test our method on SWE-bencht t− t−

3.4. Reasoning-Action Loop verified (Jimenez et al., 2023), which is a human-verified Given historyh = (q, o , a ), the agent samplessubset of SWE-bench. We additionally test our method on either a natural-language reasoning stepr∼ π(·|h )or a a subset of SWE-bench-pro (Yang et al., 2025b) (which structured tool calla∼ π(·|h ). Tool calls must satisfyis a new and more difficult benchmark) for generalization. a JSON grammar enforced via constrained decoding. The loop continues until the agent outputs its final localizationgolden patches. All datasets are open-source and are built Y .ˆ on real-world github issues.

Reference Policy Optimization (GRPO), which has the loss GRPO πθ(at|st)ˆ

θold t t

− β DKL(πθ(·|s t)∥πθ(·|s t))] (3)

Aˆ, which promotes

β, which acts as a trust region, preπθfrom moving too far from πθ. This formulation ensures stable

The reward of GRPO process is calculated as: Dice is a common metric for set-level comparison, for set Y and set Yˆ ∗ S(τ ) is the success rate of tool-calling extracted from . We consider the tool-call to be failed when the format

4. Experiment

4.1. Experimnent Setup Datasets We extract valid samples from SWE-smith (Yang et al., 2025b) to form the training set. We apply Qwen2.5-7B-Instruct with Repo Navigator to sample each

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

2

1:1 1:1

Metrics Previous works (Chen et al., 2025; Ma et al.,

3.5. Reinforcement Learning

2025) applied recall and precision as metrics. However,

We apply reinforcement learning with verifiable rewardsbecause the predicted locations and ground-truth locations to train the agent directly from the pretrained model, withare sets of strings, recall and precision singularly can not no teacher model required. In practice, we apply Groupreflect the performance fairly. Thus, we utilize Sample-F1

4

For ground-truth locations, we directly use the locations in


---

## Page 5

*Table 1.Comparison of different agent pipelines on function-level and file-level Dice/IoU metrics. We use Qwen2.5-Instruct series as*

our base model. Bold numbers denote the best performance among same-size models; free performance among same-size models;yellow backgroundillustrates training-free Repo Navigator; Repo Navigator trained with GRPO.

Function-level File-level

Agent Pipeline Model

Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU

Close-source Models

Repo Searcher Claude3.7-Sonnet 66.80 19.90 28.30 17.89 89.71 21.04 33.15 20.67 Repo Navigator Claude3.7-Sonnet 31.03 34.43 31.72 30.22 72.26 75.95 73.01 71.37 Repo Navigator GPT5-chat 30.42 34.56 31.17 29.67 58.17 61.87 58.88 57.33 Repo Navigator Claude4.5-Sonnet 43.97 45.76 43.62 41.31 80.68 81.92 79.94 77.49

Qwen2.5-7B

Locagent Training Free 17.62 11.71 12.71 10.31 60.96 34.88 40.67 33.33 CoSIL Training Free 29.30 8.98 12.90 8.07 70.12 Agentless Training Free 24.92 12.93 15.31 11.74 63.01 19.32 27.82 18.85 Orcaloca Training Free 27.70 20.29 Repo Searcher Distillation+GRPO 63.26 19.24 27.37 17.59 84.11 19.97 31.64 19.57 Repo Navigator Training Free 15.89 17.46 Repo Navigator GRPO 26.69 30.34

Qwen2.5-14B

Locagent Training Free 35.62 13.32 17.71 12.32 71.42 31.66 40.77 30.64 CoSIL Training Free 48.61 13.40 19.81 12.12 78.35 Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88 19.30 Orcaloca Training Free 29.92 20.98 22.77 18.92 52.17 52.15 50.93 48.72 Repo Searcher Training Free 26.13 11.96 14.35 10.60 74.77 18.80 28.79 18.15 Repo Navigator Training Free 27.96 25.77 Repo Navigator GRPO 31.02 30.08

Qwen2.5-32B

Locagent Training Free 46.79 16.29 21.48 14.18 79.39 34.18 44.18 33.24 CoSIL Training Free 55.38 14.85 22.11 13.52 83.50 19.34 30.77 18.93 Agentless Training Free 40.79 24.07 27.33 22.08 78.93 25.60 35.38 24.96 Orcaloca Training Free 39.14 25.59 Repo Searcher Distillation+GRPO 69.50 20.29 29.11 18.23 89.33 Repo Navigator Training Free 28.11 28.19 Repo Navigator GRPO 33.71 37.19

underline numbersdenote the best trainingblue backgroundillustrates

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

Training For the 7B model, we conduct GRPO with 8 are provided in Appendix. B. Tesla-A100-80G GPUs. For the 14B and 32B model, we train it with 16 Tesla-A100-80G GPUs. We apply verl4.2. Effectiveness (Shen, 2024) as the training framework, and we apply vLLM

Baselines We compare our method against Locagent

(Kwon et al., 2023) as the inference engine. We train the (Chen et al., 2025), CoSIL (Jiang et al., 2025), Agentmodel for 1 epoch, while the training batch size is fixed

5

(which is the averaged score of per-sample F1 values) andto 128 on 4k training samples filtered from SWE-smith, IoU (intersection out of union) as our core metrics. At thewith maximum prompt length and max response length same time, we also present the recall and precision scoresboth set to 10240. Additionally, we rollout 8 times for to align with previous methods, although they do not reflecteach sample, and the temperature is set to 1.0 to encourage the methods' performance fairly. exploration. We use greedy decoding in the inference stage

to ensure stable performance. More implementation details

17.90 27.39 17.42

21.70 17.92 48.04 48.65 47.36 45.77 16.19 15.46 42.36 43.23 42.12 40.97 27.49 26.43 50.62 53.83 51.63 50.62

18.10 28.79 17.72

25.58 23.00 59.00 56.68 56.39 53.74 29.23 26.84 61.60 58.97 58.90 56.36

28.72 22.89 59.57 59.51 58.11 55.62

20.27 32.93 20.35

27.12 25.16 63.05 62.75 61.67 59.28 34.09 32.30 67.29 70.76 67.75 65.75


---

## Page 6

*Table 2.Comparison of different agent pipelines on function-level and file-level metrics on SWE-bench*

numbers denote the best performance among same-size models;underline numbers same-size models;yellow backgroundillustrates training-free Repo Navigator; GRPO.

Function-level File-level

Agent Pipeline Model

Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU

Qwen2.5-7B

Loc Agent Training Free 1.01 0.02 0.65 0.40 12.16 0.17 10.81 8.93 CoSIL Training Free 8.64 3.33 4.58 2.87 26.64 8.47 12.11 7.70 Agentless Training Free 12.82 6.94 8.05 5.73 39.41 Repo Searcher Training Free 1.07 0.93 0.97 0.86 4.91 1.64 2.30 1.63 Repo Navigator Training Free9.84 14.65 Repo Navigator GRPO 12.33 21.26

Qwen2.5-14B

Loc Agent Training Free 6.22 0.13 3.65 2.65 15.58 0.21 11.69 9.53 CoSIL Training Free 10.73 4.67 5.96 3.94 34.31 9.97 14.81 9.30 Agentless Training Free 10.49 6.75 7.41 5.28 41.42 13.42 19.02 12.37 Repo Searcher Training Free 2.79 1.38 1.69 1.14 17.37 5.17 7.60 4.84 Repo Navigator Training Free14.36 19.74 Repo Navigator GRPO 16.05 25.25

Qwen2.5-32B

Loc Agent Training Free 8.72 0.17 4.30 2.90 25.73 0.38 19.77 16.50 CoSIL Training Free 15.00 6.35 8.14 5.21 45.37 13.04 19.42 12.36 Agentless Training Free 11.08 7.31 7.98 5.80 43.07 13.89 20.07 13.11 Repo Searcher Training Free 2.00 1.29 1.45 1.00 13.51 3.43 5.31 3.24 Repo Navigator Training Free13.96 20.25 Repo Navigator GRPO 18.13 29.44

baseline methods are presented in Appendix. A.

Pro for generalization. Bold

denote the best training-free performance among

blue backgroundillustrates RepoNavigator trained with

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

with training free, RFT, GRPO with pure outcome and hybrid reward on Qwen2.5-7B-Instruct. the tool we implement is effective and promising, and our

single tool pipeline is better than previous multiple tools pipelines.

less (Xia et al., 2024), Orcaloca (Yu et al., 2025b), and Compared with Repo Searcher, which is distilled from Repo Searcher (Ma et al., 2025). Detailed explaination ofclaude-3.7-sonnet (Anthropic, 2025) and reinforced by

6

ization, our method surpasses all baseline methods with the same model size. Moreover, if we train Repo Navigator with GRPO, our 7B model surpasses 14B baselines, and our contributes to the validness of Repo Navigator furthermore. cantly lower precision score than Repo Navigator, and result in lower S-F1 and IoU. This indicates that Repo Navigator behaves more conservatively and generates less wrong locations. For 14B and 32B models, Repo Navigator achieves

*Figure 3.Ablation study: comparison between Repo Navigator*

13.15 18.89 12.35

10.67 9.20 30.50 37.24 31.86 28.82 14.29 12.02 36.36 48.13 39.74 36.36

15.27 12.00 43.57 54.52 46.06 41.07 18.06 14.58 46.85 58.64 49.72 45.14

15.36 12.87 50.24 63.24 53.48 48.50 20.72 17.16 53.49 68.69 57.57 52.44

Results As illustrated in Table. 1, on balanced metrics (S-F1 and IoU) for both function-level and file-level local-

14B model surpasses 32B baselines on S-F1 and IoU. This Although some baselines have higher recall score signifi- SOTA among all training-free methods. This implies that


---

## Page 7

*Figure 4.Scaling law of tool-calling, where Pre and Post denote*

the corresponding metric before and after the RL training.

Agent Pipeline Func-IoU(%) Resolved(%) Agentless 5.28 10.12 Loc Agent 2.65 13.01 Repo Navigator 12.00 14.74 Repo Navigator+RL 14.58 15.03

*Table 3.We use Qwen2.5-14B-Instruct as the localization model,*

and use Qwen2.5-32B-Instruct as the repair model on SWEhybrid reward (with tool-calling success rate) has highersince additional tools often introduce new and unfamiliar

benchVerified.

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

We also remove the success rate in the reward function foractions and observable elements is smaller than when multisuccess rate). This indicates that learning to correctly callto during pretraining, potentially increasing the likelihood ablation. As presented in Fig. 3, reinforcement learning withple tools are available. This reduction is generally beneficial, performance than pure outcome reward (without tool-callinginterfaces that large language models have not been exposed tools is vital in agentic learning. of errors.

7

GRPO, trained Repo Navigator outperforms it on all metrices except recall. Moreover, we found that our training-free method outperforms Repo Searcher for 14B models. This is probably due to the simplified tool we integrate to the agent (see Sec. 5 for more details). To assess the generalizability of Repo Navigator, we present its performance on Python samples from the SWE-benchon this dataset are consistent with those observed on SWEmake a stronger claim regarding SWE-bench Pro, as it was

Pro dataset (Yang et al., 2025b) in Table 2. The results bench Verified. While we cannot fully exclude the potential influence of data leakage in SWE-bench Verified, we can released after the publication of the Qwen2.5 series.

4.3. Training Strategy Comparison To explore the capability of GRPO on agentic training, we compare GRPO against RFT-only and RFT+GRPO. As presented in Fig. 3, directly training with GRPO outperformes RFT-only and RFT+GRPO. Moreover, although RFT has accetable performance, the more steps RFT proceeds, the less improvement GRPO makes after the cold start. This conclusion contradicts with previous SWE agents trained with RL (Ma et al., 2025), however, it aligns with the broader field of reinforcement learning, where RFT and SFT (as a cold start) is effective only when the pretrained model is not strong enough (Guo et al., 2024a). When the pretrained model is strong enough and data is high-quality, directly training a model with RL is better than training after SFT (RFT) as its cold start.

4.4. Scaling Law of Tool-Calling To assess the significance of tool-calling in Repo Navigator, we varied the maximum number of tool-calling turns and reported the results in Fig. 4.2. As shown in the figure, allowing more tool-calling turns consistently leads to improved performance for Repo Navigator, both before and after reinforcement learning (RL) training. In other words, these results empirically validate the scaling law of tool-calling in this context.

4.5. Influence on Issue Resolution To evaluate the impact of different localization results on the final issue resolution performance, we test Repo Navigator against baselines on SWE-bench Verified. We directly apply the repairing phrase of Agentless while replacing its localization front-end with other methods. Table.3 illustrates the results. Compared with baselines, Repo Navigator has the highest performance on issue resolution, while reinforcement learning improves its performance furthermore.

## 5.Discussion: Building Less yet More Capable Tools

In this section, we analyze the logic behind Repo Navigasembled functions is more effective than building multiple tor: building less tools with more powerful and more entask-specific tools.

5.1. Impact on the Action Space of Agents Let the total number of available tools be denoted ask. When only a single tool-specifically thejumptool-is reto what this tool can access. In this case, the set of possible tained, the system's structural relations become simpler, as both the action space and the observation space are restricted


---

## Page 8

*Figure 5.Venn graph illustrating access scope ofjump. Compared*

with the repository scope, the access scope has a much higher IoU with the groundtruth set.

Jump Get Class Get Func Get Struc IoU ✓ ✓ ✓ ✓ 13.71 ✓ ✓ ✓ ✗ 21.44 ✓ ✗ ✗ ✓ 24.00 ✓ ✗ ✗ ✗ 24.28

*Table 4.We change the tool set of Repo Navigator and present*

the function-level IoU (%) on Qwen2.5-7B-Instruct. Apparently, excessive tools do not boost Repo Navigator's performance.

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

state-of-the-art localization performance. We theoretically

plying multiple repo-level retrivel tools results in the access

analyze the results, confirming that a single powerful tool,

scope equal to the whole repository scope.

jointly optimized with reinforcement learning, can provide

When we start from the entry point and repeatedly applystronger robustness and more reliable multi-step reasonjump-which retrieves the definition of each referenceding than previous frameworks relying on multiple narrowly symbol-we effectively traverse all symbols that are se-scoped tools.

8

5.2. Impact on Tool-Calling Success Rate For a given process in issue localization (for instance, checking the code snippet of a function), let the success probability of thei-th call bepi. For a task that requiresksequential tool invocations, the overall success rate can be expressed

Psucc(k) = pi. (6)

Since each step introduces an additional potential point of failure, the cumulative success rate typically decreases as the number of required tool calls increases. Therefore, in general, completing a task with a single, more versatile tool tends to be more reliable than relying on multiple narrowscope tools executed in sequence.

5.3. Impact on the Prediction Space The access scope of a tool is defined as the complete set of files, symbols, and other resources that the tool can access within a repository. For ajumptool that navigates to symbol definitions, its access scope can be obtained by starting from a given entry point and recursively resolving all referenced symbols until no new definitions can be reached. Apparently, its access scope is significantly smaller than the full repository scope. Consequently, when computing the Intersection over Union (IoU) between the prediction set and the groundtruth set, using thejumptool results in a

higher IoU, as depicted in Fig. 5. On the other hand, aplocation that contributes to the issue must lie on some demantically activated by that entry point. Because every

pendency path originating from the entry point, it is necessarily reachable through this recursive symbol-reference expansion. Therefore, the final access scope produced by exhaustivejumptraversal is guaranteed to contain all locations that must be modified to resolve the issue.

5.4. Verification To further verify this proposal, we change the tool set of Repo Navigator and conduct RL training with only the outcome reward. We add excessive tools which were frequently used in previous works (Chen et al., 2025; Ma et al., 2025; Jiang et al., 2025) and present the result in Table. 4. Getoutputs the class/function definition. Get Struc takes no input and outputs the repository's structure. The results clearly Class/Get Func takes a class/function name as input and implies that additional tools do not increase model's perforcapable tools. mance. This inspires researchers to develop less but more

6. Conclusion

In this work, we introduced Repo Navigator, a repositorylevel issue localization agent that departs from existing multi-tool paradigms by leveraging a single, more-capable jumptool for symbol resolution. This unified design faithfully reflects real code execution flow while significantly reducing the complexity and brittleness of multi-step tool chaining. Through tool-integrated GRPO, Repo Navigator learns to reason, invoke tools, and refine its predictions in a closed-loop manner, enabling end-to-end optimization without relying on closed-source teacher models or distillation. Extensive experiments across SWE-bench-Verified and SWE-bench-Pro demonstrate that Repo Navigator achieves


---

## Page 9

9

technical report. ar Xiv preprint ar Xiv:2409.12186, 2024. Liu, T., Zhang, J., Yu, B., Lu, K., et al. Qwen2. 5-coder Hui, B., Yang, J., Cui, Z., Yang, J., Liu, D., Zhang, L.,

ar Xiv:2402.02716, 2024. the planning of llm agents: A survey. ar Xiv preprint D., Wang, Y., Tang, R., and Chen, E. Understanding Huang, X., Liu, W., Chen, X., Wang, X., Wang, H., Lian,

Recognition, pp. 14281-14290, 2024. IEEE/CVF Conference on Computer Vision and Patternlanguage model for gui agents. In Proceedings of the Wang, Z., Dong, Y., Ding, M., et al. Cogagent: A visual

Hong, W., Wang, W., Lv, Q., Xu, J., Yu, W., Ji, J., Wang, Y., Swiss, 2025. Notion Blog. issue resolution. https://github.com/zhenyuhe00/SWE-

multi-task fine-tuning and rl recipe for high-performance Shi, W., Cai, T., He, D., Chen, J., and Xu, J. Swe-swiss: A He, Z., Yang, Q., Sheng, W., Zhong, X., Zhang, K., An, C.,

tern recognition, pp. 14953-14962, 2023. of the IEEE/CVF conference on computer vision and patsitional visual reasoning without training. In Proceedings

Gupta, T. and Kembhavi, A. Visual programming: Compoar Xiv preprint ar Xiv:2402.01680, 2024b. based multi-agents: A survey of progress and challenges.

N. V., Wiest, O., and Zhang, X. Large language model Guo, T., Chen, X., Wang, Y., Chang, R., Pei, S., Chawla, ar Xiv:2401.14196, 2024a.

programming-the rise of code intelligence. ar Xiv preprint Deepseek-coder: When the large language model meets Zhang, W., Chen, G., Bi, X., Wu, Y., Li, Y., et al. Guo, D., Zhu, Q., Yang, D., Xie, Z., Dong, K.,

//aclanthology.org/2025.acl-long.426/.

0. doi: 10.18653/v1/2025.acl-long.426. URLhttps:

for Computational Linguistics. ISBN 979-8-89176-251ceedings of the 63rd Annual Meeting of the Association pp. 8697-8727, Vienna, Austria, July 2025. Association for Computational Linguistics (Volume 1: Long Papers),

Nabende, J., Shutova, E., and Pilehvar, M. T. (eds.), Proguided LLM agents for code localization. In Che, W., Prasanna, V., Cohan, A., and Wang, X. Loc Agent: Graph- Chen, Z., Tang, R., Deng, G., Wu, F., Wu, J., Jiang, Z.,

2025-11-18. claude-3-7-sonnet , February 2025. data: https://www.anthropic.com/news/ Anthropic. Claude 3.7 sonnet and claude code.

gramming languages.

2024.

explore extending Repo Navigator from Python to more progresses and challenges. ar Xiv preprint ar Xiv:2402.00157, for medium-sized open-source models. Future work will Large language models for mathematical reasoning: Proing with real execution structure, and show that efficient reasoning-tool co-training can unlock substantial gains even

Ahn, J., Verma, R., Lou, R., Liu, D., Zhang, R., and Yin, W.

Our findings highlight the importance of aligning agent tool-References

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents


---

## Page 10

Jiang, Z., Ren, X., Yan, M., Jiang, W., Li, Y., and Liu, Z. Cosil: Software issue localization via llmdriven code repository graph searching. ar Xiv preprint ar Xiv:2503.22424, 2025. Jimenez, C. E., Yang, J., Wettig, A., Yao, S., Pei, K., Press, O., and Narasimhan, K. Swe-bench: Can language models resolve real-world github issues? ar Xiv preprint ar Xiv:2310.06770, 2023. Jin, B., Zeng, H., Yue, Z., Yoon, J., Arik, S., Wang, D., Zamani, H., and Han, J. Search-r1: Training llms to reason and leverage search engines with reinforcement learning. ar Xiv preprint ar Xiv:2503.09516, 2025. Kwon, W., Li, Z., Zhuang, S., Sheng, Y., Zheng, L., Yu, C. H., Gonzalez, J. E., Zhang, H., and Stoica, I. Efficient memory management for large language model serving with pagedattention. In Proceedings of the ACM SIGOPS 29th Symposium on Operating Systems Principles, 2023. Langley, P. Crafting papers on machine learning. In Langley, P. (ed.), on Machine Learning (ICML 2000), pp. 1207-1216, Stanford, CA, 2000. Morgan Kaufmann. Li, Y., Wen, H., Wang, W., Li, X., Yuan, Y., Liu, G., Liu, J., Xu, W., Wang, X., Sun, Y., et al. Personal llm agents: Insights and survey about the capability, efficiency and security. ar Xiv preprint ar Xiv:2401.05459, 2024. Liu, A., Feng, B., Xue, B., Wang, B., Wu, B., Lu, C., Zhao, C., Deng, C., Zhang, C., Ruan, C., et al. Deepseek-v3 technical report. ar Xiv preprint ar Xiv:2412.19437, 2024. Liu, Z., Zhang, Y., Li, P., Liu, Y., and Yang, D. Dynamic llm-agent network: An llm-agent collaboration framework with agent team optimization. ar Xiv preprint ar Xiv:2310.02170, 2023. Lu, J., Holleis, T., Zhang, Y., Aumayer, B., Nan, F., Bai, F., Ma, S., Ma, S., Li, M., Yin, G., et al. Toolsandbox: A stateful, conversational, interactive evaluation benchmark for llm tool use capabilities. ar Xiv preprint ar Xiv:2408.04682, 2024. Luo, M., Jain, N., Singh, J., Tan, S., Patel, A., Wu, Q., Ariyak, A., Cai, C., Tarun Venkat, S. Z., Athiwaratkun, B., Roongta, M., Zhang, C., Li, L. E., Popa, R. A., Sen, K., and Stoica, I. Deepswe: Training a stateof-the-art coding agent from scratch by scaling rl. https://pretty-radio-b75.notion.site/ DeepSWE-Training-a-Fully-Open-sourced-State-of-the-Art-Coding-Agent-by-Scaling-RL-22281902c1468193aabbe9a8c59bbe33

Schmidgall, S., Su, Y., Wang, Z., Sun, X., Wu, J., Yu, X., Liu, J., Moor, M., Liu, Z., and Barsoum, E. Agent labpreprint ar Xiv:2501.04227, 2025. oratory: Using llm agents as research assistants. ar Xiv Shen, Z. Llm with tools: A survey. ar Xiv preprint ar Xiv:2409.18807, 2024. Team, Q. Qwen2 technical report. ar Xiv preprint ar Xiv:2407.10671, 2024. Wang, X., Li, B., Song, Y., Xu, F. F., Tang, X., Zhuge, M., Pan, J., Song, Y., Li, B., Singh, J., Tran, H. H., Li, F., Ma, R., Zheng, M., Qian, B., Shao, Y., Muenopen platform for AI software developers as generalist agents. In nighoff, N., Zhang, Y., Hui, B., Lin, J., Brennan, R., Peng, H., Ji, H., and Neubig, G. Openhands: An on Learning Representations, 2025a. URL https: //openreview.net/forum?id=OJd3ayDDoF. Wang, Y., Mao, W., Wang, C., Zhou, Z., Zhou, Y., Zhao, W., Lou, Y., and Peng, X. Extracting conceptual knowledge to locate software issues. ar Xiv preprint ar Xiv:2509.21427, 2025b. Xia, C. S., Deng, Y., Dunn, S., and Zhang, L. Agentless: Demystifying llm-based software engineering agents. ar Xiv preprint ar Xiv:2407.01489, 2024. Yan, Y., Wang, S., Huo, J., Yu, P. S., Hu, X., and Wen, Q. Mathagent: Leveraging a mixture-of-math-agent framework for real-world multimodal mathematical error detection. ar Xiv preprint ar Xiv:2503.18132, 2025. Yang, A., Li, A., Yang, B., Zhang, B., Hui, B., Zheng, B., Yu, B., Gao, C., Huang, C., Lv, C., et al. Qwen3 technical report. ar Xiv preprint ar Xiv:2505.09388, 2025a. Yang, J., Jimenez, C. E., Wettig, A., Lieret, K., Yao, S., Narasimhan, K. R., and Press, O. SWE-agent: Agentcomputer interfaces enable automated software engineering. In Neural Information Processing Systems, 2024a. URL https://arxiv.org/abs/2405.15793. Yang, J., Jimenez, C. E., Zhang, A. L., Lieret, K., Yang, J., Wu, X., Press, O., Muennighoff, N., Synnaeve, G., Narasimhan, K. R., et al. Swe-bench multimodal: Do ai systems generalize to visual software domains? ar Xiv ,

One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

The Thirteenth International Conference

Proceedings of the 17th International Conference

The Thirty-eighth Annual Conference on

2025. Notion Blog. preprint arXiv:2410.03859, 2024b.

Ma, Z., Peng, C., Zeng, Q., Gao, P., Zou, Y., and Xie,Yang, J., Lieret, K., Jimenez, C. E., Wettig, A., Khandpur, B. Tool-integrated reinforcement learning for repo deepK., Zhang, Y., Hui, B., Press, O., Schmidt, L., and Yang, search, 2025. URL https://arxiv.org/abs/ D. Swe-smith: Scaling data for software engineering

2508.03012. agents. arXiv preprint arXiv:2504.21798, 2025b.

10