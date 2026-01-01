# One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

## One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents

**Zhaoxi Zhang** 1 **Yitong Duan** 2 **Yanzhi Zhang** 2 **Yiming Xu** 1 **Jiyan He** 2 **Yunfang Wu** 1

### Abstract

Locating the file s and functions requiring modifi  cation in large open-source software(OS S)repo s 

| 5 | itorie s i s challenging due to their scale and struc  |
| --- | --- |
| 2 | 0 | tural complexity.Exi sting large language model |
| 2 | (LLM)-b ased methods typic ally treat thi s as a |
| repo sitory-level retrieval task and rely on multiple | Figure 1.Illu stration of a LLM navigating through a code repo si  |
| D | e | c | l i d | Rog c Nan ci omp ca e Lm LoMe con ro .i e prdopoishe | a iliar tool s hich o erlook code e ec tion | ux y | epo av gator,an | ,w | li t d l t l W | v | agent equ ppe w t | x u | which i s realized through a language server. | tory.The LLM i s equipped with a single yet powerful tool:j u mp, |
| 2 | 5 | a single execution-aware tool -j umping to the |
| ] | S | E | s. | while simplifying tool manipulation.Repo Navi  | Learning(RL)directly from a pretrained model, | definition of a invoked symbol.Thi s unified de  | gator i s trained end-to-end via Reinforcement | sign reflects the actual flow of code execution | rie s remains limited.S WE -B EN C H(Jimenez et al., 2023) | itory directly due to context limits.While S WE -AGENT | evaluating whether LLM s can resolve real-world Git Hub i s  | currently serve s as the mo st comprehensive benchmark for | sues.All pretrained LLM s can not proces s the whole repo s  |
| [ | c | without any clo sed-s ource di stillation.Experi  | (Jimenez et al., 2023)provide s moderate gains,it remains |
| 2 | tor achieves state-of-the-art performance,with the | ments demonstrate that RL-trained Repo Naviga  | far from enabling robu st repo sitory-level reasoning. |
| 95 | 7 | v | 7B model outperforming 1 4B baseline s,the 1 4B | model surpas sing 32B competitors,and even the | 32B model exceeding clo sed-source model s such | rectly to pretrained LLM s(Liu et al.,2023;Chen et al.,2025; | Mo st exi sting agents rely on te st-time sc aling applied di  | S chmidgall et al., 2025).In s oftware engineering(SWE) |
| 0 | as Claude-3.7.These results confirm that integrat  | tasks,tool u s age i s es sential rather than optional:real-world |
| 2 | ing a single,structurally grounded tool with | repo sitories are far larger than the context window of current |
| 2. | RL training provide s an efficient and sc alable | LLM s,making it impo s sible to proce s s an entire codebase |
| 1 | solution for repo sitory-level i s sue localization. | in a single forward pas s.Agents mu st therefore iteratively |

5

invoke tool s to retrieve partial information from the repo s 

2

itory and interleave natural-language reas oning with tool

| : | v 1.Introduction | call s. |
| --- | --- | --- |
| i | X | With the rapid advancement of Large Language Model s However,mainstream LLM s are rarely expo sed to such |
| ra (LLM s)(Liu et al.,2024;Team,2024;Yang et al.,2025 a), | agentic interaction patterns during pretraining and typically |
| equipping LLM s with pre-built tool s to form LLM agents | acquire tool u s age only through few-shot prompting.S uch |
| has become a common paradigm for expanding their c a  in-context demonstrations are insufficient for learning com  |
| pabilitie s(Shen,2024;Yuan et al.,2024;Lu et al., 2024). plex multi-step tool-chaining behaviors,e specially under |
| In the domain of s oftware engineering(SWE),although limited context window s.Moreover,becau se tool definition |
| LLM agents c an effectively handle simple programming | spaces are effectively unbounded,pretrained model s cannot |
| tasks(Hui et al.,2024;Guo et al.,2024a),their ability to fully internalize their semantic s without po st-training.To |
| operate on large-scale open-source software(OS S)repo sito  | mitigate these i s sues,po st-training paradigms such as S uper  |
| 1 S chool of Computer S cience, Peking University Learning with Verifi able Rewards(RLVR)(Yu et al.,2025 a; | vi sed Finetuning(SFT)(Ma et al., 2025)and Reinforcement |
| 2Zhonggduancui n A@c ademi y. Correfspondence tof:@Yk itodng Yue et al., 2025)have been applied,with promi sing re sults |
| Dcnu>an<uany tong zgc .ac.cn>,Yun ang Wu<wuy p u.e u. in domains including retrieval agents(Jin et al., 2025),GUI |
| . | agents(Hong et al., 2024),and math agents(Yan et al., |
| Submitted to Inte rnational Confe rence on Machine Lea rning,2026. 2025). |
| 1 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

| Directly training an agent to fix s oftware i s sue s,however, | any tool s,mo st tool s are out-of-domain(OOD)for LLM s. |
| --- | --- |
| remains difficult.A single bug often admits multiple valid Even for the mo st powerful model s,failure s often happen |
| patche s,making string-level evaluation unreliable.The | when c alling the new-defined tool s due to wrong c alling |
| only preci se evaluation method require s executing c andi  format or failed parameter parsing.Thu s,training a LLM |
| date patche s inside a dedic ated Docker environment for | to master new-defined tool i s critic al for LLM agents.In  |
| each repo sitory(Luo et al., 2025),which i s prohibitively | tuitively,the tool-calling traj ectorie s can be generated by a |
| expensive.To make training more tractable,we adopt a | more powerful LLM,and such traj ectorie s c an be u sed to |
| simplified yet widely generalizable as signment:issue local  train a student model via supervi sed finetuning(SFT)(Chen |
| ization.Prior work show s that a s oftware i s sue become s | et al., 2025).However,thi s pipeline require s a stronger |
| sub stantially easier to re s olve once the relevant functions | teacher model which has capability to master the tool.Re  |
| and files are correctly identified(Chen et al.,2025;Ma et al., | cently,more methods have emerged with no teacher-model |
| 2025;Xia et al.,2024;Jiang et al., 2025).Since modern | required.Rej ected-s ampled finetuning(RFT)(Ahn et al., |
| OS S repo sitorie s contain a significant amount of code -far 2024)utilize s generated traj ectorie s of the agent itself via |
| beyond any LLM ' s context window -loc alization drasti  | multiple rollouts.Agentic RL(Jin et al., 2025)i s an on  |
| c ally reduce s the search space and improve s downstream policy RLVR methods requiring only the result for verifiying |
| s olvability.Crucially,loc alization outputs a di screte set | traj ectories.S uch training methods yield remarkable results |
| of paths,enabling verifi able,string-level evaluation that i s | when the tool s are search engine s(Jin et al., 2025),python |
| compatible with scalable training frameworks such as SFT | executer(Jimenez et al., 2023),calculator(Yan et al., 2025), |

| and RLVR. | and vi sual model s(Gupta&Kembhavi, 2023). |
| --- | --- |
| Exi sting loc alization agents(Ma et al.,2025;Chen | et al.,2025;He et al., 2025)typic ally rely on multiple 2.2.Software Engineering Agents |
| tool s,including S e a r c h C l a s s,S e a r c hM e t h o d s,and The introduction of SWE-bench(Jimenez et al.,2023;Yang |
| G e t I mp o r t s .Although effective to s ome extent,the se | et al.,2024b)has motivated a range of agentic pipeline s for |
| tool s considers high-level ab stractions(clas se s,function, | s oftware engineering(SWE)tasks.Among them,S WE  |
| etc)of programing language s,which do not reflect how | AGENT(Yang et al.,2024a)and O PENH AND S(Wang et al., |
| code actually execute s.High-level ab stractions,such as 2025 a)are widely adopted frameworks that equip agents |
| clas se s or inheritance,di s appear after compilation,leav  | with tool s for interacting with computing environments. |
| ing only sequential execution and j u mp operations.Since Workflow-b ased methods such as Agentle s s(Xia et al., |
| modern LLM s already excel at modeling sequential depen  2024)decompo se i s sue re s olution into localization,repair, |
| dencies,we focu s on enhancing their ability to j u mp acro s s | and validation subproblems.Chen et al.(2025)builds the re  |
| the repo sitory -that i s,to follow and inspect the source def | spo sitory as a graph and applied graph-level searching tool s |
| we introduce a single,structurally grounded tool:j u mp, grated commit hi story as agent memory.Repo Lens(Wang | inition of symbol s as they appear in execution.To thi s end, for localization and Wang et al(2025 a)furthermore inte  | , | . |
| which retrieve s the preci se definition of a given symbol. | et al.,2025b)equip conceptual information of the re spo s  |
| Detail s of thi s tool are provided in S ec.3.3. | itory to enable repo-level understanding.The se pipeline s |
| Our main contributions are threefold:(1)We propo se the | are training-free,compatible with clo sed-s ource language |
| first repo-level localization agent trained on reinforcement | model s,and yield competitive re sults. |
| learning directly from the pretrained model,regardle s s of To enable task-specific training,D EEP S WE(Luo et al., |
| di stillation from a clo se-s ource model.(2)We de sign a 2025)and S WE -S WI S S(He et al., 2025)employ reinforce  |
| repo sitory-navigation agent that operate s by performing | ment learning and achieve strong performance.However, |
| reali stic j u mp operations aligned with actual execution se  | end-to-end training remains co stly becau se patch evaluation |
| mantic s.(3)We demonstrate that one unified tool signifi  | require s executing Docker environments acro s s numerou s |
| cantly improve s efficiency and controllability compared to | repo sitorie s.Consequently,i s sue localization has emerged |

multi-tool pipeline s.

as a computationally efficient alternative,aiming to identify faulty components —at file or function level —rather than

### 2.Related Works

generating full patche s.

| 2.1.Agentic Training | Recent localization agents include L O CAGENT(Chen et al., |
| --- | --- |
| 2025)and C O S IL(Jiang et al., 2025),which model code  |
| LLM agents are promi sing methods to equip model s with b ase s as graphs and integrate s them into LLM s, and |
| complex tool s while reasoning(Li et al.,2024;Huang et al., O RC AL O C A(Yu et al 2025b)which enhance s efficiency | 2024;Guo et al.,2024b).However,becau se mo st pretrained | through priority scheduling,action decompo sition,and | ., | , |
| LLM s are trained on texts only and developers c an define | context pruning.From an open-s ource perspective,RE  |
| 2 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

*Figure 2.Overview of our Repo Navigator.During the rollout phrase,the agent can call the j u mp tool,and the language server will return*

the definition code of the symbol.Thi s proce s s i s trained by reinforcement learning.

P O S EARC HER(Ma et al.,2025),trained with di stillation **3.1.Problem Formulation**

| and Rt Lblon tdhe Qwen mtodel family(Team, 2024),represents Given a repo sitory R ={f1, . . .,fN}and an i s sue de  |
| --- |
| a no a e a vancemen . | scription q,the goal i s to output relevant code regions |
| within repo sitorie s -where module s,clas se s,and functions | Neverthele s s,prior agents overlook the structural relations Y ∗ ={(fi,gi j)},where gi j denote s a function or code | span in file fi.At each step t,the agent produces a optional | , | , |
| are cro s s-referenced acro s s files -and typically rely on mul  | reasoning step rt,a tool call at,and receives the ob servation |
| tiple search tool s for symbol definition retrieval,amplifying | ot,forming a traj ectory τ ={(rt,at,ot)}tT=1.After termi  |
| error propagation(see S ec.3).In contrast,we employ a sin  | nation,a final prediction Y i s scored by a reward R(Y,Y ∗) . | ˆ | ˆ |
| gle execution-logic-focu sed tool,reducing u s age complexity. The obj ective i s maxθ E τ ∼ π θ[R(τ)] . |
| Finally,our approach constitute s the first localization agent |
| trained directly from pretrained model s,without relying on 3.2.Agent Architecture |
| both RepoS earcher(Ma et al., 2025)and Loc Agent(Chen Repo Navigator u se s a single-tool de sign to avoid multi  | di stillation-b ased supervi sed finetuning,a crucial stage in |

et al.,2025).

tool orche stration overhead.At each step the policy *π**θ*

decide s whether to continue reas oning or to emit a JSON 

| 3.Method | formatted tool c all,while a symbol and its corre sponding |
| --- | --- |
| file are parsed to the tool.The agent receive s structured ob  |
| We present Repo Navigator,a reinforcement-learning agent | servations(code snippets or error mes s ages),then continues |
| for repo sitory-level i s sue loc alization.The method con  | reas oning until termination.The loop i s reason → act → |
| si sts of three components:(1)a unified tool to retrieve the | obse rve. |
| definition of any symbol s in a given file,(2)a reas oning - | action agent loop that alternate s between natural-language 3.3.Jump:Symbol Resolution |
| reas oning and tool invoc ation,and(3)a GRPO-b ased RL Language servers resolve the definition of a Python symbol |
| algorithm for optimizing long-horizon tool-augmented tra  through a determini stic static analy si s pipeline that approxi  |
| j ectories.B elow we provide the formal problem setting and | mate s Python ' s runtime name-binding semantic s.Given a |

the detailed method.

symbol occurrence *s* at source location *ℓ*,Pyright computes a re solution mapping

*R*(*s,ℓ*)*→{*(*f**i**,p**i*)*},*

(1)

3


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

where each pair(*f**i**,p**i*)denote s a file path and a s ource Reference Policy Optimization(GRPO),which has the lo s s po sition corre sponding to a valid definition site of *s*.In function: practice,we u se f i l e p a t h and s ymb o l to re s olve *ℓ*.If we have multiple symbol s with the s ame name exi st in the tsoaoml ewchoidcehsanlilpopwest,fworeaacdcduirtaitoenraellsyolpuatirosen aonf *ℓ*i n d e x to the

.

*L*GRPO(*θ*) =E(*s* *t* *a* *t*)*∼ π* *θ*  *π**θ*(*a*(*t**|s|**t*))*A*ˆ *t*

| , | , | old πθ old at s t |
| --- | --- | --- |
| Syntactic Analysis In thi s proce s s,the s ource file i s | − β DKL(πθ old(·|s t)∥ πθ(·|s t))](3) |
| parsed into an ab stract syntax tree(AST).The syntactic |
| role of s(e.g.,name,attribute acce s s,or c all expre s sion) | determine s the sub sequent re solution strategy.For attribute | where the first term i s the standard policy gradient obj ective | with an e stimated advantage function At,which promote s | ˆ |
| expre s sions a.b,Pyright treats a as a receiver expre s sion | actions that lead to higher-than-expected returns.The sec  |
| who se type mu st be inferred prior to member lookup. | ond term i s a Kullb ack-Leibler(KL)divergence penalty, |
| sc aled by a coefficient β,which acts as a tru st region,pre  |
| Lexical Scope Resolution For a name symbol x,c andi  | venting the updated policy πθ from moving too far from |
| date definitions are searched along a scope chain | the previou s policy πθ old .Thi s formulation ensure s stable |
| S ={local,enclo sing,module,builtins}, | and consi stent policy improvement by b alancing reward |
| (2) | maximization with behavioral consi stency. |
| following Python ' s LEGB rule.Each scope maintains a The reward of GRPO proce s s i s calculated as: |
| symbol table mapping identifiers to defining AST node s. | R(Y,Y ∗,τ) =DICE(Y,Y ∗)+S(τ) | ˆ | ˆ | (4) |
| pute s a po s s y un on-va ue type a or t e rece ver Yˆ and set Y ∗ |
| Static T(ype i Ibnlfereni ce .l Fodr)attrib Tut(e s)yfmbohl s,it ciom  Dice i s a common metric for set-level compari s on,for set |
| expres sion a u sing type annotations,as signment flow analy  |
| re solution i s then defined as |
| si s,function return type s,and stub file s( .p y i).Member | DICE , | (Yˆ Y ∗) 2 ×|Yˆ ∩ Y ∗| |
| = | |Y|+|Y ∗| | ˆ | () | 5 |
| resolve(a.b) =[lo okup(b,MRO(t)), | t ∈ T(a) | τ.We consider the tool-c all to be failed when the format | and S(τ)i s the succe s s rate of tool-c alling extracted from |
| where MRO(t)denotes the method resolution order of type i s incorrect,or the symbol parsed doe s not exi st,or for any |

| t. | other reason that cau se s the tool to quit unexpectedly. |
| --- | --- |
| Import Dependency Graph For cro s s-file resolution,im  4.Experiment |
| port dependency graph that static ally emulate s Python ' s | module loading semantic s i s built.Import statements intro  4.1.Experimnent Setup |
| duce binding s that map local symbol s to exported symbol s Datasets We extract valid s ample s from SWE-smith |
| of target module s,including re-exports and a l l -based (Yang et al.,2025b)to form the training set.We apply |
| filtering.Re s olution may therefore traverse multiple mod  Qwen2.5 -7B -Instruct with Repo Navigator to s ample each |
| ule s before reaching a concrete definition. | data for 1 6 time s.A s ample i s ab andoned if all 1 6 score s |
| 3.4.Reasoning -Action Loop | are zero.For validation,we test our method on SWE-bench  |
| verified(Jimenez et al., 2023),which i s a human-verified |
| Given hi story ht =(q,o 1:t − 1,a 1:t − 1),the agent s ample s | sub set of SWE-bench.We additionally te st our method on |
| either a natural-language reas oning step rt ∼ πθ(·|ht)or a | a sub set of SWE-bench-pro(Yang et al.,2025b)(which |
| structured tool c all at ∼ πθ(·|ht) .Tool c all s mu st s ati sfy i s a new and more difficult benchmark)for generalization. |
| a JSON grammar enforced via constrained decoding.The For ground-truth locations,we directly u se the locations in |
| loop continue s until the agent outputs its final localization golden patche s.All datasets are open-s ource and are built |

| Yˆ . | on real-world github i s sue s. |
| --- | --- |
| 3.5.Reinforcement Learning | M20e2t5r)ics l Pi rdeviou sllwordks(Chi ein et al.,20i25;Ma et al., |
| app e reca an prec s on as metr c s.However, |
| We apply reinforcement learning with verifi able rewards becau se the predicted locations and ground-truth locations |
| to train the agent directly from the pretrained model,with | are sets of string s,rec all and preci sion singularly c an not |
| no teacher model required.In practice,we apply Group | reflect the performance fairly.Thu s,we utilize S ample-F 1 |
| 4 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

*Table 1.Compari s on of different agent pipeline s on function-level and file-level Dice/IoU metric s.We u se Qwen2.5 -Instruct serie s as*

our b ase model.**B old numbers** denote the be st performance among s ame-size model s;underline numbers denote the be st training  free performance among s ame-size model s;yellow background illu strate s training-free RepoNavigator;blue background illu strate s RepoNavigator trained with GRPO.

| Agent Pipeline | Model | Function-level | File-level |
| --- | --- | --- | --- |
| Recall Preci sion S ample-F 1 IoU Recall Preci sion S ample-F 1 IoU |
| Close-so urce Models |
| RepoS earcher Claude3.7-S onnet 66.80 | 1 9.90 | 28.30 | 1 7.89 89.71 | 2 1.04 | 3 3.1 5 | 20.67 |
| Repo Navigator Claude3.7-S onnet 3 1.03 | 34.43 | 3 1.72 | 30.22 72.26 | 75.95 | 73.0 1 | 7 1.37 |
| Repo Navigator | GPT5 -chat | 30.42 | 34.56 | 3 1.1 7 | 29.67 5 8.1 7 6 1.87 | 5 8.8 8 | 57.3 3 |
| Repo Navigator Claude4.5 -S onnet 43.97 | 45.76 | 43.62 | 41.31 80.68 81.92 | 79.94 | 77.49 |
| Qwen2.5-7B |
| Locagent | Training Free | 1 7.62 | 1 1.7 1 | 1 2.7 1 | 1 0.3 1 60.96 | 34.8 8 | 40.67 | 3 3.3 3 |
| CoSIL | Training Free | 29.30 | 8.98 | 1 2.90 | 8.07 70.1 2 | 1 7.90 | 27.39 | 1 7.42 |
| Agentle s s | Training Free | 24.92 | 1 2.93 | 1 5.3 1 | 1 1.74 63.0 1 | 1 9.32 | 27.82 | 1 8.85 |
| Orcaloca | Training Free | 27.70 | 20.29 | 2 1.70 | 1 7.92 48.04 | 48.65 | 47.36 | 45.77 |
| RepoS earcher Di stillation+GRPO 63.26 | 1 9.24 | 27.37 | 1 7.59 84.1 1 | 1 9.97 | 3 1.64 | 1 9.57 |
| Repo Navigator | Training Free | 1 5.89 | 1 7.46 | 1 6.1 9 | 1 5.46 42.36 | 43.23 | 42.1 2 | 40.97 |
| Repo Navigator | GRPO | 26.69 30.34 | 27.49 | 26.43 50.62 53.83 | 51.63 | 50.62 |
| Qwen2.5-14B |
| Locagent | Training Free | 35.62 | 1 3.32 | 1 7.7 1 | 1 2.32 7 1.42 | 3 1.66 | 40.77 | 30.64 |
| CoSIL | Training Free | 48.61 | 1 3.40 | 1 9.8 1 | 1 2.1 2 78.35 | 1 8.1 0 | 28.79 | 1 7.72 |
| Agentle s s | Training Free | 25.20 | 1 4.30 | 1 6.1 4 | 1 2.28 75.65 | 1 9.76 | 29.8 8 | 1 9.30 |
| Orcaloca | Training Free | 29.92 | 20.98 | 22.77 | 1 8.92 52.1 7 52.1 5 | 50.93 | 48.72 |
| RepoS earcher | Training Free | 26.1 3 | 1 1.96 | 1 4.35 | 1 0.60 74.77 | 1 8.80 | 28.79 | 1 8.1 5 |
| Repo Navigator | Training Free | 27.96 | 25.77 | 25.5 8 | 23.00 59.00 56.68 | 56.39 | 5 3.74 |
| Repo Navigator | GRPO | 3 1.02 30.08 | 29.23 | 26.84 6 1.60 58.97 | 58.90 | 56.36 |
| Qwen2.5-32B |
| Locagent | Training Free | 46.79 | 1 6.29 | 2 1.48 | 1 4.1 8 79.39 | 34.1 8 | 44.1 8 | 3 3.24 |
| CoSIL | Training Free | 55.3 8 | 1 4.85 | 22.1 1 | 1 3.52 83.50 | 1 9.34 | 30.77 | 1 8.93 |
| Agentle s s | Training Free | 40.79 | 24.07 | 27.3 3 | 22.08 7 8.93 | 25.60 | 35.3 8 | 24.96 |
| Orcaloca | Training Free | 39.1 4 | 25.59 | 28.72 | 22.89 59.57 59.5 1 | 5 8.1 1 | 55.62 |
| RepoS earcher Di stillation+GRPO 69.50 | 20.29 | 29.1 1 | 1 8.23 89.33 | 20.27 | 32.93 | 20.35 |
| Repo Navigator | Training Free | 28.1 1 | 28.1 9 | 27.1 2 | 25.1 6 63.05 | 62.75 | 6 1.67 | 59.28 |
| Repo Navigator | GRPO | 3 3.7 1 | 37.19 | 34.09 | 32.30 67.29 | 70.76 | 67.75 | 65.75 |
| (which i s the averaged score of per-s ample F 1 value s)and | to 1 28 on 4k training s ample s filtered from SWE-smith, |
| IoU(intersection out of union)as our core metric s.At the | with maximum prompt length and max re sponse length |
| s ame time,we al s o pre sent the rec all and preci sion score s both set to 1 0240.Additionally,we rollout 8 time s for |
| to align with previou s methods,although they do not reflect | each s ample,and the temperature i s set to 1.0 to encourage |
| the methods ' performance fairly. | exploration.We u se greedy decoding in the inference stage |
| to ensure stable performance.More implementation detail s |
| Training For the 7B model,we conduct GRPO with 8 | are provided in Appendix.B. |
| Te sla-A 1 00-80G GPUs.For the 1 4B and 32B model,we |
| train it with 1 6 Te sla-A 1 00-80G GPUs.We apply verl 4.2.Effectiveness |
| ((SKhen,20t24l)a2s 0th2e3t)rainitnhg firafmework,andi we Wapplty viLLthM B aselines We compare our method against Loc agent |
| model for 1 epoch,while the training b atch size i s fixed (Chen et al., 2025),CoSIL(Jiang et al., 2025),Agent  | won e a ., | as e n erence eng ne. e ra n e |
| 5 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

*Table 2.Compari s on of different agent pipeline s on function-level and file-level metric s on SWE-bench Pro for generalization.B old*

**numbers** denote the be st performance among s ame-size model s;underline numbers denote the be st training-free performance among s ame-size model s;yellow background illu strates training-free RepoNavigator;blue background illu strates RepoNavigator trained with GRPO.

| Agent Pipeline | Model | Function-level | File-level |
| --- | --- | --- | --- |
| Recall Preci sion S ample-F 1 IoU Recall Preci sion S ample-F 1 IoU |
| Qwen2.5-7B |
| Loc Agent | Training Free 1.0 1 | 0.02 | 0.65 | 0.40 1 2.1 6 | 0.1 7 | 1 0.8 1 | 8.93 |
| CoSIL | Training Free 8.64 | 3.3 3 | 4.5 8 | 2.87 26.64 | 8.47 | 1 2.1 1 | 7.70 |
| Agentle s s | Training Free 12.82 | 6.94 | 8.05 | 5.73 39.41 | 1 3.1 5 | 1 8.89 | 1 2.35 |
| RepoS earcher Training Free 1.07 | 0.93 | 0.97 | 0.86 4.9 1 | 1.64 | 2.30 | 1.63 |
| Repo Navigator Training Free 9.84 | 1 4.65 | 1 0.67 | 9.20 30.50 37.24 | 3 1.86 | 28.82 |
| Repo Navigator | GRPO | 1 2.3 3 | 21.26 | 14.29 | 12.02 36.36 48.13 | 39.74 | 36.36 |
| Qwen2.5-14B |
| Loc Agent | Training Free 6.22 | 0.1 3 | 3.65 | 2.65 1 5.5 8 | 0.2 1 | 1 1.69 | 9.5 3 |
| CoSIL | Training Free 1 0.73 | 4.67 | 5.96 | 3.94 34.3 1 | 9.97 | 1 4.8 1 | 9.30 |
| Agentle s s | Training Free 1 0.49 | 6.75 | 7.4 1 | 5.28 4 1.42 | 1 3.42 | 1 9.02 | 1 2.37 |
| RepoS earcher Training Free 2.79 | 1.3 8 | 1.69 | 1.1 4 1 7.37 | 5.1 7 | 7.60 | 4.84 |
| Repo Navigator Training Free 1 4.36 | 1 9.74 | 1 5.27 | 1 2.00 43.57 54.52 | 46.06 | 4 1.07 |
| Repo Navigator | GRPO | 16.05 25.25 | 18.06 | 14.58 46.85 | 58.64 | 49.72 | 45.14 |
| Qwen2.5-32B |
| Loc Agent | Training Free 8.72 | 0.1 7 | 4.30 | 2.90 25.73 | 0.3 8 | 1 9.77 | 1 6.50 |
| CoSIL | Training Free 1 5.00 | 6.35 | 8.1 4 | 5.2 1 45.37 | 1 3.04 | 1 9.42 | 1 2.36 |
| Agentle s s | Training Free 1 1.08 | 7.3 1 | 7.98 | 5.80 43.07 | 1 3.89 | 20.07 | 1 3.1 1 |
| RepoS earcher Training Free 2.00 | 1.29 | 1.45 | 1.00 1 3.5 1 | 3.43 | 5.3 1 | 3.24 |
| Repo Navigator Training Free 1 3.96 | 20.25 | 1 5.36 | 1 2.87 50.24 63.24 | 5 3.48 | 48.50 |
| Repo Navigator | GRPO | 18.13 29.44 | 20.72 | 17.16 53.49 68.69 | 57.57 | 52.44 |
| baseline methods are pre sented in Appendix.A. |
| Results As illu strated in Table.1,on b alanced metric s |
| (S -F 1 and IoU)for both function-level and file-level local  |
| ization,our method surpas se s all b aseline methods with |
| the s ame model size.Moreover,if we train Repo Navigator |
| with GRPO,our 7B model surpas ses 1 4B baselines,and our |
| 1 4B model surpas se s 32B baseline s on S -F 1 and IoU.Thi s |
| contribute s to the validne s s of Repo Navigator furthermore. |
| Although s ome b aseline s have higher rec all score signifi  |
| cantly lower preci sion score than Repo Navigator,and result |
| in lower S -F 1 and IoU.Thi s indicate s that Repo Navigator |
| behave s more conservatively and generate s le s s wrong lo  |
| Figure 3.Ablation study:compari s on between Repo Navigator | c SaOti ToAns.For 1 4Bllandi 3i2B mf odel s,RhepdoNTavhiigaitor alichievhe s |
| with training free,RFT,GRPO with pure outcome and hybrid | reward on Qwen2.5 -7B -Instruct. | the tool we implement i s effective and promi sing,and our | single tool pipeline i s better than previou s multiple tool s | among a tra n ng-ree met o s. | s mp e s t at |
| pipeline s. |
| le s s(Xia et al., 2024),Orc aloc a(Yu et al.,2025b),and Compared with RepoS earcher,which i s di stilled from |
| RepoS earcher(Ma et al., 2025).Detailed explaination of | claude-3.7-s onnet(Anthropic, 2025)and reinforced by |
| 6 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

| Agent Pipeline | Func-IoU(%) Re solved(%) |
| --- | --- |
| Agentle s s | 5.28 | 1 0.1 2 |
| Loc Agent | 2.65 | 1 3.0 1 |
| Repo Navigator | 1 2.00 | 1 4.74 |
| Repo Navigator+RL | 1 4.5 8 | 1 5.03 |
| Table 3.We u se Qwen2.5 -1 4B -Instruct as the localization model, |
| and u se Qwen2.5 -32B -Instruct as the repair model on SWE  |
| bench Verified. |
| 4.4.Scaling Law of Tool-Calling |
| Figure 4.S c aling law of tool-c alling,where Pre and Post denote To as ses s the significance of tool-calling in Repo Navigator | the corre sponding metric before and after the RL training. | we varied the maximum number of tool-c alling turns and | , |
| reported the results in Fig.4.2.As shown in the figure,allow  |
| ing more tool-calling turns consi stently leads to improved |
| GRPO,trained Repo Navigator outperforms it on all metri  performance for Repo Navigator,both before and after re  |
| ces except recall.Moreover,we found that our training-free inforcement learning(RL)training.In other words,the se |
| method outperforms RepoS earcher for 1 4B model s.Thi s i s | re sults empiric ally validate the sc aling law of tool-c alling |
| probably due to the simplified tool we integrate to the agent in thi s context | (see S ec.5 for more detail s). | . |
| To as ses s the generalizability of Repo Navigator,we present 4.5.Influence on Issue Resolution |
| its performance on Python s ample s from the SWE-bench  To evaluate the impact of different loc alization re sults on |
| Pro dataset(Yang et al.,2025b)in Table 2.The re sults | on thi s dataset are consi stent with tho se ob served on SWE  tor against baseline s on SWE-bench Verified.We directly | the final i s sue resolution performance we test Repo Naviga | , |  |
| bench Verified.While we cannot fully exclude the potential | influence of data leakage in SWE-bench Verified,we c an loc alization front-end with other methods.Table.3 illu s  | apply the repairing phrase of Agentle s s while replacing its |
| released after the publication of the Qwen2.5 serie s. | make a stronger claim regarding SWE-bench Pro,as it was | has the highe st performance on i s sue re solution,while rein  | trate s the re sults.Compared with baseline s,Repo Navigator |
| 4.3.Training Strategy Comparison | forcement learning improve s its performance furthermore. |
| To explore the capability of GRPO on agentic training,we 5.Discussion:Building Less yet More Capable |
| compare GRPO against RFT-only and RFT+GRPO.As pre  | Tools |
| sented in Fig.3,directly training with GRPO outperforme s |
| RFT-only and RFT+GRPO.Moreover,although RFT has ac  In thi s section,we analyze the logic behind Repo Naviga  |
| cetable performance,the more step s RFT proceeds,the les s | tor:building le s s tool s with more powerful and more en  |
| improvement GRPO makes after the cold start.Thi s conclu  | sembled functions i s more effective than building multiple |
| sion contradicts with previou s SWE agents trained with RL | task-specific tool s. |
| (Ma et al., 2025),however,it aligns with the broader field of |
| reinforcement learning,where RFT and SFT(as a cold start) 5.1.Impact on the Action Space of Agents |
| i s effective only when the pretrained model i s not strong Let the total number of available tool s be denoted as k. |
| enoug | h(G | uohet ad.,d i a h.i h en tlie prdeitra nle mio i e s When only a single tool -specifically the j u mp tool -i s re  | l 2024)Wh h | i d d l i |
| mo e w t |
| strodngl eni ohu RgL ai nb ata hs g i-qiua tyf,Sre FcTt y(Rt FraT)n ngi a |
| s etter t an tra n ng a ter | as ts b h h i | tained,the sy stem ' s structural relations become simpler,as |

| cold start. | ot t e act on space an t e o servat on space are restr cte | d h b | i | i d |
| --- | --- | --- | --- | --- |
| to what thi s tool can acce s s.In thi s case,the set of po s sible |
| We al s o remove the succe s s rate in the reward function for | actions and ob servable elements i s smaller than when multi  |
| ablation.As presented in Fig.3,reinforcement learning with ple tool s are available.Thi s reduction i s generally beneficial, |
| hybrid reward(with tool-c alling succe s s rate)has higher | since additional tool s often introduce new and unfamiliar |
| performance than pure outcome reward(without tool-calling interfaces that large language model s have not been expo sed |
| succe s s rate).Thi s indic ate s that learning to correctly c all | to during pretraining,potentially increasing the likelihood |
| tool s i s vital in agentic learning. | of errors. |
| 7 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

### Jump GetClas s GetFunc GetStruc IoU

| ✓ | ✓ | ✓ | ✓ | 1 3.7 1 |
| --- | --- | --- | --- | --- |
| ✓ | ✓ | ✓ | ✗ | 2 1.44 |
| ✓ | ✗ | ✗ | ✓ | 24.00 |
| ✓ | ✗ | ✗ | ✗ | 24.28 |
| Table 4.We change the tool set of Repo Navigator and pre sent |
| the function-level IoU(%)on Qwen2.5 -7B -Instruct.Apparently, |
| exce s sive tool s do not boo st Repo Navigator ' s performance. |
| mantic ally activated by that entry point.B ec au se every |
| loc ation that contribute s to the i s sue mu st lie on s ome de  |
| Figure 5.Venn graph illu strating acces s scope of j u mp.Compared pendency path originating from the entry point,it i s nec  |
| with the repo sitory scope,the acces s scope has a much higher IoU | e s s arily reachable through thi s recursive symbol-reference |

| with the groundtruth set. | expansion.Therefore,the final acce s s scope produced by |
| --- | --- |
| exhau stive j u mp travers al i s guaranteed to contain all loca  |
| tions that mu st be modified to re solve the i s sue. |
| 5.2.Impact on Tool-Calling Success Rate |
| For a given proces s in i s sue localization(for instance,check  | 5.4.Verification |
| ing the code snippet of a function),let the succe s s probabil  To further verify thi s propo s al,we change the tool set of |
| ity of the i-th call be pi.For a task that requires k sequential Repo Navigator and conduct RL training with only the out  |
| tool invocations,the overall succe s s rate can be expre s sed | come reward.We add exces sive tool s which were frequently |

| as | u sed in previou s works(Chen et al.,2025;Ma et al.,2025; |
| --- | --- |
| Psucc(k) =Y pi . | k | Jiang et al., 2025)and pre sent the re sult in Table.4.Get  |
| (6) Class/Get Func take s a clas s/function name as input and |
| i=1 | outputs the clas s/function definition.Get Struc take s no in  |
| Since each step introduce s an additional potential point of put and outputs the repo sitory ' s structure.The results clearly |
| failure,the cumulative succe s s rate typic ally decrease s as implie s that additional tool s do not increase model ' s perfor  |
| the number of required tool c all s increase s.Therefore,in | mance.Thi s inspire s re searchers to develop less but more |
| general,completing a task with a single,more vers atile tool | capable tools. |
| tends to be more reliable than relying on multiple narrow  |
| scope tool s executed in sequence. | 6.Conclusion |
| 5.3.Impact on the Prediction Space | Il n thli si workl,weliintrioduced Repho Ndavigatorf,a repo siitoiry  |
| eve s sue oc a zat on agent t at eparts rom ex st ng |
| The acce s s scope of a tool i s defined as the complete set of | multi-tool paradigms by leveraging a single,more-capable |
| file s,symbol s,and other re s ource s that the tool can acce s s j u mp tool for symbol re solution.Thi s unified de sign faith  |
| within a repo sitory.For a j u mp tool that navigate s to sym  fully reflects real code execution flow while signific antly |
| bol definitions,its acce s s scope can be obtained by starting | reducing the complexity and brittlene s s of multi-step tool |
| from a given entry point and recursively re s olving all ref | chaining.Through tool-integrated GRPO,Repo Navigator |
| erenced symbol s until no new definitions c an be reached. learns to reason,invoke tool s,and refine its predictions in a |
| Apparently,its acces s scope i s significantly smaller than the | clo sed-loop manner,enabling end-to-end optimization with  |
| full repo sitory scope.Consequently,when computing the | out relying on clo sed-source teacher model s or di stillation. |
| Intdershection ovder Uhnion(IoiU)bhetwjeen the plredictlioni set Extensive experiments acro s s SWE-bench-Verified and |
| an t e groun trut set,u s ng t e u mp too re su ts n a SWE b h P d |
| higher IoU,as depicted in Fig.5.On the other hand,ap  | -fenhc -rol emli onsitrate t fat epo a Wv gathor ac i evlel s | h R N i | hi |
| plying multiple repo-level retrivel tool s results in the acces s | statel -o -the-art loca zatfion iper ohrmancie.l e t eorfetl ca yl |
| scope equal to the whole repo sitory scope. | jania yl ze t ie rei sudts,ichon irmf ng t at als ng ie power u toido , |
| o nt y opt m ze w t re n orcement earn ng,can prov e |
| When we start from the entry point and repeatedly apply | stronger robu stne s s and more reliable multi-step reas on  |
| j u mp -which retrieve s the definition of each referenced ing than previou s frameworks relying on multiple narrowly |
| symbol -we effectively traverse all symbol s that are se  | scoped tool s. |
| 8 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Our finding s highlight the importance of aligning agent tool  **References**

ing wiith real lexecutiioin structurel,aknd bshow ithlat eifficient Ahn,J.,Verma,R.,Lou,R.,Liu,D.,Zhang,R.,and Yin,W.

rfeason dnig-too i cod-tra n ng can un ocd slu sFtant a ga nks eveilnl

Large language model s for mathematical reasoning:Pro 

exp ore exten ng epo av gator rom yt on to more pro 

or lme um-sdize RopenN-s oui rce mfo e sP.huture wor w

gres ses and challenges.*a rXiv p rep rint a rXiv:2402.001 57*,

| gramming language s. | 2024. |
| --- | --- |
| Anthropic. | Claude 3.7 s onnet and claude code. |
| h t t p s://www .a n t h r o p i c .c o m/n e w s/ |
| c l a u d e -3 -7 -s o n n e t, February 2025. | data: |
| 2025 -1 1 -1 8. |
| Chen,Z.,Tang,R.,Deng,G.,Wu,F.,Wu,J.,Jiang,Z., |
| Pras anna,V.,Cohan,A.,and Wang,X.Loc Agent:Graph  |
| guided LLM agents for code loc alization.In Che,W., |
| Nabende,J.,Shutova,E.,and Pilehvar,M.T.(eds.),Pro  |
| ceedings of the 63 rd Ann ual Meeting of the Association |
| fo r Comp utational Ling uistics(Volume 1:Long Pape rs), |
| pp.8697 -8727,Vienna,Au stria,July 2025.As sociation |
| for Computational Lingui stic s.ISBN 979-8 -89 1 76-25 1 - |
| 0.doi:1 0.1 865 3/v 1/2025.acl-long.426.URL h t t p s: |
| //a c l a n t h o l o gy .o r g/2 0 2 5.a c l -l o n g .4 2 6/ . |
| Guo,D.,Zhu,Q.,Yang,D.,Xie,Z.,Dong,K., |
| Zhang,W.,Chen,G.,Bi,X.,Wu,Y.,Li,Y.,et al. |
| Deep seek-coder:When the large language model meets |
| programming -the ri se of code intelligence.a r Xiv p rep rint |
| a r Xiv:2401.141 96,2024a. |
| Guo,T.,Chen,X.,Wang,Y.,Chang,R.,Pei,S.,Chawla, |
| N.V.,Wie st,O.,and Zhang,X.Large language model |
| based multi-agents:A survey of progre s s and challenge s. |
| a r Xiv p rep rint a r Xiv:2402.01 680,2024b. |
| Gupta,T.and Kembhavi,A.Vi sual programming:Compo  |
| sitional vi sual reasoning without training.In Proceedings |
| of the IEEE/CVF confe rence on comp ute r vision and pat  |
| te rn recognition,pp.1 495 3 - 1 4962,2023. |
| He,Z.,Yang,Q.,Sheng,W.,Zhong,X.,Zhang,K.,An,C., |
| Shi,W.,Cai,T.,He,D.,Chen,J.,and Xu,J.S we-swi s s:A |
| multi-task fine-tuning and rl recipe for high-performance |
| i s sue re s olution.http s://github.com/zhenyuhe00/SWE  |
| S wi s s,2025.Notion Blog. |
| Hong,W.,Wang,W.,Lv,Q.,Xu,J.,Yu,W.,Ji,J.,Wang,Y., |
| Wang,Z.,Dong,Y.,Ding,M.,et al.Cogagent:A vi sual |
| language model for gui agents.In Proceedings of the |
| IEEE/CVF Confe rence on Comp ute r Vision and Patte rn |
| Recognition,pp.1 428 1 - 1 4290,2024. |
| Huang,X.,Liu,W.,Chen,X.,Wang,X.,Wang,H.,Lian, |
| D.,Wang,Y.,Tang,R.,and Chen,E.Understanding |
| the planning of llm agents:A survey. a r Xiv p rep rint |
| a r Xiv:2402.02 71 6,2024. |
| Hui,B.,Yang,J.,Cui,Z.,Yang,J.,Liu,D.,Zhang,L., |
| Liu,T.,Zhang,J.,Yu,B.,Lu,K.,et al.Qwen2.5 -coder |
| technical report.a r Xiv p rep rint a r Xiv:2409.1 21 86,2024. |

9


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Jiang,Z.,Ren,X.,Yan,M.,Jiang,W.,Li,Y.,and S chmidgall,S.,S u,Y.,Wang,Z.,S un,X.,Wu,J.,Yu,X., Liu,Z. Co sil:S oftware i s sue loc alization via llm 

Liu,J.,Moor,M.,Liu,Z.,and B ars oum,E.Agent lab 

driven code repo sitory graph searching.*a rXiv p rep rint*

oratory:Using llm agents as re search as si stants.*a rXiv*

| a r Xiv:2503.22424,2025. | p rep rint a r Xiv:2501.0422 7,2025. |
| --- | --- |
| Jimenez,C.E.,Yang,J.,Wettig,A.,Yao,S.,Pei,K.,Pre s s, Shen Z Llm with tool s:A survey a r Xiv p rep rint | el s re s olve real-world github i s sue s?a r Xiv p rep rint | O.,and Narasimhan,K.S we-bench:C an language mod  | a r Xiv:2409.1 8807,2024. | , . | . |
| a r Xiv:231 0.06770,2023. | Team,Q. Qwen2 technic al report. a r Xiv p rep rint |
| Jin,B.,Zeng,H.,Yue,Z.,Yoon,J.,Arik,S.,Wang,D., | a r Xiv:2407.1 0671,2024. |
| Zamani,Hd.l,and Han,J.hS earich-r 1:i Thraini infg llms to Wang,X.,Li,B.,S ong,Y.,Xu,F.F.,Tang,X.,Zhuge, |
| rl eas oin an everage searc eng2 n0e3s 0w9t 1 6re 2n02o5rcement |
| earn ng.a r Xiv p rep rint a r Xiv:5. 5 , | . |
| M.,Pan,J.,S ong,Y.,Li,B.,Singh,J.,Tran,H.H., |
| Li,F.,Ma,R.,Zheng,M.,Qian,B.,Shao,Y.,Muen  |
| Kwon,W.,Li,Z.,Zhuang,S.,Sheng,Y.,Zheng,L.,Yu, | nighoff,N.,Zhang,Y.,Hui,B.,Lin,J.,B rennan,R., |
| C.H.,Gonzalez,J.E.,Zhang,H.,and Stoica,I.Efficient | Peng,H.,Ji,H.,and Neubig,G. Openhands:An |
| memory management for large language model serving | open platform for AI s oftware developers as general  |
| with pagedattention.In Proceedings of the ACM SIGOPS | i st agents.In The Thirteenth Inte rnational Confe rence |
| 29th Symposium on Ope rating Systems Principles,2023. | on Lea rning Rep resentations,2025 a. URL h t t p s: |
| Langley,P.Crafting papers on machine learning.In Langley, | //o p e n r e v i e w .n e t/f o r u m?i d=O Jd 3 a y D D o F. |
| P.(ed.),Proceedings of the 1 7th Inte rnational Confe rence Wang,Y.,Mao,W.,Wang,C.,Zhou,Z.,Zhou,Y.,Zhao,W., |
| ford,CA,2000.Morgan Kaufmann. | on Machine Lea rning(ICML 2000),pp.1 207 - 1 2 1 6,Stan  | Lou,Y.,and Peng,X.Extracting conceptual knowledge to | locate software i s sue s.a r Xiv p rep rint a r Xiv:2509.2142 7, |
| Li,Y.,Wen,H.,Wang,W.,Li,X.,Yuan,Y.,Liu,G.,Liu, | 2025b. |
| J.,Xu,W.,Wang,X.,S un,Y.,et al.Pers onal llm agents:Xi C S D Y D S d Zh L A l D | I i h d | secur ty.a r v p rep r nt a r v: | ns g i ts an Xisurvey ai out Xt i e 2c4ap0a1 054ty5,9e 20c2e4ncy an | b h | bili ffi i | . | , | . | d | p rep rint a r Xiv:2407.01489,2024. | my stifying llm-based software engineering agents.a r Xiv | a, . ., eng, ., unn, .,an | ang, . gent es s:e  |
| Liu,A.,Feng,B.,Xue,B.,Wang,B.,Wu,B.,Lu,C.,Zhao, | C Deng C Zhang C Ruan C et al Deep seek-v3 Yan,Y.,Wang,S.,Huo,J.,Yu,P.S.,Hu,X.,and Wen,Q. |
| technical report.a r Xiv p rep rint a r Xiv:241 2.1 943 7,2024. | ., | , ., | , ., | , ., | . | Mathagent:Leveraging a mixture-of-math-agent frame  |
| Liu,Z.,Zhang,Y.,Li,P.,Liu,Y.,and Yang,D. Dy  | work for real-world multimodal mathematic al error de  |
| namic llm-agent network:An llm-agent collaboration | tection a r Xiv p rep rint a r Xiv:2503 1 81 32 2025 | . | . | , | . |
| framework with agent team optimization.a r Xiv p rep rint Yang,A.,Li,A.,Yang,B.,Zhang,B.,Hui,B.,Zheng,B., |

| a r Xiv:231 0.021 70,2023. | Yu,B.,Gao,C.,Huang,C.,Lv,C.,et al.Qwen3 technical |
| --- | --- |
| Lu,J.,Hollei s,T.,Zhang,Y.,Aumayer,B.,Nan,F.,B ai, | report.a r Xiv p rep rint a r Xiv:2505.09388,2025 a. |
| box:A stateful,convers ational,interactive evaluation | benchmark for llm tool u se c apabilitie s.a r Xiv p rep rint | F.,Ma,S.,Ma,S.,Li,M.,Yin,G.,et al.Tool s and  Y J Ji | ang, ., menez, . ., e g, ., ere, ., ao, ., | Narasimhan,K.R.,and Pre s s,O.SWE-agent:Agent  | computer interface s enable automated s oftware engi  | C E W tti A Li t K Y S |

| a r Xiv:2408.04682,2024. | neering.In The Thirty-eighth Ann ual Confe rence on |
| --- | --- |
| Luo,M.,Jain,N.,Singh,J.,Tan,S.,Patel,A.,Wu,Q., | Ne u ral Info rmation Processing Systems,2024a.URL |
| Ariyak,A.,C ai,C.,Tarun Venkat,S.Z.,Athiwaratkun, | h t t p s://a r x i v .o r g/ab s/2 4 0 5.1 5 7 9 3. |
| B.,Roongta,M.,Zhang,C.,Li,L.E.,Popa,R.A., | of-the-art coding agent from scratch by sc aling rl. | S en K and Stoic a I Deep swe:Training a state | h t t p s://p r e t t y -r a d i o -b 7 5.n o t i o n .s i t e/ | D e e p S WE -T r a i n i n g -a -F u l l y -Op e n -s o u r c e d -S t a tsey-stoemf -s tghenee-r Aalrizte-t Co ovdi siunagl -s o Aftgweanrte -dbomy -ai Sncs?a lairn Xgiv-RL -2 2 2 8 1 9 0 2 c 1 4 6 8 1 9 3 a abb e 9 a 8 c 5 9 bb e 3 3, | , ., | , . |  | Yang,J.,Jimenez,C.E.,Zhang,A.L.,Lieret,K.,Yang, | J.,Wu,X.,Pre s s,O.,Muennighoff,N.,Synnaeve,G., | Narasimhan,K.R.,et al.S we-bench multimodal:Do ai |

| 2025.Notion Blog. | p rep rint a r Xiv:241 0.03859,2024b. |
| --- | --- |
| Ma,Z.,Peng,C.,Zeng,Q.,Gao,P.,Zou,Y.,and Xie, Yang,J.,Lieret,K.,Jimenez,C.E.,Wettig,A.,Khandpur, |
| B.Tool-integrated reinforcement learning for repo deep | K.,Zhang,Y.,Hui,B.,Pre s s,O.,S chmidt,L.,and Yang, |
| search,2025. URL h t t p s://a r x i v .o r g/ab s/ | D.S we-smith:S c aling data for s oftware engineering |

2 5 0 8.0 3 0 1 2.

agents.*a rXiv p rep rint a rXiv:2504.21 798*,2025b.

1 0


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Yu,Q.,Zhang,Z.,Zhu,R.,Yuan,Y.,Zuo,X.,Yue,Y.,D ai, **A.Detailed Illustration of B aselines**

| W.,Fan,T.,Liu,G.,Liu,L.,et al.D apo:An open-source | llm reinforcement learning sy stem at scale.a r Xiv p rep rint Agentless Agentle s s(Xia et al., 2024)i s a workflow for |
| --- | --- |
| a r Xiv:2503.14476,2025 a. | i s sue localization.First,it identifie s su spiciou s file s in the |
| repo sitory.S econd,relevant clas se s and functions are de  |
| Yu,Z.,Zhang,H.,Zhao,Y.,Huang,H.,Yao,M.,Ding, tected.Third,preci se locations for edit are given by LLM s |
| K.,and Zhao,J.Orc aloc a:An llm agent framework based on the clas se s and functions. |
| for s oftware i s sue loc alization,2025b.URL h t t p s: |
| //a r x i v .o r g/ab s/2 5 0 2.0 0 3 5 0. | CoSIL CoSIL(Jiang et al., 2025)i s an agent which first |
| Yuan,S.,S ong,K.,Chen,J.,Tan,X.,Shen,Y.,Kan,R., l l l li ti C SIL d | conduct file-level loc alization and then conduct function  |
| Li,D.,and Yang,D.Easytool:Enhancing llm-b ased | efve docla z(a l on.f o ti y)ndamica tyh cons rulc s lca grahpi s | i ll | t t ll h |
| agents with conci se tool instruction. a r Xiv p rep rint | proce s s,an app e s con ex prun ng o e ec ve y re uce |
| o mo u es dc as sl,i unc otns t ur ngi et repfof-etvi e lsearcd ng |
| a r Xiv:2401 06201 2024 | . | , | . | the searching scope. |
| Yue,Y.,Yuan,Y.,Yu,Q.,Zuo,X.,Zhu,R.,Xu,W.,Chen, |
| J.,Wang,C.,Fan,T.,Du,Z.,et al.Vapo:Efficient and Loc Agent Loc Agent(Chen et al., 2025)i s almo st a fully  |
| reliable reinforcement learning for advanced reas oning | automatic LLM agent be side s its planning prompt concate  |
| tasks.a r Xiv p rep rint a r Xiv:2504.051 1 8,2025. | nated into the context at the beginning of the searching |
| proce s s.It builds the whole repo sitory into a direct hetero  |
| geneou s graph,who se nodes are files,clas ses,and functions. |
| Additionally,edge s are built by dependencie s such as im  |
| ports and invocations.Multiple graph-level searching tool s |
| are equipped to the LLM for multi-hop reasoning. |
| Repo Searcher RepoS earcher(Ma et al., 2025)i s an agent |
| that first conducts file-level localization and then function  |
| level localization,which aligns with CoSIL.RepoS earcher |
| introduced the first training framework Tool Train for lo  |
| c alization agents,which i s compo sed of di stilling from a |
| clo se-source model(Claude3.7-S onnet in RepoS eacher)as |
| warmup and reinforcement learning to further enhance the |
| performance. |
| Ours Compared with all baseline s,we are the first fully  |
| automatic LLM agent,with no fixed workflow and no plan  |
| etary prompt,and we are the first method trained directly |
| from pretrained open-source LLM s without a clo se-s ource |
| teacher model.Lastly,we only integrate a single yet power  |
| ful tool to the agent,which reduce s compounding error and |
| narrow s the acce s s scope of the agent. |
| B.Experimental Details |
| Hyperparameters We set clip ratio low to 0.2, |
| clip ratio high to 0.8,learning rate to 1 0 − 6, train  |
| ing batch size to 1 28,training temperature to 1.0,maximum |
| tool-calling times to 1 2,and max response length to 1 0240. |
| Metrics Given the set of predicted loc ations(ether file  | level or function-level)Y,and the set of groundtruth loca  | ˆ |
| tions Y ∗,the aforementioned metric s are calculated as: |
| Recall =|Y ∩ Y ∗| | ˆ | |Y ∗| | (7) |
| 1 1 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

| Jump Get Clas s Get Func Get Struc Recall Preci sion F 1 | IoU Recall Preci sion F 1 | IoU |
| --- | --- | --- |
| ✓ | ✓ | ✓ | ✓ | 1 4.28 | 1 5.44 | 1 4.40 1 3.7 1 35.7 8 | 36.76 35.59 34.55 |
| ✓ | ✓ | ✓ | ✗ | 22.60 | 25.02 22.80 2 1.44 48.49 | 50.1 3 48.52 47.1 7 |
| ✓ | ✗ | ✗ | ✓ | 24.64 | 27.48 25.05 24.00 5 3.48 | 55.76 5 3.68 52.69 |
| ✓ | ✗ | ✗ | ✗ | 25.1 1 | 29.16 25.75 24.28 55.81 | 58.71 56.32 54.89 |
| Table 5.We change the tool set of Repo Navigator and pre sent the function-level IoU.B ecau se the j u mp tool i s already powerful enough |
| for localization,exce s sive tool s do not increase its performance. |
| Preci sion=|Y|∩Yˆ Y| ∗| | ˆ | (8) | rmepanocseitoorfyt-healnadngduyangaemsiecrvimerp,oasrtistsc afunndcetigornaadleitythreelpieersfoonr  |
| S l F 1 2 ×|Y ∩ Y ∗| | ˆ | static analy si s technique s such as ab stract syntax tree s and |
| amp e- =|Yˆ|+|Y ∗| | (9) | symbol table s.When such circumstance s occur,the tool |
| returns an error mes s age indicating that the definition of the |
| IoU |Y ∩ Y ∗| | ˆ | current symbol cannot be located due to unknown reas ons. |
| = | ˆ | (1 0) Neverthele s s,in our empiric al evaluation,we did not ob  |
| |Y ∪ Y ∗| | serve any instances of monkey patching or dynamic imports |
| In practice,when the prediction set Y i s empty(for instance, | ˆ | within the analyzed datasets. |
| total failure),we set recall,preci sion,s ample-F 1,and IoU | to zero.We u se the function-level loc alization re sult of C.Threats to Validity |
| different methods and apply the patch generation backend Groundtruth Retrieval A limitation of our work lie s in |
| in Agentle s s(Xia et al., 2024)to generate patche s.Re  the extraction of groundtruth locations.We extract modified |
| s olved(%)denote s the percentage of s ample s that pas s all loc ations directly from the g o l d p a t c h in the datasets, |
| te st units after applying the patch. | which may ignore other patche s that al s o re s olve the i s sue. |
| Implementation When the re sponse exceeds the maxi  into consideration.However,u sing golden patche s i s ac  | Our evaluation metric s do not take these correct alternatives |
| tool-calling times(which i s 1 2),we add "You must not call | tool ' s response.Mo st of the time,the agent will stop calling | mum length,we clip and force the agent to stop,and we give | tools anymore,and you must give the final answer" to the | zero as its score.When the agent exceeds the maximum | reveal s golden loc ations(loc ations in golden patche s),it | undoubtedly contribute s to the re s olution of the i s sue and | the re sult in Table 3 demonstrate s thi s claim | ceptable when comparing mutliple methods If a method | . | . | . | , |
| tool s and generate the final re sponse.If not,we force it to |
| stop and give zero as its score.Note that when the maxi  Language Limit Another limitation i s that we only evalu  |
| which allow s the agent to explore in the environments with | mum tool-calling times i s not achieved and the final answer | i s generated,the agent loop will stop automatic ally.The | aforementioned proce s s i s an automatic agentic framework, | each language(C/C++,Java,etc.)has its unique language | ate Python repo sitorie s in our experiments.Thi s i s becau se | server of python.We will implement more language servers | server,and we only succeed in implementing the language |

| little constraints. | and validate our approach on more programing language s |
| --- | --- |
| in the future. |
| Preventing Data Leakage It i s a wide spread concern |
| va ty o po st-tra n ng met o s. evert e es s,we exc u e |
| thalitddi ata fleakage iati the preh-trdain Ning phhrasl e threatensl thde D.Case Study |
| thi s concern by re sults in Tabel.2.The SWE-bench Pro In thi s section,we pre sent the full traj ectory of Repo Navi  |
| dataset was publi shed in 2025,while the Qwen2.5 serie s gator on astropy astropy-1 2907 from SWE-bench Verified. |
| were publi shed in 2024.Moreover,we exclude the s ample s We apply the default tool-c alling prompt template of verl |
| bench Verified or SWE-bench Pro. | in the training dataset if the repo sitory al so appears in SWE  (Shen 2024)and pre sent an example Noted we do not | pre sent any proce s s re strictions in our prompt,encourag  | , | . | , |
| ing Repo Navigator to plan,c all tool s,and make deci sions |
| Language Server In practice,we apply a Python lan  full-automatic ally.Thi s i s di stinct with Agentle s s(which |
| guage server to extract the definition code corre sponding has a fixed workflow),Loc Agent(which predefine s a spe  |
| to an invoked symbol within a repo sitory.However,the | cific step-by-step workflow in its sy stem prompt),CoSIL |
| pre sence of monkey patche s -runtime modifications to the | and RepoS earcher(which i s half-automatic becau se s ome |
| 1 2 |


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

forced step s are added to the workflow besides the automatic multi-turns tool-calling convers ations).

1 3


---

**One Tool Is Enough:Reinforcement Learning for Repository-Level LLM Agents**

Prompt

[s y s t e m] Y o u a r e Q w e n, c r e a t e d b y A l i b ab a C l o u d . Y o u a r e a h e l p f u l a s s i s t a n t .

#T o o l s

Y o u m a y c a l l o n e o r m o r e f u n c t i o n s t o a s s i s t w i t h t h e u s e r qu e r y .

Y o u a r e p r o v i d e d w i t h f u n c t i o n s i g n a t u r e s w i t h i n <t o o l s></t o o l s>XML t a g s: <t o o l s> {"t yp e": "f u n c t i o n", "f u n c t i o n":{"n a m e": "c h e c k", "d e s c r i p t i o n": "I n t h e s p e c i f i c f i l e p a t h, a s ymb o l i s r e f e r r e d a n d t h i s t o o l c a n f i n d wh e r e t h e t o o l i s d e f i n e d .

| F o r i n s t a n c e, i n t h e f i r s t t u r n, f i l e p a t h i s t h e e n t r y p o i n t o f .", | "p a r a m e t e r s":{"p r o p e r t i e s":{"s ymb o l":{"d e s c r i p t i o n": "T h e s ymb o l wh o s e | _ |
| --- | --- | --- |
| d e f i n i t i o n c o d e w i l l b e g i v e n t o t h e a g e n t .", "t yp e": "s t r i n g"}, "f i l e p a t h": | {"d e s c r i p t i o n": "T h e r e l e v a n t p a t h t o t h e f i l e wh e r e t h e s ymb o l i s r e f e r r e d .", | _ |
| "t yp e": "s t r i n g"}}, "r e qu i r e d": ["s ymb o l", "f i l e_p a t h"], "t yp e": "o b j e c t"}}} |
| </t o o l s> |
| F o r e a c h f u n c t i o n c a l l, r e t u r n a j s o n o b j e c t w i t h f u n c t i o n n a m e a n d a r g u m e n t s |
| w i t h i n <t o o l _c a l l></t o o l _c a l l>XML t a g s: |
| <t o o l c a l l> | {"n a m e": <f u n c t i o n -n a m e>, "a r g u m e n t s": <a r g s -j s o n -o b j e c t>} | _ |
| </t o o l c a l l> | [u s e r] | _ |
| Y o u a r e g i v e n a c o d e b a s e a n d a n i s s u e, y o u n e e d t o l o c a t e t h e f i l e s a n d |
| f u n c t i o n s c a u s i n g t h i s i s s u e . |
| Y o u c a n c a l l t h e t o o l t o c h e c k t h e d e f i n i t i o n c o d e o f a s ymb o l . Y o u c a n o n l y |
| c h e c k t h e s ymb o l o n c e f o r e a c h t u r n . |
| N O T wh e r e i t i s d e f i n e d! | T h e ' f i l e p a t h ' i s t h e r e l e v a n t p a t h o f wh e r e t h e s ymb o l i s c a l l e d, | _ |
| F o r i n s t a n c e, i f ' c l a s s A .f u n c t i o n B ' i s wh a t y o u w a n t t o c h e c k(wh i c h i s c a l l e d |
| i n f i l e A .p y), y o u s h o u l d d i r e c t l y c h e c k ' f u n c t i o n B ' i n ' f i l e A .p y ' . |
| T h i s i s t h e i s s u e: |
| [P r o b l e m S t a t e m e n t] |
| T h e e n t r y f i l e o f t h e c o d e b a s e i s: |
| [R e l e v a n t P a t h T o E n t r y P o i n t] |
| [E n t r y P o i n t] |
| Y o u r f i n a l a n s w e r s h o u l d b e a l l f u n c t i o n s t h a t s h o u l d b e m o d i f i e d, s u c h a s: |
| r e l e v a n t/p a t h/t o/f i l e 1.p y::f u n c_n a m e 1,r e l e v a n t/p a t h/t o/f i l e 2.p y::f u n c_n a m e 2, |
| . . .(a s e r i e s o f f i l e::f u n c t i o n p a i r s s e p e r a t e d b y c o mm a) |
| P l e a s e p u t y o u r f i n a l a n s w e r i n s i d e\b o x e d{} o n l y i n t h e l a s t t u r n . |
| Y o u c a n o n l y c a l l t h e t o o l o n c e e a c h t u r n . |
| F o r i n s t a n c e: |
| {' n a m e ':' c h e c k ', ' a r g u m e n t s ':{' s ymb o l ':' s ymb o l _t o_b e_c h e c k e d ', ' f i l e_p a t h ': |
| ' f i l e_wh e r e_t h e_s ymb o l _i s _u s e d '}} |
| 1 4 |