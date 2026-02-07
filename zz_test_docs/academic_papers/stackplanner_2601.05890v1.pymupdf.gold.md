## **STACKPLANNER: A Centralized Hierarchical Multi-Agent System with** **Task-Experience Memory Management**

**Ruizhe Zhang** [1] _[,]_ [2][*] **, Xinke Jiang** [1] _[,]_ [2][*] **, Zhibang Yang** [1] _[,]_ [2][*] **, Zhixin Zhang** [7][*] **, Jiaran Gao** [1] _[,]_ [2][*] **,**
**Yuzhen Xiao** [1] _[,]_ [2][*] **, Hongbin Lai** [1] _[,]_ [2][*] **, Xu Chu** [1] _[,]_ [2] _[,]_ [4] _[,]_ [5][†] **, Junfeng Zhao** [1] _[,]_ [2] _[,]_ [6][†], **Yasha Wang** [2] _[,]_ [3] _[,]_ [4][†]

1 School of Computer Science and School of Software & Microelectronics, Peking University
2 Key Laboratory of High Confidence Software Technologies, Ministry of Education
3 National Engineering Research Center For Software Engineering, Peking University
4 Peking University Information Technology Institute (Tianjin Binhai)
5 Center on Frontiers of Computing Studies, Peking University
6 Big Data Technology Research Center, Nanhu Laboratory


{nostradamus, xinkejiang, yangzb}@stu.pku.edu.cn, {chu_xu, zhaojf, wangyasha}@pku.edu.cn



**Abstract**


Multi-agent systems based on large language
models, particularly centralized architectures,
have recently shown strong potential for complex and knowledge-intensive tasks. However,
central agents often suffer from unstable longhorizon collaboration due to the lack of memory management, leading to context bloat, error
accumulation, and poor cross-task generalization. To address both task-level memory inefficiency and the inability to reuse coordination
experience, we propose STACKPLANNER, a hierarchical multi-agent framework with explicit
memory control. STACKPLANNER addresses
these challenges by decoupling high-level coordination from subtask execution with active
task-level memory control, and by learning to
retrieve and exploit reusable coordination experience via structured experience memory and
reinforcement learning. Experiments on multiple deep-search and agent system benchmarks
demonstrate the effectiveness of our approach
in enabling reliable long-horizon multi-agent
collaboration.


**1** **Introduction**


Large Language Model-based multi-agent systems
(LLM-MAS) have emerged as an effective paradigm
for addressing complex, long-horizon, and knowledgeintensive tasks (Chen et al., 2025b; Guo et al., 2024).
By enabling task decomposition, parallel exploration,


*All authors listed contributed equally to this work. Ruizhe
Zhang and Xinke Jiang led the design and implementation
of the STACKPLANNER framework, including hierarchical
action space, task-level memory, and reinforcement learning
(RL) training architecture. Jiaran Gao and Xinke Jiang were
responsible for data construction and training within the RL
framework. Zhibang Yang and Yuzhen Xiao designed and
implemented the sub-agents. Zhixin Zhang and Hongbin Lai
developed the experience memory module and its retrieval
mechanism.
†Corresponding author.



and collaborative reasoning, these systems have been
applied to challenging problem-solving and informationintensive scenarios (Wu et al., 2023; Hong et al., 2024;
Qian et al., 2024). Prior work has explored a variety of
designs, including decentralized collaboration (Yang
et al., 2025; Wang et al., 2022), debate-based collectives (Du et al., 2023), and structured multi-stage
reasoning pipelines (Yao et al., 2023). However, as
system scale and task complexity increase, ensuring
reliable multi-agent collaboration over long-horizon,
information-intensive, and cross-task scenarios remains
a central dilemma (Guo et al., 2024). Decentralized
and debate-based approaches provide flexibility and
robustness but often suffer from high communication
overhead, redundant reasoning, and uncertainty in maintaining global consistency (Yang et al., 2025; Cui et al.,
2025). To mitigate these issues, most studies adopt a
centralized coordination paradigm, introducing a **cen-**
**tral agent** to unify planning, task allocation, and information integration by operating sub-agents to a unified decision-making framework. (Hou et al., 2024; Yue
et al., 2025).

Despite its advantages, most centralized multi-agent
systems **place the entire burden of coordination, in-**
**formation integration, and decision-making on a sin-**
**gle central agent** . As tasks grow in scale and complexity, the influx of information and long reasoning
chains can overwhelm the central agent’s processing
capacity (Jiang et al., 2024; Liu et al., 2023, 2024), significantly degrading its performance. This limitation is
especially pronounced in novel domains or tasks with
little prior experience. Crucially, both issues stem from
the central agent’s limited _**memory management**_ capabilities, encompassing both task-level and cross-task
memory. Addressing this deficiency gives rise to two
key challenges:

❶ _**Challenge 1. How can the central agent’s task mem-**_
_**ory be effectively managed to mitigate contextual noise**_
_**and memory bloat, ensuring stable decision-making**_
_**over long-horizon tasks?**_ As tasks unfold, information
from multiple sub-agents is often redundant or noisy, yet
it is **indiscriminately appended to the central agent’s**


**task memory** . Early errors or noise in sub-tasks or tool
invocations can propagate across long-horizon steps,
causing the central agent to become _lost in the middle of_
_reasoning_, which may result in plan deviations, imbalanced task allocations, or repeated exploration. Existing
methods largely rely on _passive memory management_
_strategies_, such as template-based summarization (Dou
et al., 2021) or heuristic truncation (Liu et al., 2023),
treating memory as a static byproduct rather than a controllable resource. However, without awareness of and
active control over its memory state, the central agent’s
performance deteriorates significantly as reasoning steps
increases.


❷ _**Challenge 2. How can valuable historical trajec-**_
_**tories (Experience Memory) of the central agent be**_
_**effectively leveraged to improve task planning and co-**_
_**ordination across new tasks?**_ When tackling new tasks,
the central agent often starts from scratch, with **little ref-**
**erence to prior successful coordination experiences** .
Although its decision-making is critical to overall system performance, LLMs are rarely trained for longhorizon, cross-agent reasoning, limiting their ability to
**plan complex tasks effectively** . As a result, systems
frequently exhibit poor cold-start performance (Li et al.,
2023, 2025a) and limited cross-task generalization (Li
et al., 2025b).


To address these challenges, we construct a **Hier-**
**archical Multi-Agent System** - STACKPLANNER,
centered on a coordinator, explicitly supporting the
management of **task memory** and **experience mem-**
**ory** . Specifically: ❶ For _**C1**_, we **decouple the central**
**coordinator’s high-level decision-making from the**
**execution details handled by specialized sub-agents** .
By strictly separating the memory of the coordinator
and sub-agents, we prevent sub-agents from indiscriminately appending raw execution results to coordinator’s
task memory, thereby alleviating cognitive and memory
pressure on the central agent. In addition, the central coordinator is equipped with an **active task memory man-**
**agement mechanism**, enabling it to **selectively store,**
**condense, and prune task-relevant information** . This
mechanism helps mitigate contextual noise and memory bloat, maintain cleaner task representations, and
enhance decision-making stability over long-horizon
multi-agent interactions. ❷ For _**C2**_, we introduce a **ex-**
**perience memory and retrieval module** that stores
valuable cross-task coordination experiences, including
factual knowledge and procedural memory. This allows
the central agent to selectively retrieve relevant historical trajectories, leveraging past strategies and decision
patterns to improve planning, delegation, and coordination across new tasks. To further enhance, we model the
full planning process as **a learnable decision process**
and train the coordinator exclusively via reinforcement
learning, which enables the coordinator to adapt its coordination behavior based on successful experiences.



**2** **Methodology**


As shown in Figure 1, STACKPLANNER follows a hierarchical multi-agent design. A **central coordinator**
is responsible for high-level decision making, including planning, subtask delegation, and active memory
operations, while specialized sub-agents handle concrete task execution. Moreover, the coordinator operates
over a _task memory_ that maintains a concise execution
trace, and leverages a _structured experience memory_ that
stores reusable knowledge and coordination experience
across tasks, which directly address _**C1**_ and _**C2**_ .


**2.1** **Hierarchical Coordination**


**Central Coordinator Action Space** The central coordinator operates over a compact discrete action space:


_A_ = _{_ PLAN _,_ DELEGATE _,_ REVISE _}._


Here PLAN determines the next coordination step based
on task memory. DELEGATE assigns a scoped subtask
to a selected sub-agent, together with task requirements
and relevant contextual information. REVISE actively
optimizes task memory via condensation and pruning.
This action space keeps the coordinator focused on
global progress, ensuring that system-wide behavior
remains strictly task-oriented. Implementation details
of central coordinator are deferred to Appendix C.


**Specialized Sub-Agents** Moreover, despite central
coordinator, we also incorporate specialized sub-agents:


- **Search Agent** : conducts key information retrieval via
external tools, following ReAct reasoning paradigm
for iterative information gathering and organization;


- **Report Agent** : adapts its behavior to the assigned
subtask, _either_ organizing previous information into
structured task reports to support subsequent coordination and execution, _or_ invoking professional writingoriented tools to design report structures and populate
content for refined textual outputs.


**2.2** **Active Task Memory Management**


The coordinator maintains a lightweight task memory
stack _M_ = _{m_ 1 _, . . ., mt}_, which sequentially stores
all task execution information, and is accessed and modified exclusively through REVISE actions. The Task
memory stack mechanisms supports three operations:


- Update : All task execution information—including
task specifications, coordinator action messages, and
sub-agent inputs and outputs—is sequentially pushed
onto the stack.


- Condensation : When the coordinator determines
that the memory becomes verbose or that a task stage
has been completed, REVISE performs _memory con-_
_densation_ by popping a contiguous segment _{mi}_ _[t]_ _i_ = _k_
from the stack, summarizing it into a compact representation _m_ _[′]_, and pushing _m_ _[′]_ back onto the stack.
This operation preserves task-relevant information
while reducing redundant context.


_**Agent Type**_



















_**Message Flow**_



































_**Tool Call**_























Figure 1: Overview of STACKPLANNER framework.




- Pruning : When the coordinator detects unproductive
or erroneous exploration, REVISE performs _memory_
_pruning_ by removing a selected segment of memory
entries from the stack. Additionally, a concise record
of failure causes is retained to guide subsequent exploration.


By exposing memory as an explicit control target,
REVISE enables active memory optimization, effectively filtering noise and correcting earlier coordination
errors with minimal overhead. Implementation details
of REVISE are deferred to Appendix C.


**2.3** **Structured Experience Memory Utilization**


To support cross-task generalization, we maintain a
structured **experience memory** that stores persistent
information beyond individual task executions. The
experience memory consists of three complementary
components: (i) _user profiles_, which capture stable user
attributes and preference signals; (ii) _semantic memory_,
which stores factual knowledge and declarative information, particularly externally retrieved evidence; and (iii)
_procedural memory (SOPs)_, which abstracts key execution steps from previously completed tasks as reusable
procedural patterns. These components are organized
with a unified storage and retrieval interface. Examples of experience memory entries, along with storage
formats and prompting details, are in Appendix C.


**Experience Retrieval** We further design an Experience Search agent queries the experience memory using the current task representation and user identifier,
retrieving relevant entries that are summarized and injected into the task memory to inform coordination and
mitigate cold-start issues.


**Reinforcement Learning Formulation** We formulate training STACKPLANNER’s coordinator as a multistep RL problem, where the policy model is augmented



with access to an external search engine and a structured memory stack. Given a query _q ∼D_, the policy
model _πθ_ generates a trajectory _y_ = ( _a_ 1 _, . . ., aT_ ) with
_T_ action steps, and the RL objective with search engine
invocations and memory stack operations is defined as:

max _θ_ E _q∼D, y∼πθ_ ( _·|q_ ; _R,M_ )[ _rϕ_ ( _q, y_ )] (1)

_−_ _β_ DKL� _πθ_ ( _y | q_ ; _R, M_ ) _∥_ _π_ ref ( _y | q_ ; _R, M_ )� _,_


where _R_ and _M_ denotes search engine and stackstructured memory respectively, _rϕ_ is the reward function, and _π_ ref is the frozen reference policy. Unlike
standard RLHF (Schulman et al., 2017) or retrievalaugmented RL methods such as Search-R1 (Jin et al.,
2025), which largely rely on parametric knowledge and
coarse-grained searching interactions, our policy follows an interleaved _**retrieval–reasoning–memory**_ execution paradigm. Concretely, _πθ_ ( _· | q_ ; _R, M_ ) can be
viewed as a sequence of _T_ alternating reasoning, searching and memorizing actions, where each step conditions
only on information obtained through retrieval or reasoned and kept in the memory stack.
Following (Jin et al., 2023), we adopt **Group Rela-**
**tive Policy Optimization (GRPO)** (Shao et al., 2024)
to optimize the policy, which eliminates the need for
a learned value function by computing relative advantages from statistics of the current rollout group. Specifically, for a rollout group consisting of _K_ trajectories
_{y_ [(] _[k]_ [)] _}_ _[K]_ _k_ =1 [sampled from the old policy] _[ π][θ]_ old [, where]
each trajectory _y_ [(] _[k]_ [)] = ( _x_ [(] 1 _[k]_ [)] _[, . . ., x]_ [(] _|y_ _[k]_ [(][)] _[k]_ [)] _|_ [)][ is a sequence]

of generated tokens [1], let _RG_ denote the set of all tokenlevel rewards _{ri_ [(] _[k]_ [)] _}_ across the group. For each token
_x_ [(] _i_ _[k]_ [)] in trajectory _y_ [(] _[k]_ [)], we compute a normalized grouprelative advantage as:

_A_ ˆ [(] _i_ _[k]_ [)] =   - _ri_ [(] _[k]_ [)] _−_ mean( _RG_ )� _/_ std( _RG_ ) _._ (2)


1In our implementation, each high-level action _at_ is realized as a contiguous sequence of generated tokens


The GRPO optimization objective is then defined as:




    1

_J_ ( _θ_ ) = E

_K_



_K_



_k_ =1



1
_|y_ [(] _[k]_ [)] _|_



_|y_ [(] _[k]_ [)] _|_

- Clip� _z_ ˜ _i_ [(] _[k]_ [)] _,_ _A_ [ˆ][(] _i_ _[k]_ [)] - [�]

_i_ =1




_−_ _β_ DKL _,_


and Clip - _z_ ˜ _i_ [(] _[k]_ [)] _,_ _A_ [ˆ][(] _i_ _[k]_ [)] - = min - _z_ ˜ _i_ [(] _[k]_ [)] _A_ ˆ [(] _i_ _[k]_ [)] _,_ clip(˜ _zi_ [(] _[k]_ [)] _,_ 1 _±_

   - _πθ_ ( _x_ [(] _i_ _[k]_ [)] _|q,x_ [(] _<i_ _[k]_ [)][;] _[R][,][M]_ [)]
_ε_ ) _A_ [ˆ][(] _i_ _[k]_ [)], importance ratio ˜ _zi_ [(] _[k]_ [)] = _πθ_ old ( _x_ [(] _i_ _[k]_ [)] _|q,x_ [(] _<i_ _[k]_ [)][;] _[R][,][M]_ [)]

denotes the probability ratio at the token level. Term
DKL( _πθ∥π_ ref ) constrains the updated policy to remain
close to a frozen reference policy _π_ ref . Notably, all rewards, advantages, and policy updates in our framework
are defined at the action level and applied at _token level_ .


**3** **Experiment**


**3.1** **Experimental Setup**


❶ **Evaluation Benchmarks.** We evaluate our method
on ten benchmarks spanning two settings: _multi-hop QA_
(2Wiki(Ho et al., 2020), MusiQue (Trivedi et al., 2022)),
and _agentic benchmarks_ (GAIA (Mialon et al., 2023)
and FRAMES (Krishna et al., 2024)). Additional benchmark details are reported in Appendix A. ❷ **Baselines.**
We compare our method against a diverse set of baselines covering _Naive_, _Single-Agent_, _Multi-Agent_, and
_Agentic-RL_ paradigms. Specifically, _Naive_ baselines
include Base and FS-RAG (Trivedi et al., 2023). _Single-_
_Agent_ approaches consist of ReAct (Yao et al., 2022) and
IRCoT (Trivedi et al., 2023). For _Multi-Agent_ methods,
we consider both centralized architectures, including
OWL (Hu et al., 2025), and automated architectures
such as MacNet (Qian et al.) and AFlow (Zhang et al.).
Finally, _Agentic-RL_ baselines include ReSearch (Chen
et al., 2025a), ARPO (Dong et al., 2025), and our proposed method. Detailed descriptions of all baselines
are provided in Appendix B. ❸ **RAG Tools.** We use
a Wikipedia-based search tool (snapshot: November 1,
2023) and Bocha for web search.


**3.2** **Main Result Analysis**


**Strong Performance Compared with Baselines.** Our
method achieves state-of-the-art performance across
all benchmarks, surpassing baselines in multi-hop QA
and agentic evaluation. It shows strong generalization on out-of-distribution datasets ( _MuSiQue_, _GAIA_,
and _FRAMES_ ), with F1 scores of 16.48%, 7.71%, and
16.23% for Qwen2.5-3B, and 22.01%, 9.45%, and
19.44% for Qwen2.5-7B, respectively. _GAIA_ is the most
challenging benchmark due to its multi-step reasoning
and memory demands; baselines such as MacNet fail
to complete reasoning because they cannot effectively
manage task memory, resulting in missing scores (“/”),
while AFlow achieves only 2.57% and 4.72%. In contrast, our method handles complex reasoning and memory managements effectively, consistently delivering
strong results across both 3B and 7B backbones.



**3.3** **Component Analysis**


**Model Component Ablation.** We conduct ablation
experiments to evaluate the contributions of the task
memory and experience memory modules in our model.
Removing the task memory leads to a drop of 3.02%,
5.72%, 3.03%, and 2.70% points on _2WikiMultiHopQA_,
_MuSiQue_, _GAIA_, and _FRAMES_, respectively. Excluding the experience memory causes declines of 4.45%,
7.49%, 2.18%, and 8.54% points, while removing both
memory components results in the largest performance
degradation, with F1 scores dropping by 15.80%, 9.05%,
5.24%, and 9.90% points across the same datasets.
These results demonstrate that both task and experience
memory modules play crucial roles in enhancing multistep reasoning and generalization, and their combined
effect is essential for achieving optimal performance.


**4** **Conclusion and Future Work**


In this paper, we present STACKPLANNER, a hierarchical centralized multi-agent framework that treats
memory as an explicit control target for coordination.
By combining decoupled coordination with active **task**
**memory** management and reusable **experience mem-**
**ory**, STACKPLANNER mitigates context bloat and error
propagation in long-horizon collaboration. Moreover,
high-level coordination and memory control are jointly
learned via reinforcement learning. Experiments on
deep-search and agent system benchmarks demonstrate
more stable coordination and stronger generalization.
Several challenges remain for future work. In particular, designing more expressive yet compact task memory
abstractions may further improve decision robustness
under longer horizons and more complex agent interactions. We also plan to extend the evaluation of STACKPLANNER to broader domains and more open-ended
real-world agentic settings, including deep research and
long-horizon analytical workflows.


**Method** **Qwen2.5-3B** **Qwen2.5-7B**
**Paradigm** **Approach** **2Wiki** **MusiQue** **GAIA** **FRAMES** **2Wiki** **MusiQue** **GAIA** **FRAMES**


Base 23.98 9.70 5.70 8.01 25.41 12.15 4.29 12.52
Naive
FS-RAG 15.47 7.64 4.30 10.42 17.71 10.74 5.02 12.52


ReACT 25.09 13.92 4.78 10.53 27.51 19.34 6.37 15.29
Single-Agent
IRCoT 15.89 12.43 2.77 6.79 36.45 8.39 5.50 6.78


OWL 17.39 14.81 3.28 13.49 29.73 17.66 5.39 14.68
Multi-Agent MacNet 25.20 13.19 / 11.92 28.19 17.81 / 12.61
AFlow 24.56 13.07 2.57 12.13 30.53 18.15 4.72 12.81


ReSearch 27.23 9.47 4.48 10.00 30.03 12.58 4.43 15.61
Agentic-RL ARPO 29.55 13.38 **7.71** 13.49 30.71 12.71 8.56 12.18
**Ours** **32.92** **16.48** **7.71** **16.23** **38.34** **22.01** **9.45** **19.44**


Table 1: Performance comparison ( **F1**, %) on multi-hop QA benchmarks ( _2WikiMultiHopQA_, _MusiQue_, _GAIA_, and
_FRAMES_ ) across different paradigms using **Qwen2.5-3B** and **Qwen2.5-7B** . The symbol “/” indicates that a model
could not produce results on a dataset, and **bold** highlights the best performance in each column.



**Method** **2Wiki** **Musique** **GAIA** **FRAMES**


**Ours** **32.92** **16.48** **7.71** **16.23**


w/o Task memory 29.90 10.76 4.68 13.53
w/o Experience memory 28.47 8.99 5.53 7.69
w/o Both memories 17.12 7.43 2.47 6.33


Table 2: Ablation analysis of component and reward
designs in STACKPLANNER on Qwen2.5-3B.


**Limitations**


Despite the promising results, our framework does have
some limitations that need to be addressed. ❶ **Lim-**
**ited support for multi-turn interactions.** The current task-level memory is primarily designed for singleturn and does not explicitly model multi-turn conversational dependencies. As a result, adapting the behavior of specific sub-agents across extended interactions
becomes cumbersome and error-prone. ❷ **Cold-start**
**challenges in long-term memory.** Long-term memory
mechanisms still suffer from cold-start issues, where
insufficient prior experience limits their effectiveness in
early stages. While simulated users can be introduced
to partially mitigate this problem, the initialized experiences often exhibit limited generalization capability
when transferred to real or diverse user behaviors.


**Ethical considerations**


All experiments in this study were conducted solely on
publicly available benchmark datasets, including _2Wiki-_
_MultiHopQA_, _MuSiQue_, _GAIA_, and _FRAMES_, in compliance with their respective licenses and usage terms.
We did not utilize any personally identifiable information, nor were any human or animal subjects involved
in the research.


**References**


Mingyang Chen, Linzhuang Sun, Tianpeng Li, Haoze
Sun, Yijie Zhou, Chenzheng Zhu, Haofen Wang,
Jeff Z Pan, Wen Zhang, Huajun Chen, and 1 others. 2025a. Learning to reason with search for



llms via reinforcement learning. _arXiv preprint_
_arXiv:2503.19470_ .


Shuaihang Chen, Yuanxing Liu, Wei Han, Weinan
[Zhang, and Ting Liu. 2025b. A survey on llm-based](https://arxiv.org/abs/2412.17481)
[multi-agent system: Recent advances and new fron-](https://arxiv.org/abs/2412.17481)
[tiers in application.](https://arxiv.org/abs/2412.17481) _Preprint_, arXiv:2412.17481.


Yu Cui, Hang Fu, Haibin Zhang, Licheng Wang, and
[Cong Zuo. 2025. Free-mad: Consensus-free multi-](https://arxiv.org/abs/2509.11035)
[agent debate.](https://arxiv.org/abs/2509.11035) _Preprint_, arXiv:2509.11035.


Guanting Dong, Hangyu Mao, Kai Ma, Licheng Bao,
Yifei Chen, Zhongyuan Wang, Zhongxia Chen, Jiazhen Du, Huiyang Wang, Fuzheng Zhang, and 1
others. 2025. Agentic reinforced policy optimization.
_arXiv preprint arXiv:2507.19849_ .


Zi-Yi Dou, Pengfei Liu, Hiroaki Hayashi, Zhengbao
[Jiang, and Graham Neubig. 2021. GSum: A gen-](https://doi.org/10.18653/v1/2021.naacl-main.384)
[eral framework for guided neural abstractive summa-](https://doi.org/10.18653/v1/2021.naacl-main.384)
[rization. In](https://doi.org/10.18653/v1/2021.naacl-main.384) _Proceedings of the 2021 Conference of_
_the North American Chapter of the Association for_
_Computational Linguistics: Human Language Tech-_
_nologies_, pages 4830–4842, Online. Association for
Computational Linguistics.


Yilun Du, Shuang Li, Antonio Torralba, Joshua B.
[Tenenbaum, and Igor Mordatch. 2023. Improving](https://arxiv.org/abs/2305.14325)
[factuality and reasoning in language models through](https://arxiv.org/abs/2305.14325)
[multiagent debate.](https://arxiv.org/abs/2305.14325) _Preprint_, arXiv:2305.14325.


Jiaxuan Gao, Wei Fu, Minyang Xie, Shusheng Xu,
Chuyi He, Zhiyu Mei, Banghua Zhu, and Yi Wu.
[2025. Beyond ten turns: Unlocking long-horizon](https://arxiv.org/abs/2508.07976)
[agentic search with large-scale asynchronous rl.](https://arxiv.org/abs/2508.07976)
_Preprint_, arXiv:2508.07976.


Taicheng Guo, Xiuying Chen, Yaqi Wang, Ruidi Chang,
Shichao Pei, Nitesh V. Chawla, Olaf Wiest, and Xi[angliang Zhang. 2024. Large language model based](https://arxiv.org/abs/2402.01680)
[multi-agents: A survey of progress and challenges.](https://arxiv.org/abs/2402.01680)
_Preprint_, arXiv:2402.01680.


Xanh Ho, Anh-Khoa Duong, Quoc-Huy Nguyen, and
Suong Nguyen. 2020. Constructing a multi-hop qa
dataset for comprehensive evaluation of reasoning
steps. In _COLING_ .


Sirui Hong, Mingchen Zhuge, Jiaqi Chen, Xiawu Zheng,
Yuheng Cheng, Ceyao Zhang, Jinlin Wang, Zili
Wang, Steven Ka Shing Yau, Zijuan Lin, Liyang
Zhou, Chenyu Ran, Lingfeng Xiao, Chenglin Wu,
[and Jürgen Schmidhuber. 2024. Metagpt: Meta pro-](https://arxiv.org/abs/2308.00352)
[gramming for a multi-agent collaborative framework.](https://arxiv.org/abs/2308.00352)
_Preprint_, arXiv:2308.00352.


Xinming Hou, Mingming Yang, Wenxiang Jiao, Xing
Wang, Zhaopeng Tu, and Wayne Xin Zhao. 2024.
[Coact: A global-local hierarchy for autonomous](https://arxiv.org/abs/2406.13381)
[agent collaboration.](https://arxiv.org/abs/2406.13381) _Preprint_, arXiv:2406.13381.


Mengkang Hu, Yuhang Zhou, Wendong Fan, Yuzhou
Nie, Bowei Xia, Tao Sun, Ziyu Ye, Zhaoxuan Jin, Yingru Li, Qiguang Chen, and 1 others. 2025. Owl: Optimized workforce learning for general multi-agent assistance in real-world task automation. _arXiv preprint_
_arXiv:2505.23885_ .


Xinke Jiang, Yue Fang, Rihong Qiu, Haoyu Zhang,
Yongxin Xu, Hao Chen, Wentao Zhang, Ruizhe
Zhang, Yuchen Fang, Xu Chu, and 1 others. 2024.
Tc-rag: Turing-complete rag’s case study on medical
llm systems. _arXiv preprint arXiv:2408.09199_ .


Bowen Jin, Hansi Zeng, Zhenrui Yue, Jinsung Yoon,
Sercan Arik, Dong Wang, Hamed Zamani, and Jiawei
Han. 2025. Search-R1: Training LLMs to reason and
leverage search engines with reinforcement learning.
_arXiv preprint arXiv:2503.09516_ .


Qiao Jin, Robert Leaman, and Zhiyong Lu. 2023. Retrieve, summarize, and verify: how will chatgpt
affect information seeking from the medical literature? _Journal of the American Society of Nephrology_,
34(8):1302–1304.


Kalpesh Krishna and 1 others. 2024. Retrieval augmented generation for long-context question answering with frames. _arXiv preprint arXiv:2409.12941_ .


Annan Li, Chufan Wu, Zengle Ge, Yee Hin Chong, Zhinan Hou, Lizhe Cao, Cheng Ju, Jianmin Wu, Huaiming Li, Haobo Zhang, Shenghao Feng, Mo Zhao,
Fengzhi Qiu, Rui Yang, Mengmeng Zhang, Wenyi
Zhu, Yingying Sun, Quan Sun, Shunhao Yan,
and 3 others. 2025a. [The fm agent.](https://arxiv.org/abs/2510.26144) _Preprint_,
arXiv:2510.26144.


Huao Li, Yu Chong, Simon Stepputtis, Joseph Campbell, Dana Hughes, Charles Lewis, and Katia Sycara.
[2023. Theory of mind for multi-agent collaboration](https://doi.org/10.18653/v1/2023.emnlp-main.13)
[via large language models. In](https://doi.org/10.18653/v1/2023.emnlp-main.13) _Proceedings of the_
_2023 Conference on Empirical Methods in Natural_
_Language Processing_, pages 180–192, Singapore. Association for Computational Linguistics.


Yilong Li, Chen Qian, Yu Xia, Ruijie Shi, Yufan Dang,
Zihao Xie, Ziming You, Weize Chen, Cheng Yang,
Weichuan Liu, Ye Tian, Xuantang Xiong, Lei Han,
[Zhiyuan Liu, and Maosong Sun. 2025b. Cross-task](https://arxiv.org/abs/2505.23187)
[experiential learning on llm-based multi-agent collab-](https://arxiv.org/abs/2505.23187)
[oration.](https://arxiv.org/abs/2505.23187) _Preprint_, arXiv:2505.23187.


Nelson F. Liu, Kevin Lin, John Hewitt, Ashwin Paranjape, Michele Bevilacqua, Fabio Petroni, and Percy



[Liang. 2023. Lost in the middle: How language mod-](https://arxiv.org/abs/2307.03172)
[els use long contexts.](https://arxiv.org/abs/2307.03172) _Preprint_, arXiv:2307.03172.


Xiang Liu, Peijie Dong, Xuming Hu, and Xiaowen
[Chu. 2024. Longgenbench: Long-context genera-](https://arxiv.org/abs/2410.04199)
[tion benchmark.](https://arxiv.org/abs/2410.04199) _Preprint_, arXiv:2410.04199.


Grégoire Mialon and 1 others. 2023. Gaia: A benchmark for general ai assistants. _arXiv preprint_
_arXiv:2311.12983_ .


Chen Qian, Wei Liu, Hongzhang Liu, Nuo Chen, Yufan
Dang, Jiahao Li, Cheng Yang, Weize Chen, Yusheng
Su, Xin Cong, Juyuan Xu, Dahai Li, Zhiyuan Liu,
and Maosong Sun. 2024. [Chatdev: Communica-](https://arxiv.org/abs/2307.07924)
[tive agents for software development.](https://arxiv.org/abs/2307.07924) _Preprint_,
arXiv:2307.07924.


Chen Qian, Zihao Xie, YiFei Wang, Wei Liu, Kunlun Zhu, Hanchen Xia, Yufan Dang, Zhuoyun Du,
Weize Chen, Cheng Yang, and 1 others. Scaling large
language model-based multi-agent collaboration. In
_The Thirteenth International Conference on Learning_
_Representations_ .


John Schulman, Filip Wolski, Prafulla Dhariwal,
Alec Radford, and Oleg Klimov. 2017. Proximal policy optimization algorithms. _arXiv preprint_
_arXiv:1707.06347_ .


Zhihong Shao, Peiyi Wang, Qihao Zhu, Runxin Xu,
Junxiao Song, Xiao Bi, Haowei Zhang, Mingchuan
Zhang, YK Li, Y Wu, and 1 others. 2024. Deepseekmath: Pushing the limits of mathematical reasoning in open language models. _arXiv preprint_
_arXiv:2402.03300_ .


Harsh Trivedi, Niranjan Balasubramanian, Tushar Khot,
and Ashish Sabharwal. 2022. Musique: Multihop
reasoning dataset with explanation. _arXiv preprint_
_arXiv:2108.00573_ .


Harsh Trivedi, Niranjan Balasubramanian, Tushar Khot,
and Ashish Sabharwal. 2023. Interleaving retrieval
with chain-of-thought reasoning for knowledgeintensive multi-step questions. In _Proceedings of the_
_61st Annual Meeting of the Association for Compu-_
_tational Linguistics (Volume 1: Long Papers)_, pages
10014–10037.


Yuanfei Wang, Fangwei Zhong, Jing Xu, and Yizhou
[Wang. 2022. Tom2c: Target-oriented multi-agent](https://arxiv.org/abs/2111.09189)
[communication and cooperation with theory of mind.](https://arxiv.org/abs/2111.09189)
_Preprint_, arXiv:2111.09189.


Qingyun Wu, Gagan Bansal, Jieyu Zhang, Yiran
Wu, Beibin Li, Erkang Zhu, Li Jiang, Xiaoyun
Zhang, Shaokun Zhang, Jiale Liu, Ahmed Hassan
Awadallah, Ryen W White, Doug Burger, and Chi
[Wang. 2023. Autogen: Enabling next-gen llm ap-](https://arxiv.org/abs/2308.08155)
[plications via multi-agent conversation.](https://arxiv.org/abs/2308.08155) _Preprint_,
arXiv:2308.08155.


Yingxuan Yang, Huacan Chai, Shuai Shao, Yuanyi
Song, Siyuan Qi, Renting Rui, and Weinan Zhang.
[2025. Agentnet: Decentralized evolutionary coordi-](https://arxiv.org/abs/2504.00587)
[nation for llm-based multi-agent systems.](https://arxiv.org/abs/2504.00587) _Preprint_,
arXiv:2504.00587.


Shunyu Yao, Dian Yu, Jeffrey Zhao, Izhak Shafran,
Thomas L. Griffiths, Yuan Cao, and Karthik
[Narasimhan. 2023. Tree of thoughts: Deliberate prob-](https://arxiv.org/abs/2305.10601)
[lem solving with large language models.](https://arxiv.org/abs/2305.10601) _Preprint_,
arXiv:2305.10601.


Shunyu Yao, Jeffrey Zhao, Dian Yu, Nan Du, Izhak
Shafran, Karthik Narasimhan, and Yuan Cao. 2022.
React: Synergizing reasoning and acting in language
models. _arXiv preprint arXiv:2210.03629_ .


Yanwei Yue, Guibin Zhang, Boyang Liu, Guancheng
Wan, Kun Wang, Dawei Cheng, and Yiyan Qi. 2025.
[Masrouter: Learning to route llms for multi-agent](https://arxiv.org/abs/2502.11133)
[systems.](https://arxiv.org/abs/2502.11133) _Preprint_, arXiv:2502.11133.


Jiayi Zhang, Jinyu Xiang, Zhaoyang Yu, Fengwei Teng,
Xiong-Hui Chen, Jiaqi Chen, Mingchen Zhuge, Xin
Cheng, Sirui Hong, Jinlin Wang, and 1 others. Aflow:
Automating agentic workflow generation. In _The_
_Thirteenth International Conference on Learning Rep-_
_resentations_ .



**A** **Experiment Datasets**


**A.1** **Training Dataset.**


Followed by (Gao et al., 2025), We train our models
and baselines on a curated multi-hop question answering dataset constructed from the training splits of 2WikiMultiHopQA (Ho et al., 2020). To focus on genuinely
non-trivial reasoning scenarios, we filter out instances
that require no external retrieval or can be solved with
only a single, trivial retrieval step.


**A.2** **Testing set.**


2Wiki 154,878 12,576 12,576
MuSiQue 19,938 2,417 2,459
GAIA 0 0 127
FRAMES 0 0 824


Table 3: Overview of datasets used in experiments.


We evaluate our approach on four widely used benchmarks covering multi-hop QA, and real-world agent
evaluation: Key statistics for training, development, and
test splits are summarized in Table 3.


❶ **Multi-Hop QA Benchmarks.** We evaluate our approach on two multi-hop question answering datasets
that require reasoning over multiple documents:


- **2WikiMultiHopQA (Ho et al., 2020).** Constructed
from Wikipedia and Wikidata, this dataset contains
192,606 question–answer pairs. It includes 154,878
training, 12,576 development, and 12,576 test instances, focusing on tasks that necessitate aggregating
evidence across multiple sources.


- **MuSiQue (Trivedi et al., 2022).** MuSiQue is designed to test multi-step reasoning over Wikipedia
data, with each reasoning step depending critically
on the previous step. The dataset comprises 19,938
training, 2,417 development, and 2,459 test examples.


❷ **Agentic Benchmarks.** We further assess our
method on two agentic benchmarks that evaluate models’ ability to handle real-world questions:


- **GAIA (Mialon et al., 2023).** GAIA measures performance on tasks requiring multi-step reasoning, web interaction, and multi-modal input handling. We choose
127 text-only questions in validation set across varying difficulty levels.


- **FRAMES (Krishna et al., 2024).** FRAMES consists
of 824 multi-hop questions, emphasizing factual accuracy, retrieval, and reasoning over multiple sources.


**B** **Baseline Implementation Details**


We compare our method with baselines from four
paradigms ( _Naive_, _Single-Agent_, _Multi-Agent_, _Agentic-_
_RL_ ) spanning different reasoning and coordination


strategies. Implementation details for each paradigms
are described below.


❶ **Naive.** Naive baselines do not involve explicit agentic reasoning or coordination mechanisms. They either
rely solely on the LLM’s parametric knowledge or incorporate retrieval in a fixed, heuristic manner.


  - **Base.** A non-retrieval baseline where LLM generates answers using only its parametric knowledge.


  - **FS-RAG (Trivedi et al., 2023).** FS-RAG retrieves
evidence at the sentence level, treating each input
sentence independently as a query.


❷ **Single-Agent.** Single-Agent baselines use a single LLM that alternates reasoning and tool usage via
prompting, without coordination between agents.


  - **ReAct (Yao et al., 2022).** ReAct interleaves reasoning and action steps, allowing interaction with
external tools such as search engines.


  - **IRCoT (Trivedi et al., 2023).** IRCoT alternates
between retrieval and chain-of-thought reasoning,
where intermediate steps guide retrieval and retrieved evidence informs subsequent reasoning.


❸ **Multi-Agent.** Multi-Agent baselines decompose
complex tasks into multiple interacting agents, leveraging either centralized coordination or automated agent
orchestration strategies.


  - **MacNet (Qian et al.).** MacNet is an automated
multi-agent architecture that organizes agent interactions via directed acyclic graphs (DAGs), enabling scalable reasoning through iterative agent
refinement while mitigating context explosion.


  - **OWL (Hu et al., 2025).** OWL is a centralized multi-agent system that decouples high-level
planning from specialized execution, using a
reinforcement-learned, domain-agnostic planner
to enable efficient cross-domain transfer.


  - **AFlow (Zhang et al.).** AFlow is an automated
agent orchestration framework that employs Monte
Carlo Tree Search (MCTS) to explore and optimize
agent workflows represented as code through iterative execution feedback.


❹ **Agentic-RL.** Agentic-RL baselines use reinforcement learning to guide agentic decisions, learning when
and how to invoke tools or coordinate actions in multistep reasoning.


  - **ReSearch (Chen et al., 2025a).** ReSearch jointly
optimizes reasoning and search behaviors via RL,
without supervision on intermediate steps.


  - **ARPO (Dong et al., 2025).** ARPO employs an
entropy-aware adaptive rollout to dynamically adjust sampling at high-uncertainty points, promoting
diverse and effective tool usage.


**C** **Prompts**


In this section, we provide a detailed introduction to the prompts used in our framework.




{
"action": "plan",
"reasoning": "The user's query involves both technical and market analysis. Current memory
_�→_ stack is empty, so I need to plan the first step.",
"params": null,
"instruction": "Reason about the next steps based on the current state",
"locale": "en-US"
}


**REFLECT Action**
(if the user query is en-US:)


{
"action": "reflect",
"reasoning": "The previous research on AI ethics trends missed recent policy updates. I
_�→_ should re-assign the task with refined instructions.",
"params": null,
"instruction": "Reflect on the previous action and its outcomes",
"locale": "en-US"
}


**SUMMARIZE Action (No Parameters)**
(if the user query is en-US:)


{
"action": "summarize",
"reasoning": "The research results are extensive. Summarizing key points will help in
_�→_ deciding the next steps.",
"params": null,
"instruction": "Condense the current information into a concise summary",
"locale": "en-US"
}


**DELEGATE Action (Assign Sub-Agent)**
(if the user query is en-US:)


{
"action": "delegate",
"reasoning": "I need to gather the latest market data on AI investments. The Researcher Agent
_�→_ is best suited for this task.",
"params": {
"agent_type": "researcher",
"task_description": "Search for global AI investment trends in 2025, focusing on ethical
_�→_ considerations"
},
"instruction": "Determine which sub-Agent to assign and define the task",
"locale": "en-US"
}


{
"action": "delegate",
"reasoning": "To further increase retrieval depth and ensure comprehensiveness and diversity,
_�→_ I need to use the replanner agent to formulate a specialized plan.",
"params": {
"agent_type": "replanner",
"task_description": "Decompose this question into multi steps: Global AI investment trends
_�→_ in 2025, focusing on ethical considerations"
}
}


**FINISH Action (Complete Task)** (if the user query is en-US:)


{
"action": "finish",
"reasoning": "All required data has been collected, analyzed, and summarized. User's
_�→_ requirements have been satisfied.",
"params": null,
"instruction": "Task completed",
"locale": "en-US"
}


**Decision Requirements**
While the step is **decision**, you must follow these requirements and return results in JSON format with the following
fields:


1. Analyze the current state and select the most appropriate action from available options.


2. Provide a clear reasoning for the decision, justifying why the action is optimal.


3. If choosing DELEGATE, specify the sub-Agent type and task instructions.


  - If choosing replanner agent: This agent can only handle **search steps planning** and is limited to decomposing
retrieval tasks into actionable steps. Do not include any requirements about report writing in the task description.
You MUST and ONLY use it at the beginning of the task.


4. Please remember to check if report is generated before you decide to FINISH the task.


5. **You must carefully check if the current information is sufficient to support the current decision-making**
**requirements** . Regardless of whether the information is sufficient or not, you must provide detailed reasoning. If
the information is insufficient, you must take appropriate actions to supplement it (for example, by delegating to a
sub-agent capable of information gathering); if the information is sufficient, you must provide detailed reasoning
explaining why the current information supports the decision.


6. **Typically, after confirming the outline, it does not mean that the current information is sufficient to cover the**
**generation requirements** . After the outline is confirmed, you usually need to delegate a **researcher agent** to gather
sufficient information to support the task fully.


7. Return results in JSON format with the following fields:


  - action: Type of action (required)

  - reasoning: Justification for the decision (required)

  - params: Action parameters (e.g., agent_type and task_description for DELEGATE)

  - instruction: Instruction corresponding to the action

  - locale: Language of the user query (e.g., "en-US", "zh-CN", etc.)


{% endif %}


{% if current_action == "plan" %}


**Output Key Points For PLAN**


if the **current action** is **PLAN**, DO NOT give the json output, provide comprehensive reasoning and analysis in natural
language format:


**Strategic Analysis Framework**


 - **Current Situation Assessment** : Thoroughly analyze the user query, available resources, and system state

 - **Problem Decomposition** : Break down complex queries into manageable components and identify core objectives

 - **Resource Evaluation** : Assess available sub-agents, tools, and information to determine optimal approach

 - **Risk and Constraint Analysis** : Identify potential obstacles, limitations, and dependencies

 - **Strategic Planning** : Develop a step-by-step plan with clear priorities and sequencing


**Key Focus Areas**


 - **Goal Clarification** : Ensure clear understanding of what needs to be accomplished

 - **Approach Selection** : Choose the most effective methodology based on the query type and complexity

 - **Resource Allocation** : Determine which sub-agents or tools are best suited for each task component

 - **Timeline and Dependencies** : Consider the logical sequence of actions and any interdependencies

 - **Success Criteria** : Define what constitutes successful completion of each planned step


**Output Requirements**


 - Present analysis in clear, structured format using bullet points or numbered lists

 - Provide specific, actionable insights rather than generic observations

- Include concrete next steps with rationale for each recommendation

 - Highlight critical decision points and potential alternative approaches

- Maintain focus on practical implementation while considering broader strategic implications


{% endif %}


{% if current_action == "reflect" %}


**Output Key Points For REFLECT**


if the **current action** is **REFLECT**, return JSON format with reflection analysis and memory cleanup decision:


{
"analysis": "Detailed reflection analysis here",
"pop_count": 2,
"reasoning": "Explain why these items should be removed and what the reflection concluded"
}


**D** **Case Study**


We present two representative case studies to qualitatively illustrate how the proposed framework operates
under different task settings, with a particular focus on
task-level memory control and cross-task experience
utilization.


**Case 1: Multi-step Medical Question Answering.**
As shown in Table 4, the system initially issues a broad
retrieval query that returns irrelevant medical content.
Instead of committing this noisy information to its internal state, the central coordinator explicitly invokes
REVISE action and modifies the retrieval key to progressively narrow the search scope. Through multiple
iterations of retrieval, inspection, and memory revision,
the system successfully identifies evidence relevant to
cerebrospinal fluid pressure and arrives at the correct
answer.


**Case 2: Deep Research and Report Generation.**
The second case in Figure 2 examines a long-horizon
deep research task involving open-ended information
gathering and report synthesis. Through active memory
management and long-term memory storage, STACKPLANNER retains a larger amount of high-quality, taskoriented information across extended reasoning steps,
resulting in a final report that is more insightful, comprehensive, and complete.


**E** **Computational Resources and Software**
**Environment**


Experiments were performed on a machine running
**Ubuntu 18.04.6 LTS** ( _bionic_ ), equipped with two Intel
Xeon E5-2680 v4 processors providing a total of 56
logical cores (28 cores per CPU) and 377 GB of RAM.
The system featured eight NVIDIA GeForce RTX 3090
GPUs, each with 24 GB of memory. The software
environment consisted of **Python 3.13.5** managed via
**Conda 23.5.2** . Model implementation and training employed **PyTorch 2.9.0** with CUDA 12.2, along with
HuggingFace **Transformers 4.57.1** . All software and
models were used with their default configurations unless otherwise noted. Training our method on the described hardware took roughly 45,713 seconds, while
inference times varied between 40 and 300 seconds per
sample depending on task complexity.


**F** **The Use of Large Language Models**


In this work, Large Language Models (LLMs) were used
solely to support language polishing and programming
tasks, including improving grammar, clarity, readability,
and providing general coding suggestions or debugging
advice. All outputs generated with LLM assistance
were carefully reviewed and verified by the authors.
The study’s conceptualization, experimental design, and
result analyses were conducted entirely by the authors,
with LLMs having no role in formulating research ideas
or drawing conclusions.


The normal value of cerebrospinal fluid pressure for lumbar puncture in the
supine position is ().

**Question**

A. 80-180mmH2O (0.78-1.76kPa) B. 50-70mmH2O (0.49-0.69kPa)
C. 230-250mmH2O (2.25-2.45kPa) D. 260-280mmH2O (2.55-2.74kPa)

**Ground**
A. 80-180mmH2O (0.78-1.76kPa)
**Truth**


**Central Coordinator**


<Plan> The question is asking about the normal cerebrospinal fluid (CSF) pressure range during lumbar puncture in the supine position. This is a factual
medical question that requires accurate clinical knowledge. Since no prior information is available in the memory stack, the next step should involve **retrieving**
**the correct and up-to-date medical standard for CSF pressure** . </Plan>


<Delegate> {"handsoff":"Search Agent", "subtask":"Please find the medical
standard of cerebrospinal fluid pressure"} </Delegate>


**Search Agent**


<Search> [Wiki_RAG]: Cerebrospinal fluid pressure </Search>


<Observation> Observation: {’title’: ’Hydrocephalus’, ’text’: ’Hydrocephalus
(also known as water on the brain) is a condition occurring in the brain ...’}
</Observation>


**Central Coordinator**


<Reflect> After reviewing the information, I found that the provided content does
**not directly mention the CSF pressure value** during lumbar puncture. I need
to perform a new search for more specific information. </Reflect>
REVISE ACTION: Last search results have been removed


<Delegate> {"handsoff":"Search Agent", "subtask":"Search authoritative medical sources for the normal range of cerebrospinal fluid (CSF) pressure during
lumbar puncture in the supine position, with a focus on standard reference values
expressed in both mmH2O and kPa."} </Delegate>



**Trajectory**



**Search Agent**


<Search>[Bocha_RAG]: Lumbar puncture, cerebrospinal fluid pressure</Search>

<Observation> Observation:{’title’: ’Normal Cerebrospinal Fluid Opening Pressure’, ’text’: ’...CSF pressure is commonly measured through a lumbar puncture...’} </Observation>


**Central Coordinator**


<Summarize> The previous action returned authoritative medical data regarding
the normal range of cerebrospinal fluid (CSF) pressure, including standard values,
abnormal conditions, and underlying causes. The information from high-quality
sources is summarized as follows:
Lumbar puncture is a commonly used clinical diagnostic and therapeutic procedure ... **the normal CSF pressure in supine position ranges from 80 to 180**
**mmH2O (0.78 to 1.764 kPa)** . If the pressure exceeds 300 mmH2O (2.94 kPa),
further CSF drainage should be avoided to prevent severe complications such as
brain herniation ...</Summarize>
REVISE ACTION: Previous search results have been summarized


<Delegate> {"handsoff":"Report Agent", "subtask":"Answer initial query based
on previous observations."} </Delegate>


**Report Agent**


<Conclusion> According to authoritative medical references, the normal CSF
pressure in supine position ranges from 80 to 180 mmH2O (0.78 to 1.764 kPa).
Among the options, **the one closest** to this range is **option A: 80–180** mmH2O
(0.78–1.76 kPa). Therefore, my answer is A. </Conclusion>


Table 4: Case Study of STACKPLANNER on a medical QA.


Survey Report on Multi-Agent System Frameworks (2025–2026)


Key Takeaways


**AutoGen** and **CrewAI** are currently the most popular multi-agent frameworks, particularly well-suited for report generation tasks.

**LangChain/LangGraph** are known for their visual workflow design and persistent state management, making them suitable for complex report

generation.

**Google ADK** and **Microsoft Agent Framework** provide enterprise-grade reliability and comprehensive documentation support.

**Atomic Agents**, as an open-source option, also demonstrates potential in report generation scenarios.

All mainstream frameworks support task planning, tool interaction, and long-horizon task execution, which are core requirements for report generation.

Standardized protocols (e.g., MCP, ACP) are promoting interoperability and production deployment of multi-agent systems.


Overview


With the rapid advancement of artificial intelligence technologies, Multi-Agent Systems (MAS) are emerging as a major trend in AI applications for 2025–

2026. By coordinating multiple specialized AI agents to accomplish complex tasks, MAS exhibit strong collaborative capabilities, particularly in scenarios

such as report generation that require multi-step reasoning and data integration.


Report generation typically involves multiple stages, including data collection, analysis, content organization, and formatted output. Through division of

labor and collaboration, multi-agent systems can assign these steps to agents with different areas of expertise, thereby improving efficiency and accuracy.

For example, one agent may be responsible for web search, another for data analysis, a third for content writing, and a master agent for final integration

and output.


This report provides an in-depth analysis of mainstream multi-agent frameworks, examines their suitability for report generation tasks, and, combined with

industry trends and practical application cases, offers guidance for technical decision-makers.


Detailed Analysis


1. Overview of Popular Multi-Agent Frameworks











**Applicable Scenarios** : Market analysis reports, financial report compilation.


LangChain/LangGraph


**Visual Workflows** : Manages report generation processes using graph structures, facilitating debugging.

**Persistent State** : Supports long-running task execution while ensuring data consistency.

**Modular Components** : Reusable modules improve development efficiency.

**Applicable Scenarios** : Academic research reports, multi-source data integration reports.


Google ADK


**Enterprise-Grade Reliability** : Validated at large scale and suitable for mission-critical tasks.

**Standardized Protocols** : Supports protocols such as MCP to ensure multi-agent interoperability.

**Applicable Scenarios** : Government reports, compliance document generation.


Microsoft Agent Framework


**Integration Capability** : Combines AutoGen’s collaboration strengths with Semantic Kernel’s enterprise features.

**Security** : Meets enterprise-grade security standards, suitable for sensitive data processing.

**Applicable Scenarios** : Internal audit reports, legal document generation.


Atomic Agents


**Open-Source Flexibility** : Allows customization of report generation pipelines based on specific needs.

**Distributed Architecture** : Supports large-scale data processing, suitable for complex report tasks.

**Applicable Scenarios** : Scientific papers, technical white papers.


Example Workflow for Multi-Agent Report Generation


1. **User Input** : The user submits a report request (e.g., “Generate a 2025 AI industry trend report”).

2. **Task Decomposition** : A master agent splits the task into subtasks (data collection, analysis, writing, formatting, etc.).

3. **Agent Collaboration** :


**Data Agent** : Responsible for web search and data collection.

**Analysis Agent** : Processes data and generates charts.

**Content Agent** : Writes the main body of the report.

**Formatting Agent** : Handles layout and formatting adjustments.


4. **Integrated Output** : The master agent aggregates the results of all subtasks and produces the final report.


Technical Challenges and Solutions


1. Task Planning and Coordination


**Challenge** : Multi-agent systems must effectively coordinate task assignment and progress synchronization among agents.

**Solutions** :


Use graph-based structures (e.g., LangGraph) to manage task workflows.

Introduce standardized protocols (e.g., MCP) to ensure consistent communication.


2. Tool Interaction and API Invocation


**Challenge** : Report generation often requires calls to external APIs (e.g., database queries, web search).

**Solutions** :


Integrate tool libraries (e.g., CrewAI’s tool system).

Use streaming function calling to improve response latency.


3. Long-Horizon Task Management


**Challenge** : Report generation may involve multi-step, long-running tasks.

**Solutions** :


Implement persistent state management (e.g., LangGraph).

Introduce error recovery mechanisms to allow task resumption after interruptions.


(b)















|Framework Name|Characteristics|Strengths|Weaknesses|
|---|---|---|---|
|AutoGen|Multi-agent conversations, integration with<br>the Microsoft ecosystem|Intuitive multi-agent collaboration, strong<br>performance on coding tasks|Steep learning curve|
|CrewAI|Developer-friendly,<br>strong<br>orchestration<br>capabilities|Rapid construction of production systems,<br>well-documented|Limited support for highly<br>complex tasks|
|LangChain/LangGraph|Visual workfows, modular design|High fexibility, active community|Requires strong engineering<br>expertise|
|Google ADK|Enterprise-grade reliability, comprehensive<br>documentation|Strong ecosystem support, widely adopted in<br>enterprises|Complex confguration|
|Microsoft<br>Agent<br>Framework|Integration of AutoGen and Semantic Kernel|Strong enterprise application support, high<br>stability|High initial learning cost|
|Atomic Agents|Open-source, distributed agents|Highly customizable, suitable for specifc<br>applications|Relatively small community|


2. Evaluation of Report Generation Capabilities


AutoGen


**Task Planning** : Supports multi-agent collaboration and decomposition of complex tasks.

**Tool Interaction** : Integrated with Semantic Kernel, providing rich API invocation capabilities.

**Long-Horizon Tasks** : Manages task states through conversational mechanisms, suitable for multi-step report generation.

**Applicable Scenarios** : Enterprise reports, technical documentation.


CrewAI


**Task Decomposition** : Automatically breaks down user requirements into multiple subtasks.

**Team Collaboration** : Supports parallel processing by multiple agents across different modules.

**Tool Invocation** : Built-in tools such as web search and database queries.


(a)


4. Data Consistency and Accuracy



**Challenge** : Ensuring accuracy and consistency of data transferred across agents.

**Solutions** :


Use version control and data validation mechanisms.

Introduce a supervisory agent for quality assurance.


Industry Trends and Future Outlook


1. Multi-Agent Collaboration as the Mainstream Architecture


McKinsey predicts widespread adoption of collaborative agent workflows by 2026.

Multi-agent systems will become the default form of Agentic AI, evolving from task execution tools into autonomous business process engines.


2. Standardized Protocols Driving Interoperability


The promotion of standardized protocols such as MCP and ACP will enable interoperability across agents from different vendors.

This will foster an open ecosystem for multi-agent systems and enhance cross-platform collaboration.


3. Human–Agent Collaborative Teams


By 2026, every employee will become a manager of intelligent agents, shifting focus from execution to strategic supervision.

New roles, such as agent team leaders, will emerge to oversee resource allocation and quality control.


4. China as a “Super Engine” for Multi-Agent Deployment


With a complete industrial chain, leading open-source models, and a massive market, China provides an ideal testing ground for multi-agent systems.

Government-led digital and intelligent transformation initiatives will drive enterprise upgrades and accelerate multi-agent adoption in sectors such as

public administration, finance, and industry.


Conclusion


Mainstream multi-agent frameworks—including AutoGen, CrewAI, LangChain/LangGraph, Google ADK, Microsoft Agent Framework, and Atomic Agents—

are all capable of supporting report generation tasks. Through task planning, tool interaction, and long-horizon task management, they can efficiently

complete the full pipeline from data collection to content generation.


Looking ahead, as standardized protocols become more widespread and human–agent collaboration models mature, multi-agent systems will play an

increasingly important role in enterprise-level report generation. Organizations should select frameworks that align with their specific needs while closely

monitoring technological trends to fully realize the potential of multi-agent systems.


(c)


Figure 2: Case Study of STACKPLANNER on a deepresearch task.
**Task:** “Please summarize the recently popular multi-agent system frameworks that are capable of performing report
generation tasks.”


