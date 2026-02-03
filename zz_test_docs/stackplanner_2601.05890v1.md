## STACKPLANNER: A Centralized Hierarchical Multi-Agent System with Task-Experience Memory Management

Ruizhe Zhang, Xinke Jiang, Zhibang Yang, Zhixin Zhang, Jiaran Gao,Yuzhen Xiao, Hongbin Lai, Xu Chu, Junfeng Zhao, Yasha WangSchool of Computer Science and School of Software & Microelectronics, Peking UniversityKey Laboratory of High Confidence Software Technologies, Ministry of EducationNational Engineering Research Center For Software Engineering, Peking University

6Big Data Technology Research Center, Nanhu Laboratory

### Abstract

Multi-agent systems based on large language models, particularly centralized architectures, 

memory control. STACKPLANNER addresses 

task-level memory control, and by learning to 

demonstrate the effectiveness of our approach in enabling reliable long-horizon multi-agent collaboration.

## 1 Introduction

Large Language Model-based multi-agent systems (LLM-MAS) have emerged as an effective paradigm 

By enabling task decomposition, parallel exploration,

*All authors listed contributed equally to this work. Ruizhe

Zhang and Xinke Jiang led the design and implementation of the STACKPLANNER framework, including hierarchical action space, task-level memory, and reinforcement learning (RL) training architecture. Jiaran Gao and Xinke Jiang were

1,2,6†

2,3,4†

and collaborative reasoning, these systems have been 

Qian et al., 2024). Prior work has explored a variety of designs, including decentralized collaboration (Yang 

reasoning pipelines (Yao et al., 2023). However, as system scale and task complexity increase, ensuring reliable multi-agent collaboration over long-horizon, information-intensive, and cross-task scenarios remains a central dilemma (Guo et al., 2024). Decentralized and debate-based approaches provide flexibility and robustness but often suffer from high communication 

2025). To mitigate these issues, most studies adopt a 

et al., 2025). Despite its advantages, most centralized multi-agent 

chains can overwhelm the central agent's processing 

especially pronounced in novel domains or tasks with, 2024). little prior experience. Crucially, both issues stem from 

memory. Addressing this deficiency gives rise to two

key challenges: ❶Challenge 1. How can the central agent's task mem-

## Peking University Information Technology Institute (Tianjin Binhai)

Center on Frontiers of Computing Studies, Peking University

{nostradamus, xinkejiang, yangzb}@stu.pku.edu.cn, {chu_xu, zhaojf, wangyasha}@pku.edu.cn

responsible for data construction and training within the RL framework. Zhibang Yang and Yuzhen Xiao designed and implemented the sub-agents. Zhixin Zhang and Hongbin Lai developed the experience memory module and its retrieval mechanism.

Corresponding author.

ory be effectively managed to mitigate contextual noise and memory bloat, ensuring stable decision-making over long-horizon tasks? As tasks unfold, information from multiple sub-agents is often redundant or noisy, yet it is indiscriminately appended to the central agent's


---

## 2 Methodology

task memory. Early errors or noise in sub-tasks or tool invocations can propagate across long-horizon steps, causing the central agent to become lost in the middle of 

methods largely rely on passive memory management strategies, such as template-based summarization ( et al., 2021) or heuristic truncation (Liu et al. 

active control over its memory state, the central agent's performance deteriorates significantly as reasoning steps increases.

❷Challenge 2. How can valuable historical trajectories (Experience Memory) of the central agent be effectively leveraged to improve task planning and co-ordination across new tasks? When tackling new tasks, the central agent often starts from scratch, with erence to prior successful coordination experiences Although its decision-making is critical to overall system performance, LLMs are rarely trained for long-horizon, cross-agent reasoning, limiting their ability to

plan complex tasks effectively. As a result, systems frequently exhibit poor cold-start performance ( 2023, 2025a) and limited cross-task generalization ( et al., 2025b).

To address these challenges, we construct a archical Multi-Agent System - STACKPLANNER, centered on a coordinator, explicitly supporting the 

coordinator's high-level decision-making from the execution details handled by specialized sub-agents By strictly separating the memory of the coordinator 

task memory, thereby alleviating cognitive and memory 

condense, and prune task-relevant information 

enhance decision-making stability over long-horizon 

valuable cross-task coordination experiences, including factual knowledge and procedural memory. This allows 

patterns to improve planning, delegation, and coordina- content for refined textual outputs.

As shown in Figure 1, STACKPLANNER follows a hierarchical multi-agent design. A central coordinator is responsible for high-level decision making, including planning, subtask delegation, and active memory operations, while specialized sub-agents handle concrete task execution. Moreover, the coordinator operates , 2023), over a task memory that maintains a concise execution trace, and leverages a structured experience memory that stores reusable knowledge and coordination experience across tasks, which directly address C1 and C2.

### 2.1 Hierarchical Coordination

Central Coordinator Action Space The central co-ordinator operates over a compact discrete action space: A = {PLAN, DELEGATE, REVISE}. Here PLAN determines the next coordination step based on task memory. DELEGATE assigns a scoped subtask little refto a selected sub-agent, together with task requirements.

and relevant contextual information. REVISE actively optimizes task memory via condensation and pruning. This action space keeps the coordinator focused on global progress, ensuring that system-wide behavior remains strictly task-oriented. Implementation details Li et al., of central coordinator are deferred to Appendix C. Li Specialized Sub-Agents Moreover, despite central coordinator, we also incorporate specialized sub-agents: Hier- •Search Agent: conducts key information retrieval via external tools, following ReAct reasoning paradigm for iterative information gathering and organization; •Report Agent: adapts its behavior to the assigned subtask, either organizing previous information into structured task reports to support subsequent coordina-. 

### 2.2 Active Task Memory Management

The coordinator maintains a lightweight task memory 

memory stack mechanisms supports three operations: •Update: All task execution information-including task specifications, coordinator action messages, and sub-agent inputs and outputs-is sequentially pushed onto the stack. •Condensation: When the coordinator determines that the memory becomes verbose or that a task stage has been completed, REVISE performs memory con-

tion across new tasks. To further enhance, we model the full planning process as a learnable decision process and train the coordinator exclusively via reinforcement 

densation by popping a contiguous segment{m}i *t* 

This operation preserves task-relevant information while reducing redundant context.


---

•Pruning: When the coordinator detects unproductive or erroneous exploration, REVISE performs memorytured memory stack. Given a queryq ∼ D, the policy pruning by removing a selected segment of memory entries from the stack. Additionally, a concise record 

By exposing memory as an explicit control target, 

errors with minimal overhead. Implementation details of REVISE are deferred to Appendix C.

### 2.3 Structured Experience Memory Utilization

To support cross-task generalization, we maintain a structured experience memory that stores persistent information beyond individual task executions. The experience memory consists of three complementary components: (i) user profiles, which capture stable user attributes and preference signals; (ii) semantic memory 

procedural patterns. These components are organized 

formats and prompting details, are in Appendix 

mitigate cold-start issues.

TACKPLANNER framework.

with access to an external search engine and a struc-

*Taction steps, and the RL objective with search engine* invocations and memory stack operations is defined as: max E *q∼D, y∼π* (·|q;R,M) [r(q, y)]ϕ

where RandM denotes search engine and stackstructured memory respectively,ris the reward func-ϕ tion, andπ is the frozen reference policy. Unlikeref standard RLHF (Schulman et al., 2017) or retrievalaugmented RL methods such as Search-R1 (Jin et al.,

2025), which largely rely on parametric knowledge and 

only on information obtained through retrieval or rea-, soned and kept in the memory stack. 

to optimize the policy, which eliminates the need for 

 C.(k)K{y } sampled from the old policyπ, whereθ

(k) (k) (k)

of generated tokens, let Rdenote the set of all token-G level rewards{r}across the group. For each token *i* 

Reinforcement Learning Formulation We formulate training STACKPLANNER's coordinator as a multi-step RL problem, where the policy model is augmented

ˆ(k) (k)

*Ai*=*ri*− mean(R )/std(R). (2)GG

1In our implementation, each high-level actionais real-t ized as a contiguous sequence of generated tokens


---

The GRPO optimization objective is then defined as:

X*K* |yX |

(k) ˆ(k)

Clip˜z,A *i i*

*K* |y|(k) *k=1 i=1*

− βD*,KL*

(k) ˆ(k) (k) (k)ˆ (k)

andClip˜z,A *i i* = min˜zA *i i, clip(˜z, 1±i*

(k) (k)

ˆ(k)

(k) *π(xθi*|q,x ;R,M)*<i*

*ε)A i*, importance ratio˜z= *i* (k) (k)

*πθ* (x*i*|q,x ;R,M)*<i*

denotes the probability ratio at the token level. Term DKL(π∥π )constrains the updated policy to remain*θ*ref

close to a frozen reference policyπ. Notably, all re-ref

wards, advantages, and policy updates in our framework are defined at the action level and applied at

## 3 Experiment

### 3.1 Experimental Setup

❶Evaluation Benchmarks. We evaluate our method on ten benchmarks spanning two settings: multi-hop QA (2Wiki(Ho et al., 2020), MusiQue (Trivedi et al. and agentic benchmarks (GAIA (Mialon et al., 2023) 

Agentic-RL paradigms. Specifically, Naive baselines include Base and FS-RAG (Trivedi et al., 2023). Single- Agent approaches consist of ReAct (Yao et al., 2022) and IRCoT (Trivedi et al., 2023). For Multi-Agent we consider both centralized architectures, including OWL (Hu et al., 2025), and automated architectures such as MacNet (Qian et al.) and AFlow (Zhang et al. Finally, Agentic-RL baselines include ReSearch ( 

are provided in Appendix❸ BRAG Tools. We use a Wikipedia-based search tool (snapshot: November 1,

2023. and Bocha for web search.

### 3.2 Main Result Analysis

Strong Performance Compared with Baselines. Our method achieves state-of-the-art performance across all benchmarks, surpassing baselines in multi-hop QA 

and FRAMES), with F1 scores of 16.48%, 7.71%, and 16.23% for Qwen2.5-3B, and 22.01%, 9.45%, and

19.44% for Qwen2.5-7B, respectively. GAIA is the most challenging benchmark due to its multi-step reasoning and memory demands; baselines such as MacNet fail to complete reasoning because they cannot effectively

### 3.3 Component Analysis

Model Component Ablation. We conduct ablation experiments to evaluate the contributions of the task memory and experience memory modules in our model. Removing the task memory leads to a drop of 3.02%, 5.72%, 3.03%, and 2.70%points on 2WikiMultiHopQA, 

7.49%, 2.18%, and 8.54%points, while removing both memory components results in the largest performance degradation, with F1 scores dropping by 15.80%, 9.05%, 5.24%, and 9.90%points across the same datasets. These results demonstrate that both task and experience 

 token level.effect is essential for achieving optimal performance.

## 4 Conclusion and Future Work

In this paper, we present STACKPLANNER, a hierarchical centralized multi-agent framework that treats memory as an explicit control target for coordination. By combining decoupled coordination with active task, 2022)), memory management and reusable experience memory, STACKPLANNER mitigates context bloat and error

propagation in long-horizon collaboration. Moreover, high-level coordination and memory control are jointly learned via reinforcement learning. Experiments on, and deep-search and agent system benchmarks demonstrate more stable coordination and stronger generalization. 

abstractions may further improve decision robustness 

PLANNER to broader domains and more open-endedChen real-world agentic settings, including deep research and long-horizon analytical workflows.

manage task memory, resulting in missing scores ("/"), 

strong results across both 3B and 7B backbones.

, GAIA,


---

Method

Qwen2.5-3B

Paradigm Approach 2Wiki MusiQue

Base 23.98 9.70 5.70 8.01

Naive

FS-RAG 15.47 7.64 4.30 10.42 ReACT 25.09 13.92 4.78 10.53

Single-Agent

IRCoT 15.89 12.43 2.77 6.79 OWL 17.39 14.81 3.28 13.49

Multi-Agent MacNet 25.20 13.19 / 11.92

AFlow 24.56 13.07 2.57 12.13 ReSearch 27.23 9.47 4.48 10.00

Agentic-RL ARPO 29.55 13.38

Ours 32.92 16.48 7.71 16.23

Method

2Wiki Musique GAIA FRAMES

Ours

32.92 16.48 7.71 16.23

> Table 1: Performance comparison (F1, %) on multi-hop QA benchmarks ( w/o Task memory 29.90 10.76 4.68 13.53 w/o Experience memory28.47 8.99 5.53 7.69
>

FRAMES) across different paradigms using Qwen2.5-3B w/o Both memories 17.12 7.43 2.47 6.33

could not produce results on a dataset, and bold highlights the best performance in each column.

> Table 2: Ablation analysis of component and reward designs in STACKPLANNER on Qwen2.5-3B.
>

Qwen2.5-7B

GAIA FRAMES 2Wiki MusiQue GAIA FRAMES

25.41 12.15 4.29 12.52 17.71 10.74 5.02 12.52 27.51 19.34 6.37 15.29 36.45 8.39 5.50 6.78 29.73 17.66 5.39 14.68 28.19 17.81 / 12.61 30.53 18.15 4.72 12.81 30.03 12.58 4.43 15.61

 7.71 13.49 30.71 12.71 8.56 12.18

38.34 22.01 9.45 19.44

2WikiMultiHopQA, MusiQue, GAIA, and

 and Qwen2.5-7B. The symbol "/" indicates that a model

llms via reinforcement learning. arXiv preprint

arXiv:2503.19470.

Shuaihang Chen, Yuanxing Liu, Wei Han, Weinan Zhang, and Ting Liu. 2025b. A survey on llm-based 

Yu Cui, Hang Fu, Haibin Zhang, Licheng Wang, and 

Guanting Dong, Hangyu Mao, Kai Ma, Licheng Bao, 

others. 2025. Agentic reinforced policy optimization.

arXiv preprint arXiv:2507.19849. Zi-Yi Dou, Pengfei Liu, Hiroaki Hayashi, Zhengbao 

the North American Chapter of the Association for 

Computational Linguistics. Yilun Du, Shuang Li, Antonio Torralba, Joshua B. Tenenbaum, and Igor Mordatch. 2023. Improving factuality and reasoning in language models through multiagent debate. Preprint, arXiv:2305.14325. Jiaxuan Gao, Wei Fu, Minyang Xie, Shusheng Xu, Chuyi He, Zhiyu Mei, Banghua Zhu, and Yi Wu. 2Wiki-

                      2025. Beyond ten turns: Unlocking long-horizon

agentic search with large-scale asynchronous rl. Preprint, arXiv:2508.07976. Taicheng Guo, Xiuying Chen, Yaqi Wang, Ruidi Chang, Shichao Pei, Nitesh V. Chawla, Olaf Wiest, and Xi-

## References

Mingyang Chen, Linzhuang Sun, Tianpeng Li, Haoze Sun, Yijie Zhou, Chenzheng Zhu, Haofen Wang, 

angliang Zhang. 2024. Large language model based multi-agents: A survey of progress and challenges. Preprint, arXiv:2402.01680. Xanh Ho, Anh-Khoa Duong, Quoc-Huy Nguyen, and Suong Nguyen. 2020. Constructing a multi-hop qa dataset for comprehensive evaluation of reasoning steps. In COLING.

## Limitations

Despite the promising results, our framework does have 

becomes cumbersome and error-prone.❷Cold-start challenges in long-term memory. Long-term memory mechanisms still suffer from cold-start issues, where insufficient prior experience limits their effectiveness in early stages. While simulated users can be introduced 

when transferred to real or diverse user behaviors.

## Ethical considerations

All experiments in this study were conducted solely on publicly available benchmark datasets, including 

in the research.


---

Sirui Hong, Mingchen Zhuge, Jiaqi Chen, Xiawu Zheng, Yuheng Cheng, Ceyao Zhang, Jinlin Wang, Zili Wang, Steven Ka Shing Yau, Zijuan Lin, Liyang Zhou, Chenyu Ran, Lingfeng Xiao, Chenglin Wu, 

Preprint, arXiv:2308.00352. Xinming Hou, Mingming Yang, Wenxiang Jiao, Xing Wang, Zhaopeng Tu, and Wayne Xin Zhao. 2024. Coact: A global-local hierarchy for autonomous agent collaboration. Preprint, arXiv:2406.13381. Mengkang Hu, Yuhang Zhou, Wendong Fan, Yuzhou 

arXiv:2505.23885.

Xinke Jiang, Yue Fang, Rihong Qiu, Haoyu Zhang, Yongxin Xu, Hao Chen, Wentao Zhang, Ruizhe Zhang, Yuchen Fang, Xu Chu, and 1 others. 2024. Tc-rag: Turing-complete rag's case study on medical llm systems. arXiv preprint arXiv:2408.09199 Bowen Jin, Hansi Zeng, Zhenrui Yue, Jinsung Yoon, Sercan Arik, Dong Wang, Hamed Zamani, and Jiawei Han. 2025. Search-R1: Training LLMs to reason and leverage search engines with reinforcement learning.

arXiv preprint arXiv:2503.09516. 

34(8):1302-1304.

Kalpesh Krishna and 1 others. 2024. Retrieval augmented generation for long-context question answering with frames. arXiv preprint arXiv:2409.12941 Annan Li, Chufan Wu, Zengle Ge, Yee Hin Chong, Zhinan Hou, Lizhe Cao, Cheng Ju, Jianmin Wu, Huaiming Li, Haobo Zhang, Shenghao Feng, Mo Zhao, Fengzhi Qiu, Rui Yang, Mengmeng Zhang, Wenyi Zhu, Yingying Sun, Quan Sun, Shunhao Yan, and 3 others. 2025a. The fm agent. Preprint

arXiv:2510.26144.

Huao Li, Yu Chong, Simon Stepputtis, Joseph Campbell, Dana Hughes, Charles Lewis, and Katia Sycara.

2023. Theory of mind for multi-agent collaboration

via large language models. In Proceedings of the 2023 Conference on Empirical Methods in Natural 

Yilong Li, Chen Qian, Yu Xia, Ruijie Shi, Yufan Dang, Zihao Xie, Ziming You, Weize Chen, Cheng Yang, Weichuan Liu, Ye Tian, Xuantang Xiong, Lei Han,

Liang. 2023. Lost in the middle: How language models use long contexts. Preprint, arXiv:2307.03172. Xiang Liu, Peijie Dong, Xuming Hu, and Xiaowen Chu. 2024. Longgenbench: Long-context generation benchmark. Preprint, arXiv:2410.04199.. Grégoire Mialon and 1 others. 2023. Gaia: A benchmark for general ai assistants. arXiv preprint

arXiv:2311.12983.

Chen Qian, Wei Liu, Hongzhang Liu, Nuo Chen, Yufan Dang, Jiahao Li, Cheng Yang, Weize Chen, Yusheng Su, Xin Cong, Juyuan Xu, Dahai Li, Zhiyuan Liu, 

arXiv:2307.07924.

Chen Qian, Zihao Xie, YiFei Wang, Wei Liu, Kunlun Zhu, Hanchen Xia, Yufan Dang, Zhuoyun Du, Weize Chen, Cheng Yang, and 1 others. Scaling large language model-based multi-agent collaboration. In The Thirteenth International Conference on Learning Representations. .John Schulman, Filip Wolski, Prafulla Dhariwal, Alec Radford, and Oleg Klimov. 2017. Proximal policy optimization algorithms. arXiv preprint

arXiv:1707.06347.

Zhihong Shao, Peiyi Wang, Qihao Zhu, Runxin Xu, Junxiao Song, Xiao Bi, Haowei Zhang, Mingchuan 

arXiv:2402.03300.

Harsh Trivedi, Niranjan Balasubramanian, Tushar Khot, and Ashish Sabharwal. 2022. Musique: Multihop reasoning dataset with explanation. arXiv preprint

arXiv:2108.00573.

. Harsh Trivedi, Niranjan Balasubramanian, Tushar Khot, and Ashish Sabharwal. 2023. Interleaving retrieval 

10014-10037., Yuanfei Wang, Fangwei Zhong, Jing Xu, and Yizhou Wang. 2022. Tom2c: Target-oriented multi-agent communication and cooperation with theory of mind. Preprint, arXiv:2111.09189. Qingyun Wu, Gagan Bansal, Jieyu Zhang, Yiran Wu, Beibin Li, Erkang Zhu, Li Jiang, Xiaoyun Zhang, Shaokun Zhang, Jiale Liu, Ahmed Hassan Awadallah, Ryen W White, Doug Burger, and Chi 

arXiv:2308.08155.

Zhiyuan Liu, and Maosong Sun. 2025b. Cross-task 

Yingxuan Yang, Huacan Chai, Shuai Shao, Yuanyi Song, Siyuan Qi, Renting Rui, and Weinan Zhang.

                      2025. Agentnet: Decentralized evolutionary coordi-

nation for llm-based multi-agent systems. Preprint,

arXiv:2504.00587.


---

## A Experiment Datasets

Shunyu Yao, Dian Yu, Jeffrey Zhao, Izhak Shafran, Thomas L. Griffiths, Yuan Cao, and Karthik Narasimhan. 2023. Tree of thoughts: Deliberate prob- lem solving with large language models. Preprint,Followed by (Gao et al., 2025), We train our models

arXiv:2305.10601.

Shunyu Yao, Jeffrey Zhao, Dian Yu, Nan Du, Izhak Shafran, Karthik Narasimhan, and Yuan Cao. 2022. React: Synergizing reasoning and acting in language models. arXiv preprint arXiv:2210.03629. Yanwei Yue, Guibin Zhang, Boyang Liu, Guancheng Wan, Kun Wang, Dawei Cheng, and Yiyan Qi. 2025. Masrouter: Learning to route llms for multi-agent

systems. Preprint, arXiv:2502.11133. Jiayi Zhang, Jinyu Xiang, Zhaoyang Yu, Fengwei Teng, Xiong-Hui Chen, Jiaqi Chen, Mingchen Zhuge, Xin Cheng, Sirui Hong, Jinlin Wang, and 1 others. Aflow: Automating agentic workflow generation. In 

A.1 Training Dataset. 

MultiHopQA (Ho et al., 2020). To focus on genuinely non-trivial reasoning scenarios, we filter out instances that require no external retrieval or can be solved with only a single, trivial retrieval step. A.2 Testing set. Dataset Train Dev Test 2Wiki 154,87812,576 12,576 MuSiQue 19,938 2,417 2,459 GAIA 0 0

> Table 3: Overview of datasets used in experiments.
>

| We | evaluate | our | approach | on | four | widely | used | bench- |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| test splits are summarized in Table | 3. |  |  |  |  |  |  |  |

that require reasoning over multiple documents: •2WikiMultiHopQA (Ho et al., 2020). Constructed from Wikipedia and Wikidata, this dataset contains

192,606 question-answer pairs. It includes 154,878 

evidence across multiple sources. 

data, with each reasoning step depending critically on the previous step. The dataset comprises 19,938 training, 2,417 development, and 2,459 test examples. ❷ Agentic Benchmarks. We further assess our 

•FRAMES (Krishna et al., 2024). FRAMES consists 

127

## B Baseline Implementation Details

We compare our method with baselines from four paradigms (Naive, Single-Agent, Multi-Agent, Agentic- RL) spanning different reasoning and coordination


---

strategies. Implementation details for each paradigms are described below. 

•FS-RAG (Trivedi et al., 2023). FS-RAG retrieves evidence at the sentence level, treating each input sentence independently as a query. 

prompting, without coordination between agents. 

external tools such as search engines. •IRCoT (Trivedi et al., 2023). IRCoT alternates between retrieval and chain-of-thought reasoning, 

❸Multi-Agent. Multi-Agent baselines decompose 

orchestration strategies. •MacNet (Qian et al.). MacNet is an automated 

refinement while mitigating context explosion. 

planning from specialized execution, using a reinforcement-learned, domain-agnostic planner to enable efficient cross-domain transfer. •AFlow (Zhang et al.). AFlow is an automated agent orchestration framework that employs Monte Carlo Tree Search (MCTS) to explore and optimize 

•ReSearch (Chen et al., 2025a). ReSearch jointly optimizes reasoning and search behaviors via RL, without supervision on intermediate steps. •ARPO (Dong et al., 2025). ARPO employs an 

diverse and effective tool usage.


---

## C Prompts

In this section, we provide a detailed introduction to the prompts used in our framework. STACKPLANNER Central Coordinator System Prompt

You are an intelligent central agent responsible for managing a multi-agent system. You not only make decisions but also execute five key actions: PLAN, REFLECT, SUMMARIZE, DELEGATE, and FINISH (specific details for each action are provided below). Your role is critical for ensuring the stable operation and coordinated execution of the entire multi-agent system. Current System State

| • Current Node: {{current_node}} | • Current Action: {{current_action}} | • Memory History: | {{memory_stack}} |
| --- | --- | --- | --- |
| • Available Actions: {{available_actions}} | Description: |

- PLAN = Reason about the current situation, analyze it, and clarify what should be done next - REFLECT = Reflect on previous step and POP several no-longer-used items from the memory stack - SUMMARIZE = Condense long histories - DELEGATE = Assign to sub-Agent - FINISH = Terminate the task only when all subtasks are completed and user requirements are fully satisfied

| • Available Sub-Agents: {{available_sub_agents}} | (Description: {{sub_agents_description}}) |
| --- | --- |
| {% endif %} |
| {% endif %} | Current Progress: {{current_progress}} |
| {% endif %} | Decision Reasoning: {{decision_reasoning}} |
| {% endif %} | Current Instruction: {{instruction}} |
| {% endif %} | Summarization Focus: {{summarization_focus}} |

{% if current_action == "summarize" or current_action == "reflect" or current_action == "plan" While the step is PLAN, SUMMARIZE, or REFLECT, provide detailed analysis in natural language format with the same language as the user query: •For PLAN: Analyze the current situation comprehensively, break down complex problems, identify key factors, and develop strategic plans for next steps •For REFLECT: Analyze the reflection_target based on need_reflect_context, evaluate outcomes, identify issues, and suggest improvements •For SUMMARIZE: Condense need_summary_context according to summarization_focus, highlighting key points, patterns, and actionable insights

- Include specific observations, conclusions, and recommendations for next steps
- Maintain clarity and conciseness while preserving essential information

Output Examples For Decision If the current action is Decision, determine the next step as follows. (if the user query is en-US:) PLAN Action (Reasoning)


---

"action": "plan", "reasoning": "The user's query involves both technical and market analysis. Current memory

*,→* stack is empty, so I need to plan the first step.",

"instruction": "Reason about the next steps based on the current state",

REFLECT Action

"action": "reflect", "reasoning": "The previous research on AI ethics trends missed recent policy updates. I

*,→* should re-assign the task with refined instructions.",

"instruction": "Reflect on the previous action and its outcomes",

SUMMARIZE Action (No Parameters)

"action": "summarize", "reasoning": "The research results are extensive. Summarizing key points will help in

→deciding the next steps.",*,*

"instruction": "Condense the current information into a concise summary",

DELEGATE Action (Assign Sub-Agent)

"action": "delegate", "reasoning": "I need to gather the latest market data on AI investments. The Researcher Agent

→is best suited for this task.",

"params": { "agent_type": "researcher", "task_description": "Search for global AI investment trends in 2025, focusing on ethical

→considerations"*,*

"instruction": "Determine which sub-Agent to assign and define the task",

"action": "delegate", "reasoning": "To further increase retrieval depth and ensure comprehensiveness and diversity,

*,→* I need to use the replanner agent to formulate a specialized plan.", "params": { "agent_type": "replanner", "task_description": "Decompose this question into multi steps: Global AI investment trends

*,→* in 2025, focusing on ethical considerations"

FINISH Action (Complete Task) (if the user query is en-US:) "action": "finish", "reasoning": "All required data has been collected, analyzed, and summarized. User requirements have been satisfied.",→*,*

"instruction": "Task completed",

Decision Requirements While the step is decision, you must follow these requirements and return results in JSON format with the following

fields:


---

1. Analyze the current state and select the most appropriate action from available options.
2. Provide a clear reasoning for the decision, justifying why the action is optimal.
3. If choosing DELEGATE, specify the sub-Agent type and task instructions.

•If choosing replanner agent: This agent can only handle retrieval tasks into actionable steps. Do not include any requirements about report writing in the task description. You MUST and ONLY use it at the beginning of the task.

4. Please remember to check if report is generated before you decide to FINISH the task.
5. You must carefully check if the current information is sufficient to support the current decision-making

requirements. Regardless of whether the information is sufficient or not, you must provide detailed reasoning. If the information is insufficient, you must take appropriate actions to supplement it (for example, by delegating to a sub-agent capable of information gathering); if the information is sufficient, you must provide detailed reasoning explaining why the current information supports the decision.

6. Typically, after confirming the outline, it does not mean that the current information is sufficient to cover the

generation requirements. After the outline is confirmed, you usually need to delegate a sufficient information to support the task fully.

7. Return results in JSON format with the following fields:
- action: Type of action (required)
- reasoning: Justification for the decision (required)
- params: Action parameters (e.g., agent_type and task_description for DELEGATE)
- instruction: Instruction corresponding to the action
- locale: Language of the user query (e.g., "en-US", "zh-CN", etc.)

{% endif %}

Output Key Points For PLAN if the current action is PLAN, DO NOT give the json output, provide comprehensive reasoning and analysis in natural

language format: Strategic Analysis Framework

- Current Situation Assessment: Thoroughly analyze the user query, available resources, and system state
- Problem Decomposition: Break down complex queries into manageable components and identify core objectives
- Resource Evaluation: Assess available sub-agents, tools, and information to determine optimal approach
- Risk and Constraint Analysis: Identify potential obstacles, limitations, and dependencies
- Strategic Planning: Develop a step-by-step plan with clear priorities and sequencing

Key Focus Areas

- Goal Clarification: Ensure clear understanding of what needs to be accomplished
- Approach Selection: Choose the most effective methodology based on the query type and complexity
- Resource Allocation: Determine which sub-agents or tools are best suited for each task component
- Timeline and Dependencies: Consider the logical sequence of actions and any interdependencies
- Success Criteria: Define what constitutes successful completion of each planned step

Output Requirements

- Present analysis in clear, structured format using bullet points or numbered lists
- Provide specific, actionable insights rather than generic observations
- Include concrete next steps with rationale for each recommendation
- Highlight critical decision points and potential alternative approaches
- Maintain focus on practical implementation while considering broader strategic implications

{% endif %}

Output Key Points For REFLECT if the current action is REFLECT, return JSON format with reflection analysis and memory cleanup decision:

 search steps planning and is limited to decomposing

 researcher agent to gather

"analysis": "Detailed reflection analysis here", "pop_count": 2, "reasoning": "Explain why these items should be removed and what the reflection concluded"


---

Reflection Guidelines

- analysis: Provide comprehensive reflection on the previous action
- pop_count: Number (0 or positive integer) indicating how many recent memory stack items to remove
- reasoning: Explain the reflection conclusion and memory cleanup decision

Memory Stack Management Criteria

- Remove duplicate or redundant information
- Remove outdated information that no longer applies
- Keep essential information supporting ongoing work
- Remove failed attempts or incorrect reasoning
- DO NOT REMOVE any history that made progress towards the final goal or decision

•Only remove the most recent memory stack items. Older items should not be removed unless all recent items are cleared first. {% endif %}

Output Key Points For SUMMARY if the current action is SUMMARIZE, condense information based on {{need_summary_context}}, must meet the following requirements: •Comprehensiveness: Ensure that all key points and critical information are included. No important content should be omitted. •Completeness: Capture all valid inputs, core arguments, supporting data, conclusions, and recommendations from the original context. •Structured Output: Present the summary in a clear, organized format-such as bullet points or numbered lists-to enhance readability and usability.

- Information Preservation: Even when condensing large volumes of text, prioritize distillation over omission to

retain essential meaning. •Semantic Accuracy: Maintain the original intent and meaning during summarization to avoid misinterpretation or distortion. •Highlight Key Insights: Clearly emphasize or mark important findings, trends, and actionable recommendations (when applicable).

- Contextual Relevance: If the summary will be used in subsequent steps (e.g., decision-making or reporting), preserve

logical connections to the broader context. •URL Completeness: Ensure that ALL relevant URLs (include image URLs) are included in the summary to provide context and ensure that the summary is complete and accurate. {% endif %}

Experience Memory Curator Prompt Role You are a Experience Memory Curator. Your responsibility is to maintain a structured experience memory that supports cross-task generalization by consolidating information beyond individual task executions. The experience memory consists of three complementary components:

- User Profiles: capture stable user attributes and preference signals.
- Semantic Memory: store factual knowledge and declarative information, particularly externally retrieved evidence.

•Procedural Memory (SOPs): abstract key execution steps from previously completed tasks as reusable procedural patterns. These components are organized with a unified storage and retrieval interface. Objectives

1. Extract stable user attributes and preference signals into

{{summarization_focus}}and

2. Record atomic factual statements into semantic_memory
3. Abstract reusable execution patterns into procedural_memory (SOPs). 4.Merge new information withexisting_long_term_memory_json

redundancy.

. 


---

5. Return JSON only, strictly matching the required schema.

Input Task Memory: {{task_memory_json}} Existing Experience Memory (can be empty): {{existing_long_term_memory_json}} Current Timestamp: now_timestamp Output Schema (strictly required) "user_profiles": [ "<stable user attribute or preference signal>" "semantic_memory": [ "<atomic factual statement or retrieved evidence>" "procedural_memory": [ "scenario": "<task context or trigger condition>", "procedure": "<abstracted execution steps>", "rationale": "<why this procedure is effective or reusable>"

Transformation Rules User Profiles

- Capture stable user attributes, preferences, and experience behavior signals.
- Must remain valid across tasks and sessions.
- Avoid task-specific, transient, or procedural details.

Semantic Memory

- Each item is a single factual or declarative statement
- Focus on externally retrieved or verified information when applicable.
- Remove duplicates or merge paraphrases.
- Do not include user-specific preferences or procedural knowledge.

Procedural Memory (SOPs)

- Abstract reusable execution patterns from completed tasks.
- Describe how a task is effectively performed, not what happened in a single instance.
- Generalize across similar task types and contexts.
- Avoid time-specific or one-off execution traces.

Merging Behavior

- Combine with existing_long_term_memory_json
- Preserve existing entries unless they are refined or superseded by more accurate information.
- Append new user profile signals, semantic facts, or procedural patterns when identified.

.

Style Requirements

- Write factual, neutral English.
- No markdown formatting, commentary, or explanations outside JSON.
- No internal reasoning or justification.
- Output plain JSON text only.

.


---

## D Case Study

We present two representative case studies to qualitatively illustrate how the proposed framework operates under different task settings, with a particular focus on task-level memory control and cross-task experience utilization. Case 1: Multi-step Medical Question Answering. As shown in Table 4, the system initially issues a broad retrieval query that returns irrelevant medical content. Instead of committing this noisy information to its internal state, the central coordinator explicitly invokes REVISE action and modifies the retrieval key to progressively narrow the search scope. Through multiple

iterations of retrieval, inspection, and memory revision, the system successfully identifies evidence relevant to cerebrospinal fluid pressure and arrives at the correct answer. Case 2: Deep Research and Report Generation. The second case in Figure 2 examines a long-horizon deep research task involving open-ended information gathering and report synthesis. Through active memory management and long-term memory storage, S TACK- 

## E Computational Resources and Software Environment

Experiments were performed on a machine running Ubuntu 18.04.6 LTS (bionic), equipped with two Intel Xeon E5-2680 v4 processors providing a total of 56 logical cores (28 cores per CPU) and 377 GB of RAM. The system featured eight NVIDIA GeForce RTX 3090 GPUs, each with 24 GB of memory. The software environment consisted of Python 3.13.5 managed via 

HuggingFace Transformers 4.57.1. All software and 

inference times varied between 40 and 300 seconds per sample depending on task complexity.

## F The Use of Large Language Models

In this work, Large Language Models (LLMs) were used solely to support language polishing and programming tasks, including improving grammar, clarity, readability, and providing general coding suggestions or debugging advice. All outputs generated with LLM assistance were carefully reviewed and verified by the authors. The study's conceptualization, experimental design, and result analyses were conducted entirely by the authors, with LLMs having no role in formulating research ideas or drawing conclusions.


---

The normal value of cerebrospinal fluid pressure for lumbar puncture in the supine position is ().

Question

A. 80-180mmH2O (0.78-1.76kPa) B. 50-70mmH2O (0.49-0.69kPa) C. 230-250mmH2O (2.25-2.45kPa) D. 260-280mmH2O (2.55-2.74kPa)

Ground

A. 80-180mmH2O (0.78-1.76kPa)

Truth

<Plan> The question is asking about the normal cerebrospinal fluid (CSF) pressure range during lumbar puncture in the supine position. This is a factual medical question that requires accurate clinical knowledge. Since no prior information is available in the memory stack, the next step should involve the correct and up-to-date medical standard for CSF pressure <Delegate> {"handsoff":"Search Agent", "subtask":"Please find the medical standard of cerebrospinal fluid pressure"} </Delegate>

<Search> [Wiki_RAG]: Cerebrospinal fluid pressure </Search> <Observation> Observation: {'title': 'Hydrocephalus', 'text': 'Hydrocephalus (also known as water on the brain) is a condition occurring in the brain ...'} </Observation>

<Reflect> After reviewing the information, I found that the provided content does not directly mention the CSF pressure value to perform a new search for more specific information. </Reflect> REVISE ACTION: Last search results have been removed 

lumbar puncture in the supine position, with a focus on standard reference values expressed in both mmH2O and k Pa."} </Delegate>

Trajectory

<Search>[Bocha_RAG]: Lumbar puncture, cerebrospinal fluid pres- sure</Search> <Observation> Observation:{'title': 'Normal Cerebrospinal Fluid Opening Pressure', 'text': '...CSF pressure is commonly measured through a lumbar punc- retrieving . </Plan>

ture...'} </Observation>

<Summarize> The previous action returned authoritative medical data regarding the normal range of cerebrospinal fluid (CSF) pressure, including standard values, abnormal conditions, and underlying causes. The information from high-quality sources is summarized as follows: 

mmH2O (0.78 to 1.764 k Pa) further CSF drainage should be avoided to prevent severe complications such as brain herniation ...</Summarize> REVISE ACTION: Previous search results have been summarized <Delegate> {"handsoff":"Report Agent", "subtask":"Answer initial query based on previous observations."} </Delegate>

Search Agent

 during lumbar puncture. I need

Search Agent

pressure in supine position ranges from 80 to 180 mmH2O (0.78 to 1.764 k Pa). Among the options, the one closest (0.78-1.76 k Pa). Therefore, my answer is A. </Conclusion>

> Table 4: Case Study of S
>

 to this range is option A: 80-180 mmH2O

TACKPLANNER on a medical QA.

<Conclusion> According to authoritative medical references, the normal CSF

. If the pressure exceeds 300 mmH2O (2.94 k Pa),

Report Agent


---

> Figure 2: Case Study of S TACKPLANNER on a deepresearch task.
>

Task: "Please summarize the recently popular multi-agent system frameworks that are capable of performing report

generation tasks."
