# One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

## One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents

**Zhaoxi Zhang** 1 **Yitong Duan** 2 **Yanzhi Zhang** 2 **Yiming Xu** 1 **Jiyan He** 2 **Yunfang Wu** 1

### Abstract

Locating the files and functions requiring modifi  cation in large open-source software(OSS)repos 

5

itories is challenging due to their scale and struc 

2

tural complexity.Existing large language model

0

(LLM)-based methods typically treat this as a

2 

repository-level retrieval task and rely on multiple

c

*Figure 1.Illustration of a LLM navigating through a code reposi *

a iliar tools hich o erlook code e ec tion

ux y

,w

v

x u

e

tory.The LLM is equipped with a single yet powerful tool:j ump,

l i d

**R**og c **N**an c**i** omp ca e LmLoMe con ro .i e prdopoishe

li t d l t l W

D

which is realized through a language server.

**epo av gator**,an

agent equ ppe w t

5

a **single execution-aware tool** —jumping to the

2

definition of a invoked symbol.This unified de 

] 

sign reflects the actual flow of code execution

ries remains limited.SWE -BENCH(Jimenez et al.,2023) currently serves as the most comprehensive benchmark for

E

while simplifying tool manipulation.RepoNavi 

evaluating whether LLMs can resolve real-world GitHub is 

S

gator is **trained end-to-end via Reinforcement**

sues.All pretrained LLMs can not process the whole repos 

s.

**Learning(RL)**directly from a pretrained model,

itory directly due to context limits.While SWE -AGENT

c

without any closed-source distillation.Experi 

[

ments demonstrate that RL-trained RepoNaviga 

(Jimenez et al.,2023)provides moderate gains,it remains

2 

far from enabling robust repository-level reasoning.

tor achieves state-of-the-art performance,with the

v

7B model outperforming 14B baselines,the 14B

Most existing agents rely on test-time scaling applied di 

7

model surpassing 32B competitors,and even the

rectly to pretrained LLMs(Liu et al.,2023;Chen et al.,2025;

95

32B model exceeding closed-source models such

Schmidgall et al.,2025).In software engineering(SWE)

as Claude-3.7.These results confirm that integrat 

0

tasks,tool usage is essential rather than optional:real-world

2

ing **a single,structurally grounded tool with**

repositories are far larger than the context window of current

2.

**RL training** provides an efficient and scalable

LLMs,making it impossible to process an entire codebase

solution for repository-level issue localization.

in a single forward pass.Agents must therefore iteratively

1 5

invoke tools to retrieve partial information from the repos 

2

itory and interleave natural-language reasoning with tool

: v **1.Introduction**

calls.

i X

With the rapid advancement of Large Language Models However,mainstream LLMs are rarely exposed to such

ra (LLMs)(Liu et al.,2024;Team,2024;Yang et al.,2025a),

agentic interaction patterns during pretraining and typically acquire tool usage only through few-shot prompting.Such

equipping LLMs with pre-built tools to form LLM agents has become a common paradigm for expanding their ca  in-context demonstrations are insufficient for learning com  pabilities(Shen,2024;Yuan et al.,2024;Lu et al.,2024). plex multi-step tool-chaining behaviors,especially under In the domain of software engineering(SWE),although limited context windows.Moreover,because tool definition

spaces are effectively unbounded,pretrained models cannot

LLM agents can effectively handle simple programming tasks(Hui et al.,2024;Guo et al.,2024a),their ability to fully internalize their semantics without post-training.To operate on large-scale open-source software(OSS)reposito 

mitigate these issues,post-training paradigms such as Super vised Finetuning(SFT)(Ma et al.,2025)and Reinforcement

1School of Computer Science, Peking University Learning with Verifiable Rewards(RLVR)(Yu et al.,2025a;

2Zhonggduancui n A@cademi y. Correfspondence tof:@Ykitodng Yue et al.,2025)have been applied,with promising results

Dcnu*>*an*<*uany tong zgc .ac.cn*>*,Yun ang Wu*<*wuy p u.e u. in domains including retrieval agents(Jin et al.,2025),GUI .

agents(Hong et al.,2024),and math agents(Yan et al.,

*Submitted to International Conference on Machine Learning*,2026. 2025).

1


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Directly training an agent to fix software issues,however,

any tools,most tools are out-of-domain(OOD)for LLMs.

remains difficult.A single bug often admits multiple valid Even for the most powerful models,failures often happen patches,making string-level evaluation unreliable.The

when calling the new-defined tools due to wrong calling

only precise evaluation method requires executing candi  format or failed parameter parsing.Thus,training a LLM date patches inside a dedicated Docker environment for

to master new-defined tool is critical for LLM agents.In 

each repository(Luo et al.,2025),which is prohibitively

tuitively,the tool-calling trajectories can be generated by a

expensive.To make training more tractable,we adopt a

more powerful LLM,and such trajectories can be used to

simplified yet widely generalizable assignment:**issue local ** train a student model via supervised finetuning(SFT)(Chen

**ization**.Prior work shows that a software issue becomes

et al.,2025).However,this pipeline requires a stronger

substantially easier to resolve once the relevant functions

teacher model which has capability to master the tool.Re 

and files are correctly identified(Chen et al.,2025;Ma et al.,

cently,more methods have emerged with no teacher-model

2025;Xia et al.,2024;Jiang et al.,2025).Since modern

required.Rejected-sampled finetuning(RFT)(Ahn et al.,

OSS repositories contain a significant amount of code —far 2024)utilizes generated trajectories of the agent itself via beyond any LLM ’s context window —localization drasti 

multiple rollouts.Agentic RL(Jin et al.,2025)is an on 

cally reduces the search space and improves downstream policy RLVR methods requiring only the result for verifiying solvability.Crucially,localization outputs a discrete set

trajectories.Such training methods yield remarkable results

ofpaths,enabling verifiable,string-level evaluation that is

when the tools are search engines(Jin et al.,2025),python

compatible with scalable training frameworks such as SFT

executer(Jimenez et al.,2023),calculator(Yan et al.,2025),

and RLVR.

and visual models(Gupta&Kembhavi,2023).

Existing localization agents(Ma et al.,2025;Chen et al.,2025;He et al.,2025)typically rely on multiple **2.2.Software Engineering Agents**

tools,including S e a r chCl a s s,S e a r chMethods,and The introduction of SWE-bench(Jimenez et al.,2023;Yang Get Impo rt s .Although effective to some extent,these

et al.,2024b)has motivated a range of agentic pipelines for

tools considers high-level abstractions(classes,function,

software engineering(SWE)tasks.Among them,SWE 

etc)of programing languages,which do not reflect how

AGENT(Yang et al.,2024a)and OPENHANDS(Wang et al.,

code actually executes.High-level abstractions,such as 2025a)are widely adopted frameworks that equip agents classes or inheritance,disappear after compilation,leav 

with tools for interacting with computing environments.

ing only sequential execution and j ump operations.Since Workflow-based methods such as Agentless(Xia et al., modern LLMs already excel at modeling sequential depen  2024)decompose issue resolution into localization,repair, dencies,we focus on enhancing their ability to j ump across

and validation subproblems.Chen et al.(2025)builds the re 

the repository —that is,to follow and inspect the source def

spository as a graph and applied graph-level searching tools

inition of symbols as they appear in execution.To this end, for localization and Wang et al(2025a)furthermore inte 

,

.

we introduce a single,structurally grounded tool:j ump, grated commit history as agent memory.RepoLens(Wang which retrieves the precise definition of a given symbol.

et al.,2025b)equip conceptual information of the respos 

Details of this tool are provided in Sec.3.3.

itory to enable repo-level understanding.These pipelines are training-free,compatible with closed-source language

Our main contributions are threefold:(1)We propose the

models,and yield competitive results.

first repo-level localization agent trained on reinforcement learning directly from the pretrained model,regardless of To enable task-specific training,DEEPSWE(Luo et al., distillation from a close-source model.(2)We design a 2025)and SWE -SWISS(He et al.,2025)employ reinforce  repository-navigation agent that operates by performing

ment learning and achieve strong performance.However,

realistic j ump operations aligned with actual execution se 

end-to-end training remains costly because patch evaluation

mantics.(3)We demonstrate that one unified tool signifi 

requires executing Docker environments across numerous

cantly improves efficiency and controllability compared to

repositories.Consequently,issue localization has emerged

multi-tool pipelines.

as a computationally efficient alternative,aiming to identify faulty components —at file or function level —rather than

### 2.Related Works

generating full patches.

**2.1.Agentic Training**

Recent localization agents include LOCAGENT(Chen et al., 2025)and COSIL(Jiang et al.,2025),which model code 

LLM agents are promising methods to equip models with bases as graphs and integrates them into LLMs, and complex tools while reasoning(Li et al.,2024;Huang et al., ORCALOCA(Yu et al 2025b)which enhances efficiency

.,

,

2024;Guo et al.,2024b).However,because most pretrained

through priority scheduling,action decomposition,and

LLMs are trained on texts only and developers can define

context pruning.From an open-source perspective,RE 

2


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

*Figure 2.Overview of our Repo Navigator.During the rollout phrase,the agent can call the j ump tool,and the language server will return*

the definition code of the symbol.This process is trained by reinforcement learning.

POSEARCHER(Ma et al.,2025),trained with distillation **3.1.Problem Formulation**

and Rt Lblon tdhe Qwen mtodel family(Team,2024),represents Given a repository *R* =*{f*1*, . . .,f**N**}*and an issue de 

a no a e a vancemen .

scription *q*,the goal is to output relevant code regions

Nevertheless,prior agents overlook the structural relations *Y* *∗* =*{*(*f**i**,g**i j*)*}*,where *g**i j* denotes a function or code within repositories —where modules,classes,and functions

span in file *f**i*.At each step *t*,the agent produces a optional

*,*

*,*

are cross-referenced across files —and typically rely on mul 

reasoning step *r**t*,a tool call *a**t*,and receives the observation

tiple search tools for symbol definition retrieval,amplifying

*o**t*,forming a trajectory *τ* =*{*(*r**t**,a**t**,o**t*)*}**tT*=1.After termi 

error propagation(see Sec.3).In contrast,we employ a sin 

ˆ

ˆ

nation,a final prediction *Y* is scored by a reward *R*(*Y,Y* *∗*) .

gle execution-logic-focused tool,reducing usage complexity. The objective is max*θ* E*τ ∼π**θ*[*R*(*τ*)] . Finally,our approach constitutes the first localization agent trained directly from pretrained models,without relying on **3.2.Agent Architecture**

distillation-based supervised finetuning,a crucial stage in both RepoSearcher(Ma et al.,2025)and LocAgent(Chen RepoNavigator uses a *single-tool* design to avoid multi 

tool orchestration overhead.At each step the policy *π**θ*

et al.,2025).

decides whether to continue reasoning or to emit a JSON  formatted tool call,while a symbol and its corresponding

### 3.Method

file are parsed to the tool.The agent receives structured ob 

We present **RepoNavigator**,a reinforcement-learning agent

servations(code snippets or error messages),then continues reasoning until termination.The loop is *reason → act →*

for repository-level issue localization.The method con  sists of three components:(1)a unified tool to retrieve the

*observe*.

definition of any symbols in a given file,(2)a reasoning – action agent loop that alternates between natural-language **3.3.Jump:Symbol Resolution**

reasoning and tool invocation,and(3)a GRPO-based RL Language servers resolve the definition of a Python symbol algorithm for optimizing long-horizon tool-augmented tra  through a deterministic static analysis pipeline that approxi  jectories.Below we provide the formal problem setting and

mates Python ’s runtime name-binding semantics.Given a

the detailed method.

symbol occurrence *s* at source location *ℓ*,Pyright computes a resolution mapping

*R*(*s,ℓ*)*→{*(*f**i**,p**i*)*},*

(1)

3


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

where each pair(*f**i**,p**i*)denotes a file path and a source Reference Policy Optimization(GRPO),which has the loss position corresponding to a valid definition site of *s*.In function: practice,we use fi l e path and symbol to resolve *ℓ*.If we have multiple symbols with the same name exist in the tsoaomlewchoidcehsanlilpopwest,fworeaacdcduirtaitoenraellsyolpuatirosen aonf*ℓ*i nde x to the

*L*GRPO(*θ*) =E(*s**t* *a**t*)*∼π**θ*  *π**θ*(*a*(*t**|s|**t*))*A*ˆ*t*

,

.

*,*

old *π**θ*old *a**t* *s**t*

*− β D*KL(*π**θ*old(*·|s**t*)*∥ π**θ*(*·|s**t*))](3)

**Syntactic Analysis** In this process,the source file is parsed into an abstract syntax tree(AST).The syntactic role of *s*(e.g.,name,attribute access,or call expression)

where the first term is the standard policy gradient objective with an estimated advantage function *A**t*,which promotes

ˆ

determines the subsequent resolution strategy.For attribute expressions *a.b*,Pyright treats *a* as a receiver expression

actions that lead to higher-than-expected returns.The sec 

whose type must be inferred prior to member lookup.

ond term is a Kullback-Leibler(KL)divergence penalty, scaled by a coefficient *β*,which acts as a trust region,pre 

**Lexical Scope Resolution** For a name symbol *x*,candi 

venting the updated policy *π**θ* from moving too far from the previous policy *π**θ*old .This formulation ensures stable

date definitions are searched along a scope chain

and consistent policy improvement by balancing reward

*S* =*{*local*,*enclosing*,*module*,*builtins*},*

(2)

maximization with behavioral consistency.

following Python ’s LEGB rule.Each scope maintains a The reward of GRPO process is calculated as: symbol table mapping identifiers to defining AST nodes.

ˆ

ˆ

*R*(*Y,Y* *∗**,τ*) =DICE(*Y,Y* *∗*)+S(*τ*)

(4)

**Static T**(**ype** i**I**b**n**l**feren**i **ce** .l Fodr)attrib*T*ut(e s)yfmbohls,it ciom  Dice is a common metric for set-level comparison,for set

putes a poss y un on-va ue type *a* or t e rece ver *Y*ˆ and set *Y* *∗*

expression *a* using type annotations,assignment flow analy  sis,function return types,and stub files( .pyi).Member

(*Y*ˆ *Y* *∗*) 2 *×|Y*ˆ *∩ Y* *∗**|*

DICE *,*

()

5

resolution is then defined as

=

ˆ *|Y|*+*|Y* *∗**|*

resolve(*a.b*) =[lookup(*b,*MRO(*t*))*,*

and *S*(*τ*)is the success rate of tool-calling extracted from

*t∈T*(*a*)

*τ*.We consider the tool-call to be failed when the format

where MRO(*t*)denotes the method resolution order of type is incorrect,or the symbol parsed does not exist,or for any

other reason that causes the tool to quit unexpectedly.

*t*.

**Import Dependency Graph** For cross-file resolution,im  **4.Experiment**

port dependency graph that statically emulates Python ’s module loading semantics is built.Import statements intro  **4.1.Experimnent Setup**

duce bindings that map local symbols to exported symbols **Datasets** We extract valid samples from SWE-smith of target modules,including re-exports and al l -based (Yang et al.,2025b)to form the training set.We apply filtering.Resolution may therefore traverse multiple mod  Qwen2.5 -7B -Instruct with RepoNavigator to sample each ules before reaching a concrete definition.

data for 16 times.A sample is abandoned if all 16 scores are zero.For validation,we test our method on SWE-bench 

**3.4.Reasoning–Action Loop**

verified(Jimenez et al.,2023),which is a human-verified subset of SWE-bench.We additionally test our method on

Given history *h**t* =(*q,o*1:*t −* 1*,a*1:*t −* 1),the agent samples

a subset of SWE-bench-pro(Yang et al.,2025b)(which

either a natural-language reasoning step *r**t* *∼ π**θ*(*·|h**t*)or a

structured tool call *a**t* *∼ π**θ*(*·|h**t*) .Tool calls must satisfy is a new and more difficult benchmark)for generalization.

a JSON grammar enforced via constrained decoding.The For ground-truth locations,we directly use the locations in loop continues until the agent outputs its final localization golden patches.All datasets are open-source and are built *Y*ˆ .

on real-world github issues.

**M**20**e**2**t**5**r**)**ics** lPi rdeviousllwordks(Chi ein et al.,20i25;Ma et al.,

**3.5.Reinforcement Learning**

app e reca an prec s on as metr cs.However,

We apply reinforcement learning with verifiable rewards because the predicted locations and ground-truth locations to train the agent directly from the pretrained model,with

are sets of strings,recall and precision singularly can not

no teacher model required.In practice,we apply Group

reflect the performance fairly.Thus,we utilize Sample-F1

4


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

*Table 1.Comparison of different agent pipelines on function-level and file-level Dice/IoU metrics.We use Qwen2.5 -Instruct series as*

our base model.**Bold numbers** denote the best performance among same-size models;underline numbers denote the best training  free performance among same-size models;yellow background illustrates training-free RepoNavigator;blue background illustrates RepoNavigator trained with GRPO.

**Function-level**

**File-level**

**Agent Pipeline**

**Model**

Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU

***Close-source Models***

RepoSearcher Claude3.7-Sonnet **66.80**

19.90

28.30

17.89 **89.71**

21.04

33.15

20.67

RepoNavigator Claude3.7-Sonnet 31.03

34.43

31.72

30.22 72.26

75.95

73.01

71.37

RepoNavigator

GPT5 -chat

30.42

34.56

31.17

29.67 58.17 61.87

58.88

57.33

RepoNavigator Claude4.5 -Sonnet 43.97

**45.76**

**43.62**

**41.31** 80.68 **81.92**

**79.94**

**77.49**

***Qwen2.5-7B***

Locagent

Training Free

17.62

1 1.71

12.71

10.31 60.96

34.88

40.67

33.33

CoSIL

Training Free

29.30

8.98

12.90

8.07 70.12

17.90

27.39

17.42

Agentless

Training Free

24.92

12.93

15.31

1 1.74 63.01

19.32

27.82

18.85

Orcaloca

Training Free

27.70

20.29

21.70

17.92 48.04

48.65

47.36

45.77

RepoSearcher Distillation+GRPO **63.26**

19.24

27.37

17.59 **84.11**

19.97

31.64

19.57

RepoNavigator

Training Free

15.89

17.46

16.19

15.46 42.36

43.23

42.12

40.97

RepoNavigator

GRPO

26.69 **30.34**

**27.49**

**26.43** 50.62 **53.83**

**51.63**

**50.62**

***Qwen2.5-14B***

Locagent

Training Free

35.62

13.32

17.71

12.32 71.42

31.66

40.77

30.64

CoSIL

Training Free

**48.61**

13.40

19.81

12.12 **78.35**

18.10

28.79

17.72

Agentless

Training Free

25.20

14.30

16.14

12.28 75.65

19.76

29.88

19.30

Orcaloca

Training Free

29.92

20.98

22.77

18.92 52.17 52.15

50.93

48.72

RepoSearcher

Training Free

26.13

1 1.96

14.35

10.60 74.77

18.80

28.79

18.15

RepoNavigator

Training Free

27.96

25.77

25.58

23.00 59.00 56.68

56.39

53.74

RepoNavigator

GRPO

31.02 **30.08**

**29.23**

**26.84** 61.60 **58.97**

**58.90**

**56.36**

***Qwen2.5-32B***

Locagent

Training Free

46.79

16.29

21.48

14.18 79.39

34.18

44.18

33.24

CoSIL

Training Free

55.38

14.85

22.1 1

13.52 83.50

19.34

30.77

18.93

Agentless

Training Free

40.79

24.07

27.33

22.08 78.93

25.60

35.38

24.96

Orcaloca

Training Free

39.14

25.59

28.72

22.89 59.57 59.51

58.1 1

55.62

RepoSearcher Distillation+GRPO **69.50**

20.29

29.1 1

18.23 **89.33**

20.27

32.93

20.35

RepoNavigator

Training Free

28.1 1

28.19

27.12

25.16 63.05

62.75

61.67

59.28

RepoNavigator

GRPO

33.71

**37.19**

**34.09**

**32.30** 67.29

**70.76**

**67.75**

**65.75**

(which is the averaged score of per-sample F1 values)and

to 128 on 4k training samples filtered from SWE-smith,

IoU(intersection out of union)as our core metrics.At the

with maximum prompt length and max response length

same time,we also present the recall and precision scores both set to 10240.Additionally,we rollout 8 times for to align with previous methods,although they do not reflect

each sample,and the temperature is set to 1.0 to encourage

the methods ’ performance fairly.

exploration.We use greedy decoding in the inference stage to ensure stable performance.More implementation details

**Training** For the 7B model,we conduct GRPO with 8

are provided in Appendix.B.

Tesla-A100-80G GPUs.For the 14B and 32B model,we train it with 16 Tesla-A100-80G GPUs.We apply verl **4.2.Effectiveness**

((SKhen,20t24l)a2s0th2e3t)rainitnhg firafmework,andi weWapplty viLLthM **Baselines** We compare our method against Locagent

won e a .,

as e n erence eng ne. e ra n e

model for 1 epoch,while the training batch size is fixed (Chen et al.,2025),CoSIL(Jiang et al.,2025),Agent 

5


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

*Table 2.Comparison of different agent pipelines on function-level and file-level metrics on SWE-bench Pro for generalization.Bold*

**numbers** denote the best performance among same-size models;underline numbers denote the best training-free performance among same-size models;yellow background illustrates training-free RepoNavigator;blue background illustrates RepoNavigator trained with GRPO.

**Function-level**

**File-level**

**Agent Pipeline**

**Model**

Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU

***Qwen2.5-7B***

LocAgent

Training Free 1.01

0.02

0.65

0.40 12.16

0.17

10.81

8.93

CoSIL

Training Free 8.64

3.33

4.58

2.87 26.64

8.47

12.1 1

7.70

Agentless

Training Free **12.82**

6.94

8.05

5.73 **39.41**

13.15

18.89

12.35

RepoSearcher Training Free 1.07

0.93

0.97

0.86 4.91

1.64

2.30

1.63

RepoNavigator Training Free 9.84

14.65

10.67

9.20 30.50 37.24

31.86

28.82

RepoNavigator

GRPO

12.33

**21.26**

**14.29**

**12.02** 36.36 **48.13**

**39.74**

**36.36**

***Qwen2.5-14B***

LocAgent

Training Free 6.22

0.13

3.65

2.65 15.58

0.21

1 1.69

9.53

CoSIL

Training Free 10.73

4.67

5.96

3.94 34.31

9.97

14.81

9.30

Agentless

Training Free 10.49

6.75

7.41

5.28 41.42

13.42

19.02

12.37

RepoSearcher Training Free 2.79

1.38

1.69

1.14 17.37

5.17

7.60

4.84

RepoNavigator Training Free 14.36

19.74

15.27

12.00 43.57 54.52

46.06

41.07

RepoNavigator

GRPO

**16.05 25.25**

**18.06**

**14.58 46.85**

**58.64**

**49.72**

**45.14**

***Qwen2.5-32B***

LocAgent

Training Free 8.72

0.17

4.30

2.90 25.73

0.38

19.77

16.50

CoSIL

Training Free 15.00

6.35

8.14

5.21 45.37

13.04

19.42

12.36

Agentless

Training Free 1 1.08

7.31

7.98

5.80 43.07

13.89

20.07

13.1 1

RepoSearcher Training Free 2.00

1.29

1.45

1.00 13.51

3.43

5.31

3.24

RepoNavigator Training Free 13.96

20.25

15.36

12.87 50.24 63.24

53.48

48.50

RepoNavigator

GRPO

**18.13 29.44**

**20.72**

**17.16 53.49 68.69**

**57.57**

**52.44**

baseline methods are presented in Appendix.A.

**Results** As illustrated in Table.1,on balanced metrics (S -F1 and IoU)for both function-level and file-level local  ization,our method surpasses all baseline methods with the same model size.Moreover,if we train RepoNavigator with GRPO,our 7B model surpasses 14B baselines,and our 14B model surpasses 32B baselines on S -F1 and IoU.This contributes to the validness of RepoNavigator furthermore. Although some baselines have higher recall score signifi  cantly lower precision score than RepoNavigator,and result in lower S -F1 and IoU.This indicates that RepoNavigator behaves more conservatively and generates less wrong lo  cSaOtiToAns.For 14Bllandi3i2B mf odels,RhepdoNTavhiigaitor alichievhes

*Figure 3.Ablation study:comparison between Repo Navigator*

among a tra n ng-ree met o s.

s mp es t at

with training free,RFT,GRPO with pure outcome and hybrid

the tool we implement is effective and promising,and our

reward on Qwen2.5 -7B -Instruct.

single tool pipeline is better than previous multiple tools pipelines.

less(Xia et al.,2024),Orcaloca(Yu et al.,2025b),and Compared with RepoSearcher,which is distilled from RepoSearcher(Ma et al.,2025).Detailed explaination of

claude-3.7-sonnet(Anthropic,2025)and reinforced by

6


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Agent Pipeline

Func-IoU(%) Resolved(%)

Agentless

5.28

10.12

LocAgent

2.65

13.01

RepoNavigator

12.00

14.74

RepoNavigator+RL

14.58

15.03

*Table 3.We use Qwen2.5 -14B -Instruct as the localization model,*

and use Qwen2.5 -32B -Instruct as the repair model on SWE  bench Verified.

**4.4.Scaling Law of Tool-Calling**

*Figure 4.Scaling law of tool-calling,where Pre and Post denote To assess the significance of tool-calling in Repo Navigator*

,

the corresponding metric before and after the RL training.

we varied the maximum number of tool-calling turns and reported the results in Fig.4.2.As shown in the figure,allow  ing more tool-calling turns consistently leads to improved

GRPO,trained RepoNavigator outperforms it on all metri  performance for RepoNavigator,both before and after re  ces except recall.Moreover,we found that our training-free inforcement learning(RL)training.In other words,these method outperforms RepoSearcher for 14B models.This is

results empirically validate the scaling law of tool-calling

probably due to the simplified tool we integrate to the agent in this context (see Sec.5 for more details).

.

To assess the generalizability of RepoNavigator,we present **4.5.Influence on Issue Resolution**

its performance on Python samples from the SWE-bench  To evaluate the impact of different localization results on Pro dataset(Yang et al.,2025b)in Table 2.The results

the final issue resolution performance we test RepoNaviga



on this dataset are consistent with those observed on SWE  tor against baselines on SWE-bench Verified.We directly

,

bench Verified.While we cannot fully exclude the potential

apply the repairing phrase of Agentless while replacing its

influence of data leakage in SWE-bench Verified,we can localization front-end with other methods.Table.3 illus  make a stronger claim regarding SWE-bench Pro,as it was

trates the results.Compared with baselines,RepoNavigator

released after the publication of the Qwen2.5 series.

has the highest performance on issue resolution,while rein  forcement learning improves its performance furthermore.

**4.3.Training Strategy Comparison**

To explore the capability of GRPO on agentic training,we **5.Discussion:Building Less yet More Capable**

compare GRPO against RFT-only and RFT+GRPO.As pre 

### Tools

sented in Fig.3,directly training with GRPO outperformes RFT-only and RFT+GRPO.Moreover,although RFT has ac  In this section,we analyze the logic behind RepoNaviga  cetable performance,the more steps RFT proceeds,the less

tor:building less tools with more powerful and more en 

improvement GRPO makes after the cold start.This conclu 

sembled functions is more effective than building multiple task-specific tools.

sion contradicts with previous SWE agents trained with RL (Ma et al.,2025),however,it aligns with the broader field of reinforcement learning,where RFT and SFT(as a cold start) **5.1.Impact on the Action Space ofAgents**

is effective only when the pretrained model is not strong Let the total number of available tools be denoted as *k*.

h(G

l 2024)Wh h

i d d l i

enoug

uohet ad.,d ia h.i h en tlie prdeitra nle mioie s When only a single tool —specifically the j ump tool —is re 

strodngl eni ohuRgL ainb ata hs g i-qiua tyf,SreFcTt y(RtFraT)n ngi a

tained,the system ’s structural relations become simpler,as

mo e w t

s etter t an tra n ng a ter

as ts b h h i

d h b

i

i d

ot t e act on space an t e o servat on space are restr cte

cold start.

to what this tool can access.In this case,the set of possible

We also remove the success rate in the reward function for

actions and observable elements is smaller than when multi 

ablation.As presented in Fig.3,reinforcement learning with ple tools are available.This reduction is generally beneficial, hybrid reward(with tool-calling success rate)has higher

since additional tools often introduce new and unfamiliar

performance than pure outcome reward(without tool-calling interfaces that large language models have not been exposed success rate).This indicates that learning to correctly call

to during pretraining,potentially increasing the likelihood

tools is vital in agentic learning.

of errors.

7


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

### Jump GetClass GetFunc GetStruc IoU ✓

### ✓

### ✓

### ✓

### 13.71

### ✓

### ✓

### ✓

### ✗

### 21.44

### ✓

### ✗

### ✗

### ✓

### 24.00

### ✓

### ✗

### ✗

### ✗

### 24.28

*Table 4.We change the tool set of Repo Navigator and present*

the function-level IoU(%)on Qwen2.5 -7B -Instruct.Apparently, excessive tools do not boost RepoNavigator ’s performance.

mantically activated by that entry point.Because every location that contributes to the issue must lie on some de 

*Figure 5.Venn graph illustrating access scope of j ump.Compared pendency path originating from the entry point,it is nec *

essarily reachable through this recursive symbol-reference

with the repository scope,the access scope has a much higher IoU

expansion.Therefore,the final access scope produced by

with the groundtruth set.

exhaustive j ump traversal is guaranteed to contain all loca  tions that must be modified to resolve the issue.

**5.2.Impact on Tool-Calling Success Rate**

**5.4.Verification**

For a given process in issue localization(for instance,check  ing the code snippet of a function),let the success probabil  To further verify this proposal,we change the tool set of ity of the *i*-th call be *p**i*.For a task that requires *k* sequential RepoNavigator and conduct RL training with only the out  tool invocations,the overall success rate can be expressed

come reward.We add excessive tools which were frequently

as

used in previous works(Chen et al.,2025;Ma et al.,2025;

*k*

Jiang et al.,2025)and present the result in Table.4.*Get *

*P*succ(*k*) =Y *p**i* *.*

(6) *Class/GetFunc* takes a class/function name as input and

*i*=1

outputs the class/function definition.*GetStruc* takes no in 

Since each step introduces an additional potential point of put and outputs the repository ’s structure.The results clearly failure,the cumulative success rate typically decreases as implies that additional tools do not increase model ’s perfor

mance.This inspires researchers to develop **less but more**

the number of required tool calls increases.Therefore,in

**capable tools**.

general,completing a task with a single,more versatile tool tends to be more reliable than relying on multiple narrow  scope tools executed in sequence.

### 6.Conclusion

Iln thlisi workl,weliintrioduced RephoNdavigatorf,a reposiitoiry 

**5.3.Impact on the Prediction Space**

eve ssue oca zat on agent t at eparts rom ex st ng

The access scope of a tool is defined as the complete set of

multi-tool paradigms by leveraging a single,more-capable

files,symbols,and other resources that the tool can access j ump tool for symbol resolution.This unified design faith  within a repository.For a j ump tool that navigates to sym  fully reflects real code execution flow while significantly bol definitions,its access scope can be obtained by starting

reducing the complexity and brittleness of multi-step tool

from a given entry point and recursively resolving all ref

chaining.Through tool-integrated GRPO,RepoNavigator

erenced symbols until no new definitions can be reached. learns to reason,invoke tools,and refine its predictions in a Apparently,its access scope is significantly smaller than the

closed-loop manner,enabling end-to-end optimization with 

full repository scope.Consequently,when computing the

out relying on closed-source teacher models or distillation.

Intdershection ovder Uhnion(IoiU)bhetwjeen the plredictlioni set Extensive experiments across SWE-bench-Verified and

an t e groun trut set,us ng t e ump too resu ts n a SWE b h P d

-fenhc -rol emlionsitrate t fat epo aWv gathor ac i evlel s

h R N i

hi

higher IoU,as depicted in Fig.5.On the other hand,ap 

statel -o -the-art loca zatfion iper ohrmancie.l e t eorfetlca yl

plying multiple repo-level retrivel tools results in the access

jania yl ze t ie rei sudts,ichon irmf ng t at als ngie power u toido ,

scope equal to the whole repository scope.

o nt y opt m ze w t re n orcement earn ng,can prov e

When we start from the entry point and repeatedly apply

stronger robustness and more reliable multi-step reason 

j ump —which retrieves the definition of each referenced ing than previous frameworks relying on multiple narrowly symbol —we effectively traverse all symbols that are se 

scoped tools.

8


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Our findings highlight the importance of aligning agent tool  **References**

ing wiith reallexecutiioin structurel,aknd bshow ithlat eifficient Ahn,J.,Verma,R.,Lou,R.,Liu,D.,Zhang,R.,and Yin,W.

rfeason dnig-too icod-tra n ng can un ocd slu sFtant a ga nks eveilnl

Large language models for mathematical reasoning:Pro 

or lme um-sdize RopenN-souirce mfo e sP.huture wor w

gresses and challenges.*arXivpreprint arXiv:2402.00157*,

exp ore exten ng epo av gator rom yt on to more pro 

2024.

gramming languages.

Anthropic.

Claude 3.7 sonnet and claude code.

http s://www .anth ropi c .c om/news/ cl aude -3 -7 -s onnet, February 2025.

data:

2025 -1 1 -18. Chen,Z.,Tang,R.,Deng,G.,Wu,F.,Wu,J.,Jiang,Z., Prasanna,V.,Cohan,A.,and Wang,X.LocAgent:Graph  guided LLM agents for code localization.In Che,W., Nabende,J.,Shutova,E.,and Pilehvar,M.T.(eds.),*Pro * *ceedings ofthe 63rdAnnual Meeting ofthe Association* *for Computational Linguistics(Volume 1:Long Papers)*, pp.8697 –8727,Vienna,Austria,July 2025.Association for Computational Linguistics.ISBN 979-8 -89176-251 - 0.doi:10.18653/v1/2025.acl-long.426.URL http s: //a cl anthol ogy .o rg/2 0 2 5.a cl -l ong .4 2 6/ . Guo,D.,Zhu,Q.,Yang,D.,Xie,Z.,Dong,K., Zhang,W.,Chen,G.,Bi,X.,Wu,Y.,Li,Y.,et al. Deepseek-coder:When the large language model meets programming –the rise of code intelligence.*arXivpreprint* *arXiv:2401.14196*,2024a. Guo,T.,Chen,X.,Wang,Y.,Chang,R.,Pei,S.,Chawla, N.V.,Wiest,O.,and Zhang,X.Large language model based multi-agents:A survey of progress and challenges. *arXiv preprint arXiv:2402.01680*,2024b. Gupta,T.and Kembhavi,A.Visual programming:Compo  sitional visual reasoning without training.In *Proceedings* *ofthe IEEE/CVF conference on computer vision andpat * *tern recognition*,pp.14953 –14962,2023. He,Z.,Yang,Q.,Sheng,W.,Zhong,X.,Zhang,K.,An,C., Shi,W.,Cai,T.,He,D.,Chen,J.,and Xu,J.Swe-swiss:A multi-task fine-tuning and rl recipe for high-performance issue resolution.https://github.com/zhenyuhe00/SWE  Swiss,2025.Notion Blog. Hong,W.,Wang,W.,Lv,Q.,Xu,J.,Yu,W.,Ji,J.,Wang,Y., Wang,Z.,Dong,Y.,Ding,M.,et al.Cogagent:A visual language model for gui agents.In *Proceedings of the* *IEEE/CVF Conference on Computer Vision and Pattern* *Recognition*,pp.14281 –14290,2024. Huang,X.,Liu,W.,Chen,X.,Wang,X.,Wang,H.,Lian, D.,Wang,Y.,Tang,R.,and Chen,E.Understanding the planning of llm agents:A survey. *arXiv preprint* *arXiv:2402.02716*,2024. Hui,B.,Yang,J.,Cui,Z.,Yang,J.,Liu,D.,Zhang,L., Liu,T.,Zhang,J.,Yu,B.,Lu,K.,et al.Qwen2.5 -coder technical report.*arXiv preprint arXiv:2409.12186*,2024.

9


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Jiang,Z.,Ren,X.,Yan,M.,Jiang,W.,Li,Y.,and Schmidgall,S.,Su,Y.,Wang,Z.,Sun,X.,Wu,J.,Yu,X., Liu,Z. Cosil:Software issue localization via llm 

Liu,J.,Moor,M.,Liu,Z.,and Barsoum,E.Agent lab 

driven code repository graph searching.*arXiv preprint*

oratory:Using llm agents as research assistants.*arXiv*

*arXiv:2503.22424*,2025.

*preprint arXiv:2501.04227*,2025.

Jimenez,C.E.,Yang,J.,Wettig,A.,Yao,S.,Pei,K.,Press, Shen Z Llm with tools:A survey *arXiv preprint*

, .

.

O.,and Narasimhan,K.Swe-bench:Can language mod 

*arXiv:2409.18807*,2024.

els resolve real-world github issues?*arXiv preprint* *arXiv:2310.06770*,2023.

Team,Q. Qwen2 technical report. *arXiv preprint* *arXiv:2407.10671*,2024.

Jin,B.,Zeng,H.,Yue,Z.,Yoon,J.,Arik,S.,Wang,D., Zamani,Hd.l,and Han,J.hSearich-r1:iThrainiinfg llms to Wang,X.,Li,B.,Song,Y.,Xu,F.F.,Tang,X.,Zhuge,

rleasoin an everage searc eng*2* n*0*e*3*s *0*w*9*t *16*re2n02o5rcement

M.,Pan,J.,Song,Y.,Li,B.,Singh,J.,Tran,H.H.,

earn ng.*arXiv preprint arXiv:5. 5* ,

Li,F.,Ma,R.,Zheng,M.,Qian,B.,Shao,Y.,Muen 

.

Kwon,W.,Li,Z.,Zhuang,S.,Sheng,Y.,Zheng,L.,Yu,

nighoff,N.,Zhang,Y.,Hui,B.,Lin,J.,Brennan,R.,

C.H.,Gonzalez,J.E.,Zhang,H.,and Stoica,I.Efficient

Peng,H.,Ji,H.,and Neubig,G. Openhands:An

memory management for large language model serving

open platform for AI software developers as general 

with pagedattention.In *Proceedings ofthe ACM SIGOPS*

ist agents.In *The Thirteenth International Conference* *on Learning Representations*,2025a. URL http s:

*29th Symposium on Operating Systems Principles*,2023.

//ope n revi ew .net/fo rum?id=OJd3 ayDD oF.

Langley,P.Crafting papers on machine learning.In Langley, P.(ed.),*Proceedings ofthe 17th International Conference* Wang,Y.,Mao,W.,Wang,C.,Zhou,Z.,Zhou,Y.,Zhao,W., *on Machine Learning(ICML 2000)*,pp.1207 –1216,Stan 

Lou,Y.,and Peng,X.Extracting conceptual knowledge to

ford,CA,2000.Morgan Kaufmann.

locate software issues.*arXivpreprint arXiv:2509.21427*, 2025b.

Li,Y.,Wen,H.,Wang,W.,Li,X.,Yuan,Y.,Liu,G.,Liu, J.,Xu,W.,Wang,X.,Sun,Y.,et al.Personal llm agents:Xi C S D Y D S d Zh L A l D

a, . ., eng, ., unn, .,an

ang, . gent ess:e 

I i h d

ns gi ts an*Xi*survey a*i* out*X*t*i*e *2*c*4*ap*0*a*1 054*ty*5*,*9*e 20c2e4ncy an

b h

bili ffi i

d

mystifying llm-based software engineering agents.*arXiv*

secur ty.*ar v prepr nt ar v:*

*.*

,

.

*preprint arXiv:2407.01489*,2024.

Liu,A.,Feng,B.,Xue,B.,Wang,B.,Wu,B.,Lu,C.,Zhao, C Deng C Zhang C Ruan C et al Deepseek-v3 Yan,Y.,Wang,S.,Huo,J.,Yu,P.S.,Hu,X.,and Wen,Q.

.,

, .,

, .,

, .,

.

Mathagent:Leveraging a mixture-of-math-agent frame 

technical report.*arXiv preprint arXiv:2412.19437*,2024.

work for real-world multimodal mathematical error de 

Liu,Z.,Zhang,Y.,Li,P.,Liu,Y.,and Yang,D. Dy 

tection *arXiv preprint arXiv:2503 18132* 2025

.

*.*

,

.

namic llm-agent network:An llm-agent collaboration framework with agent team optimization.*arXiv preprint* Yang,A.,Li,A.,Yang,B.,Zhang,B.,Hui,B.,Zheng,B., *arXiv:2310.02170*,2023.

Yu,B.,Gao,C.,Huang,C.,Lv,C.,et al.Qwen3 technical report.*arXiv preprint arXiv:2505.09388*,2025a.

Lu,J.,Holleis,T.,Zhang,Y.,Aumayer,B.,Nan,F.,Bai, F.,Ma,S.,Ma,S.,Li,M.,Yin,G.,et al.Toolsand  Y J Ji

ang, ., menez, . ., e g, ., ere, ., ao, .,

C E W tti A Li t K Y S

box:A stateful,conversational,interactive evaluation

Narasimhan,K.R.,and Press,O.SWE-agent:Agent 

benchmark for llm tool use capabilities.*arXiv preprint*

computer interfaces enable automated software engi 

*arXiv:2408.04682*,2024.

neering.In *The Thirty-eighth Annual Conference on* *Neural Information Processing Systems*,2024a.URL

Luo,M.,Jain,N.,Singh,J.,Tan,S.,Patel,A.,Wu,Q., Ariyak,A.,Cai,C.,Tarun Venkat,S.Z.,Athiwaratkun,

http s://a rxiv .o rg/ab s/2 4 0 5.1 5 7 93.

B.,Roongta,M.,Zhang,C.,Li,L.E.,Popa,R.A.,

Yang,J.,Jimenez,C.E.,Zhang,A.L.,Lieret,K.,Yang,

Sen K and Stoica I Deepswe:Training a state

, .,

, .



J.,Wu,X.,Press,O.,Muennighoff,N.,Synnaeve,G.,

of-the-art coding agent from scratch by scaling rl.

Narasimhan,K.R.,et al.Swe-bench multimodal:Do ai

http s://p retty -radi o -b7 5.not i on .s it e/ D e epSWE -T rai ni ng -a -Ful ly -Ope n -s ou r ced -St atsey-stoemf -stghenee-rAalrizte-tCoovdisiunagl -soAftgweanrte -dbomy -aiSncs?al*a*i*r*n*X*g*iv*-RL -2 2 2 8 1 9 0 2 c1 4 6 8 1 93 aabbe 9 a8 c5 9bbe3 3, 2025.Notion Blog.

*preprint arXiv:2410.03859*,2024b.

Ma,Z.,Peng,C.,Zeng,Q.,Gao,P.,Zou,Y.,and Xie, Yang,J.,Lieret,K.,Jimenez,C.E.,Wettig,A.,Khandpur, B.Tool-integrated reinforcement learning for repo deep

K.,Zhang,Y.,Hui,B.,Press,O.,Schmidt,L.,and Yang,

search,2025. URL http s://a rxiv .o rg/ab s/

D.Swe-smith:Scaling data for software engineering

2 5 0 8.0 3 0 1 2.

agents.*arXiv preprint arXiv:2504.21798*,2025b.

10


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Yu,Q.,Zhang,Z.,Zhu,R.,Yuan,Y.,Zuo,X.,Yue,Y.,Dai, **A.Detailed Illustration of Baselines**

W.,Fan,T.,Liu,G.,Liu,L.,et al.Dapo:An open-source llm reinforcement learning system at scale.*arXivpreprint* **Agentless** Agentless(Xia et al.,2024)is a workflow for

issue localization.First,it identifies suspicious files in the

*arXiv:2503.14476*,2025a.

repository.Second,relevant classes and functions are de 

Yu,Z.,Zhang,H.,Zhao,Y.,Huang,H.,Yao,M.,Ding, tected.Third,precise locations for edit are given by LLMs K.,and Zhao,J.Orcaloca:An llm agent framework based on the classes and functions. for software issue localization,2025b.URL http s: //a rxiv .o rg/ab s/2 5 0 2.0 0 3 5 0.

**CoSIL** CoSIL(Jiang et al.,2025)is an agent which first conduct file-level localization and then conduct function 

Yuan,S.,Song,K.,Chen,J.,Tan,X.,Shen,Y.,Kan,R., l l l li ti C SIL d

efve docla z(a lon.f o ti y)ndamica tyh cons rulc s lca grahpi s

i ll

t t ll h

Li,D.,and Yang,D.Easytool:Enhancing llm-based

o mo u es dc assl,i unc otns t ur ngi et repfof-etvie lsearcd ng

agents with concise tool instruction. *arXiv preprint*

process,an app es con ex prun ng o e ec ve y re uce

*arXiv:2401 06201* 2024

*.*

,

.

the searching scope.

Yue,Y.,Yuan,Y.,Yu,Q.,Zuo,X.,Zhu,R.,Xu,W.,Chen, J.,Wang,C.,Fan,T.,Du,Z.,et al.Vapo:Efficient and **LocAgent** LocAgent(Chen et al.,2025)is almost a fully  reliable reinforcement learning for advanced reasoning

automatic LLM agent besides its planning prompt concate 

tasks.*arXiv preprint arXiv:2504.05118*,2025.

nated into the context at the beginning of the searching process.It builds the whole repository into a direct hetero  geneous graph,whose nodes are files,classes,and functions. Additionally,edges are built by dependencies such as im  ports and invocations.Multiple graph-level searching tools are equipped to the LLM for multi-hop reasoning.

**RepoSearcher** RepoSearcher(Ma et al.,2025)is an agent that first conducts file-level localization and then function  level localization,which aligns with CoSIL.RepoSearcher introduced the first training framework *ToolTrain* for lo  calization agents,which is composed of distilling from a close-source model(Claude3.7-Sonnet in RepoSeacher)as warmup and reinforcement learning to further enhance the performance.

**Ours** Compared with all baselines,we are the first fully  automatic LLM agent,with no fixed workflow and no plan  etary prompt,and we are the first method trained directly from pretrained open-source LLMs without a close-source teacher model.Lastly,we only integrate a single yet power ful tool to the agent,which reduces compounding error and narrows the access scope of the agent.

### B.Experimental Details

**Hyperparameters** We set clip ratio low to 0.2, clip ratio high to 0.8,learning rate to 10 *−*6, train  ing batch size to 128,training temperature to 1.0,maximum tool-calling times to 12,and max response length to 10240.

**Metrics** Given the set of predicted locations(ether file 

ˆ

level or function-level)*Y*,and the set of groundtruth loca  tions *Y* *∗*,the aforementioned metrics are calculated as:

ˆ

Recall =*|Y ∩ Y* *∗**|*

(7)

*|Y* *∗**|*

1 1


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Jump GetClass GetFunc GetStruc Recall Precision F1

IoU Recall Precision F1

IoU

✓

✓

✓

✓

14.28

15.44

14.40 13.71 35.78

36.76 35.59 34.55

✓

✓

✓

✗

22.60

25.02 22.80 21.44 48.49

50.13 48.52 47.17

✓

✗

✗

✓

24.64

27.48 25.05 24.00 53.48

55.76 53.68 52.69

✓

✗

✗

✗

**25.11**

**29.16 25.75 24.28 55.81**

**58.71 56.32 54.89**

*Table 5.We change the tool set of Repo Navigator and present the function-level IoU.Because the j ump tool is already powerful enough*

for localization,excessive tools do not increase its performance.

Precision=*|Y|∩Y*ˆ *Y|* *∗**|*

ˆ

rmepanocseitoorfyt—healnadngduyangaemsiecrvimerp,oasrtistscafunndcetigornaadleitythreelpieersfoonr

(8)

static analysis techniques such as abstract syntax trees and

S l F1 2 *×|Y ∩ Y* *∗**|*

ˆ

symbol tables.When such circumstances occur,the tool

amp e- =*|Y*ˆ*|*+*|Y* *∗**|*

(9)

returns an error message indicating that the definition of the current symbol cannot be located due to unknown reasons.

IoU *|Y ∩ Y* *∗**|*

ˆ

(10) Nevertheless,in our empirical evaluation,we did not ob 

= *|Y ∪ Y* *∗**|*

ˆ

serve any instances of monkey patching or dynamic imports within the analyzed datasets.

In practice,when the prediction set *Y* is empty(for instance,

ˆ

total failure),we set recall,precision,sample-F1,and IoU to zero.We use the function-level localization result of **C.Threats to Validity**

different methods and apply the patch generation backend **Groundtruth Retrieval** A limitation of our work lies in in Agentless(Xia et al.,2024)to generate patches.Re  the extraction of groundtruth locations.We extract modified solved(%)denotes the percentage of samples that pass all locations directly from the gold pat ch in the datasets, test units after applying the patch.

which may ignore other patches that also resolve the issue. Our evaluation metrics do not take these correct alternatives

**Implementation** When the response exceeds the maxi  into consideration.However,using golden patches is ac  mum length,we clip and force the agent to stop,and we give

ceptable when comparing mutliple methods If a method

zero as its score.When the agent exceeds the maximum

.

reveals golden locations(locations in golden patches),it

tool-calling times(which is 12),we add **”You must not call**

undoubtedly contributes to the resolution of the issue and

**tools anymore,and you must give the final answer”** to the

,

the result in Table 3 demonstrates this claim

.

.

tool ’s response.Most of the time,the agent will stop calling tools and generate the final response.If not,we force it to stop and give zero as its score.Note that when the maxi  **Language Limit** Another limitation is that we only evalu  mum tool-calling times is not achieved and the final answer

ate Python repositories in our experiments.This is because

is generated,the agent loop will stop automatically.The

each language(C/C++,Java,etc.)has its unique language

aforementioned process is an automatic agentic framework,

server,and we only succeed in implementing the language

which allows the agent to explore in the environments with

server ofpython.We will implement more language servers

little constraints.

and validate our approach on more programing languages in the future.

**Preventing Data Leakage** It is a widespread concern thalitddi atafleakage iatithe preh-trdainNing phhrasle threatensl thde **D.Case Study**

va ty o post-tra n ng met o s. evert e ess,we exc u e this concern by results in Tabel.2.The SWE-bench Pro In this section,we present the full trajectory of RepoNavi  dataset was published in 2025,while the Qwen2.5 series gator on *astropy astropy-12907* from SWE-bench Verified. were published in 2024.Moreover,we exclude the samples We apply the default tool-calling prompt template of verl in the training dataset if the repository also appears in SWE  (Shen 2024)and present an example Noted we do not

,

.

,

bench Verified or SWE-bench Pro.

present any process restrictions in our prompt,encourag  ing RepoNavigator to plan,call tools,and make decisions

**Language Server** In practice,we apply a Python lan  full-automatically.This is distinct with Agentless(which guage server to extract the definition code corresponding has a fixed workflow),LocAgent(which predefines a spe  to an invoked symbol within a repository.However,the

cific step-by-step workflow in its system prompt),CoSIL

presence of monkey patches —runtime modifications to the

and RepoSearcher(which is half-automatic because some

12


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

forced steps are added to the workflow besides the automatic multi-turns tool-calling conversations).

13


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Prompt

[sy st em] You a re Qwe n, c re at ed by Alibaba Cl oud . You a re a helpful a s s i st ant .

#T o ol s

You may c al l one o r mo re fun ct i on s t o a s s i st with the u s e r que ry .

You a re p rovided with fun ct i on s ignat u re s withi n <t o ol s></t o ol s>XML t ag s: <t o ol s> {"type": "fun ct i on", "fun ct i on":{"name": "che ck", "de s c ript i on": "I n the spe ci fi c fi l e path, a symbol i s re fe r red and thi s t o ol c an fi nd whe re the t o ol i s de fi ned . F o r i n st an ce, i n the fi r st t u rn, fi l e path i s the e nt ry poi nt of .",

_

"pa ramet e r s":{"p rope rt i e s":{"symbol":{"de s c ript i on": "The symbol who s e de fi nit i on c ode wi l l be give n t o the age nt .", "type": "st ri ng"}, "fi l e path":

_

{"de s c ript i on": "The rel evant path t o the fi l e whe re the symbol i s re fe r red .", "type": "st ri ng"}}, "requi red": ["symbol", "fi l e_path"], "type": "obj e ct"}}} </t o ol s>

F o r e a ch fun ct i on c al l, ret u rn a j s on obj e ct with fun ct i on name and a rgume nt s withi n <t o ol _c al l></t o ol _c al l>XML t ag s: <t o ol c al l>

_

{"name": <fun ct i on -name>, "a rgume nt s": <a rg s -j s on -obj e ct>} </t o ol c al l>

_

[u s e r] You a re give n a c odeba s e and an i s s ue, you ne ed t o l o c at e the fi l e s and fun ct i on s c au s i ng thi s i s s ue . You c an c al l the t o ol t o che ck the de fi nit i on c ode of a symbol . You c an only che ck the symbol on ce fo r e a ch t u rn . The ’ fi l e path ’ i s the rel evant path of whe re the symbol i s c al l ed,

_

NOT whe re it i s de fi ned! F o r i n st an ce, i f ’ cl a s sA .fun ct i onB ’ i s what you want t o che ck(whi ch i s c al l ed i n fi l eA .py), you should di re ct ly che ck ’ fun ct i onB ’ i n ’ fi l eA .py ’ .

Thi s i s the i s s ue: [P robl em St at eme nt]

The e nt ry fi l e of the c ode ba s e i s: [Rel evant P ath T o E nt ry P oi nt] [E nt ry P oi nt]

You r fi nal an s we r should be al l fun ct i on s that should be modi fi ed, s u ch a s: rel evant/path/t o/fi l e 1.py::fun c_name 1,rel evant/path/t o/fi l e2.py::fun c_name2, . . .(a s e ri e s of fi l e::fun ct i on pai r s s epe rat ed by c omma) P l e a s e put you r fi nal an s we r i n s ide\boxed{} only i n the l a st t u rn . You c an only c al l the t o ol on ce e a ch t u rn . F o r i n st an ce: {’ name ’:’ che ck ’, ’ a rgume nt s ’:{’ symbol ’:’ symbol _t o_be_che cked ’, ’ fi l e_path ’: ’ fi l e_whe re_the_symbol _i s _u s ed ’}}

14