# One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

## OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents

**ZhaoxiZhang** 1 **YitongDuan**2 **YanzhiZhang**2 **YimingXu** 1 **JiyanHe**2 **YunfangWu** 1

### Abstract

Locatingthefiles andfunctions requiring modifi  cationinlarge open-source software(OSS)repos 

| 5 | itoriesis challengingdue to their scale and struc  |
| --- | --- |
| 2 | 0 | tural complexity.Existinglargelanguage model |
| 2 | (LLM)-based methods typically treat this as a |
| repository-level retrievaltask and rely on multiple | Figure 1.Illustration of aLLM navigatingthrough a code reposi  |
| D | e | c | l i d | Rog c Nan ciomp ca e LmLo Me con ro .i eprdopoishe | a iliar tools hich o erlook code e ec tion | ux y | epo avgator,an | ,w | li t d l t l W | v | agent equppe w t | x u | whichis realizedthrough alanguage server. | tory.TheLLMis equipped with a singleyetpowerfultool:jump, |
| 2 | 5 | a single execution-awaretool-jumping to the |
| ] | S | E | s. | while simplifyingtool manipulation.Repo Navi  | Learning(RL)directlyfrom apretrained model, | definition of ainvoked symbol.This unifiedde  | gatoristrained end-to-end via Reinforcement | sign reflects the actual flow of code execution | ries remainslimited.SWE -BENCH(Jimenez et al., 2023) | itory directly due to contextlimits.While SWE -AGENT | evaluating whetherLLMs can resolve real-world Git Hubis  | currently serves as the most comprehensivebenchmarkfor | sues.AllpretrainedLLMs can notprocess the whole repos  |
| [ | c | without any closed-source distillation.Experi  | (Jimenez et al., 2023)provides moderategains,it remains |
| 2 | tor achieves state-of-the-artperformance,withthe | mentsdemonstratethatRL-trained Repo Naviga  | farfrom enabling robust repository-level reasoning. |
| 95 | 7 | v | 7B model outperforming 14Bbaselines,the 14B | model surpassing32B competitors,and eventhe | 32B model exceeding closed-source models such | rectlytopretrainedLLMs(Liu et al.,2023;Chenet al.,2025; | Most existing agents rely on test-time scaling applied di  | Schmidgall et al., 2025).In software engineering(SWE) |
| 0 | as Claude-3.7.These results confirmthatintegrat  | tasks,tool usageis essential ratherthan optional:real-world |
| 2 | ing a single,structurally grounded tool with | repositories arefarlargerthanthe contextwindow ofcurrent |
| 2. | RL training provides an efficient and scalable | LLMs,makingitimpossibletoprocess an entire codebase |
| 1 | solutionfor repository-levelissuelocalization. | in a singleforwardpass.Agents mustthereforeiteratively |

5

invoketools to retrievepartialinformationfromthe repos 

2

itory andinterleave natural-language reasoning with tool

| : | v 1.Introduction | calls. |
| --- | --- | --- |
| i | X | With the rapid advancement of Large Language Models However,mainstream LLMs are rarely exposed to such |
| ra (LLMs)(Liu et al.,2024;Team,2024;Yang et al.,2025a), | agenticinteractionpatternsduringpretraining andtypically |
| equippingLLMs withpre-builttools toformLLM agents | acquiretool usage onlythroughfew-shotprompting.Such |
| has become a common paradigm for expanding their ca  in-contextdemonstrations areinsufficientforlearning com  |
| pabilities(Shen,2024;Yuan et al.,2024;Lu et al., 2024). plex multi-step tool-chaining behaviors,especially under |
| In the domain of software engineering(SWE),although limited context windows.Moreover,becausetooldefinition |
| LLM agents can effectively handle simple programming | spaces are effectively unbounded,pretrained models cannot |
| tasks(Hui et al.,2024;Guo et al.,2024a),their ability to fullyinternalize their semantics withoutpost-training.To |
| operate onlarge-scale open-source software(OSS)reposito  | mitigatetheseissues,post-trainingparadigms such as Super |
| 1School of Computer Science, Peking University Learning with Verifiable Rewards(RLVR)(Yu et al.,2025a; | vised Finetuning(SFT)(Ma et al., 2025)and Reinforcement |
| 2Zhonggduancuin A@cademi y. Correfspondence tof:@Ykitodng Yue et al., 2025)havebeen applied,withpromising results |
| Dcnu>an<uany tong zgc .ac.cn>,Yun ang Wu<wuy p u.e u. indomainsincluding retrieval agents(Jin et al., 2025),GUI |
| . | agents(Hong et al., 2024),and math agents(Yan et al., |
| Submittedto International Conference on Machine Learning,2026. 2025). |
| 1 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

| Directly training an agenttofix softwareissues,however, | anytools,mosttools are out-of-domain(OOD)forLLMs. |
| --- | --- |
| remainsdifficult.A singlebug often admits multiple valid Evenfor the mostpowerful models,failures oftenhappen |
| patches,making string-level evaluation unreliable.The | when calling the new-defined tools due to wrong calling |
| only precise evaluation method requires executing candi  format orfailedparameterparsing.Thus,training aLLM |
| date patches inside a dedicated Docker environment for | to master new-defined toolis criticalforLLM agents.In  |
| each repository(Luo et al., 2025),which is prohibitively | tuitively,thetool-callingtrajectories can begeneratedby a |
| expensive.To make training more tractable,we adopt a | morepowerfulLLM,and suchtrajectories can be usedto |
| simplifiedyet widelygeneralizable assignment:issuelocal  train a student model via supervisedfinetuning(SFT)(Chen |
| ization.Prior work shows that a softwareissuebecomes | et al., 2025).However,this pipeline requires a stronger |
| substantially easier to resolve once the relevantfunctions | teacher model whichhas capabilityto masterthetool.Re  |
| andfiles are correctlyidentified(Chen et al.,2025;Ma et al., | cently,more methodshave emerged with noteacher-model |
| 2025;Xia et al.,2024;Jiang et al., 2025).Since modern | required.Rejected-sampledfinetuning(RFT)(Ahn et al., |
| OSS repositories contain a significant amount ofcode-far 2024)utilizesgeneratedtrajectories of the agentitselfvia |
| beyond any LLM's context window -localization drasti  | multiple rollouts.Agentic RL(Jin et al., 2025)is an on  |
| cally reduces the search space andimproves downstream policyRLVRmethods requiring onlytheresultforverifiying |
| solvability.Crucially,localization outputs a discrete set | trajectories.Suchtraining methodsyield remarkable results |
| ofpaths,enabling verifiable,string-level evaluationthatis | whenthetools are search engines(Jin et al., 2025),python |
| compatible with scalabletrainingframeworks such asSFT | executer(Jimenez et al., 2023),calculator(Yan et al., 2025), |

| andRLVR. | and visual models(Gupta&Kembhavi, 2023). |
| --- | --- |
| Existing localization agents(Ma et al.,2025;Chen | et al.,2025;He et al., 2025)typically rely on multiple 2.2.Software Engineering Agents |
| tools,including Search Class,Search Methods,and Theintroduction ofSWE-bench(Jimenez et al.,2023;Yang |
| Get Imports .Although effective to some extent,these | et al.,2024b)has motivated a range of agenticpipelinesfor |
| tools considers high-level abstractions(classes,function, | software engineering(SWE)tasks.Among them,SWE  |
| etc)ofprograming languages,which do not reflect how | AGENT(Yang et al.,2024a)andOPENHANDS(Wang et al., |
| code actually executes.High-level abstractions,such as 2025a)are widely adoptedframeworks that equip agents |
| classes or inheritance,disappear after compilation,leav  | with tools for interacting with computing environments. |
| ing only sequential execution and jump operations.Since Workflow-based methods such as Agentless(Xia et al., |
| modernLLMs already excel at modeling sequentialdepen  2024)decomposeissue resolutionintolocalization,repair, |
| dencies,wefocus on enhancingtheir abilityto jump across | and validation subproblems.Chen et al.(2025)buildsthe re  |
| the repository -thatis,tofollow andinspectthe sourcedef | spository as agraph and appliedgraph-level searchingtools |
| we introduce a single,structurally grounded tool:jump, grated commithistory as agent memory.Repo Lens(Wang | inition of symbols as they appearin execution.Tothis end, forlocalization and Wang et al(2025a)furthermoreinte  | , | . |
| which retrieves the precise definition of a given symbol. | et al.,2025b)equip conceptualinformation of the respos  |
| Details ofthis tool areprovidedin Sec.3.3. | itory to enable repo-level understanding.Thesepipelines |
| Our main contributions are threefold:(1)Wepropose the | aretraining-free,compatible with closed-sourcelanguage |
| first repo-levellocalization agenttrained on reinforcement | models,andyield competitive results. |
| learningdirectlyfromthepretrained model,regardless of To enable task-specific training,DEEPSWE(Luo et al., |
| distillation from a close-source model.(2)We design a 2025)andSWE -SWISS(He et al., 2025)employ reinforce  |
| repository-navigation agent that operates by performing | mentlearning and achieve strongperformance.However, |
| realistic jump operations aligned with actual execution se  | end-to-endtraining remains costlybecausepatch evaluation |
| mantics.(3)Wedemonstrate that one unified tool signifi  | requires executing Docker environments across numerous |
| cantlyimproves efficiency and controllability comparedto | repositories.Consequently,issuelocalizationhas emerged |

multi-toolpipelines.

as a computationally efficient alternative,aimingtoidentify faulty components —atfile orfunctionlevel—rather than

### 2.RelatedWorks

generatingfullpatches.

| 2.1.Agentic Training | Recentlocalization agentsincludeLOCAGENT(Chen et al., |
| --- | --- |
| 2025)andCOSIL(Jiang et al., 2025),which model code  |
| LLM agents arepromising methods to equip models with bases as graphs and integrates them into LLMs, and |
| complextools while reasoning(Li et al.,2024;Huang et al., ORCALOCA(Yu et al 2025b)which enhances efficiency | 2024;Guo et al.,2024b).However,because mostpretrained | through priority scheduling,action decomposition,and | ., | , |
| LLMs are trained ontexts only anddevelopers candefine | context pruning.From an open-source perspective,RE  |
| 2 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

*Figure2.Overview ofour Repo Navigator.Duringthe rolloutphrase,the agent can callthe jump tool,andthelanguage server will return*

thedefinition code ofthe symbol.Thisprocessis trainedby reinforcementlearning.

POSEARCHER(Ma et al.,2025),trained with distillation **3.1.ProblemFormulation**

| and Rt Lblontdhe Qwen mtodelfamily(Team, 2024),represents Given a repository R ={f1, . . .,fN}and an issue de  |
| --- |
| a no a e a vancemen . | scription q,the goal is to output relevant code regions |
| within repositories -where modules,classes,andfunctions | Nevertheless,prior agents overlookthe structural relations Y∗ ={(fi,gij)},where gij denotes a function or code | spaninfilefi.At each stept,the agentproduces a optional | , | , |
| are cross-referenced acrossfiles -andtypically rely on mul  | reasoning step rt,atool call at,and receivesthe observation |
| tiple searchtoolsfor symboldefinition retrieval,amplifying | ot,forming atrajectory τ ={(rt,at,ot)}tT=1.Aftertermi  |
| errorpropagation(see Sec.3).In contrast,we employ a sin  | nation,afinalprediction Yis scoredby a rewardR(Y,Y∗) . | ˆ | ˆ |
| gle execution-logic-focusedtool,reducing usage complexity. The objectiveis maxθEτ ∼πθ[R(τ)] . |
| Finally,our approach constitutes thefirstlocalization agent |
| traineddirectlyfrompretrained models,without relying on 3.2.Agent Architecture |
| both Repo Searcher(Ma et al., 2025)and Loc Agent(Chen Repo Navigator uses a single-tool design to avoid multi  | distillation-based supervisedfinetuning,a crucial stagein |

et al.,2025).

tool orchestration overhead.At each step the policy *π**θ*

decides whetherto continue reasoning orto emit aJSON 

| 3.Method | formatted tool call,while a symbol andits corresponding |
| --- | --- |
| file areparsedto thetool.The agent receives structured ob  |
| Wepresent Repo Navigator,a reinforcement-learning agent | servations(code snippets or error messages),then continues |
| for repository-level issue localization.The method con  | reasoning until termination.Theloopis reason → act → |
| sists ofthree components:(1)a unifiedtoolto retrievethe | observe. |
| definition of any symbols in agivenfile,(2)a reasoning - | action agentloopthat alternatesbetween natural-language 3.3.Jump:Symbol Resolution |
| reasoning and toolinvocation,and(3)aGRPO-basedRL Language servers resolvethedefinition of a Python symbol |
| algorithmfor optimizinglong-horizontool-augmentedtra  through adeterministic static analysispipelinethat approxi  |
| jectories.Below weprovidetheformalproblem setting and | mates Python's runtime name-binding semantics.Given a |

thedetailed method.

symbol occurrence *s* at sourcelocation*ℓ*,Pyright computes a resolution mapping

*R*(*s,ℓ*)*→{*(*f**i**,p**i*)*},*

(1)

3


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

where each pair(*f**i**,p**i*)denotes a file path and a source ReferencePolicyOptimization(GRPO),whichhas theloss position corresponding to a valid definition site of *s*.In function: practice,we use file path and symbol to resolve*ℓ*.If wehave multiple symbols withthe same name existinthe tsoaomlewchoidcehsanlilpopwest,fworeaacdcduirtaitoenraellsyolpuatirosen aonf*ℓ*index tothe

.

*L*GRPO(*θ*) =E(*s**t* *a**t*)*∼π**θ*  *π**θ*(*a*(*t**|s|**t*))*A*ˆ*t*

| , | , | old πθold at st |
| --- | --- | --- |
| Syntactic Analysis In this process,the source file is | − βDKL(πθold(·|st)∥πθ(·|st))](3) |
| parsed into an abstract syntax tree(AST).The syntactic |
| role of s(e.g.,name,attribute access,or call expression) | determines the subsequent resolution strategy.For attribute | wherethefirsttermis the standardpolicygradient objective | with an estimated advantagefunction At,whichpromotes | ˆ |
| expressions a.b,Pyright treats a as a receiver expression | actions thatlead tohigher-than-expected returns.The sec  |
| whosetype mustbeinferredpriorto memberlookup. | ond term is a Kullback-Leibler(KL)divergence penalty, |
| scaledby a coefficientβ,which acts as atrust region,pre  |
| Lexical Scope Resolution For a name symbol x,candi  | venting the updated policy πθ from moving too far from |
| datedefinitions are searched along a scope chain | theprevious policy πθold .This formulation ensures stable |
| S ={local,enclosing,module,builtins}, | and consistent policy improvement by balancing reward |
| (2) | maximization withbehavioral consistency. |
| following Python's LEGB rule.Each scope maintains a The reward ofGRPOprocessis calculated as: |
| symboltable mappingidentifiers todefiningAST nodes. | R(Y,Y∗,τ) =DICE(Y,Y∗)+S(τ) | ˆ | ˆ | (4) |
| putes a poss y un on-va ue type a ort e rece ver Yˆ and setY∗ |
| Static T(ypei Ibnlferenice .l Fodr)attrib Tut(e s)yfmbohls,it ciom  Diceis a common metricfor set-level comparison,for set |
| expression a usingtype annotations,assignmentflow analy  |
| resolutionis thendefined as |
| sis,function return types,and stubfiles( .pyi).Member | DICE , | (Yˆ Y∗) 2 ×|Yˆ ∩Y∗| |
| = | |Y|+|Y∗| | ˆ | () | 5 |
| resolve(a.b) =[lookup(b,MRO(t)), | t∈T(a) | τ.We consider the tool-call tobefailed when theformat | andS(τ)is the success rate oftool-calling extractedfrom |
| whereMRO(t)denotes the method resolution order oftype isincorrect,orthe symbolparseddoes not exist,orfor any |

| t. | other reasonthat causes thetooltoquit unexpectedly. |
| --- | --- |
| Import Dependency Graph For cross-file resolution,im  4.Experiment |
| port dependency graph that statically emulates Python's | moduleloading semanticsisbuilt.Import statementsintro  4.1.Experimnent Setup |
| ducebindings that maplocal symbols to exported symbols Datasets We extract valid samples from SWE-smith |
| oftarget modules,including re-exports and all -based (Yang et al.,2025b)to form the training set.We apply |
| filtering.Resolution maythereforetraverse multiple mod  Qwen2.5 -7B -Instruct with Repo Navigatorto sample each |
| ulesbefore reaching a concretedefinition. | datafor 16 times.A sampleis abandonedif all 16 scores |
| 3.4.Reasoning-Action Loop | are zero.For validation,wetest our method onSWE-bench  |
| verified(Jimenez et al., 2023),whichis ahuman-verified |
| Givenhistory ht =(q,o1:t −1,a1:t −1),the agent samples | subset ofSWE-bench.We additionallytest our method on |
| either a natural-language reasoning step rt ∼ πθ(·|ht)or a | a subset of SWE-bench-pro(Yang et al.,2025b)(which |
| structured tool call at ∼ πθ(·|ht) .Tool calls must satisfy is a new and moredifficultbenchmark)forgeneralization. |
| aJSONgrammar enforced via constraineddecoding.The Forground-truthlocations,wedirectly usethelocationsin |
| loop continues untilthe agent outputsitsfinallocalization goldenpatches.Alldatasets are open-source and arebuilt |

| Yˆ . | on real-worldgithubissues. |
| --- | --- |
| 3.5.Reinforcement Learning | M20e2t5r)ics l Pirdeviousllwordks(Chiein et al.,20i25;Ma et al., |
| app e reca an prec s on as metr cs.However, |
| We apply reinforcement learning with verifiable rewards becausethepredictedlocations andground-truthlocations |
| to train the agentdirectlyfromthepretrained model,with | are sets of strings,recall andprecision singularly can not |
| no teacher model required.In practice,we apply Group | reflecttheperformancefairly.Thus,we utilize Sample-F1 |
| 4 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

*Table 1.Comparison ofdifferent agentpipelines onfunction-level andfile-level Dice/IoU metrics.We use Qwen2.5 -Instruct series as*

ourbase model.**Bold numbers** denote thebestperformance among same-size models;underline numbers denote thebest training  freeperformance among same-size models;yellowbackground illustrates training-freeRepoNavigator;bluebackground illustrates RepoNavigatortrained withGRPO.

| Agent Pipeline | Model | Function-level | File-level |
| --- | --- | --- | --- |
| Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU |
| Close-source Models |
| Repo Searcher Claude3.7-Sonnet 66.80 | 19.90 | 28.30 | 17.89 89.71 | 21.04 | 33.15 | 20.67 |
| Repo Navigator Claude3.7-Sonnet 31.03 | 34.43 | 31.72 | 30.22 72.26 | 75.95 | 73.01 | 71.37 |
| Repo Navigator | GPT5 -chat | 30.42 | 34.56 | 31.17 | 29.67 58.17 61.87 | 58.88 | 57.33 |
| Repo Navigator Claude4.5 -Sonnet 43.97 | 45.76 | 43.62 | 41.31 80.68 81.92 | 79.94 | 77.49 |
| Qwen2.5-7B |
| Locagent | Training Free | 17.62 | 11.71 | 12.71 | 10.31 60.96 | 34.88 | 40.67 | 33.33 |
| CoSIL | Training Free | 29.30 | 8.98 | 12.90 | 8.07 70.12 | 17.90 | 27.39 | 17.42 |
| Agentless | Training Free | 24.92 | 12.93 | 15.31 | 11.74 63.01 | 19.32 | 27.82 | 18.85 |
| Orcaloca | Training Free | 27.70 | 20.29 | 21.70 | 17.92 48.04 | 48.65 | 47.36 | 45.77 |
| Repo Searcher Distillation+GRPO 63.26 | 19.24 | 27.37 | 17.59 84.11 | 19.97 | 31.64 | 19.57 |
| Repo Navigator | Training Free | 15.89 | 17.46 | 16.19 | 15.46 42.36 | 43.23 | 42.12 | 40.97 |
| Repo Navigator | GRPO | 26.69 30.34 | 27.49 | 26.43 50.62 53.83 | 51.63 | 50.62 |
| Qwen2.5-14B |
| Locagent | Training Free | 35.62 | 13.32 | 17.71 | 12.32 71.42 | 31.66 | 40.77 | 30.64 |
| CoSIL | Training Free | 48.61 | 13.40 | 19.81 | 12.12 78.35 | 18.10 | 28.79 | 17.72 |
| Agentless | Training Free | 25.20 | 14.30 | 16.14 | 12.28 75.65 | 19.76 | 29.88 | 19.30 |
| Orcaloca | Training Free | 29.92 | 20.98 | 22.77 | 18.92 52.17 52.15 | 50.93 | 48.72 |
| Repo Searcher | Training Free | 26.13 | 11.96 | 14.35 | 10.60 74.77 | 18.80 | 28.79 | 18.15 |
| Repo Navigator | Training Free | 27.96 | 25.77 | 25.58 | 23.00 59.00 56.68 | 56.39 | 53.74 |
| Repo Navigator | GRPO | 31.02 30.08 | 29.23 | 26.84 61.60 58.97 | 58.90 | 56.36 |
| Qwen2.5-32B |
| Locagent | Training Free | 46.79 | 16.29 | 21.48 | 14.18 79.39 | 34.18 | 44.18 | 33.24 |
| CoSIL | Training Free | 55.38 | 14.85 | 22.11 | 13.52 83.50 | 19.34 | 30.77 | 18.93 |
| Agentless | Training Free | 40.79 | 24.07 | 27.33 | 22.08 78.93 | 25.60 | 35.38 | 24.96 |
| Orcaloca | Training Free | 39.14 | 25.59 | 28.72 | 22.89 59.57 59.51 | 58.11 | 55.62 |
| Repo Searcher Distillation+GRPO 69.50 | 20.29 | 29.11 | 18.23 89.33 | 20.27 | 32.93 | 20.35 |
| Repo Navigator | Training Free | 28.11 | 28.19 | 27.12 | 25.16 63.05 | 62.75 | 61.67 | 59.28 |
| Repo Navigator | GRPO | 33.71 | 37.19 | 34.09 | 32.30 67.29 | 70.76 | 67.75 | 65.75 |
| (whichis the averaged score ofper-sampleF1 values)and | to 128 on 4k training samples filtered from SWE-smith, |
| IoU(intersection out ofunion)as our core metrics.Atthe | with maximum prompt length and max response length |
| sametime,we alsopresentthe recall andprecision scores both set to 10240.Additionally,we rollout 8 times for |
| to align withprevious methods,althoughtheydo not reflect | each sample,andthetemperatureis setto 1.0to encourage |
| the methods' performancefairly. | exploration.We usegreedydecodingin theinference stage |
| to ensure stableperformance.Moreimplementationdetails |
| Training For the 7B model,we conduct GRPO with 8 | areprovidedin Appendix.B. |
| Tesla-A100-80GGPUs.For the 14B and32B model,we |
| train it with 16 Tesla-A100-80G GPUs.We apply verl 4.2.Effectiveness |
| ((SKhen,20t24l)a2s0th2e3t)rainitnhgfirafmework,andi we Wapplty viLLthM Baselines We compare our method against Locagent |
| model for 1 epoch,while the training batch size is fixed (Chen et al., 2025),CoSIL(Jiang et al., 2025),Agent  | won e a ., | as e n erence eng ne. e ra n e |
| 5 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

*Table2.Comparison ofdifferent agentpipelines onfunction-level andfile-level metrics onSWE-bench Proforgeneralization.Bold*

**numbers**denotethebestperformance among same-size models;underline numbersdenotethebesttraining-freeperformance among same-size models;yellowbackground illustratestraining-freeRepoNavigator;bluebackground illustratesRepoNavigatortrained with GRPO.

| Agent Pipeline | Model | Function-level | File-level |
| --- | --- | --- | --- |
| Recall Precision Sample-F1 IoU Recall Precision Sample-F1 IoU |
| Qwen2.5-7B |
| Loc Agent | Training Free 1.01 | 0.02 | 0.65 | 0.40 12.16 | 0.17 | 10.81 | 8.93 |
| CoSIL | Training Free 8.64 | 3.33 | 4.58 | 2.87 26.64 | 8.47 | 12.11 | 7.70 |
| Agentless | Training Free 12.82 | 6.94 | 8.05 | 5.73 39.41 | 13.15 | 18.89 | 12.35 |
| Repo Searcher Training Free 1.07 | 0.93 | 0.97 | 0.86 4.91 | 1.64 | 2.30 | 1.63 |
| Repo Navigator Training Free 9.84 | 14.65 | 10.67 | 9.20 30.50 37.24 | 31.86 | 28.82 |
| Repo Navigator | GRPO | 12.33 | 21.26 | 14.29 | 12.02 36.36 48.13 | 39.74 | 36.36 |
| Qwen2.5-14B |
| Loc Agent | Training Free 6.22 | 0.13 | 3.65 | 2.65 15.58 | 0.21 | 11.69 | 9.53 |
| CoSIL | Training Free 10.73 | 4.67 | 5.96 | 3.94 34.31 | 9.97 | 14.81 | 9.30 |
| Agentless | Training Free 10.49 | 6.75 | 7.41 | 5.28 41.42 | 13.42 | 19.02 | 12.37 |
| Repo Searcher Training Free 2.79 | 1.38 | 1.69 | 1.14 17.37 | 5.17 | 7.60 | 4.84 |
| Repo Navigator Training Free 14.36 | 19.74 | 15.27 | 12.00 43.57 54.52 | 46.06 | 41.07 |
| Repo Navigator | GRPO | 16.05 25.25 | 18.06 | 14.58 46.85 | 58.64 | 49.72 | 45.14 |
| Qwen2.5-32B |
| Loc Agent | Training Free 8.72 | 0.17 | 4.30 | 2.90 25.73 | 0.38 | 19.77 | 16.50 |
| CoSIL | Training Free 15.00 | 6.35 | 8.14 | 5.21 45.37 | 13.04 | 19.42 | 12.36 |
| Agentless | Training Free 11.08 | 7.31 | 7.98 | 5.80 43.07 | 13.89 | 20.07 | 13.11 |
| Repo Searcher Training Free 2.00 | 1.29 | 1.45 | 1.00 13.51 | 3.43 | 5.31 | 3.24 |
| Repo Navigator Training Free 13.96 | 20.25 | 15.36 | 12.87 50.24 63.24 | 53.48 | 48.50 |
| Repo Navigator | GRPO | 18.13 29.44 | 20.72 | 17.16 53.49 68.69 | 57.57 | 52.44 |
| baseline methods arepresentedin Appendix.A. |
| Results As illustrated in Table.1,on balanced metrics |
| (S -F1 and IoU)forbothfunction-level andfile-levellocal  |
| ization,our method surpasses all baseline methods with |
| the same model size.Moreover,ifwetrain Repo Navigator |
| withGRPO,our7B model surpasses 14Bbaselines,and our |
| 14B model surpasses32Bbaselines onS -F1 and IoU.This |
| contributes to the validness of Repo Navigatorfurthermore. |
| Although somebaselines havehigher recall score signifi  |
| cantlylowerprecision scorethan Repo Navigator,and result |
| inlowerS -F1 and IoU.Thisindicates that Repo Navigator |
| behaves more conservatively andgeneratesless wronglo  |
| Figure3.Ablation study:comparison between Repo Navigator | c SaOti ToAns.For 14Bllandi3i2B mfodels,RhepdoNTavhiigaitor alichievhes |
| with training free,RFT,GRPO with pure outcome and hybrid | reward on Qwen2.5 -7B -Instruct. | thetool weimplementis effective andpromising,and our | single tool pipeline is better than previous multiple tools | among a tra n ng-ree met o s. | s mp es t at |
| pipelines. |
| less(Xia et al., 2024),Orcaloca(Yu et al.,2025b),and Compared with Repo Searcher,which is distilled from |
| Repo Searcher(Ma et al., 2025).Detailed explaination of | claude-3.7-sonnet(Anthropic, 2025)and reinforced by |
| 6 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

| Agent Pipeline | Func-IoU(%) Resolved(%) |
| --- | --- |
| Agentless | 5.28 | 10.12 |
| Loc Agent | 2.65 | 13.01 |
| Repo Navigator | 12.00 | 14.74 |
| Repo Navigator+RL | 14.58 | 15.03 |
| Table3.We use Qwen2.5 -14B -Instruct as thelocalization model, |
| and use Qwen2.5 -32B -Instruct as the repair model on SWE  |
| bench Verified. |
| 4.4.Scaling Law of Tool-Calling |
| Figure4.Scalinglaw oftool-calling,where Pre and Postdenote To assess the significance oftool-callingin Repo Navigator | the corresponding metricbefore and aftertheRLtraining. | we varied the maximum number oftool-calling turns and | , |
| reportedtheresultsin Fig.4.2.As shownin thefigure,allow  |
| ing moretool-callingturns consistentlyleads toimproved |
| GRPO,trained Repo Navigator outperformsit on all metri  performancefor Repo Navigator,bothbefore and after re  |
| ces except recall.Moreover,wefoundthat ourtraining-free inforcementlearning(RL)training.In other words,these |
| method outperforms Repo Searcherfor 14B models.Thisis | results empirically validate the scalinglaw oftool-calling |
| probablydue to the simplifiedtool weintegrateto the agent inthis context | (see Sec.5for moredetails). | . |
| To assess thegeneralizability of Repo Navigator,wepresent 4.5.Influence on Issue Resolution |
| itsperformance on Python samplesfromtheSWE-bench  To evaluate theimpact ofdifferentlocalization results on |
| Pro dataset(Yang et al.,2025b)in Table 2.The results | onthisdataset are consistent withthose observed onSWE  tor againstbaselines onSWE-bench Verified.Wedirectly | thefinalissue resolutionperformance wetest Repo Naviga | , |  |
| bench Verified.While we cannotfully excludethepotential | influence ofdataleakageinSWE-bench Verified,we can localization front-end with other methods.Table.3 illus  | applythe repairingphrase of Agentless while replacingits |
| released afterthepublication of the Qwen2.5 series. | make a stronger claim regardingSWE-bench Pro,asit was | has thehighestperformance onissue resolution,while rein  | trates the results.Compared withbaselines,Repo Navigator |
| 4.3.Training Strategy Comparison | forcementlearningimprovesitsperformancefurthermore. |
| To explorethe capability ofGRPO on agentic training,we 5.Discussion:Building Lessyet More Capable |
| compareGRPO againstRFT-only andRFT+GRPO.Aspre  | Tools |
| sentedin Fig.3,directlytraining withGRPO outperformes |
| RFT-only andRFT+GRPO.Moreover,althoughRFThas ac  In this section,we analyze thelogicbehind Repo Naviga  |
| cetableperformance,the more stepsRFTproceeds,theless | tor:building less tools with morepowerful and more en  |
| improvementGRPO makes afterthe cold start.This conclu  | sembledfunctionsis more effectivethanbuilding multiple |
| sion contradicts withpreviousSWE agents trained withRL | task-specific tools. |
| (Ma et al., 2025),however,it aligns withthebroaderfield of |
| reinforcementlearning,whereRFT andSFT(as a cold start) 5.1.Impact on the Action Space of Agents |
| is effective only when the pretrained model is not strong Let the total number of available tools be denoted as k. |
| enoug | h(G | uohet ad.,d iah.i h entlieprdeitra nle mioie s When only a singletool-specificallythe jump tool-is re  | l 2024)Wh h | i d d li |
| mo e w t |
| strodngl eniohu RgLainb atahs g i-qiua tyf,Sre FcTty(Rt FraT)n ngi a |
| s ettert antra n ng a ter | as ts b h h i | tained,the system's structural relationsbecome simpler,as |

| cold start. | ot t e act on space an t e o servat on space arerestr cte | d h b | i | i d |
| --- | --- | --- | --- | --- |
| to whatthis tool can access.Inthis case,the set ofpossible |
| We also removethe success ratein the rewardfunctionfor | actions and observable elementsis smallerthan when multi  |
| ablation.Aspresentedin Fig.3,reinforcementlearning with pletools are available.This reductionisgenerallybeneficial, |
| hybrid reward(with tool-calling success rate)has higher | since additional tools oftenintroduce new and unfamiliar |
| performancethanpure outcomereward(withouttool-calling interfacesthatlargelanguage modelshave notbeen exposed |
| success rate).Thisindicates thatlearning to correctly call | toduringpretraining,potentiallyincreasing thelikelihood |
| toolsis vitalin agenticlearning. | oferrors. |
| 7 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

### Jump GetClass GetFunc GetStruc IoU

| ✓ | ✓ | ✓ | ✓ | 13.71 |
| --- | --- | --- | --- | --- |
| ✓ | ✓ | ✓ | ✗ | 21.44 |
| ✓ | ✗ | ✗ | ✓ | 24.00 |
| ✓ | ✗ | ✗ | ✗ | 24.28 |
| Table4.We change the tool set of Repo Navigator and present |
| thefunction-level IoU(%)on Qwen2.5 -7B -Instruct.Apparently, |
| excessivetoolsdo notboost Repo Navigator'sperformance. |
| mantically activated by that entry point.Because every |
| location that contributes to theissue mustlie on somede  |
| Figure5.Venngraphillustrating access scope ofjump.Compared pendency path originating from the entry point,itis nec  |
| withthe repository scope,the access scopehas a muchhigher IoU | essarily reachablethroughthis recursive symbol-reference |

| withthegroundtruth set. | expansion.Therefore,thefinal access scopeproducedby |
| --- | --- |
| exhaustive jump traversalisguaranteedto contain allloca  |
| tions that mustbe modifiedto resolvetheissue. |
| 5.2.Impact on Tool-Calling Success Rate |
| For agivenprocessinissuelocalization(forinstance,check  | 5.4.Verification |
| ing the code snippet of afunction),letthe successprobabil  To further verify this proposal,we change the tool set of |
| ity of thei-th callbepi.For ataskthat requiresk sequential Repo Navigator and conductRLtraining with onlythe out  |
| toolinvocations,the overall success rate can be expressed | come reward.We add excessivetools which werefrequently |

| as | usedinprevious works(Chen et al.,2025;Ma et al.,2025; |
| --- | --- |
| Psucc(k) =Ypi . | k | Jiang et al., 2025)andpresent the result in Table.4.Get  |
| (6) Class/Get Func takes a class/function name as input and |
| i=1 | outputs the class/functiondefinition.Get Structakes noin  |
| Since each stepintroduces an additionalpotentialpoint of put and outputstherepository's structure.Theresults clearly |
| failure,the cumulative success rate typicallydecreases as implies that additionaltoolsdo notincrease model'sperfor |
| the number of required tool callsincreases.Therefore,in | mance.Thisinspires researchers todeveloplessbut more |
| general,completing atask with a single,more versatiletool | capabletools. |
| tends tobe more reliablethan relying on multiple narrow  |
| scopetools executedin sequence. | 6.Conclusion |
| 5.3.Impact on the Prediction Space | Iln thlisi workl,weliintrioduced Repho Ndavigatorf,a reposiitoiry  |
| eve ssue oca zat on agent t at eparts rom ex st ng |
| The access scope of atoolisdefined as the complete set of | multi-toolparadigmsbyleveraging a single,more-capable |
| files,symbols,and other resources that thetool can access jump toolfor symbol resolution.This unifieddesignfaith  |
| within a repository.For ajump toolthat navigates to sym  fully reflects real code execution flow while significantly |
| boldefinitions,its access scope can be obtainedby starting | reducing the complexity andbrittleness of multi-step tool |
| from agiven entrypoint and recursively resolving all ref | chaining.Throughtool-integratedGRPO,Repo Navigator |
| erenced symbols until no new definitions can be reached. learns to reason,invoketools,and refineitspredictionsin a |
| Apparently,its access scopeis significantly smallerthanthe | closed-loop manner,enabling end-to-end optimization with  |
| full repository scope.Consequently,when computing the | out relying on closed-sourceteacher models ordistillation. |
| Intdershection ovder Uhnion(IoiU)bhetwjeen theplredictlioniset Extensive experiments across SWE-bench-Verified and |
| an t e groun trut set,us ng t e ump too resu ts n a SWE b h P d |
| higher IoU,as depictedin Fig.5.On the otherhand,ap  | -fenhc -rol emlionsitratetfat epo a Wvgathor ac i evlels | h R N i | hi |
| plying multiple repo-level retriveltools resultsin the access | statel -o -the-art loca zatfioniper ohrmancie.l et eorfetlca yl |
| scope equalto the whole repository scope. | janiaylzet ie reisudts,ichon irmf ngt at als ngiepower u toido , |
| o nty opt m ze w t re n orcement earn ng,canprov e |
| When we startfrom the entry point and repeatedly apply | stronger robustness and more reliable multi-step reason  |
| jump -which retrieves the definition of each referenced ingthanpreviousframeworks relying on multiple narrowly |
| symbol-we effectively traverse all symbols that are se  | scopedtools. |
| 8 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

Ourfindingshighlighttheimportance ofaligning agenttool  **References**

ing wiith reallexecutiioin structurel,akndbshowithlat eifficient Ahn,J.,Verma,R.,Lou,R.,Liu,D.,Zhang,R.,andYin,W.

rfeasondnig-tooicod-tra n ng can un ocd slu sFtant a ga nks eveilnl

Largelanguage modelsfor mathematical reasoning:Pro 

exp ore exten ng epo avgator rom yt onto morepro 

orlme um-sdize RopenN-souirce mfo e sP.huture wor w

gresses and challenges.*arXivpreprint arXiv:2402.00157*,

| gramminglanguages. | 2024. |
| --- | --- |
| Anthropic. | Claude 3.7 sonnet and claude code. |
| https://www .anthropic .com/news/ |
| claude -3 -7 -sonnet, February 2025. | data: |
| 2025 -11 -18. |
| Chen,Z.,Tang,R.,Deng,G.,Wu,F.,Wu,J.,Jiang,Z., |
| Prasanna,V.,Cohan,A.,and Wang,X.Loc Agent:Graph  |
| guided LLM agents for code localization.In Che,W., |
| Nabende,J.,Shutova,E.,and Pilehvar,M.T.(eds.),Pro  |
| ceedings of the63rd Annual Meeting of the Association |
| for Computational Linguistics(Volume1:Long Papers), |
| pp.8697 -8727,Vienna,Austria,July2025.Association |
| for Computational Linguistics.ISBN979-8 -89176-251 - |
| 0.doi:10.18653/v1/2025.acl-long.426.URL https: |
| //aclanthology .org/2025.acl -long .426/ . |
| Guo,D.,Zhu,Q.,Yang,D.,Xie,Z.,Dong,K., |
| Zhang,W.,Chen,G.,Bi,X.,Wu,Y.,Li,Y.,et al. |
| Deepseek-coder:Whenthelargelanguage model meets |
| programming -therise ofcodeintelligence.ar Xivpreprint |
| ar Xiv:2401.14196,2024a. |
| Guo,T.,Chen,X.,Wang,Y.,Chang,R.,Pei,S.,Chawla, |
| N.V.,Wiest,O.,and Zhang,X.Largelanguage model |
| based multi-agents:A survey ofprogress and challenges. |
| ar Xivpreprint ar Xiv:2402.01680,2024b. |
| Gupta,T.and Kembhavi,A.Visualprogramming:Compo  |
| sitional visual reasoning withouttraining.In Proceedings |
| of theIEEE/CVF conference on computer vision andpat |
| tern recognition,pp.14953 -14962,2023. |
| He,Z.,Yang,Q.,Sheng,W.,Zhong,X.,Zhang,K.,An,C., |
| Shi,W.,Cai,T.,He,D.,Chen,J.,and Xu,J.Swe-swiss:A |
| multi-taskfine-tuning and rl recipeforhigh-performance |
| issue resolution.https://github.com/zhenyuhe00/SWE  |
| Swiss,2025.Notion Blog. |
| Hong,W.,Wang,W.,Lv,Q.,Xu,J.,Yu,W.,Ji,J.,Wang,Y., |
| Wang,Z.,Dong,Y.,Ding,M.,et al.Cogagent:A visual |
| language model for gui agents.In Proceedings of the |
| IEEE/CVFConference on Computer Vision and Pattern |
| Recognition,pp.14281 -14290,2024. |
| Huang,X.,Liu,W.,Chen,X.,Wang,X.,Wang,H.,Lian, |
| D.,Wang,Y.,Tang,R.,and Chen,E.Understanding |
| the planning ofllm agents:A survey. ar Xivpreprint |
| ar Xiv:2402.02716,2024. |
| Hui,B.,Yang,J.,Cui,Z.,Yang,J.,Liu,D.,Zhang,L., |
| Liu,T.,Zhang,J.,Yu,B.,Lu,K.,et al.Qwen2.5 -coder |
| technical report.ar Xivpreprint ar Xiv:2409.12186,2024. |

9


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

Jiang,Z.,Ren,X.,Yan,M.,Jiang,W.,Li,Y.,and Schmidgall,S.,Su,Y.,Wang,Z.,Sun,X.,Wu,J.,Yu,X., Liu,Z. Cosil:Software issue localization via llm 

Liu,J.,Moor,M.,Liu,Z.,andBarsoum,E.Agentlab 

driven code repositorygraph searching.*arXivpreprint*

oratory:Usingllm agents as research assistants.*arXiv*

| ar Xiv:2503.22424,2025. | preprint ar Xiv:2501.04227,2025. |
| --- | --- |
| Jimenez,C.E.,Yang,J.,Wettig,A.,Yao,S.,Pei,K.,Press, Shen Z Llm with tools:A survey ar Xiv preprint | els resolve real-world github issues?ar Xiv preprint | O.,and Narasimhan,K.Swe-bench:Canlanguage mod  | ar Xiv:2409.18807,2024. | , . | . |
| ar Xiv:2310.06770,2023. | Team,Q. Qwen2 technical report. ar Xiv preprint |
| Jin,B.,Zeng,H.,Yue,Z.,Yoon,J.,Arik,S.,Wang,D., | ar Xiv:2407.10671,2024. |
| Zamani,Hd.l,and Han,J.h Searich-r1:i Thrainiinfg llms to Wang,X.,Li,B.,Song,Y.,Xu,F.F.,Tang,X.,Zhuge, |
| rleasoin an everage searc eng2n0e3s0w9t16re2n02o5rcement |
| earn ng.ar Xivpreprint ar Xiv:5. 5 , | . |
| M.,Pan,J.,Song,Y.,Li,B.,Singh,J.,Tran,H.H., |
| Li,F.,Ma,R.,Zheng,M.,Qian,B.,Shao,Y.,Muen  |
| Kwon,W.,Li,Z.,Zhuang,S.,Sheng,Y.,Zheng,L.,Yu, | nighoff,N.,Zhang,Y.,Hui,B.,Lin,J.,Brennan,R., |
| C.H.,Gonzalez,J.E.,Zhang,H.,and Stoica,I.Efficient | Peng,H.,Ji,H.,and Neubig,G. Openhands:An |
| memory managementforlargelanguage model serving | open platform for AI software developers as general  |
| withpagedattention.In Proceedings of theACMSIGOPS | ist agents.In The Thirteenth International Conference |
| 29th Symposium on Operating Systems Principles,2023. | on Learning Representations,2025a. URL https: |
| Langley,P.Craftingpapers on machinelearning.In Langley, | //openreview .net/forum?id=OJd3ayDDoF. |
| P.(ed.),Proceedings of the17th International Conference Wang,Y.,Mao,W.,Wang,C.,Zhou,Z.,Zhou,Y.,Zhao,W., |
| ford,CA,2000.Morgan Kaufmann. | on Machine Learning(ICML2000),pp.1207 -1216,Stan  | Lou,Y.,and Peng,X.Extracting conceptualknowledgeto | locate softwareissues.ar Xivpreprint ar Xiv:2509.21427, |
| Li,Y.,Wen,H.,Wang,W.,Li,X.,Yuan,Y.,Liu,G.,Liu, | 2025b. |
| J.,Xu,W.,Wang,X.,Sun,Y.,et al.Personalllm agents:Xi C S D Y D S d Zh L A l D | I i h d | securty.ar vprepr nt ar v: | ns gits an Xisurvey ai out Xtie2c4ap0a1 054ty5,9e20c2e4ncy an | b h | bili ffi i | . | , | . | d | preprint ar Xiv:2407.01489,2024. | mystifyingllm-based software engineering agents.ar Xiv | a, . ., eng, ., unn, .,an | ang, . gent ess:e  |
| Liu,A.,Feng,B.,Xue,B.,Wang,B.,Wu,B.,Lu,C.,Zhao, | C Deng C Zhang C Ruan C et al Deepseek-v3 Yan,Y.,Wang,S.,Huo,J.,Yu,P.S.,Hu,X.,and Wen,Q. |
| technical report.ar Xivpreprint ar Xiv:2412.19437,2024. | ., | , ., | , ., | , ., | . | Mathagent:Leveraging a mixture-of-math-agentframe  |
| Liu,Z.,Zhang,Y.,Li,P.,Liu,Y.,and Yang,D. Dy  | workfor real-world multimodal mathematical errorde  |
| namic llm-agent network:An llm-agent collaboration | tection ar Xivpreprint ar Xiv:250318132 2025 | . | . | , | . |
| framework with agentteam optimization.ar Xivpreprint Yang,A.,Li,A.,Yang,B.,Zhang,B.,Hui,B.,Zheng,B., |

| ar Xiv:2310.02170,2023. | Yu,B.,Gao,C.,Huang,C.,Lv,C.,et al.Qwen3 technical |
| --- | --- |
| Lu,J.,Holleis,T.,Zhang,Y.,Aumayer,B.,Nan,F.,Bai, | report.ar Xivpreprint ar Xiv:2505.09388,2025a. |
| box:A stateful,conversational,interactive evaluation | benchmarkforllmtool use capabilities.ar Xivpreprint | F.,Ma,S.,Ma,S.,Li,M.,Yin,G.,et al.Toolsand  Y J Ji | ang, ., menez, . ., e g, ., ere, ., ao, ., | Narasimhan,K.R.,and Press,O.SWE-agent:Agent  | computer interfaces enable automated software engi  | C E W tti A Li t K Y S |

| ar Xiv:2408.04682,2024. | neering.In The Thirty-eighth Annual Conference on |
| --- | --- |
| Luo,M.,Jain,N.,Singh,J.,Tan,S.,Patel,A.,Wu,Q., | Neural Information Processing Systems,2024a.URL |
| Ariyak,A.,Cai,C.,Tarun Venkat,S.Z.,Athiwaratkun, | https://arxiv .org/abs/2405.15793. |
| B.,Roongta,M.,Zhang,C.,Li,L.E.,Popa,R.A., | of-the-art coding agent from scratch by scaling rl. | Sen K and Stoica I Deepswe:Training a state | https://pretty -radio -b75.notion .site/ | DeepSWE -Training -a -Fully -Open -sourced -Statsey-stoemf -stghenee-r Aalrizte-t Coovdisiunagl -so Aftgweanrte -dbomy -ai Sncs?alairn Xgiv-RL -22281902c1468193aabbe9a8c59bbe33, | , ., | , . |  | Yang,J.,Jimenez,C.E.,Zhang,A.L.,Lieret,K.,Yang, | J.,Wu,X.,Press,O.,Muennighoff,N.,Synnaeve,G., | Narasimhan,K.R.,et al.Swe-bench multimodal:Do ai |

| 2025.Notion Blog. | preprint ar Xiv:2410.03859,2024b. |
| --- | --- |
| Ma,Z.,Peng,C.,Zeng,Q.,Gao,P.,Zou,Y.,and Xie, Yang,J.,Lieret,K.,Jimenez,C.E.,Wettig,A.,Khandpur, |
| B.Tool-integrated reinforcementlearningfor repodeep | K.,Zhang,Y.,Hui,B.,Press,O.,Schmidt,L.,and Yang, |
| search,2025. URL https://arxiv .org/abs/ | D.Swe-smith:Scaling data for software engineering |

2508.03012.

agents.*arXivpreprint arXiv:2504.21798*,2025b.

10


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

Yu,Q.,Zhang,Z.,Zhu,R.,Yuan,Y.,Zuo,X.,Yue,Y.,Dai, **A.DetailedIllustration ofBaselines**

| W.,Fan,T.,Liu,G.,Liu,L.,et al.Dapo:An open-source | llmreinforcementlearning system at scale.ar Xivpreprint Agentless Agentless(Xia et al., 2024)is a workflowfor |
| --- | --- |
| ar Xiv:2503.14476,2025a. | issuelocalization.First,itidentifies suspiciousfilesin the |
| repository.Second,relevant classes andfunctions arede  |
| Yu,Z.,Zhang,H.,Zhao,Y.,Huang,H.,Yao,M.,Ding, tected.Third,preciselocationsfor edit aregivenbyLLMs |
| K.,and Zhao,J.Orcaloca:An llm agent framework based on the classes andfunctions. |
| for software issue localization,2025b.URL https: |
| //arxiv .org/abs/2502.00350. | CoSIL CoSIL(Jiang et al., 2025)is an agent whichfirst |
| Yuan,S.,Song,K.,Chen,J.,Tan,X.,Shen,Y.,Kan,R., l ll li ti C SILd | conduct file-level localization and then conductfunction  |
| Li,D.,and Yang,D.Easytool:Enhancing llm-based | efve docla z(alon.f o ti y)ndamica tyhcons rulc slca grahpi s | i ll | t t ll h |
| agents with concise tool instruction. ar Xiv preprint | process,an app es con ex prun ng o e ec vey re uce |
| o mo u esdc assl,i unc otns t ur ngi etrepfof-etvie lsearcd ng |
| ar Xiv:2401 06201 2024 | . | , | . | the searching scope. |
| Yue,Y.,Yuan,Y.,Yu,Q.,Zuo,X.,Zhu,R.,Xu,W.,Chen, |
| J.,Wang,C.,Fan,T.,Du,Z.,et al.Vapo:Efficient and Loc Agent Loc Agent(Chen et al., 2025)is almost afully  |
| reliable reinforcementlearningfor advanced reasoning | automaticLLM agentbesidesitsplanningprompt concate  |
| tasks.ar Xivpreprint ar Xiv:2504.05118,2025. | nated into the context at the beginning of the searching |
| process.Itbuilds the whole repositoryinto adirecthetero  |
| geneousgraph,whose nodes arefiles,classes,andfunctions. |
| Additionally,edges arebuiltbydependencies such asim  |
| ports andinvocations.Multiplegraph-level searchingtools |
| are equippedto theLLMfor multi-hop reasoning. |
| Repo Searcher Repo Searcher(Ma et al., 2025)is an agent |
| thatfirst conductsfile-levellocalization andthenfunction  |
| levellocalization,which aligns with CoSIL.Repo Searcher |
| introduced the first training framework Tool Train for lo  |
| calization agents,whichis composed ofdistilling from a |
| close-source model(Claude3.7-Sonnetin Repo Seacher)as |
| warmup and reinforcementlearning tofurther enhancethe |
| performance. |
| Ours Compared with allbaselines,we arethefirstfully  |
| automaticLLM agent,with nofixed workflow and noplan  |
| etaryprompt,and we are thefirst method traineddirectly |
| frompretrained open-sourceLLMs without a close-source |
| teacher model.Lastly,we onlyintegrate a singleyetpower |
| fultoolto the agent,which reduces compounding error and |
| narrows the access scope of the agent. |
| B.Experimental Details |
| Hyperparameters We set clip ratio low to 0.2, |
| clip ratio high to 0.8,learning rate to 10 −6, train  |
| ing batch sizeto 128,trainingtemperatureto 1.0,maximum |
| tool-callingtimes to 12,and max response lengthto 10240. |
| Metrics Given the set ofpredictedlocations(etherfile  | level orfunction-level)Y,andthe set ofgroundtruthloca  | ˆ |
| tionsY∗,the aforementioned metrics are calculated as: |
| Recall =|Y ∩Y∗| | ˆ | |Y∗| | (7) |
| 11 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

| Jump Get Class Get Func Get Struc Recall Precision F1 | IoU Recall Precision F1 | IoU |
| --- | --- | --- |
| ✓ | ✓ | ✓ | ✓ | 14.28 | 15.44 | 14.40 13.71 35.78 | 36.76 35.59 34.55 |
| ✓ | ✓ | ✓ | ✗ | 22.60 | 25.02 22.80 21.44 48.49 | 50.13 48.52 47.17 |
| ✓ | ✗ | ✗ | ✓ | 24.64 | 27.48 25.05 24.00 53.48 | 55.76 53.68 52.69 |
| ✓ | ✗ | ✗ | ✗ | 25.11 | 29.16 25.75 24.28 55.81 | 58.71 56.32 54.89 |
| Table5.We changethetool set of Repo Navigator andpresentthefunction-level IoU.Becausethe jump toolis alreadypowerful enough |
| forlocalization,excessivetoolsdo notincreaseitsperformance. |
| Precision=|Y|∩YˆY| ∗| | ˆ | (8) | rmepanocseitoorfyt-healnadngduyangaemsiecrvimerp,oasrtistscafunndcetigornaadleitythreelpieersfoonr |
| S l F1 2 ×|Y ∩Y∗| | ˆ | static analysis techniques such as abstract syntaxtrees and |
| amp e- =|Yˆ|+|Y∗| | (9) | symbol tables.When such circumstances occur,the tool |
| returns an error messageindicatingthat thedefinition of the |
| IoU |Y ∩Y∗| | ˆ | current symbol cannotbelocateddue to unknown reasons. |
| = | ˆ | (10) Nevertheless,in our empirical evaluation,we did not ob  |
| |Y ∪Y∗| | serve anyinstances ofmonkeypatching ordynamicimports |
| Inpractice,whentheprediction set Yis empty(forinstance, | ˆ | within the analyzeddatasets. |
| totalfailure),we set recall,precision,sample-F1,and IoU | to zero.We use the function-level localization result of C.Threatsto Validity |
| different methods and applythepatchgenerationbackend Groundtruth Retrieval Alimitation ofour workliesin |
| in Agentless(Xia et al., 2024)to generate patches.Re  the extraction ofgroundtruthlocations.We extract modified |
| solved(%)denotes thepercentage of samples thatpass all locations directly from the gold patch in the datasets, |
| test units after applyingthepatch. | which mayignore otherpatches that also resolvetheissue. |
| Implementation When the response exceeds the maxi  into consideration.However,using golden patches is ac  | Our evaluation metricsdo nottakethese correct alternatives |
| tool-callingtimes(whichis 12),we add "You must not call | tool's response.Most of thetime,the agent will stop calling | mumlength,we clip andforcethe agentto stop,and wegive | tools anymore,andyou mustgivethefinal answer"to the | zero as its score.When the agent exceeds the maximum | reveals golden locations(locations in golden patches),it | undoubtedly contributes to the resolution of theissue and | the result in Table 3demonstrates this claim | ceptable when comparing mutliple methods If a method | . | . | . | , |
| tools andgeneratethefinal response.Ifnot,weforceitto |
| stop andgive zero as its score.Note that when the maxi  Language Limit Anotherlimitationis that we only evalu  |
| which allows the agentto explorein the environments with | mumtool-callingtimesis not achieved andthefinal answer | is generated,the agentloop will stop automatically.The | aforementionedprocessis an automatic agenticframework, | eachlanguage(C/C++,Java,etc.)hasits uniquelanguage | ate Python repositoriesin our experiments.Thisisbecause | server ofpython.We willimplement morelanguage servers | server,and we only succeedinimplementingthelanguage |

| little constraints. | and validate our approach on moreprograminglanguages |
| --- | --- |
| in thefuture. |
| Preventing Data Leakage It is a widespread concern |
| va ty o post-tra n ng met o s. evert e ess,we exc u e |
| thalitddi atafleakageiatithe preh-trdain Ning phhrasle threatenslthde D.Case Study |
| this concernby results in Tabel.2.The SWE-bench Pro Inthis section,wepresentthefulltrajectory of Repo Navi  |
| dataset was publishedin 2025,while the Qwen2.5 series gator on astropy astropy-12907fromSWE-bench Verified. |
| werepublishedin2024.Moreover,we excludethe samples We apply thedefault tool-callingprompt template of verl |
| bench Verified orSWE-bench Pro. | in thetrainingdatasetifthe repository also appearsinSWE  (Shen 2024)and present an example Noted we do not | present any process restrictions in ourprompt,encourag  | , | . | , |
| ing Repo Navigatortoplan,calltools,and makedecisions |
| Language Server In practice,we apply a Python lan  full-automatically.This is distinct with Agentless(which |
| guage server to extract the definition code corresponding has afixed workflow),Loc Agent(whichpredefines a spe  |
| to an invoked symbol within a repository.However,the | cific step-by-step workflowinits systemprompt),CoSIL |
| presence ofmonkeypatches -runtime modifications to the | and Repo Searcher(whichishalf-automaticbecause some |
| 12 |


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

forced steps are addedtothe workflowbesidesthe automatic multi-turns tool-calling conversations).

13


---

**OneToolIsEnough:ReinforcementLearningforRepository-LevelLLMAgents**

Prompt

[system] You are Qwen, created by Alibaba Cloud . You are a helpful assistant .

#Tools

You may call one or more functions to assist with the user query .

You are provided with function signatures within <tools></tools>XML tags: <tools> {"type": "function", "function":{"name": "check", "description": "In the specific file path, a symbol is referred and this tool can find where the tool is defined .

| For instance, in the first turn, file path is the entry point of .", | "parameters":{"properties":{"symbol":{"description": "The symbol whose | _ |
| --- | --- | --- |
| definition code will be given to the agent .", "type": "string"}, "file path": | {"description": "The relevant path to the file where the symbol is referred .", | _ |
| "type": "string"}}, "required": ["symbol", "file_path"], "type": "object"}}} |
| </tools> |
| For each function call, return a j son object with function name and arguments |
| within <tool _call></tool _call>XML tags: |
| <tool call> | {"name": <function -name>, "arguments": <args -j son -object>} | _ |
| </tool call> | [user] | _ |
| You are given a codebase and an issue, you need to locate the files and |
| functions causing this issue . |
| You can call the tool to check the definition code of a symbol . You can only |
| check the symbol once for each turn . |
| NOT where it is defined! | The ' file path ' is the relevant path of where the symbol is called, | _ |
| For instance, if ' classA .functionB ' is what you want to check(which is called |
| in fileA .py), you should directly check ' functionB ' in ' fileA .py ' . |
| This is the issue: |
| [Problem Statement] |
| The entry file of the code base is: |
| [Relevant Path To Entry Point] |
| [Entry Point] |
| Your final answer should be all functions that should be modified, such as: |
| relevant/path/to/file1.py::func_name1,relevant/path/to/file2.py::func_name2, |
| . . .(a series of file::function pairs seperated by comma) |
| Please put your final answer inside\boxed{} only in the last turn . |
| You can only call the tool once each turn . |
| For instance: |
| {' name ':' check ', ' arguments ':{' symbol ':' symbol _to_be_checked ', ' file_path ': |
| ' file_where_the_symbol _is _used '}} |
| 14 |