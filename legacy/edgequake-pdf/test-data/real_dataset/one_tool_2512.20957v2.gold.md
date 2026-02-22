# One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

Zhaoxi Zhang, Yitong Duan, Yanzhi Zhang, Yiming Xu, Jiyan He, Yunfang Wu

## Abstract

Locating the files and functions requiring modification in large open-source software (OSS) repositories is challenging due to their scale and structural complexity. Existing large language model (LLM)-based methods typically treat this as a repository-level retrieval task and rely on multiple auxiliary tools, which overlook code execution logic and complicate model control. We propose **RepoNavigator**, an LLM agent equipped with a single execution-aware tool—jumping to the definition of an invoked symbol. This unified design reflects the actual flow of code execution while simplifying tool manipulation. RepoNavigator is trained end-to-end via Reinforcement Learning (RL) directly from a pretrained model, without any closed-source distillation. Experiments demonstrate that RL-trained RepoNavigator achieves state-of-the-art performance, with the 7B model outperforming 14B baselines, the 14B model surpassing 32B competitors, and even the 32B model exceeding closed-source models such as Claude-3.7. These results confirm that integrating a single, structurally grounded tool with RL training provides an efficient and scalable solution for repository-level issue localization.

---

## 1. Introduction

With the rapid advancement of Large Language Models (LLMs), equipping LLMs with pre-built tools to form LLM agents has become a common paradigm for expanding their capabilities. In the domain of software engineering (SWE), although LLM agents can effectively handle simple programming tasks, their ability to operate on large-scale open-source software (OSS) repositories remains limited. SWE-BENCH currently serves as the most comprehensive benchmark for evaluating whether LLMs can resolve real-world GitHub issues.

All pretrained LLMs cannot process the whole repository directly due to context limits. While SWE-AGENT provides moderate gains, it remains far from enabling robust repository-level reasoning. Most existing agents rely on test-time scaling applied directly to pretrained LLMs. In SWE tasks, tool usage is essential rather than optional: real-world repositories are far larger than the context window of current LLMs, making it impossible to process an entire codebase in a single forward pass. Agents must therefore iteratively invoke tools to retrieve partial information from the repository and interleave natural-language reasoning with tool calls.

> **Figure 1 Description:** An illustration of an LLM navigating through a code repository. The LLM is equipped with a single yet powerful tool: `jump`, which is realized through a language server.

However, mainstream LLMs are rarely exposed to such agentic interaction patterns during pretraining and typically acquire tool usage only through few-shot prompting. Such in-context demonstrations are insufficient for learning complex multi-step tool-chaining behaviors, especially under limited context windows. Moreover, because tool definition spaces are effectively unbounded, pretrained models cannot fully internalize their semantics without post-training. To mitigate these issues, post-training paradigms such as Supervised Finetuning (SFT) and Reinforcement Learning with Verifiable Rewards (RLVR) have been applied, with promising results in domains including retrieval agents, GUI agents, and math agents.

Directly training an agent to fix software issues, however, remains difficult. A single bug often admits multiple valid patches, making string-level evaluation unreliable. The only precise evaluation method requires executing candidate patches inside a dedicated Docker environment for each repository, which is prohibitively expensive. To make training more tractable, we adopt a simplified yet widely generalizable assignment: issue localization. Prior work shows that a software issue becomes substantially easier to resolve once the relevant functions and files are correctly identified. Since modern OSS repositories contain a significant amount of code—far beyond any LLM's context window—localization drastically reduces the search space and improves downstream solvability. Crucially, localization outputs a discrete set of paths, enabling verifiable, string-level evaluation that is compatible with scalable training frameworks such as SFT and RLVR.

Existing localization agents typically rely on multiple tools, including `SearchClass`, `SearchMethods`, and `GetImports`. Although effective to some extent, these tools consider high-level abstractions (classes, functions, etc.) of programming languages, which do not reflect how code actually executes. High-level abstractions, such as classes or inheritance, disappear after compilation, leaving only sequential execution and jump operations. Since modern LLMs already excel at modeling sequential dependencies, we focus on enhancing their ability to jump across the repository—that is, to follow and inspect the source definition of symbols as they appear in execution. To this end, we introduce a single, structurally grounded tool: **jump**, which retrieves the precise definition of a given symbol.

Our main contributions are threefold:

1. We propose the first repo-level localization agent trained on reinforcement learning directly from the pretrained model, regardless of distillation from a closed-source model.

2. We design a repository-navigation agent that operates by performing realistic jump operations aligned with actual execution semantics.

3. We demonstrate that one unified tool significantly improves efficiency and controllability compared to multi-tool pipelines.

---

## 2. Related Works

### 2.1. Agentic Training

LLM agents are promising methods to equip models with complex tools while reasoning. However, because most pretrained LLMs are trained on texts only and developers can define any tools, most tools are out-of-domain (OOD) for LLMs. Thus, training an LLM to master new-defined tools is critical for LLM agents. Intuitively, the tool-calling trajectories can be generated by a more powerful LLM, and such trajectories can be used to train a student model via supervised finetuning (SFT). However, this pipeline requires a stronger teacher model which has the capability to master the tool.

Recently, more methods have emerged with no teacher-model required. Rejected-sampled finetuning (RFT) utilizes generated trajectories of the agent itself via multiple rollouts. Agentic RL is an on-policy RLVR method requiring only the result for verifying trajectories. Such training methods yield remarkable results when the tools are search engines, Python executors, calculators, and visual models.

### 2.2. Software Engineering Agents

The introduction of SWE-bench has motivated a range of agentic pipelines for software engineering (SWE) tasks. Among them, SWE-AGENT and OPENHANDS are widely adopted frameworks that equip agents with tools for interacting with computing environments. Workflow-based methods such as Agentless decompose issue resolution into localization, repair, and validation subproblems. Chen et al. (2025) builds the repository as a graph and applied graph-level searching tools for localization, and Wang et al. (2025a) furthermore integrated commit history as agent memory. These pipelines are training-free, compatible with closed-source language models, and yield competitive results.

To enable task-specific training, DEEPSWE and SWE-SWISS employ reinforcement learning and achieve strong performance. However, end-to-end training remains costly because patch evaluation requires executing Docker environments across numerous repositories. Consequently, issue localization has emerged as a computationally efficient alternative, aiming to identify faulty components—at file or function level—rather than generating full patches. Recent localization agents include LOCAGENT and COSIL, which model codebases as graphs and integrate them into LLMs, and ORCALOCA, which enhances efficiency through priority scheduling, action decomposition, and context pruning. From an open-source perspective, REPOSEARCHER, trained with distillation and RL on the Qwen model family, represents a notable advancement. Nevertheless, prior agents overlook the structural relations within repositories and typically rely on multiple search tools, amplifying error propagation.

---

## 3. Method

We present **RepoNavigator**, a reinforcement-learning agent for repository-level issue localization. The method consists of three components: (1) a unified tool to retrieve the definition of any symbols in a given file, (2) a reasoning-action agent loop that alternates between natural-language reasoning and tool invocation, and (3) a GRPO-based RL algorithm for optimizing long-horizon tool-augmented trajectories.

> **Figure 2 Description:** Overview of our RepoNavigator. During the rollout phrase, the agent can call the jump tool, and the language server will return the definition code of the symbol. This process is trained by reinforcement learning. The diagram illustrates the flow from Query to Trajectories via RL Training, interacting with a Language Server that accesses the Raw Repo, AST, Symbol Table, and Definition Searcher.

### 3.1. Problem Formulation

Given a repository and an issue description , the goal is to output relevant code regions , where denotes a function or code span in file . At each step , the agent produces an optional reasoning step , a tool call , and receives the observation , forming a trajectory . After termination, a final prediction is scored by a reward . The objective is .

### 3.2. Agent Architecture

RepoNavigator uses a single-tool design to avoid multi-tool orchestration overhead. At each step, the policy decides whether to continue reasoning or to emit a JSON-formatted tool call, while a symbol and its corresponding file are parsed to the tool. The agent receives structured observations (code snippets or error messages), then continues reasoning until termination. The loop is `reason` `act` `observe`.

### 3.3. Jump: Symbol Resolution

Language servers resolve the definition of a Python symbol through a deterministic static analysis pipeline that approximates Python's runtime name-binding semantics. Given a symbol occurrence at source location , Pyright computes a resolution mapping:

where each pair denotes a file path and a source position corresponding to a valid definition site of . In practice, we use file path and symbol to resolve . If multiple symbols with the same name exist in the same code snippet, we additionally parse an index to the tool.

- **Syntactic Analysis:** The source file is parsed into an abstract syntax tree (AST). The syntactic role of (e.g., name, attribute access, or call expression) determines the subsequent resolution strategy.

- **Lexical Scope Resolution:** For a name symbol , candidate definitions are searched along a scope chain following Python's LEGB rule:

- **Static Type Inference:** For attribute symbols, it computes a (possibly union-valued) type for the receiver expression . Member resolution is defined as , where denotes the method resolution order.

- **Import Dependency Graph:** For cross-file resolution, an import dependency graph emulates Python's module loading semantics. Resolution may traverse multiple modules before reaching a concrete definition.

### 3.4. Reasoning-Action Loop

Given history , the agent samples either a natural-language reasoning step or a structured tool call . Tool calls must satisfy a JSON grammar enforced via constrained decoding. The loop continues until the agent outputs its final localization .

### 3.5. Reinforcement Learning

We apply reinforcement learning with verifiable rewards to train the agent directly from the pretrained model, with no teacher model required. In practice, we apply Group Reference Policy Optimization (GRPO), which has the loss function:

where the first term is the standard policy gradient objective with an estimated advantage function , and the second term is a Kullback-Leibler (KL) divergence penalty. The reward of the GRPO process is calculated as:

where Dice is a common metric for set-level comparison:

and is the success rate of tool-calling extracted from .

---

## 4. Experiment

### 4.1. Experiment Setup

- **Datasets:** We extract valid samples from SWE-smith to form the training set (4k samples). For validation, we use SWE-bench-verified (human-verified subset) and a subset of SWE-bench-pro for generalization.

- **Metrics:** We utilize Sample-F1 (averaged score of per-sample F1 values) and IoU (Intersection over Union) as core metrics. We also present recall and precision scores.

- **Training:** For the 7B model, we conduct GRPO with 8 Tesla-A100-80G GPUs. For the 14B and 32B models, we use 16 Tesla-A100-80G GPUs. We apply `verl` as the training framework and `vLLM` as the inference engine. We train for 1 epoch with a batch size of 128, max prompt/response length of 10240, and 8 rollouts per sample.

### 4.2. Effectiveness

**Baselines:** We compare against Locagent, CoSIL, Agentless, Orcaloca, and RepoSearcher.

**Results:** As illustrated in Table 1, on balanced metrics (S-F1 and IoU) for both function-level and file-level localization, our method surpasses all baseline methods with the same model size. Moreover, if we train RepoNavigator with GRPO, our 7B model surpasses 14B baselines, and our 14B model surpasses 32B baselines on S-F1 and IoU.

**Table 1.** Comparison of different agent pipelines on function-level and file-level Dice/IoU metrics. Bold numbers denote best performance among same-size models; underline numbers denote best training-free performance. (Yellow: Training-free RepoNavigator; Blue: RepoNavigator trained with GRPO) .

| Agent Pipeline    | Model             | Function-level Recall | Funct Precision | Funct Sample-F1 | Funct IoU    | File-level Recall | File Precision | File Sample-F1 | File IoU  |
| ----------------- | ----------------- | --------------------- | --------------- | --------------- | ------------ | ----------------- | -------------- | -------------- | --------- |
| **Closed-source** |                   |                       |                 |                 |              |                   |                |                |           |
| RepoSearcher      | Claude3.7-Sonnet  | 66.80                 | 28.30           | 19.90           | 17.89        | 89.71             | 33.15          | 21.04          | 20.67     |
| RepoNavigator     | Claude3.7-Sonnet  | 31.03                 | 31.72           | 34.43           | 30.22        | 72.26             | 75.95          | 73.01          | 71.37     |
| RepoNavigator     | GPT5-chat         | 30.42                 | 31.17           | 34.56           | 29.67        | 58.17             | 61.87          | 58.88          | 57.33     |
| RepoNavigator     | Claude4.5-Sonnet  | 43.97                 | 43.62           | 45.76           | 41.31        | 80.68             | 79.94          | 81.92          | 77.49     |
| **Qwen2.5-7B**    |                   |                       |                 |                 |              |                   |                |                |           |
| Locagent          | Training Free     | 17.62                 | 12.71           | 11.71           | 10.31        | 60.96             | 34.88          | 40.67          | 33.33     |
| COSIL             | Training Free     | 29.30                 | 12.90           | 8.98            | 8.07         | 70.12             | 17.90          | 27.39          | 17.42     |
| Agentless         | Training Free     | 24.92                 | 15.31           | 12.93           | 11.74        | 63.01             | 19.32          | 27.82          | 18.85     |
| Orcaloca          | Training Free     | 27.70                 | 21.70           | 20.29           | 17.92        | 48.04             | 47.36          | 48.65          | 45.77     |
| RepoSearcher      | Distillation+GRPO | **63.26**             | 19.24           | 27.37           | 17.59        | **84.11**         | 19.97          | 31.64          | 19.57     |
| RepoNavigator     | Training Free     | 15.89                 | 16.19           | <u>17.46</u>    | <u>15.46</u> | 42.36             | 43.23          | 42.12          | 40.97     |
| RepoNavigator     | GRPO              | 26.69                 | **27.49**       | **30.34**       | **26.43**    | 50.62             | **51.63**      | **53.83**      | **50.62** |
| **Qwen2.5-14B**   |                   |                       |                 |                 |              |                   |                |                |           |
| Locagent          | Training Free     | 35.62                 | 13.32           | 17.71           | 12.32        | 71.42             | 31.66          | 40.77          | 30.64     |
| COSIL             | Training Free     | **48.61**             | 19.81           | 13.40           | 12.12        | **78.35**         | 18.10          | 28.79          | 17.72     |
| Agentless         | Training Free     | 25.20                 | 16.14           | 14.30           | 12.28        | 75.65             | 19.76          | 29.88          | 19.30     |
| Orcaloca          | Training Free     | 29.92                 | 20.98           | 22.77           | 18.92        | 52.17             | 52.15          | 50.93          | 48.72     |
| RepoSearcher      | Training Free     | 26.13                 | 11.96           | 14.35           | 10.60        | 74.77             | 18.80          | 28.79          | 18.15     |
| RepoNavigator     | Training Free     | 27.96                 | 25.58           | <u>25.77</u>    | <u>23.00</u> | 59.00             | 56.68          | 56.39          | 53.74     |
| RepoNavigator     | GRPO              | 31.02                 | **29.23**       | **30.08**       | **26.84**    | 61.60             | **58.90**      | **58.97**      | **56.36** |
| **Qwen2.5-32B**   |                   |                       |                 |                 |              |                   |                |                |           |
| Locagent          | Training Free     | 46.79                 | 16.29           | 21.48           | 14.18        | 79.39             | 34.18          | 44.18          | 33.24     |
| COSIL             | Training Free     | 55.38                 | 22.11           | 14.85           | 13.52        | 83.50             | 19.34          | 30.77          | 18.93     |
| Agentless         | Training Free     | 40.79                 | 27.33           | 24.07           | 22.08        | 78.93             | 35.38          | 25.60          | 24.96     |
| Orcaloca          | Training Free     | 39.14                 | 25.59           | **28.72**       | 22.89        | 59.57             | 58.11          | 59.51          | 55.62     |
| RepoSearcher      | Distillation+GRPO | **69.50**             | 29.11           | 20.29           | 18.23        | **89.33**         | 32.93          | 20.27          | 20.35     |
| RepoNavigator     | Training Free     | 28.11                 | 27.12           | 28.19           | <u>25.16</u> | 63.05             | 61.67          | 62.75          | 59.28     |
| RepoNavigator     | GRPO              | 33.71                 | **34.09**       | **37.19**       | **32.30**    | 67.29             | **70.76**      | **67.75**      | **65.75** |

To assess generalizability, we present results on SWE-bench Pro in Table 2.

**Table 2.** Comparison of different agent pipelines on SWE-bench Pro for generalization.

| Agent Pipeline  | Model         | Funct Recall | Funct Precision | Funct Sample-F1 | Funct IoU    | File Recall | File Precision | File Sample-F1 | File IoU  |
| --------------- | ------------- | ------------ | --------------- | --------------- | ------------ | ----------- | -------------- | -------------- | --------- |
| **Qwen2.5-7B**  |               |              |                 |                 |              |             |                |                |           |
| LocAgent        | Training Free | 1.01         | 0.65            | 0.02            | 0.40         | 12.16       | 0.17           | 10.81          | 8.93      |
| COSIL           | Training Free | 8.64         | 3.33            | 4.58            | 2.87         | 26.64       | 12.11          | 8.47           | 7.70      |
| Agentless       | Training Free | 12.82        | 6.94            | 8.05            | 5.73         | **39.41**   | 13.15          | 18.89          | 12.35     |
| RepoSearcher    | Training Free | 1.07         | 0.93            | 0.97            | 0.86         | 4.91        | 1.64           | 2.30           | 1.63      |
| RepoNavigator   | Training Free | 9.84         | 10.67           | <u>14.65</u>    | <u>9.20</u>  | 30.50       | 37.24          | 31.86          | 28.82     |
| RepoNavigator   | GRPO          | **12.33**    | **14.29**       | **21.26**       | **12.02**    | 36.36       | **39.74**      | **48.13**      | **36.36** |
| **Qwen2.5-14B** |               |              |                 |                 |              |             |                |                |           |
| LocAgent        | Training Free | 6.22         | 0.13            | 3.65            | 2.65         | 15.58       | 11.69          | 0.21           | 9.53      |
| COSIL           | Training Free | 10.73        | 4.67            | 5.96            | 3.94         | 34.31       | 9.97           | 14.81          | 9.30      |
| Agentless       | Training Free | 10.49        | 6.75            | 7.41            | 5.28         | 41.42       | 19.02          | 13.42          | 12.37     |
| RepoSearcher    | Training Free | 2.79         | 1.69            | 1.38            | 1.14         | 17.37       | 7.60           | 5.17           | 4.84      |
| RepoNavigator   | Training Free | 14.36        | 19.74           | <u>18.06</u>    | <u>12.00</u> | 43.57       | 46.85          | 54.52          | 49.72     |
| RepoNavigator   | GRPO          | **16.05**    | **25.25**       | **25.25**       | **14.58**    | **46.06**   | **58.64**      | **41.07**      | **45.14** |
| **Qwen2.5-32B** |               |              |                 |                 |              |             |                |                |           |
| LocAgent        | Training Free | 8.72         | 4.30            | 0.17            | 2.90         | 25.73       | 19.77          | 0.38           | 16.50     |
| COSIL           | Training Free | 15.00        | 8.14            | 6.35            | 5.21         | 45.37       | 19.42          | 13.04          | 12.36     |
| Agentless       | Training Free | 11.08        | 7.31            | 7.98            | 5.80         | 43.07       | 13.89          | 20.07          | 13.11     |
| RepoSearcher    | Training Free | 2.00         | 1.29            | 1.45            | 1.00         | 13.51       | 3.43           | 5.31           | 3.24      |
| RepoNavigator   | Training Free | 13.96        | 15.36           | <u>20.25</u>    | <u>12.87</u> | 50.24       | 63.24          | 53.48          | 48.50     |
| RepoNavigator   | GRPO          | **18.13**    | **29.44**       | **20.72**       | **17.16**    | **53.49**   | **57.57**      | **68.69**      | **52.44** |

### 4.3. Training Strategy Comparison

> **Figure 3 Description:** Ablation study: comparison between RepoNavigator with training free, RFT, GRPO with pure outcome and hybrid reward on Qwen2.5-7B-Instruct. The chart shows that RL with hybrid reward achieves the highest scores across most metrics (Function/File Recall, Precision, F1, IoU) compared to Training free, SFT, and RL without hybrid reward.

Directly training with GRPO outperforms RFT-only and RFT+GRPO. When the pretrained model is strong enough and data is high-quality, directly training a model with RL is better than training after SFT (RFT) as its cold start. Reinforcement learning with hybrid reward (with tool-calling success rate) has higher performance than pure outcome reward.

### 4.4. Scaling Law of Tool-Calling

> **Figure 4 Description:** Scaling law of tool-calling, where Pre and Post denote the corresponding metric before and after the RL training. The four line graphs (Function Level DICE/IoU, File Level DICE/IoU) show that metrics consistently improve as the number of Turns increases (from 2 to 14), both for Pre-training and Post-training models.

Allowing more tool-calling turns consistently leads to improved performance for RepoNavigator, both before and after reinforcement learning (RL) training.

### 4.5. Influence on Issue Resolution

We test RepoNavigator against baselines on SWE-bench_Verified by applying the repairing phrase of Agentless while replacing its localization front-end.

**Table 3.** We use Qwen2.5-14B-Instruct as the localization model and Qwen2.5-32B-Instruct as the repair model on SWE-bench Verified.

| Agent Pipeline   | Func-IoU(%) | Resolved(%) |
| ---------------- | ----------- | ----------- |
| Agentless        | 5.28        | 10.12       |
| LocAgent         | 2.65        | 13.01       |
| RepoNavigator    | 12.00       | 14.74       |
| RepoNavigator+RL | 14.58       | 15.03       |

Compared with baselines, RepoNavigator has the highest performance on issue resolution.

---

## 5. Discussion: Building Less yet More Capable Tools

### 5.1. Impact on the Action Space of Agents

When only a single tool—specifically the jump tool—is retained, the system's structural relations become simpler, as both the action space and the observation space are restricted to what this tool can access. This reduction is generally beneficial, since additional tools often introduce new and unfamiliar interfaces.

### 5.2. Impact on Tool-Calling Success Rate

For a task that requires sequential tool invocations, the overall success rate can be expressed as . Therefore, completing a task with a single, more versatile tool tends to be more reliable than relying on multiple narrow-scope tools executed in sequence.

### 5.3. Impact on the Prediction Space

The access scope of a jump tool starts from a given entry point and recursively resolves all referenced symbols. Consequently, when computing the Intersection over Union (IoU) between the prediction set and the groundtruth set, using the jump tool results in a higher IoU.

> **Figure 5 Description:** Venn graph illustrating access scope of jump. Compared with the repository scope, the access scope has a much higher IoU with the groundtruth set. The image depicts three concentric circles: "Repo Scope" (outermost), "Access Scope" (middle), and "GT" (Ground Truth, innermost), showing that the Access Scope is a subset of Repo Scope and contains the GT.

### 5.4. Verification

We change the tool set of RepoNavigator and conduct RL training with only the outcome reward, adding excessive tools used in previous works.

**Table 4.** We change the tool set of RepoNavigator and present the function-level IoU (%) on Qwen2.5-7B-Instruct.

| Jump | GetClass | GetFunc | GetStruc | IoU   |
| ---- | -------- | ------- | -------- | ----- |
| X    | ✓        | ✓       | ✓        | 13.71 |
| ✓    | X        | X       | X        | 21.44 |
| X    | X        | X       | ✓        | 24.00 |
| ✓    | X        | X       | X        | 24.28 |

The results clearly imply that additional tools do not increase the model's performance.

---

## 6. Conclusion

In this work, we introduced RepoNavigator, a repository-level issue localization agent that departs from existing multi-tool paradigms by leveraging a single, more-capable **jump** tool for symbol resolution. Through tool-integrated GRPO, RepoNavigator learns to reason, invoke tools, and refine its predictions in a closed-loop manner. Extensive experiments demonstrate that RepoNavigator achieves state-of-the-art localization performance. Our findings highlight the importance of aligning agent tooling with real execution structure.

---

## References

1. Ahn, J., et al. Large language models for mathematical reasoning: Progresses and challenges. arXiv preprint arXiv:2402.00157, 2024.

2. Anthropic. Claude 3.7 sonnet and claude code. 2025.

3. Chen, Z., et al. LocAgent: Graph-guided LLM agents for code localization. ACL 2025.

4. Guo, D., et al. Deepseek-coder: When the large language model meets programming. arXiv:2401.14196, 2024.

5. Guo, T., et al. Large language model based multi-agents: A survey. arXiv:2402.01680, 2024.

6. Gupta, T. and Kembhavi, A. Visual programming: Compositional visual reasoning without training. CVPR 2023.

7. He, Z., et al. Swe-swiss: A multi-task fine-tuning and rl recipe for high-performance issue resolution. 2025.

8. Hong, W., et al. Cogagent: A visual language model for gui agents. CVPR 2024.

9. Huang, X., et al. Understanding the planning of llm agents: A survey. arXiv:2402.02716, 2024.

10. Hui, B., et al. Qwen2.5-coder technical report. arXiv:2409.12186, 2024.

11. Jiang, Z., et al. Cosil: Software issue localization via llm-driven code repository graph searching. arXiv:2503.22424, 2025.

12. Jimenez, C. E., et al. Swe-bench: Can language models resolve real-world github issues? arXiv:2310.06770, 2023.

13. Jin, B., et al. Search-r1: Training llms to reason and leverage search engines with reinforcement learning. arXiv:2503.09516, 2025.

14. Kwon, W., et al. Efficient memory management for large language model serving with pagedattention. SOSP 2023.

15. Li, Y., et al. Personal llm agents: Insights and survey. arXiv:2401.05459, 2024.

16. Liu, A., et al. Deepseek-v3 technical report. arXiv:2412.19437, 2024.

17. Liu, Z., et al. Dynamic llm-agent network. arXiv:2310.02170, 2023.

18. Lu, J., et al. Toolsandbox: A stateful, conversational, interactive evaluation benchmark for llm tool use capabilities. arXiv:2408.04682, 2024.

19. Luo, M., et al. Deepswe: Training a state-of-the-art coding agent from scratch by scaling rl. 2025.

20. Ma, Z., et al. Tool-integrated reinforcement learning for repo deep search. arXiv:2508.03012, 2025.

21. Schmidgall, S., et al. Agent laboratory: Using llm agents as research assistants. arXiv:2501.04227, 2025.

22. Shen, Z. Llm with tools: A survey. arXiv:2409.18807, 2024.

23. Team, Q. Qwen2 technical report. arXiv:2407.10671, 2024.

24. Wang, X., et al. Openhands: An open platform for AI software developers as generalist agents. ICLR 2025.

25. Wang, Y., et al. Extracting conceptual knowledge to locate software issues. arXiv:2509.21427, 2025.

26. Xia, C. S., et al. Agentless: Demystifying llm-based software engineering agents. arXiv:2407.01489, 2024.

27. Yan, Y., et al. Mathagent. arXiv:2503.18132, 2025.

28. Yang, A., et al. Qwen3 technical report. arXiv:2505.09388, 2025.

29. Yang, J., et al. SWE-agent. NeurIPS 2024.

30. Yang, J., et al. Swe-bench multimodal. arXiv:2410.03859, 2024.

31. Yang, J., et al. Swe-smith: Scaling data for software engineering agents. arXiv:2504.21798, 2025.

32. Yu, Q., et al. Dapo: An open-source llm reinforcement learning system at scale. arXiv:2503.14476, 2025.

33. Yu, Z., et al. Orcaloca: An llm agent framework for software issue localization. 2025.

34. Yuan, S., et al. Easytool: Enhancing llm-based agents with concise tool instruction. arXiv:2401.06201, 2024.

35. Yue, Y., et al. Vapo: Efficient and reliable reinforcement learning for advanced reasoning tasks. arXiv:2504.05118, 2025.

---

## A. Detailed Illustration of Baselines

- **Agentless:** A workflow for issue localization identifying suspicious files, then classes/functions, then precise locations.

- **COSIL:** Conducts file-level then function-level localization using call graphs and context pruning.

- **LocAgent:** A graph-guided agent using multiple tools and a planning prompt.

- **RepoSearcher:** Uses file-level then function-level localization; introduced ToolTrain framework with distillation.

- **Ours:** The first fully-automatic agent with no fixed workflow, trained directly from pretrained models using a single tool.

## B. Experimental Details

- **Hyperparameters:** Clip ratio 0.2/0.8, LR , Batch size 128, Temp 1.0, Max tool calls 12, Max response length 10240.

- **Metrics Calculation:**
- (7)

- (Eq 8 implied)
- (9)

- (10)

- **Implementation:** When response exceeds max length or tool calls, the agent is forced to stop. If total failure, scores are zero.

**Table 5.** We change the tool set of RepoNavigator and present the function-level IoU. Because the jump tool is already powerful enough, excessive tools do not increase its performance.

| Jump | GetClass | GetFunc | GetStruc | Recall | Precision | F1    | IoU   |
| ---- | -------- | ------- | -------- | ------ | --------- | ----- | ----- |
| ✓    | ✓        | ✓       | ✓        | 14.28  | 15.44     | 14.40 | 13.71 |
| X    | X        | X       | X        | 22.60  | 25.02     | 22.80 | 21.44 |
| X    | X        | X       | X        | 24.64  | 27.48     | 25.05 | 24.00 |
| ✓    | X        | X       | X        | 25.11  | 29.16     | 25.75 | 24.28 |

## C. Threats to Validity

- **Groundtruth Retrieval:** We extract modified locations directly from the gold patch, which may ignore alternative correct patches.

- **Language Limit:** We only evaluate Python repositories due to language server implementation constraints.

## D. Case Study

(See section content in source regarding full trajectory on `astropy_astropy-12907` ).

---

## Prompt Template

```json
[system]
You are Qwen, created by Alibaba Cloud. You are a helpful assistant.
# Tools
You may call one or more functions to assist with the user query.
You are provided with function signatures within <tools></tools> XML tags:
<tools>
{"type": "function", "function": {"name": "check", "description": "In the specific file path, a symbol is referred and this tool can find where the tool is defined. For instance, in the first turn, file_path is the entry point of.",
"parameters": {"properties": {"symbol": {"description": "The symbol whose definition code will be given to the agent.", "type": "string"}, "file_path":
{"description": "The relevant path to the file where the symbol is referred.", "type": "string"}}, "required": ["symbol", "file_path"], "type": "object"}}}
</tools>
For each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:
<tool_call>
{"name": <function-name>, "arguments": <args-json-object>}
</tool_call>

[user]
You are given a codebase and an issue, you need to locate the files and functions causing this issue.
You can call the tool to check the definition code of a symbol.
You can only check the symbol once for each turn.
The 'file_path' is the relevant path of where the symbol is called, NOT where it is defined!
For instance, if 'classA.functionB' is what you want to check (which is called in fileA.py), you should directly check 'functionB' in 'fileA.py'.
This is the issue:
[Problem Statement]
The entry file of the code base is:
[Relevant Path To Entry Point]
[Entry Point]
Your final answer should be all functions that should be modified, such as:
relevant/path/to/file1.py::func_name1, relevant/path/to/file2.py::func_name2,
...(a series of file::function pairs seperated by comma)
Please put your final answer inside \boxed{} only in the last turn.
You can only call the tool once each turn.
For instance:
{'name': 'check', 'arguments': {'symbol': 'symbol_to_be_checked', 'file_path': 'file_where_the_symbol_is_used'}}

```
