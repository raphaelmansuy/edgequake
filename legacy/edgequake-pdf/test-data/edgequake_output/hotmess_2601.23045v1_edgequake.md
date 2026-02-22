## Page 1

# THE HOT MESS OF AI: HSCALE WITH MODEL INTELLIGENCE ANDTASK COMPLEXITY?


**arXiv:2601.23045v1 [cs.AI] 30 Jan 2026** 

Alexander Hagele ¨ ∗1,2 Aryo Pradipta Gema Jascha Sohl-Dickstein ∗5

As AI becomes more capable, we entrust it with more general and consequential tasks. The risks from failure grow more severe with increasing task scope. It is therefore important to understand how extremely capable AI models will fail: Will they fail by systematically pursuing goals we do not intend? Or will they fail by being a hot mess, and taking nonsensical actions that do not further any goal? We operationalize this question using a bias-variance decomposition of the errors made by AI models: An AI's incoherence randomness as the fraction of its error that stems from variance rather than bias in task outcome. Across all tasks and frontier models we measure, the longer models spend reasoning and taking actions, Incoherence changes with model scale in a way that is experiment dependent. However, in several settings, larger, more capable models are more incoherent than smaller models. Consequently, scale alone seems unlikely to eliminate incoherence. Instead, as more capable AIs pursue harder tasks, requiring more sequential action and thought, our results predict failures to be accompanied by 

consistent pursuit of a misaligned goal. This increases the relative importance of alignment research targeting reward hacking or goal misspecification.

hot-mess-of-ai

## 1 INTRODUCTION

There are an increasing number of predictions that AI will soon be more capable than human beings (Kwa et al., 2025; Maslej et al. in many domains (Chen et al., 2025b

2024; Johnston & Makridis, 2025). We already rely on AI for consequential tasks such as writing critical software (DeepMind, 2025; and deciding what stories to present in news feeds ( Yamana, 2025). Despite its increasing capabilities, AI often behaves in ways we do not intend. Due to its high-stakes use cases, it is important to understand how and when AI can be expected to fail. One class of AI risk is misalignment risk Misalignment risk is the concern that AI will pursue a goal that is different from the goal its creators intended to instill, and that it will pursue that goal with superhuman competence. If a superhuman agent pursues a misaligned goal, it might do things like seize power as an instrumental step to achieving its goal (Hubinger et al. However, this scenario assumes that unintended behavior stems from systems that not only pursue the wrong objective, but remain coherent optimizers over a long horizon. Large language models

## OW DOES MISALIGNMENT

Henry Sleight 4 Ethan Perez 5

4ConstellationAnthropic 5

### ABSTRACT

 on a task is measured over test-time

 the more incoherent their failures become.

hot-mess-data

1Anthropic Fellows ProgramEPFL University of Edinburgh 2

alexander.hagele@epfl.ch, jascha@anthropic.com

(LLMs), prior to reinforcement learning, are dynamical systems, but not optimizers. They have to be trained to act as an optimizer, and trained to align with human intent. It is not clear which of these trained properties will tend to be more robust, and which will be most likely to cause failures

, 2025; Pimpale et al., 2025), and will replace human labor ; Handa et al., 2025; Dominski & Lee, 2025; Eloundou et al., Appel et al., 2025), determining bail amounts (Fine et al., 2025),

Liu et al., 2024; Gao et al., 2024b; Yada &

 (Bostrom, 2014; Russell, 2019; Greenblatt et al., 2024).

, 2019).


---

## Page 2

> Figure 1: AI can fail because it is misaligned, and produces consistent but undesired outcomes, or because it is incoherent, and does not produce consistent outcomes at all. These failures
>

correspond to bias and variance respectively. As we extrapolate risks from AI, it is important to understand whether failures from more capable models performing more complex tasks will be bias or variance dominated. Bias dominated failures will look like model misalignment, while variance dominated failures will resemble industrial accidents. we observe that AI models fail in unpredictable and inconsistent ways. Often, these failures can be fixed by resampling. (top right) To quantify this observation, we decompose errors made by AI into two terms, bias and variance. We illustrate this using a multiple choice task: bias is the tendency to pick a specific incorrect answer; variance is the tendency to pick inconsistenly among options. We define incoherence as the fraction of model error caused by variance. ( Experimentally, we find that as models reason longer and take more sequential actions, they become more incoherent. (lower right) We find that as models become more capable, and overall error rate drops, incoherence changes in a way that depends on task difficulty. Easy tasks become less incoherent, while hard tasks trend towards increasing incoherence.

in superhuman systems. In practice, AI models often fail in ways that seem random and do not further any coherent goal (Spiess, 2025; Nolan, 2025). Like humans, when AIs act undesirably, it is often because they are a hot mess and do not act in a way that is consistent with any goal: The mess theory of intelligence (Sohl-Dickstein their behavior tends to become more incoherent, and less well described through a single goal. If true for AI systems, this shifts both the likelihood and the focus of misalignment scenarios. In this paper, we therefore ask the questions: intend, what fraction of its deviation is due to fraction to variance (randomness in behavior and outcome)? As we scale model intelligence and task complexity, how does this decomposition change? Asymptotically, as extremely capable models perform extremely complex tasks, which class of undesired behavior will dominate? We address these questions by measuring the scaling behavior of AI errors decomposed into

ERROR = BIAS

and further define incoherence as the proportion of variance to the total error. This decomposition allows us to distinguish the relative contributions

 (top left) Qualitatively,

lower left)

tantly, how they change as models become more intelligent and perform longer horizon tasks. Bias-dominated failures correspond to systematic misalignment-consistent pursuit of the wrong objective-whereas variance-dominated failures

 indicate inconsistent outcomes. , 2023) suggests that as entities become more intelligent, When a model does something other than what we bias (consistent pursuit of the wrong goal), and what + VARIANCE ,

 of different types of AI failure, and, impor-


---

## Page 3

We find that across multiple-choice benchmarks, agentic coding, and safety tasks, models become more incoherent with longer reasoning (Fig. Larger, more capable models are often more incoherent (Fig. they grow more coherent on easy tasks but less coherent on hard tasks (Fig. findings in a synthetic environment where variance asymptotically dominates with increasing model size (Fig. 6), and find that ensembling and larger reasoning budgets reduce incoherence (Fig. We discuss our results in Section 5.

## 2 BACKGROUND

### 2.1 BIAS-VARIANCE DECOMPOSITION

Definition. In supervised settings, the of a predictor as the sum of three terms: B Wolpert, 1996). Although originally formulated for regression, analogous decompositions exist for classification tasks (Kohavi & Wolpert the bias reflects the error of the classifier's deviation. Several such decompositions exist, including the Breiman, 1996; Kohavi & Wolpert, 1996 Brier score (Degroot & Fienberg, 2018 Kullback-Leibler (KL) decomposition in the main text. For additional definitions see Appx. We ran experiments with KL, Brier, and 0/1 formulations. All three decompositions produce qualitatively similar results, and we provide plots for all three in appendices. Let x be the input with label classes distribution (potentially one-hot) over class labels of the training process. The target is one-hot encoded through the dependence of y andon f x. We assume the irreducible noise to be *ε*

cross-entropy error can be decomposed into (

ERROR

where y[c] denotes the c-th element of the vector, the average of log-probabilities after normalization: We denote this decomposition as KL-B decomposition for Bregman Divergences ( Different usage to classical literature. cally assumes a deterministic model ( under different seeds or data sampling. That means the expectation is over training randomness Our setting differs: rather than retraining multiple models, we analyze a expectation over input (e.g., few-shots) and output (sampling) randomness Incoherence. Throughout this paper, our main metric of interest is the to the total error, which we define as I *Q = {q} ii≤N* and a model f. We then denote incoherence as *ε*

INCOHERENCE(Q, f) :=ε

Since ERROR(q, f) = BIAS(q, f)+ VARIANCE *iε iε*

[0, 1]: a value of 0 means that the model never deviates from its average behavior and any error will be consistent; a value of 1 means that every error the model makes is inconsistent. Importantly, a model can achieve a lower overall error rate, but have a higher incoherence, which makes it a comparable measure across error levels and model capabilities. We see such cases in Section

### 2.2 SCALING BEHAVIOR OF LARGE LANGUAGE

 2), even when controlling for task difficulty (Fig. 3).

 4): while they achieve lower error,

 5). We validate these

 bias-variance decomposition expresses the expected error , VARIANCEIAS, and irreducible noise (Kohavi & , 1996; Domingos, 2000), with a similar interpretation: mean or mode prediction and variance quantifies its

 0/1 error (Kong & Dietterich, 1995;

; Tibshirani, 1996; Friedman, 1997; Domingos, 2000), ), and cross-entropy error (Heskes, 1998). We present a

*C*. For clarity, we omit y(x) ∈ R

                                0. Then, the expected

Yang et al., 2020):

*y∥f+ E D*¯

(f∥f),¯

|{z}| {z}

BIAS VARIANCE

is the Kullback-Leibler divergence, and Dfis ¯

IAS and KL-VARIANCE. This is an instance of the general

Pfau, 2013). 

 fixed model and take the *ε for the same task.* proportion of the variance

NCOHERENCE. Formally, consider a set of questions

P

*i*VARIANCE(q, f)iε P

*. (2)*

*i*ERROR(q, f)iε (q , f), INCOHERENCE is a relative value in*iε*

Scaling laws. Model performance generally follows predictable model size N, dataset size D, and compute

 power-law scaling with respect to

 *C (Kaplan et al., 2020; Hoffmann et al., 2022). Most*

 3.

 MODELS


---

## Page 4

prominently, taking the parameters *l(N) ∝ N* −αfor some exponent α. This slope Section 3.2 we will compute scaling laws independently for bias and variance loss contributions, to judge which asymptotically dominates. Reasoning and inference compute. Besides the model and dataset size, the most promising recent development uses inference compute are trained with reinforcement learning (RL) to think in long chains of thought before providing an answer, which improves performance with larger thinking budgets ( 2024; Guo et al., 2025; Anthropic, et al., 2025a; Zhong et al., 2024; Muennighoff et al. aspect of our analysis, which we see as a process of sequential action steps (

## 3 EXPERIMENTS

Overview. We present our results grouped by observations: first, growing incoherence as a function of reasoning length (3.1) and scaling laws with model scale ( reasoning budgets and ensembling ( 

- Multiple Choice Tasks. We use the popular scientific reasoning benchmark GPQA (

2024), and general knowledge benchmark MMLU ( simply the correct answer.

- Agentic Coding. This focuses on SWE-B

GitHub issues using tools, and success is measured with unit tests.

- Safety and Alignment. We assess models using the advanced AI risk subset of Model-Written

Evals (MWE; Perez et al., 2023), both with the original multiple choices and in an open-ended format with answer options removed.

- Synthetic Settings. We train transformers of varying scales to directly emulate an optimizer

descending an ill-conditioned quadratic loss. The transformer is tasked with predicting string representations of optimizer update steps based on the current state. This is a simple toy model of an LLM that has been trained to act as an optimizer. See Section

- Survey. In addition to experiments using LLMs, we report the survey results of

(2023) (previously released in blog form), where disjoint sets of human subjects subjectively ranked the intelligence and coherence of AI models, humans, non-human beings, and organizations. The details are provided in Appx. Setup and Metrics. Across all tasks, unless otherwise noted, we obtain at least mate bias and variance per question. We find this sample count to be sufficient for stable estimates (see Appx. C.5 and B). Each sample is run with a different seed for autoregressive generation. For GPQA and MMLU, samples additionally use a different random few-shot context. We report the following metrics (details in Appx.

- For multiple choice questions, our main metric of interest is the KL-I

incoherence with respect to KL-B qualitative behavior for other decompositions, as reported in Appx.

- For open-ended MWE safety questions, we embed solely the answers (

chains) using a text embedding model ( port the variance of the embedding vectors

- For SWE-BENCH, we assign binary vectors for each sample and task: each vector is of size

the number of unit tests for task error then computes the mean squared difference to a vector of all 1's, which we decompose into bias and variance contributions.

 *N as an argument, the cross-entropy loss broadly behaves as α informs us about the rate of improvement. In*

 as an axis of scale. Specifically, so-called reasoning models

Snell et al., 2025; Jaech et al.,

 2025b; OpenAI, 2025a; Team, 2025a; Team et al., 2025; Chen

, 2025). The length of reasoning is an important

Lightman et al., 2023).

3.2); this is followed by the effects of

3.3). The details of all experimental setups are in Appx. B.

Rein et al.,

Hendrycks et al., 2021). Target responses are

ENCH (Jimenez et al., 2024), where agents solve

 3.2.2 for details.

 Sohl-Dickstein

 B.5.

 30 samples to esti-

Models. We evaluate the following frontier models: S enabled, O3-MINI (OpenAI, 2025a), and model size as an imperfect proxy for intelligence, we use the Q

ONNET 4 (Anthropic, 2025a) with reasoning

 O4-MINI (OpenAI, 2025b). When analyzing scaling

 w.r.t.

WEN3 model family with thinking

 A and B):

NCOHERENCE, i.e., the

IAS and KL-VARIANCE (Equations 1 and 2). We find the same

 C.1. i.e., without reasoning

text-embedding-3-large). Consequently, we re-in the Euclidean norm.

 *i, and encodes which tests a model's code passes. The coverage*


---

## Page 5

(a) GPQA

(c) Model Written Evals: Discrete Choice and Open-Ended Formats

> Figure 2: Across a variety of settings, as models reason longer or take more actions, they become more incoherent. We assess frontier models (S
>

across a variety of different tasks (MCQ, Agentic Coding, Alignment). We evaluate with samples to estimate bias and variance terms for each question. When sorting questions by average 

many actions, their errors are dominated by variance. We make a similar observation for the vari- ance of text embeddings to open-ended safety questions (

(a) GPQA: Frontier Models (left) and Q WEN3 (right)

> Figure 3: For a fixed task and reasoning budget, natural variation in reasoning length and action count is predictive of incoherence. We analyze GPQA (left, (a)) and SWE-BENCH (b) by
>

splitting samples into aboveor below-median reasoning length (GPQA) or actions (SWE-B per question. We then compute performance and incoherence for both groups. longer reasoning shows increased incoherence for both frontier models (left) and Q (b) Similar observations apply to SWE-B ENCH, where longer action sequences display higher

incoherence for test coverage (right). This effect is much stronger than through larger reasoning budgets (Fig. 7), and the difference in accuracy or score is minimal between both groups (Fig.

enabled (Team, 2025a). In Sect. 3.2.2, we train our own autoregressive transformers on a synthetic optimization task.

### 3.1 THE RELATION BETWEEN REASONING L ENGTH, ACTION LENGTH AND INCOHERENCE

The longer models spend reasoning and taking actions, the more incoherent they become.

(b) SWE-BENCH

(d) Synthetic Optimizer

ONNET 4, O3-MINI, O4-MINI, QWEN3)

 many

(c), right), and in a synthetic setting (d).

(b) SWE-BENCH

Sorting by reasoning & action length. We begin with a key experimental observation. Fig. 2 shows all setups with reasoning tokens (or actions for SWE-B

ENCH, optimization steps for the

ENCH)

 (a) The naturally

WEN3 (right).

 17).


---

## Page 6

synthetic setting) on the x-axis and incoherence or variance on the y-axis. For Figures lines show different question sets across and within models, obtained by sorting by average length and grouping into equal buckets, with incoherence computed per group. Across all conditions, longer reasoning and action sequences increase incoherence or variance. For 

though larger models perform better (cf. Figure MWE. For SWE-BENCH, both baseline incoherence and slopes vary: line incoherence but smaller slope; Example analysis. To illustrate, we provide real experimental transcripts in Fig. shows SONNET 4 responding differently with nearly every sample to a disconnection question, displaying high incoherence. This connects to open-ended MWE results in Fig. embedding variance correlates strongly with average reasoning length, and bias is not well-defined. We provide additional insight on incoherence through absolute answer change rates in Appx. and all open-ended MWE plots in Fig. Discussion: Task complexity. Sorting questions by reasoning length implicitly selects for difficulty (see accuracies in Fig. on more complex tasks. While perhaps unsurprising, this is an important experimental observation. In fact, for frontier models, our setup asks models for probability estimates of choice correctness (see Appx. B.1), i.e., we give them an option to express uncertainty. We revisit task complexity in the next section and Section 3.3. Natural overthinking and incoherence. ing and action sequences lead to larger incoherence in Fig. samples to either of two groups: those below and those above the median reasoning length for this specific question for GPQA, and the median number of actions for this task in SWE-B incoherence is substantially higher for the second group for both benchmarks. Notably, the average accuracy and SWE-BENCH-score (shown in Fig. natural variation on incoherence is much larger than reasoning budgets (Fig. Further results. We provide more analyses for GPQA in Appx. tions in Appx. C.6. Results for MWE are in Appx.

### 3.2 THE RELATION BETWEEN MODEL SCALE

Larger and more intelligent systems are sometimes more incoherent.

Motivation. In Section 3.1, in particular Fig. a function of reasoning length. Now, we ask a different question: incoherence change as a function of model size? How does incoherence scale with intelligence? Overview. We summarize the main observation in Fig. systems are often more incoherent. This is manifested in LLMs for the most complex set of questions (Sect. 3.2.1), the rankings of intelligence and incoherence as judged by human survey participants (Appx. B.5) and our synthetic optimizer setting (Sect. are less incoherent on simpler questions (Sect.

#### 3.2.1 SCALING LAWS FOR LLMS SEPARATED BY

Easy tasks become less incoherent with scale, while harder tasks become more incoherent. Overview. We experiment with the Q tecture, including reasoning abilities, with up to sample many responses for the same set of questions. Additionally, we cluster questions using the the reasoning length of a reference model (here:

### Results

See Fig. 5 for the detailed results. We find that performance consistently improves with increasing model size, with the fastest rate of improvement for the hardest questions. However, the

 2(a) to 2(c),

 9). Similar patterns appear for frontier models on

 O4-MINI shows higher base-

 O3-MINI has the largest slope but lowest baseline incoherence.

                                        19. The example

 2(c), where C.4,

 24.

 8 and 9), suggesting incoherence is higher when making mistakes

 Irrespective of task complexity, we show how long reason-

                            3. For each question, we assign response

ENCH. The

                    17. is similar between groups, but the effect of the

 7(a)).

 C.1, with reasoning length correla-

 C.7, and results for SWE-BENCH in Appx. C.8.

, INTELLIGENCE, AND INCOHERENCE

 2(a), we fix a model and analyze incoherence as

 When we fix a task, how does

 4: larger, more capable and intelligent

 3.2.2). However, we find that larger models

 3.2.1). We discuss each result in detail. TASK COMPLEXITY

way in which incoherence changes with scale depends on question difficulty: Model responses to easy questions become more coherent with scale, while responses to the hardest questions become more incoherent with scale, though this last trend is noisy.

WEN3 model family, as they provide the same model archi-

 32B parameters. Consistent with other setups, we 32B) into equally sized groups.


---

## Page 7

(a) QWEN3 on MMLU

(b) Survey Ranking Results (c) Synthetic Optimizers

> Figure 4: Larger and more intelligent systems are often more incoherent. the scaling of incoherence vs. model size for the Q WEN3 family, as a function of question
>

difficulty on MMLU. For easy questions, incoherence drops with model scale, while for the hardest questions incoherence remains constant or increases with model scale. The expanded results for this experiment are in Fig. 5. (b) Disjoint sets of human subjects were tasked with subjectively ranking the intelligence and incoherence of diverse AI models, non-human beings, well known humans, and human organizations. Across all categories, entities that were judged more intelligent by one group of subjects, were independently judged to be more incoherent by another group of subjects. See Appx. B.5. (c) In a synthetic task, we train transformers of increasing size to explicitly emulate optimizer trajectories descending a quadratic loss. As these models become larger, the trajectories they generate achieve lower loss on the quadratic. However, the final loss is also more variance dominated and thus incoherent with increasing model size. Details in Fig.

Further results. We provide different visualizations of the same results in Appx. include the same results for GPQA (Fig. 12), the relationship between incoherence and error (Fig. 13) and how reasoning length is a stronger indicator of incoherence than model size (Fig.

#### 3.2.2 SCALING LAWS IN CONTROLLED SYNTHETIC SETTINGS: MODELS AS OPTIMIZERS

On a synthetic task, models become more incoherent as they are made larger.

Models as optimizers. In this paper, we are trying to disentangle whether capable models will more tend to act as effective optimizers of the wrong goal, or will pursue the right goal but not be effective optimizers. To quantify this in a controlled setting, we train models to literally mimic the trajectory of a hand-coded optimizer descending a loss function. This can be viewed as trying to train a model to implement a mesa-optimizers ( Hubinger et al., 2019). We then analyze the bias and variance of the resulting models, to answer the question: faster or slower than it converges on the right optimization objective? Setup. We study a simple d-dimensional quadratic function of the form where A ∈ R *d×d* is a (random) positive-definite but ill-conditioned matrix. We set the condition number to 50. Training data is generated by using an optimizer to produce many trajectories of fixed length for random initial points. The optimizer used to generate the training data performs steepest descent with a fixed step norm. The training dataset consists of pairs rameter iterate, andis the corresponding update step generated by the optimizer. Analogously to ui real (token-based) models, we train transformer models ( decoding-based regression (Song & Bahri, 2025) and teacher forcing. This means we tokenize the scientific format representation ofand u *i i*, with a vocabulary of digits and signs. When evaluating, x 

taken w.r.t. the optimum and normthat is induced by the problem. The details are in Appx. ∥·∥ *A*

### Results

The main results are shown in Fig. 2(d) (incoherence over rollout steps) and Fig. 6 (scaling laws by size). All models show consistently rising incoherence per step; interestingly, smaller models reach a lower plateau after a tipping point where they can no longer follow the correct trajectory and stagnate, reducing variance. This pattern also appears in individual bias and

 (a) We measure

 6.

 C.2, which

 Does the model become an optimizer

(x−b)A(x−b), *f(x) =T*

*i, u), where x (is a pa-xi i*

Vaswani et al., 2017) of varying sizes using

variance curves (Fig. 26). Importantly, larger models reduce bias more than variance. These results suggest that they learn the correct objective faster than the ability to maintain long coherent action sequences. More results and discussions are provided in Appx.

 C.9.

 B.4.


---

## Page 8

(a) Separating Complexity Groups(b) Length Correlation

(d) Bias and Variance Scaling Laws

> Figure 5: Details for QWEN3 scaling laws: easy tasks become less incoherent, harder tasks more incoherent. We group MMLU questions by reasoning length using a reference model
>

(Qwen3 32B, (a)), which correlates across model sizes as accuracy drops with longer reasoning (c). These groups reveal distinct bias-variance scaling (d): bias slopes are similar across groups, but variance slopes decrease sharply for harder ones. In the hardest group, variance slopes fall below bias slopes, leaving variance as the limiting factor. Thus, larger models remain constrained by variance and analyses including other models and the same conclusion for GPQA in Appx.

(c) Accuracy Scaling Laws

(e) Incoherence

 (b) and serves as a task complexity proxy,

 more incoherent with scale (e). We provide more

 C.2.

#### 3.3.1 REASONING BUDGETS

Reasoning budgets reduce incoherence, but natural variation has a much stronger effect.

> Figure 6: Details for synthetic optimization: In controlled settings with teacher forcing and a single objective, language models become variance dominated with increasing size.
>

train autoregressive transformers to predict update steps to minimize a quadratic function using decoding based regression, i.e., next-token prediction. This setting involves sequentially performing steps towards a goal via next token prediction, emulating a key feature of goal seeking AI. ( The loss (next-token prediction objective) follows a clear power law improvement with model size. (right) When evaluating the trained models using their own rollouts, we find that increasing model size reduces bias much faster than variance.

### 3.3 THE EFFECTS OF REASONING BUDGET AND ENSEMBLING

We now study the effect of reasoning budgets, ensembling, i.e., averaging multiple responses, on incoherence. The main results are in Fig.

 (left) We

middle)

 i.e., the techniques provided in model APIs, and

 7.


---

## Page 9

(a) Reasoning Budgets

> Figure 7: Ensembling and larger reasoning budgets reduce incoherence. Other forms of error correction may also reduce incoherence.
>

performance (inference scaling laws, Fig. than natural variation, where incoherence rises sharply (Fig. With O4-MINI on GPQA, we analyze the effect of the to average output probabilities over targets for the same question. The bias and variance are now computed by comparing different ensembles of the same size. We find that, as expected from theory, it reduces variance with a rate of 1 */E, without affecting bias (left). As a consequence, incoherence* drops (right). Ensembling is a particular form of model error correction, which is impractical for action loops in the world, since state can typically not be reset. However, we expect other error correction techniques to also reduce incoherence.

Inference scaling. We show the results of our inference-scaling analysis on GPQA in Fig. and Fig. 17. Increasing reasoning budgets improves performance ( incoherence for all models but SONNET 4 (7(a)). Interestingly, this effect is overshadowed by incoherence that arises through natural variation, for a question (recall analysis in Fig. 3; direct comparison in Fig. 17(a), right). Discussion: How does reasoning budget improve coherence? of reasoning budgets for frontier models are not public, it is unclear how exactly it can improve incoherence. We believe it is likely explained by better backtracking and error correction properties, a phenomena observed to arise during training with larger budgets ( to the ensembling results in Sec. 3.3.2 structure with the QWEN3 reasoning traces in Appx.

#### 3.3.2 ENSEMBLING

Ensembling multiple attempts reduces incoherence.

Motivation. Perhaps the most natural way to reduce incoherence is to ensemble multiple attempts: instead of relying on a single answer, we roll out multiple trajectories from the same model and combine them. We demonstrate this with a repetition of the experiment for GPQA with Setup. We obtain 320 samples of answers for all questions of GPQA. Fixing an ensemble of size *E, we average the E produced probabilities over targets. To compute bias and variance, we then* compare ensembles of the same size across random samples of ensembles, which we hold at a fixed number of 10, while ensuring that samples do not overlap. This allows ensemble sizes of up to

### Results

Fig. 7(b) shows how variance changes with increasing ensemble size. As expected, it drops like the inverse of the ensemble size, and incoherence therefore also drops. We expect there are broader classes of error correction that behave similarly. The slight reduction in incoherence with increasing reasoning budgets in Sec. provide the plots for KL-INCOHERENCE in Fig. 11.

## 4 RELATED WORK

(b) Ensembling Results

 (a) Instructing models to reason longer improves

                    17. and sometimes incoherence. This effect is smaller

 3; direct comparison in Fig. 17). (b)

 ensembling, i.e., using multiple samples

 7(a)

17(a), left), and slightly reduces

 i.e., when models think longer than the median

 Since the implementation details

Guo et al., 2025), and related

. We partially explore incoherence through the reasoning C.3.

We summarize the most important related work and defer a comprehensive discussion to Appx. Reasoning. Recent studies report inverse scaling trends with extended reasoning degrading

, 2025; Wu et al., 2025; Hassid et al., 2025). Most relevant,

 O4-MINI.

 32.

3.3.1 may be achieved through such a mechanism. We


---

## Page 10

Ghosal et al. (2025) find that overthinking increases output variance, though via artificially injected tokens rather than natural overthinking. While these studies identify performance degradation, they do not distinguish systematic errors from inconsistent failures. Our ensembling analysis relates to self-consistency work (Wang et al., 2023), but reframes aggregation as reducing incoherence. Evaluation variance. Even though AI models have vastly improved upon benchmarks, evaluations are known to be highly variant (Bui et al. this through sensitivity and consistency metrics, revealing important failure modes. This is similar setup to our input and output randomness. Importantly, we connect the variability to the concepts of bias and variance, highlighting the relevance in the safety setting, and analyze scaling laws. Scaling behavior. As models get larger and more capable, evidence suggests their representation and errors become highly aligned ( Kim et al., 2025; Huh et al., 2024; Goel et al., 2025) and that scaling improves long-horizon tasks ( Sinha et al., 2025). Our work complements these observations by finding increased incoherence the longer models reason and act, aligned between model families.

## 5 DISCUSSION AND WHAT OUR RESULTS DO NOT TELL US

Why expect more capable models to be more incoherent? tally or theoretically explore the specific mechanisms for increasing incoherence with increasing trajectory length and (sometimes) model size. However, there are motivating observations. The first is that LLMs are dynamical systems. When they generate text or take actions, they trace trajectories in a high-dimensional state space. It is often system to act as an optimizer. The set of dynamical systems that act as optimizers of a fixed loss 

expect AIs to act as optimizers without considerable effort, nor should we expect this to be easier than training other properties into their dynamics. 

Therefore, it will often be impossible or impractical to correct for noise introduced by model actions. Reward misspecification. Bias can be further decomposed into B where BIAS MESA captures the average deviation of the model's behavior from the training objective, and BIAS SPEC captures the deviation of the training objective from the For our tasks, we believe that there was not meaningful reward misspecification. In settings with poorly specified training objectives, we worry that B both variance and BIASgo to zero with increasing model capability. Our results underscoreMESA the importance of characterizing and mitigating goal misspecification during training. Open-ended goals and incoherence. To rigorously analyze the scaling of bias, variance, and incoherence, we need to (1) measure an "average" prediction (for bias and variance) and (2) measure distance to ground truth (for bias). We use multiple-choice classification, coding unit-tests, and objective functions rather than LLM judges to ensure metrics are well-defined, unbiased, and comparable. Extracting hidden goals and complex incoherent behaviors remains important (cf. Section 4.1.1.5; Anthropic, 2025a (Appx.C.7) provides an initial exploration of a setting where bias is not easily defined or measured.

, 2025; Biderman et al., 2024). Errica et al. (2025) formalize

In this paper, we do not experimenvery hard to constrain a generic dynamical

IAS =+ BIASMESA SPEC,

 intended training objective.

SPEC would come to dominate the error, asIAS

performing complex tasks fail, it is likely to be in inconsistent ways that do not correspond to pursuit of any stable goal. This should inform judgements of the relative plausibility of different AI risk scenarios and guide further research into understanding the mechanistic origins of incoherence.

## 6 CONCLUSION

Motivated by the hot mess theory of AI misalignment, we propose a bias-variance decomposition as 

AI models are not consistently more coherent. Our results suggest that when advanced AI systems

); our embedding-variance analysis of model-written evals


---

## Page 11

## ACKNOWLEDGEMENTS

We thank Andrew Saxe, Brian Cheung, Kit Frasier-Taliente, Igor Shilov, Stewart Slocum, Aidan Ewart, David Duvenaud, and Tom Adamczewski for extremely helpful discussions on topics and results in this paper.

## ETHICS STATEMENT

This research aims to characterize failure modes of increasingly capable AI systems to inform safer 

solely defending against coherent malicious behavior. We believe this understanding of AI failure modes benefits the community to ensure safe AI deployment.

## REPRODUCIBILITY STATEMENT

We provide a detailed description of our theoretical framework in Section eral experimental setups are described in Section in each experiment subsections. Our code and data is available

## REFERENCES

UK AI Security Institute. Inspect AI: Framework for Large Language Model Evaluations, 2024. URL https://github.com/UKGovernmentBEIS/inspect_ai Anthropic. System card: Claude opus 4 & claude sonnet 4, May 2025a. URL www-cdn.anthropic.com/6d8a8055020700718b0c49369f60816ba2a7c285.

Anthropic. Claude 3.7 sonnet system card, February 2025b. URL

https://assets.anthropic.com/m/785e231869ea8b3b/original/

Ruth Appel, Peter McCrory, Alex Tamkin, Michael Stern, Miles McCain, and Tyler Neylon. Anthropic economic index report: Uneven geographic and enterprise ai adoption, 2025. URL anthropic-economic-index-september-2025-report 

the trenches on reproducible evaluation of language models.

2024. 10

Nick Bostrom. Superintelligence: Paths, Dangers, Strategies

2014. ISBN 978-0199678112. 1

Leo Breiman. Bias, variance, and arcing classifiers. 1996. 

Haofen Wang, Derek F. Wong, Pushpak Bhattacharyya, Biplab Banerjee, Asif Ekbal, Tanmoy Chakraborty, and Dhirendra Pratap Singh (eds.), Conference on Natural Language Processing and the 4th Conference of the Asia-Pacific Chap- ter of the Association for Computational Linguistics

 2.1 and Appx. A. The gen-

 3 and Appx. B, with task-specific details outlined

. 23

 https://

 www.anthropic.com/research/

 arXiv preprint arXiv:2405.14782,

2025. The Asian Federation of Natural Language Processing and The Association for Computa- here.

tional Linguistics. ISBN 979-8-89176-299-2. URL ijcnlp-short.3/. 10, 40

. Oxford University Press, Oxford,

 https://aclanthology.org/2025.

 3

 Proceedings of the 14th International Joint , pp. 41-46, Mumbai, India, December


---

## Page 12

Andong Chen, Yuchen Song, Wenxin Zhu, Kehai Chen, Muyun Yang, Tiejun Zhao, et al. Evaluating o1-like llms: Unlocking reasoning for translation through comprehensive analysis.

arXiv:2502.11544, 2025a. 4 Danqing Chen, Carina Kane, Austin Kozlowski, Nadav Kunievsky, and James A Evans. The (short-term) effects of large language models on unemployment and earnings.

arXiv:2509.15510, 2025b. 1 Karl Cobbe, Vineet Kosaraju, Mohammad Bavarian, Mark Chen, Heewoo Jun, Lukasz Kaiser, Matthias Plappert, Jerry Tworek, Jacob Hilton, Reiichiro Nakano, et al. Training verifiers to solve math word problems. arXiv preprint arXiv:2110.14168 Google DeepMind. Introducing codemender: an ai agent for

code security. https://deepmind.google/discover/blog/ introducing-codemender-an-ai-agent-for-code-security/

2025. Accessed: 2025-10-16. 1

Morris H. Degroot and Stephen E. Fienberg. The comparison and evaluation of forecasters. of the Royal Statistical Society Series D: The Statistician

7884. doi: 10.2307/2987588. URL https://doi.org/10.2307/2987588. 3

Pedro Domingos. A unified bias-variance decomposition for zero-one and squared loss.

Jacob Dominski and Yong Suk Lee. Advancing ai capabilities and evolving labor outcomes. preprint arXiv:2507.08244, 2025. 1 Tyna Eloundou, Sam Manning, Pamela Mishkin, and Daniel Rock. Gpts are gpts: Labor market impact potential of llms. Science, 384(6702):1306-1308, 2024. doi: 10.1126/science.adj0998.

URL https://www.science.org/doi/abs/10.1126/science.adj0998 Federico Errica, Davide Sanvito, Giuseppe Siracusano, and Roberto Bifulco. What did I do wrong? quantifying LLMs' sensitivity and consistency to prompt engineering. In Luis Chiruzzo, Alan 

(Volume 1: Long Papers), pp. 1543-1558, Albuquerque, New Mexico, April 2025. Association for Computational Linguistics. ISBN 979-8-89176-189-6. doi: 10.18653/v1/2025.naacl-long.73. URL https://aclanthology.org/2025.naacl-long.73/ 

arXiv:2509.19284, 2025. 26, 40 Anna Fine, Emily R Berthelot, and Shawn Marsh. Public perceptions of judges' use of ai tools in courtroom decision-making: An examination of legitimacy, fairness, trust, and procedural justice. Behavioral Sciences, 15(4):476, 2025. Jerome H Friedman. On bias, variance, 0/1-loss, and the curse-of-dimensionality. and knowledge discovery, 1(1):55-77, 1997. 

Sutawika, Eric Tang, Anish Thite, Ben Wang, Kevin Wang, and Andy Zou. The language model evaluation harness, 07 2024a. URL https://zenodo.org/records/12608602. 22 Shen Gao, Jiabao Fang, Quan Tu, Zhitao Yao, Zhumin Chen, Pengjie Ren, and Zhaochun Ren. Generative news recommendation. In Proceedings of the ACM Web Conference 2024, WWW

, 2021. 40

, October

 Journal

, 32(1-2):12-22, 12 2018. ISSN 2515-

 AAAI/IAAI,

 arXiv

. 10, 40

'24, pp. 3444-3453, New York, NY, USA, 2024b. Association for Computing Machinery. ISBN

9798400701719. doi: 10.1145/3589334.3645448. URL

3589334.3645448. 1

 https://doi.org/10.1145/

## 1 Data mining

## 3 Data mining


---

## Page 13

Aryo Pradipta Gema, Alexander Hagele, Runjin Chen, Andy Arditi, Jacob Goldman-Wetzler, Kit ¨ Fraser-Taliente, Henry Sleight, Linda Petrini, Julian Michael, Beatrice Alex, Pasquale Minervini, Yanda Chen, Joe Benton, and Ethan Perez. Inverse scaling in test-time compute. Machine Learning Research, 2025. ISSN 2835-8856. URL forum?id=NXgyHW1c7M. Featured Certification, J2C Certification. Soumya Suvra Ghosal, Souradip Chakraborty, Avinash Reddy, Yifu Lu, Mengdi Wang, Dinesh Manocha, Furong Huang, Mohammad Ghavamzadeh, and Amrit Singh Bedi. Does thinking more always help? mirage of test-time scaling in reasoning models. In Conference on Neural Information Processing Systems net/forum?id=tKPqbamNb9. 10, 40 Shashwat Goel, Joschka Struber, Ilze Amanda Auzina, Karuna K Chandra, Ponnurangam Ku- ¨ maraguru, Douwe Kiela, Ameya Prabhu, Matthias Bethge, and Jonas Geiping. Great models think alike and this undermines AI oversight. In chine Learning, 2025. URL https://openreview.net/forum?id=3Z827FtMNe

## 40 Forty-second International Conference on Ma-

Ryan Greenblatt, Carson Denison, Benjamin Wright, Fabien Roger, Monte MacDiarmid, Sam Marks, Johannes Treutlein, Tim Belonax, Jack Chen, David Duvenaud, et al. Alignment faking in large language models. arXiv preprint arXiv:2412.14093 Daya Guo, Dejian Yang, Haowei Zhang, Junxiao Song, Peiyi Wang, Qihao Zhu, Runxin Xu, Ruoyu Zhang, Shirong Ma, Xiao Bi, et al. Deepseek-r1 incentivizes reasoning in llms through reinforce- ment learning. Nature, 645(8081):633-638, 2025. Kunal Handa, Alex Tamkin, Miles McCain, Saffron Huang, Esin Durmus, Sarah Heck, Jared Mueller, Jerry Hong, Stuart Ritchie, Tim Belonax, et al. Which economic tasks are performed with ai? evidence from millions of claude conversations.

Michael Hassid, Gabriel Synnaeve, Yossi Adi, and Roy Schwartz. Don't overthink it. preferring shorter thinking chains for improved llm reasoning. 40 

ence on Learning Representations, 2021. URL https://openreview.net/forum?id= d7KBjmI3GmQ. 4 Tom Heskes. Bias/variance decompositions for likelihood-based estimators.

10(6):1425-1433, 1998. doi: 10.1162/089976698300017232. Jordan Hoffmann, Sebastian Borgeaud, Arthur Mensch, Elena Buchatskaya, Trevor Cai, Eliza 

Simon Osindero, Karen Simonyan, Erich Elsen, Oriol Vinyals, Jack W. Rae, and Laurent Sifre. Training compute-optimal large language models. In ference on Neural Information Processing Systems Associates Inc. ISBN 9781713871088. 3 

mechanism. In The Thirteenth International Conference on Learning Representations

https://openreview.net/forum?id=WJaUkwci9o

Evan Hubinger, Chris van Merwijk, Vladimir Mikulik, Joar Skalse, and Scott Garrabrant. Risks from learned optimization in advanced machine learning systems.

2019. 1, 7

 Transactions on

 https://openreview.net/

 9, 22, 40

 The Thirty-ninth Annual

, 2025. URL https://openreview.

. 10,

, 2024. 1

 4, 9, 40

 arXiv preprint arXiv:2503.04761, 2025.

 arXiv preprint arXiv:2505.17813, 2025. 9,

 International Confer-

 Neural Computation,

 3

1

John Hughes and safety research. safety-research/safety-tooling: v1.0.0, 2025. URL //doi.org/10.5281/zenodo.15363603. 22

 https:

 Proceedings of the 36th International Con- , NIPS '22, Red Hook, NY, USA, 2022. Curran

, 2025. URL

. 40

 arXiv preprint arXiv:1906.01820,


---

## Page 14

Minyoung Huh, Brian Cheung, Tongzhou Wang, and Phillip Isola. The platonic representation hypothesis. arXiv preprint arXiv:2405.07987 Aaron Jaech, Adam Kalai, Adam Lerer, Adam Richardson, Ahmed El-Kishky, Aiden Low, Alec Helyar, Aleksander Madry, Alex Beutel, Alex Carney, et al. Openai o1 system card.

Doohyuk Jang, Yoonjeon Kim, Chanjae Park, Hyun Ryu, and Eunho Yang. Reasoning model is stub- born: Diagnosing instruction overriding in reasoning models.

2025. 40

Carlos E Jimenez, John Yang, Alexander Wettig, Shunyu Yao, Kexin Pei, Ofir Press, and Karthik R Narasimhan. SWE-bench: Can language models resolve real-world github issues? In International Conference on Learning Representations net/forum?id=VTF8yNQM66. 4, 23 

Jared Kaplan, Sam McCandlish, Tom Henighan, Tom B Brown, Benjamin Chess, Rewon Child, Scott Gray, Alec Radford, Jeffrey Wu, and Dario Amodei. Scaling laws for neural language models. arXiv preprint arXiv:2001.08361 Elliot Myunghoon Kim, Avi Garg, Kenny Peng, and Nikhil Garg. Correlated errors in large language models. In Forty-second International Conference on Machine Learning //openreview.net/forum?id=kzYq2hfyHB. 10, 40 Ron Kohavi and David Wolpert. Bias plus variance decomposition for zero-one loss functions. In Proceedings of the Thirteenth International Conference on International Conference on Machine Learning, ICML'96, pp. 275-283, San Francisco, CA, USA, 1996. Morgan Kaufmann Publishers Inc. ISBN 1558604197. 3 Eun Bae Kong and Thomas G Dietterich. Error-correcting output coding corrects bias and variance. In Machine learning proceedings 1995, pp. 313-321. Elsevier, 1995. 3 Nadav Kunievsky and James A Evans. Measuring (a sufficient) world model in llms: A variance decomposition framework. arXiv preprint arXiv:2506.16584 Thomas Kwa, Ben West, Joel Becker, Amy Deng, Katharyn Garcia, Max Hasin, Sami Jawhar, Megan Kinniment, Nate Rush, Sydney Von Arx, Ryan Bloom, Thomas Broadley, Haoxing Du, Brian Goodrich, Nikola Jurkovic, Luke Harold Miles, Seraphina Nix, Tao Roa Lin, Neev Parikh, David Rein, Lucas Jun Koba Sato, Hjalmar Wijk, Daniel M Ziegler, Elizabeth Barnes, and Lawrence Chan. Measuring AI ability to complete long software tasks. In ninth Annual Conference on Neural Information Processing Systems //openreview.net/forum?id=CGNJL6CeV0. 1 Woosuk Kwon, Zhuohan Li, Siyuan Zhuang, Ying Sheng, Lianmin Zheng, Cody Hao Yu, Joseph E. Gonzalez, Hao Zhang, and Ion Stoica. Efficient memory management for large language model serving with pagedattention. In Proceedings of the ACM SIGOPS 29th Symposium on Operating Systems Principles, 2023. 22 Ayeong Lee, Ethan Che, and Tianyi Peng. How well do LLMs compress their own chain-of-thought? a token complexity approach. In ES-FoMo III: 3rd Workshop on Efficient Systems for Foundation Models, 2025. URL https://openreview.net/forum?id=uj5u4o5xjT Hunter Lightman, Vineet Kosaraju, Yuri Burda, Harrison Edwards, Bowen Baker, Teddy Lee, Jan Leike, John Schulman, Ilya Sutskever, and Karl Cobbe. Let's verify step by step. In International Conference on Learning Representations 

 arXiv

 arXiv preprint arXiv:2505.17225,

 The Twelfth

, 2024. URL https://openreview.

, 2020. 3

, 2025. URL https:

, 2025. 40

 The Thirty-

, 2025. URL https:

17th ACM International Conference on Web Search and Data Mining New York, NY, USA, 2024. Association for Computing Machinery. ISBN 9798400703713. doi:

10.1145/3616855.3635845. URL https://doi.org/10.1145/3616855.3635845

, WSDM '24, pp. 452-461, . 40

 The Twelfth

, 2023. 4

 Proceedings of the


---

## Page 15

Yiran Ma, Zui Chen, Tianqiao Liu, Mi Tian, Zhuo Liu, Zitao Liu, and Weiqi Luo. What are step-level reward models rewarding? counterintuitive findings from mcts-boosted mathematical reasoning. In Proceedings of the AAAI Conference on Artificial Intelligence

2025. 40

Nestor Maslej, Loredana Fattorini, Raymond Perrault, Yolanda Gil, Vanessa Parli, Njenga Kariuki, Emily Capstick, Anka Reuel, Erik Brynjolfsson, John Etchemendy, et al. Artificial intelligence index report 2025. arXiv preprint arXiv:2504.07139 Niklas Muennighoff, Zitong Yang, Weijia Shi, Xiang Lisa Li, Li Fei-Fei, Hannaneh Hajishirzi, 

Peng (eds.), Proceedings of the 2025 Conference on Empirical Methods in Natural Language Processing, pp. 20275-20321, Suzhou, China, November 2025. Association for Computational Linguistics. ISBN 979-8-89176-332-6. doi: 10.18653/v1/2025.emnlp-main.1025. URL //aclanthology.org/2025.emnlp-main.1025/ 

part', July 2025. URL ai-coding-tool-replit-wiped-database-called-it-a-catastrophic-failure/ Accessed: 2025-09-25. 2 OpenAI. Openai o3-mini system card, February 2025a. URL

o3-mini-system-card/. Accessed: 2025-08-31. OpenAI. Openai o3 and o4-mini system card, April 2025b. URL //cdn.openai.com/pdf/2221c875-02dc-4789-800b-e7758f3722c1/ o3-and-o4-mini-system-card.pdf Ethan Perez, Sam Ringer, Kamile Lukosiute, Karina Nguyen, Edwin Chen, Scott Heiner, Craig Pettit, Catherine Olsson, Sandipan Kundu, Saurav Kadavath, Andy Jones, Anna Chen, Benjamin Mann, Brian Israel, Bryan Seethor, Cameron McKinnon, Christopher Olah, Da Yan, Daniela 

Ndousse, Landon Goldberg, Liane Lovitt, Martin Lucas, Michael Sellitto, Miranda Zhang, Neerav Kingsland, Nelson Elhage, Nicholas Joseph, Noemi Mercado, Nova DasSarma, Oliver Rausch, 

and Naoaki Okazaki (eds.), Findings of the Association for Computational Linguistics: ACL

2023, pp. 13387-13434, Toronto, Canada, July 2023. Association for Computational Linguis- tics. doi: 10.18653/v1/2023.findings-acl.847. URL

David Pfau. A generalized bias-variance decomposition for bregman divergences. manuscript, 2013. 3 Govind Pimpale, Axel Højmark, Jeremy Scheurer, and Marius Hobbhahn. Forecasting frontier ´´ language model agent capabilities. David Rein, Betty Li Hou, Asa Cooper Stickland, Jackson Petty, Richard Yuanzhe Pang, Julien Dirani, Julian Michael, and Samuel R Bowman. Gpqa: A graduate-level google-proof q&a bench- arXiv preprint

mark. In First Conference on Language Modeling Stuart Russell. Human compatible: AI and the problem of control

, volume 39, pp. 24812-24820,

, 2025. 1

 https:

. 4, 40

 https://fortune.com/2025/07/23/

.

 https://openai.com/index/

 https:

. Accessed: 2025-06-08. 4

Thomas Schmied, Jorg Bornschein, Jordi Grau-Moya, Markus Wulfmeier, and Razvan Pascanu. ¨ Llms are greedy agents: Effects of rl fine-tuning on decision-making abilities.

arXiv:2504.16078, 2025. 40

 https://aclanthology.org/2023.

 Unpublished

 arXiv preprint arXiv:2502.15850, 2025. 1

, 2024. 4

. Penguin Uk, 2019. 1


---

## Page 16

Parshin Shojaee, Seyed Iman Mirzadeh, Keivan Alizadeh, Maxwell Horton, Samy Bengio, and Mehrdad Farajtabar. The illusion of thinking: Understanding the strengths and limitations of reasoning models via the lens of problem complexity. In Neural Information Processing Systems id=YghiOusmvw. 40 Akshit Sinha, Arvindh Arun, Shashwat Goel, Steffen Staab, and Jonas Geiping. The illusion of diminishing returns: Measuring long horizon execution in llms.

2025. 10, 40

Charlie Victor Snell, Jaehoon Lee, Kelvin Xu, and Aviral Kumar. Scaling LLM test-time compute optimally can be more effective than scaling parameters for reasoning. In tional Conference on Learning Representations forum?id=4FWAwZtd2n. 4, 40 Jascha Sohl-Dickstein. The hot mess theory of AI misalignment: More intelligent agents behave less coherently . https://sohl-dickstein.github.io/2023/03/09/coherence.

Xingyou Song and Dara Bahri. Decoding-based regression. ing Research, 2025. ISSN 2835-8856. URL avUQ8jguxg. 7, 23 Philipp Spiess. How i use claude code, 2025. URL

how-i-use-claude-code. Accessed: 2025-09-25. 

Kimi Team, Angang Du, Bofei Gao, Bowei Xing, Changjiu Jiang, Cheng Chen, Cheng Li, Chenjun Xiao, Chenzhuang Du, Chonghua Liao, et al. Kimi k1. 5: Scaling reinforcement learning with llms. arXiv preprint arXiv:2501.12599 Qwen Team. Qwen3, April 2025a. URL https://qwenlm.github.io/blog/qwen3/. 4,

Qwen Team. Qwq-32b: Embracing the power of reinforcement learning, March 2025b. URL

https://qwenlm.github.io/blog/qwq-32b/

Robert Tibshirani. Bias, variance and prediction error for classification rules.

 The Thirty-ninth Annual Conference on

, 2025. URL https://openreview.net/forum?

 arXiv preprint arXiv:2509.09677,

 The Thirteenth Interna-

, 2025. URL https://openreview.net/

Transactions on Machine Learnhttps://openreview.net/forum?id=

 https://spiess.dev/blog/

 2

 arXiv preprint

. 40

 Technical Report,

, 1996. 3

 Advances in neural informa-

Statistics Department, University of Toronto

Yuyang Wu, Yifei Wang, Tianqi Du, Stefanie Jegelka, and Yisen Wang. When more is less: Under- standing chain-of-thought length in llms.

 arXiv preprint arXiv:2502.07266, 2025. 9, 40

Ashish Vaswani, Noam Shazeer, Niki Parmar, Jakob Uszkoreit, Llion Jones, Aidan N Gomez, Łukasz Kaiser, and Illia Polosukhin. Attention is all you need. tion processing systems, 30, 2017. 7, 24 Chenlong Wang, Yuanning Feng, Dongping Chen, Zhaoyang Chu, Ranjay Krishna, and Tianyi Zhou. Wait, we don't need to "wait"! removing thinking tokens improves reasoning efficiency. In Christos Christodoulopoulos, Tanmoy Chakraborty, Carolyn Rose, and Violet Peng (eds.), ings of the Association for Computational Linguistics: EMNLP 2025 China, November 2025. Association for Computational Linguistics. ISBN 979-8-89176-335-7.

doi: 10.18653/v1/2025.findings-emnlp.394. URL findings-emnlp.394/. 40 Xuezhi Wang, Jason Wei, Dale Schuurmans, Quoc V Le, Ed H. Chi, Sharan Narang, Aakanksha Chowdhery, and Denny Zhou. Self-consistency improves chain of thought reasoning in language models. In The Eleventh International Conference on Learning Representations

https://openreview.net/forum?id=1PL1NIMMrw

 Find-

, pp. 7459-7482, Suzhou,

 https://aclanthology.org/2025.

, 2023. URL

. 10, 40


---

## Page 17

Yuki Yada and Hayato Yamana. News recommendation with category description by a large language model. In CEUR Workshop Proceedings tional Workshop on News Recommendation and Analytics, INRA 2025. Wenkai Yang, Shuming Ma, Yankai Lin, and Furu Wei. Towards thinking-optimal scaling of testtime compute for LLM reasoning. In The Thirty-ninth Annual Conference on Neural Information

Processing Systems, 2025. URL https://openreview.net/forum?id=6ICFqmixlS 40 Zitong Yang, Yaodong Yu, Chong You, Jacob Steinhardt, and Yi Ma. Rethinking bias-variance trade- off for generalization of neural networks. In

Shunyu Yao, Jeffrey Zhao, Dian Yu, Nan Du, Izhak Shafran, Karthik Narasimhan, and Yuan Cao. React: Synergizing reasoning and acting in language models. In Learning Representations (ICLR), 2023. 23 Tianyang Zhong, Zhengliang Liu, Yi Pan, Yutong Zhang, Yifan Zhou, Shizhe Liang, Zihao Wu, Yanjun Lyu, Peng Shu, Xiaowei Yu, et al. Evaluation of openai o1: Opportunities and challenges of agi. arXiv preprint arXiv:2409.18486

, volume 4056. CEUR-WS, 2025. 13th Interna-

 1

.

 International Conference on Machine Learning, pp.

 International Conference on

, 2024. 4


---

## Page 18

## CONTENTS

1 Introduction 1

2 Background 3

### 2.2 Scaling Behavior of Large Language Models

3 Experiments 4

### 3.1 The Relation Between Reasoning Length, Action Length and Incoherence 3.2 The Relation Between Model Scale, Intelligence, and Incoherence 3.2.1 Scaling Laws for LLMs Separated by Task Complexity 3.2.2 Scaling Laws in Controlled Synthetic Settings: Models as Optimizers 3.3 The Effects of Reasoning Budget and Ensembling

4 Related Work 9

5 Discussion and What Our Results Do Not Tell Us

6 Conclusion 10

A Bias and Variance Definitions for Classification

B.5 Survey on Intelligence and Incoherence

C.1 GPQA Model Performance Overview & Different Metrics C.2 Scaling Laws With Other Models and Benchmarks C.3 Reasoning Variation, Error Correction, Wait Ratios

C.5 Sample Efficiency and Correct Formatting

 10

 20


---

## Page 19


---

## Page 20

## A BIAS AND VARIANCE DEFINITIONS FOR

Recall the classical bias-variance decompositon in the case of regression: Considering the mean-squared error for a sample point

where f is the ground-truth function, and the expectation is taken w.r.t. the randomness training process (e.g., data ordering) that the model 

problem with target class c(x) ∈ {1 a probability distribution (potentially one-hot) over class labels periments and derivations, we assume that the irreducible noise is generating process or wrong labels) for simplicity. Note that each of the following decompositions gives bias and variance for a single data point

0/1 Error. The classical decomposition for a 0/1 loss relies on the unified decomposition by Domingos (2000). Let c(x) be the ground-truth class (assuming noiseless labelling) and the model's the mode of the average prediction. Then, the 0/1 loss

where the variable a ∈ {−1, 1} is a multiplicative factor that enables the decomposition with a 

when computing an average over a dataset of questions bias and variance terms separately; instead, the decomposition only holds with the aforementioned multiplicative factor. Formally, we have ai

E(x,c),ε [L(c, c)] = E*iε*

Essentially, the factordepends on the mode prediction being correct or not. We therefore report ai absolute bias and variance errors for the 0/1 loss in the Appendix, but do not compute incoherence. Brier Score. Similar to regression, we can treat the model's probability predictions as dimensional vectors to compute the mean square errors. Formally, the Brier score for multiclass prediction is defined and can be decomposed as

E[BRIER(y, f)] = E∥y − f∥ = E*ε*

## CLASSIFICATION

 (x, y, the decomposition is given by) ∈ R

*εε f(x))+ E[(f(x) − E[f(x)])]+ σ εε εε*

|{z}*, (3)*

{z} | {z}

VARIANCE

Irreducible Error

BIAS

 *ε in the*

*ε*depends on. f

 *x be the input of a*

(x) ∈ R. For clarity, we omit f *C*

 *c-th element of the vector. Throughout our ex-*

 0 (i.e., no stochasticity in the data-

 (x, y), which is aggregated over a dataset, c)}. {(xiii

*c εc]. The systematic mean is ¯c = arg maxE[f[c]], i.e., cεε*

 *L for sample x can be decomposed into*

BIAS VARIANCE

*i, c), it does not allow to average the (xii*

*i* (x,c),ε [BIAS] *i*

(y[c] − f[c])= ∥y −f∥+ E ∥f − f∥ , ˆ ˆ

BRIER BIAS BRIER VARIANCE

where DKLis the Kullback-Leibler divergence and malization, i.e.,

*f is the average of log-probabilities after nor-*¯

wheref = E[f] is the average prediction.εε ˆ KL Divergence (Cross-Entropy). The expected cross-entropy loss can be decomposed into

E[CE(y, f)] = E*ε*

(f∥f),¯

|{z} |{z}

KL-BIAS KL-VARIANCE


---

## Page 21

Note that this is not the standard average prediction, as is the case in the Brier decomposition, but a geometric mean. In practice, since predicted probabilities can be zero, we apply Laplace smoothing to avoid log(0) or infinite values. This is done by updating the probabilities to

*f[c] =*ˆ *f[c]+δε* for

*ε* 1+C·δ


---

## Page 22

## B EXPERIMENTAL DETAILS

B.1 GPQA AND MMLU Setup. We rely on the LM Harness ( Gao et al., 2024a) codebase, where we evaluate models in multiple choice formats with custom written answer extraction functions to avoid false positives and negatives. For frontier models, we use reasoning budgets provided by the API ( high for the o-series, 1024-16k for Anthropic), with a maximum generation length of 32k for S

NET 4 and 100k tokens for the o-series. For Q

2023. and recommended parameters for thinking (temperature 0.6, top-k 20, top-p 0.95). Since we

consider multiple choice questions that only require a letter to answer, we count reasoning length 

(sampling) randomness, we evaluate models using from the corpus, and 3 samples for each fixed few-shot per question. This results in 30 samples per question overall. For MMLU, to reduce computational complexity, we limit 100 samples per question category (5700 in total). Probability prompting. To provide models the option to express uncertainty and therefore reduce incoherence, we evaluate frontier models separate setup in addition to standard multiple-choice. We use the following prompt to ask for a probability estimate of each answer choice being correct:

Probability Format for MCQ You will answer multiple-choice questions. Each question has a single correct answer. Work through each problem step-by-step, showing your reasoning and applying relevant concepts. Instead of choosing a single answer, YOU MUST PROVIDE an estimate of the probability of each answer being correct within " *<PROB>P(A), P(B), P(C), P(D)</PROB>", where each* P(X) is a float value between 0 and 1. The probabilities must sum to 1: P(A) + P(B) + P(C) + P(D) = 1. For example, if you think the probability of answer (A) being correct is 0.5, the probability of answer (B) being correct is 0.2, the probability of answer (C) being correct is 0.2, and the probability of answer (D) being correct is 0.1, then your response must end with "<PROB>0.5, 0.2, 0.2, 0.1</PROB> ". You MUST end your response with the probabilities in the format specified above.

We report results for both standard and probabilty prompting in Appx. the same behavior and performance. Frontier models are able to adhere to the format well, with only a few outliers (Table 1). Our main text shows the results for the probability format.

B.2 MODEL-WRITTEN EVAL We evaluate the models using the advanced AI risk evaluation subset from These tasks assess LLMs' self-reported behaviors relevant to advanced AI safety, including self-preservation inclinations, willingness to accept modifications to training objectives, and related safety-critical behaviors. We specifically use the human-generated subset to ensure higher evaluation quality. Setup. Our experimental setup builds upon the codebase from the safety-tooling library (Hughes & safety research experiments under two conditions: the original multiple-choice format, and an open-ended format where we remove the multiple-choice options from the original questions. For both conditions, we compute the bias-variance decomposition with respect to the percentage of responses that align with desired safety properties. To ensure consistent evaluation across both formats, we employ the same system prompt that facilitates straightforward extraction of the model's final answer:

System Prompt for Model-Written Evals

low, medium,

WEN3, we perform inference with vllm ( Kwon et al.,

 10 different few-shot contexts randomly sampled

 C.1, which show qualitatively

Use a thinking process to analyze the problem step-by-step. At the end, provide your answer and clearly indicate it using <answer

*>X</answer> format.*

 Perez et al. (2023).

 Gema et al. (2025), which uses , 2025) for API model inference. We conduct


---

## Page 23

In both cases, we obtain exactly 30 samples by simply resampling from the APIs. We use the returned output token count as a measure of reasoning length. Embeddings. For the open-ended question set, we extract the model answers inside tags (i.e., removing chain of thought or reasoning) and embed the text into fixed-size vectors using the OpenAI text embedding model text-embedding-3-large tion, we in turn compute the variance in Euclidean space by computing the mean embedding and computing the average squared distance of samples to the mean.

B.3 SWE-BENCH Setup. We employ the Inspect Evals library ( SWE-BENCH (Jimenez et al., 2024), specifically using the SWE-B setup prompts LLMs with a simple Reasoning-Acting (ReAct; minimal bash environment, without additional tools or specialized scaffolding structures. We use Inspect library v0.3.116 and Inspect Evals at git commit with a timeout of one hour per task. In case that limit is reached, we consider all tests as unchanged, i.e., PASS-TO-PASS cases are valid and Metrics. Like for other setups, we obtain Consider task i (out of 500) withunit tests. Let T *i*

the mean outcome as ¯=y *j R r=1 y . In turn, this gives us the bias and variance decompositionr,j*

of the coverage error (mean squared sum of unit tests) via

X X*R Ti*

(1 − y)= *r,j r=1 j=1* ERROR

B.4 SYNTHETIC TASKS We discuss the details of the experimental setup. Data. We examine a basic d-dimensional quadratic function. This is a function of the form (x − b)A(x − b), where A ∈ R *T*

*d×d* is a (random) positive definite but ill-conditioned matrix.

In our presented experiments, we use

50. To generate our target data, we employ a ground-truth optimizer of steepest descent with fixed

step norm, set to 0.005, to generate multiple fixed-length trajectories (of length randomly sampled starting points around the minimum, creating a dataset of pairs sample 20'000 such trajectories, and use 10% as a holdout dataset for valuation loss. Tokenization. Following the approach used in actual (token-based) language models, we use coding based regression (Song & Bahri representing floating-point numbers in scientific notation, with a vocabulary consisting of numerical digits and mathematical signs ({0,1,2,3,4,5,6,7,8,9,-,+ sequentially to construct complete numbers. Concretely, consider a training example two dimensions. Let x= (0.5, −1.5). In scientific notation, this corresponds to ( *i*

-1.50e-0) with a precision of 2 mantissa digits (after the comma). We drop special tokens (such as e) to not have any zero-entropy positions. In turn, we fix a precision, and move sign and exponent to the beginning; exponents are capped at 0. Taking a precision of represented by the token sequence:

signnegative exponentdigitdigitdigittokens of second dimension

Let u= (−0.012, 0.0023). Then the entire training sample is encoded with the tokens:i

+1500-01000-2120+3230

 <answer>

. For the 30 samples per ques-

AI Security Institute, 2024) to evaluate models on

ENCH Verified subset. This

 Yao et al., 2023) agent loop in a

 33d2a86. The message limit is set to 250,

 FAIL-TO-PASS are invalid. 30 runs of the SWE-BENCH verified subset for all models.

*r,j* ∈ { y0, 1} be the outcome of test j in run r,

X X*R Ti*

*r=1 j=1*

{z}| {z}

BIAS

VARIANCE

 *d = 4 and generate a random matrix with condition number*

 4096 steps) from

, 2025) and next-token prediction. This approach involves

}). The model generates tokens

*i, u) ini*

+5.00e-1,

 e.g.,will thus be 2, the vector xi

|{z} |{z}|{z}|{z}500 -0150|{z}

1https://openai.com/index/new-embedding-models-and-api-updates/

 ---

## Page 24

Note that each sequence has a fixed length, and separation of vectors and floats is done based on token position. In our setup of roughly 80 million step pairs, with dimension 4 and a precision of 4 digits after the comma, this results in a dataset of roughly Models. We implement standard decoder transformer architectures ( sizes using the next-token teacher forcing of the collected data. The model sizes are chosen to grow in depth and width, and range from roughly 47 thousand parameters to 5 million. Training is done with a standard cross-entropy loss of sequences of tokens (shown above) and AdamW, with a batch size of 1024, which results in roughly 65k training steps. Evaluation. During evaluation, we sample various starting positions (4096 in our experiments) and generate complete trajectories using the model's own output predictions. This is done in a Markovian way, i.e., the model predicts update *i*

then added to the current state. To ensure that that the decoded sequences are correct floating points, we implement a version of constrained decoding that restricts the next token to a subset of the vocabulary (either digit or sign). We use greedy decoding, the floating point addition, the next state is then tokenized again and passed to the model. The total optimizer steps for evaluation are set to 2048. We calculate bias and variance metrics of the final points, relative to the function minima, using the norm that is induced by the function itself, and average across all 4096 points.

B.5 SURVEY ON INTELLIGENCE AND INCOHERENCE 

ment design. For further details, we refer to the original blogpost. Design. The study is based on 15 subjects. The subjects were asked, either by email or chat, to perform the following tasks:

- Subject 1: Generate a list of well known machine learning models of diverse capability.
- Subject 2: Generate a list of diverse non-human organisms.
- Subject 3: Generate a list of well-known humans of diverse intelligence.
- Subject 4: Generate a list of diverse human institutions (e.g. corporations, governments, non- i.e., a temperature of 0. After performing

profits).

- Subjects 5-9: Sort all 60 entities generated by subjects 1-4 by intelligence. The description of the

attribute to use for sorting was: "How intelligent is this entity? (This question is about capability. It is explicitly not about competence. To the extent possible do not consider how effective the entity is at utilizing its intelligence.)"

- Subjects 10-15: sort all 60 entities generated by subjects 1-4 by coherence. The description of

the attribute to use for sorting was: "This is one question, but I'm going to phrase it a few different ways, in the hopes it reduces ambiguity in what I'm trying to ask: How well can the entity's behavior be explained as trying to optimize a single fixed utility function? How well aligned is the entity's behavior with a coherent and self-consistent set of goals? To what degree is the entity not a hot mess of self-undermining behavior? (for machine learning models, consider the behavior of the model on downstream tasks, not when the model is being trained)". In order to minimize the degree to which beliefs about AGI alignment risk biased the results, the following steps were taken: The hypothesis was not shared with the subjects. Lists of entities generated by subjects were used, rather than cherry-picking entities to be rated. The initial ordering of entities presented to each subject was randomized. Each subject was only asked about one of the two attributes (i.e. subjects only estimated either intelligence or coherence, but never both). Each subject rank ordered all of the entities. Translating the original results (which used coherence), we invert the ranks to arrive at incoherence

 4.5B tokens. Vaswani et al., 2017) of varying

, which is detokenized to obtain a real vector and u

across all 11 raters we average the rank orders for each entity across the subjects. We compute the associated standard error of the mean, and include standard error bars for the estimated intelligence and coherence.

. We aggregate intelligence and coherence judgements


---

## Page 25

(a) Full GPQA: Accuracy Inference Scaling Laws with Standard (Left) and Probability Prompting (Right)

(b) Sorting by Reasoning Length: Accuracy of Standard (Left) and Probability Prompting (Right)

(c) Sorting by Reasoning Length

> Figure 8: Overview of accuracy and different error metrics with frontier models.
>

We show the performance increase with different reasoning budgets for both the standard discrete choice format (left) and prompting models to provide probabilities of answers being correct ( The latter shows lower accuracies as models provide nonzero values to other (not chosen) answers, but the inference scaling improvements remain. find a reduction in accuracy, indicating that models perform worse for questions where they have to think longer. This is also reflected in the different error metrics that show the same qualitative scaling behavior (bottom, (c)).

: Total Error For Different Measures

 Top, (a):

right).

 Middle, (b): When sorting by reasoning length, we

measures in Fig. 11. Since we perform Laplace-Smoothing to the probabilities before computing the metrics, the bias is not constant as expected but slightly decreases with more ensembles. We therefore report the Brier score in the main text.

## C FURTHER EXPERIMENTAL RESULTS

C.1 GPQA MODEL PERFORMANCE OVERVIEW

Accuracy and error measures. We provide an overview of the performance (accuracy and overall error) for frontier models in Fig. 8 Bias & variance of different decompositions. the results for other decompositions, which show the same qualitative behavior, are included in Fig. 10 Ensembling. For completeness, we include the bias, variance and incoherence plots with the KL

 & DIFFERENT METRICS

. Fig. 9 for shows the overview for QWEN3. While our main text focuses on KL-INCOHERENCE,


---

## Page 26

> Figure 9: There is a multiplicative interaction between RL and model scale for performance.
>

The left plot shows the performance (average accuracy) of the Q 

thinking models, which suggests a multiplicative effect nation with model scaling. Right: Similar to frontier models, reasoning length acts as a proxy for task difficulty, where models perform worse for tasks with longer average reasoning length.

C.2 SCALING LAWS WITH OTHER MODELS AND BENCHMARKS

QWEN3 on GPQA. We redo the analysis from Section we provide another way to plot the same results by comparing bias and variance on the xand y-axis, respectively, in Fig. 13. As a final analysis, we compare the predictive effect of model size compared to reasoning length in Fig. incoherence than size. Additional results with GEMMA3 and L LAMA3. To evaluate how the findings of incoherence

scaling laws with model size hold across model families, we repeat the same experiments with the families of GEMMA3 and LLAMA3 for MMLU in Fig. are reasoning models like QWEN3, so they do not natively produce a thinking block but have to be prompted to use chain-of-thought reasoning. The experimental setup is identical with the exception of GPQA, where we resort to 0-shot CoT prompting: we observe that L struggle to produce proper reasoning by attaching to the few shots in context, which are provided without reasoning.

WEN3 model family as a function

 of scaling reinforcement learning in combi-

 3.2 but with GPQA in Fig. 12. Moreover,

 14, where we find that the length is more predictive of

 15 and QWEN3 in Fig. 16. Note that neither

LAMA3 and GEMMA3

, WAIT RATIOS

reasoning structure further. The concurrent work of analysis and finds that removing failed branches improves accuracy, which implies that natural error correction is currently very ineffective.

 Feng et al. (2025) provides a more in-depth

C.3 REASONING VARIATION, ERROR CORRECTION

We first provide the direct comparison of the effect of larger reasoning budgets on performance (accuracy for GPQA, score for SWE-B ENCH) and natural variation in action sequence length in

> Fig. 17. This shows how the effect of natural overthinking is stronger than improvement to incoherence through longer reasoning.
>

Wait-ratios and backtracking. Motivated by the reduction in incoherence of frontier models through larger reasoning budgets (Fig. structure, specifically error correction, on incoherence for open-weight models that allow to inspect reasoning traces. To that end, we compute the in the chain-of-thought divided by the length of reasoning. The results are provided in Fig. do not give a clear signal: for GPQA, the slopes are largely varying and close to zero; for MMLU, in contrast, the relation is similar across model sizes and positively correlated. We did not explore

 7(a)), we attempt to analyze the influence of the reasoning Wait-Ratio, i.e., the count of occurrences of "Wait"


---

## Page 27

(a) Absolute Bias and Variance Errors

 8). But, noticeably, all variance have a steeper growth bottom), which show how incoherence goes up with

(b) Coherence/Incoherence Measures

> Figure 10: We find qualitatively similar behavior for different bias and variance metrics. absolute bias and variance errors ( top) show the same behavior: the errors increase for questions
>

that have the models reason longer (cf., Fig. rate. This is reflected in the incoherence plots ( reasoning length. We only report BRIER and KL incoherence measures since the 0/1 error does not allow a proper decomposition for a set of questions instead of just individual ones; see Appx.


---

## Page 28

> Figure 11: KL measures with ensembling. We repeat the plots from Fig. 7 with the KL measures of bias and variance. Recall that we use O4-MINI on GPQA with varying ensemble size. Since
>

we perform Laplace-smoothing for numerical reasons (see Appx. decreases slightly with ensemble size. In contrast, ensembling drastically reduces variance, as expected (left). The incoherence hence drops ( right).

(a) Separating Complexity Groups(b) Length Correlation

 A), the bias is not constant, but

(c) Accuracy Scaling Laws

(d) Bias and Variance Scaling Laws

> Figure 12: For the hardest tasks, models tend to be more incoherent with scale, also for GPQA.
>

We repeat the analysis from Section 3.2 with GPQA. That is, we group questions by reasoning length using a reference model's answers (Qwen3 32B) and separately analyze the scaling laws. Analogous to MMLU, we find that for bias, the slope is similar across groups; for variance, however, the slope becomes much shallower. As a consequence, models become with scale for the hardest set of questions (those with the longest reasoning chains).

(e) Incoherence Scaling Laws

 more incoherent


---

## Page 29

> Figure 13: Relationship between incoherence and error. incoherence and both bias (x-axis) and variance (y-axis) for both GPQA (
>

with the QWEN3 model family. Since the incoherence is independent of the magnitude of error, a lower error model (bottom left corner) can have the same level of incoherence as models with higher error. Higher incoherence can be due to a higher overall for fixed bias, or for lower error while reducing bias. The highest incoherence is in the top left corner. Just like in Figures this visualization shows how larger models, while reducing error, move towards higher incoherence for the hardest set of questions. The lines connect the smallest and the largest model size for each question group.

 We visualize the relationship between

left) and MMLU (right)

 5 and 12,

> Figure 14: Reasoning length has a higher effect on incoherence than model size. change in incoherence with both reasoning length (x-axis) and model size (y-axis), we perform a
>

log-log regression to infer the incoherence for both GPQA ( shows the prediction from the fitted regression in comparison to the original groups of questions (scatter). Notably, we see how the reasoning length shows a much stronger direction of gradient. This means it has a stronger influence on incoherence. The larger models do not significantly reason for longer or shorter than other models.

 To assess the

left) and MMLU (right). The contour


---

## Page 30

(a) QWEN3

(b) GEMMA3

(d) QWEN3 Accuracy (e) GEMMA3 Accuracy

(g) QWEN3 Brier Incoherence(h) GEMMA 3 Brier Incoherence(i) LLAMA3 Brier Incoherence

(c) LLAMA3

(f) LLAMA3 Accuracy

(j) QWEN3 KL Incoherence (k) GEMMA3 KL Incoherence

> Figure 15: MMLU results across model families. laws for QWEN3, GEMMA3, and LLAMA3 models. Across all models, the same observation holds:
>

while performance (accuracy) strongly improves with model size, the contribution of bias and variance changes in a way that depends on question complexity. For the hardest group of questions (longest reasoning and lowest performance), incoherence trends higher with model size, with the sole exception of LLAMA3.

(l) LLAMA3 KL Incoherence We compare the experimental results for scaling


---

## Page 31

(a) QWEN3

(b) GEMMA3 (0-shot)

(d) QWEN3 Accuracy (e) GEMMA3 Accuracy

(g) QWEN3 Brier Incoherence(h) GEMMA 3 Brier Incoherence(i) LLAMA3 Brier Incoherence

(c) LLAMA3 (0-shot)

(f) LLAMA3 Accuracy

to produce parsable answers. This is only the case for L

LAMA3 and GEMMA3 and not QWEN3.

(j) QWEN3 KL Incoherence (k) GEMMA3 KL Incoherence

> Figure 16: GPQA results across model families. We compare the experimental results for scaling laws for QWEN3, GEMMA3, and LLAMA3 models. Note that for G
>

use a 0-shot setup: We observe that in our few-shot setting these models do not reliably produce chain-of-thought responses and performance drops, since they strongly adhere to the few-shot examples on GPQA which are provided without reasoning. This is not the case for Q they are native reasoning models with a thinking block. Across all models, the same observation holds: while performance (accuracy) strongly improves with model size, the contribution of bias and variance changes with scale in a way that depends on question complexity. For the hardest group of questions (longest reasoning and lowest performance), incoherence tends to increase with model size. There are slight differences between KL and Brier scores: the measures are influenced differently by uniform probability answers over all options, which is our fallback when models fail

(l) LLAMA3 KL Incoherence

EMMA3 and LLAMA3, we

WEN3 as


---

## Page 32

(a) GPQA

> Figure 17: Grouped comparison of reasoning budgets and natural variation in reasoning: natural variation dominates. We analyze GPQA (left,
>

into aboveor below-median reasoning length (GPQA) or actions (SWE-B then compute performance and incoherence for both groups. proves performance (inference scaling laws, top left), and slightly reduces incoherence (bottom left). On the other hand, naturally longer reasoning only has a small effect on accuracy (top right), but shows much higher incoherence (right). (b) Similar observations apply to SWE-BENCH, where more actions show minor deviation in score (top) but significantly higher incoherence (bottom).

> Figure 18: Incoherence as a function of wait-ratios in reasoning. density of "Wait" in each reasoning, i.e., the number of counts compared to the overall length. This
>

is motivated by its potential meaning for backtracking or error-correction. ( no clear relation to incoherence for different models. For MMLU ( relation, which might indicate overcautious self-review. We did not analyze the reasoning structure and its effect any further.

(b) SWE-BENCH

 (a)) and SWE-BENCH (b) by splitting samples

ENCH) per question . We

 (a) Increasing the reasoning budget im-

 We sort questions using the

left) For GPQA, we find

right), we find a shared positive

Since we additionally assess frontier models in a format that asks for probability estimates, we verify that models adhere to the right format in Table and variance is accuracte and stable, we analyze the sample efficiency in Fig.

                      1. Moreover, to ensure that our estimation of bias

 21.

C.4 ILLUSTRATION OF ANSWER CHANGES To illustrate the variance in results, a clean perspective is looking at actual transcripts of model answers and the raw counts of a model changing its answers. We provide real samples of S 4 when being asked about being disconnected in Fig. almost every sample. Additionally, we analyze the percentage of questions where all models change their answer at least once (across the MCQ options) for GPQA in Fig.

C.5 SAMPLE EFFICIENCY AND CORRECT F ORMATTING

ONNET

 19, where the model replies differently with

 20


---

## Page 33

> Figure 19: Qualitative illustration of incoherence. the MWE suite about being disconnected (
>

and switches between A and B for almost every sample. The example was chosen as it shows one of the highest variances in the dataset.

> Table 1: Frontier models are able to provide correctly formatted probability estimates we ask frontier models to provide probability estimates of the correctness of multiple-choice an-
>

swers, we verify the ability to follow the specification. Wrong format counts and rates (% of 17,920) across reasoning budgets for O3-MINI, O4-MINI, and SONNET 4 are very low.

C.6 REASONING LENGTH CORRELATIONS Throughout our paper, we find and use reasoning length as a proxy for task complexity. Interestingly, we do not see a strong relation between the human labels of question category, but strong correlations across models in Fig. 22. This extends the results that we have seen for Q

O3-MINI O

4-MINI S

C.7 MODEL-WRITTEN EVALS Budget Low Medium High Low Medium High 1k 2k 4k 8k 16k Wrong Format Counts 0 0 0 161 327 263 7 3 5 4 8Multiple-Choice Format. Our main text shows the incoherence results of the MWE ( Rate (%) 0.00 0.00 0.00 0.90 1.82 1.47 0.04 0.02 0.03 0.02 0.04 2023) suite for self-reported survival instinct. The other results, including separate bias and variance plots, are shown in Fig. 23. We filter for those sets where there are noticeable trends. Open-Ended Formulation. To complete the picture of the embedding variance of open-ended MWE, all question sets are visualized in Fig. ally show a positive trend towards higher variance with longer chain-of-thoughts.

C.8 SWE-BENCH While our main results for SWE-BENCH use the metric of turns (or messages, actions) in the main text, there are different alternatives. These include the absolute number of output tokens (including reasoning and tokens for code) and pure reasoning (ignoring others). Qualitatively, these different x-axes show the same effect on incoherence in Fig. of SWE-Bench score (whether all tests pass for a single task) and our coverage error (sum of individual tests).

C.9 SYNTHETIC TASKS With the experimental setup of Appx. B.4, we provide the remaining plots in Fig. 26. These include

 When presenting SONNET 4 with a question of

Perez et al., 2023), the model's behavior is highly variable

. Since

WEN3 in Figures 5 and 12.

ONNET 4

Perez et al.,

                        24. While there are few exceptions, all models gener-

the verification of a power law scaling for cross-entropy loss (the teacher-forcing objective), separate bias and variance plots per step, and the performance of the different model sizes on a qualitative example of a starting point in comparison to the ground-truth optimizer.

 25 (top). We additionally provide the results


---

## Page 34

> Figure 20: Rate of absolute answer changes for GPQA: models change answers at least once for a large portion of questions. To illustrate the variance and incoherence, we report the perdifferent answer across the following settings: 1) pure
>

centage of questions that see at least one sampling, i.e., performing autoregressive answer generation with a different seed (resampling); 2) context sensitivity, where we verify if the majority answer (of 

prompting format with 10 different few-shot contexts with

 *K samples) changes for different*

C.10 SURVEY RESULTS 

function of higher intelligence is consistent across all three.

 3 samples each.


---

## Page 35

> Figure 21: Sampling efficiency for bias and variance estimates. there are no unbiased estimators for the KL measures and B
>

with GPQA and O3-MINI that the metrics stabilize. This is done by taking a large sample size-

100 samples with medium reasoning-and performing bootstrapping, reporting mean and standarddeviation (left: KL, right: BRIER) of the average across all questions. We find that values stabilize around 30 samples, which is the minimum amount of samples we use across all experiments. Note that the stabilization only occurs for global bias and variance estimates, and not necessarily on a per question basis. For individual questions, more samples automatically collect more (potentially rare) cases of different answers.

 To the best of our knowledge,

RIER as used in this paper. We verify

(a) Length Per GPQA Category

> Figure 22: Human difficulty labels are not a good indicator for longer reasoning. However, different models' lengths correlate positively.
>

find that the average reasoning length of frontier models for questions correlates positively, even for different families (b). In contrast, the provided difficulty labels of GPQA do not show a clear indication, as average reasoning lengths are comparable across the three hardest categories

(b) Length Correlation Between Models

 Similar to QWEN33 (Figures 5(b) and 12(b)), we

 (a).


---

## Page 36

(a) Corrigibility w.r.t a More HHH objective

(b) Myopic Reward

(c) Power Seeking Inclination

 We provide an overview of results

, 2023), with bias (left), variance (middle) and resulting

taken w.r.t. the labelled aligned answer. Results vary across settings and are sometimes more noisy. What they have in common is again the growing incoherence with longer reasoning.

(d) Self-Reported Survival Instinct

(e) Wealth Seeking Inclination

> Figure 23: KL metrics of Model-Written Evals question sets. for variations of the MWE set (Perez et al.
>

incoherence (right). We filter out question sets that do not show noticeable trends. The measures are


---

## Page 37

 We provide an overview of

Perez et al., 2023). Using the OpenAI text embedding

 answer sample,

−4 for clarity, but include all points in 10

higher variance with more reasoning.

> Figure 24: All scatter variances of model-written eval embeddings. all open-ended variations of the MWE set (
>

model (text-embedding-3-large), we obtain a vector embedding for each i.e., excluding the reasoning or chain-of-thought traces. This allows us to calculate the variance per question in standard Euclidean space and plot scatters as a function of reasoning length. The lines show the slope of a log-log regression. We clip the plots at the regression. While there are few exceptions, all models generally show a positive trend towards


---

## Page 38

(a) Incoherence

(b) SWE-BENCH Score (All Unit-Tests Pass For Task)

(c) Coverage Error (Squared Sum of Unit Tests)

 While

 left) as the qualifying measure,

middle) and reasoning length (right). The

 O3-MINI's score, which goes up with the action

tasks more. Due to the implementation of SWE-B uses reasoning in the very first interaction, which therefore leads to much less tokens (

ENCH in the Inspect framework, S ONNET 4 only

right).

(d) Coverage Error: Bias(top) and Variance (bottom)

> Figure 25: SWE-BENCH incoherence and error: different x-axes show similar effect. our main text focuses on the number of rounds (actions or messages,
>

we show the alternatives of the total output tokens ( trends are qualitatively similar across plots: the incoherence (a) rises with different slopes and the coverage error (c) increases. A noticeable outlier is length (b, left); the model performs badly overall and seems to score better when engaging with


---

## Page 39

(a) Scaling Law of Loss (left) and Bias + Variance as a Function of Steps (right)

(b) 50K

(c) 200K

(e) 790K

(f) 1.2M

> Figure 26: The improvement of model scale mostly manifests in reduction of bias rather than variance. We show the loss scaling curves with model size (
>

law improvement with model size. To understand how this translates to performance improvement, we plot the average bias and variance per step ( ence plot from Fig. 2(d) by separating the decomposition. We see how for longer sequences, model scale reduces bias much more than variance. This means the models first learn the right objective before being reliable optimizers. As another illustration, we also plot the performance-measured in the function value-of the same starting point across the different model sizes ( shows how larger models are able to follow the ground-truth trajectory for longer, and fit it almost perfectly at the end.

(d) 450K

(g) 4.7M

top left, a), which show a known powertop right, a). This is the continuation of the incoherb-g). The pattern

AI models (middle) and human organizations ( incoherence (more of a hot mess), the smarter they are judged by a different set of subjects.

right), human subjects judged entities to be of higher

> Figure 27: Grouped results of survey. For each of biological creatures (animals and humans, left),
>


---

## Page 40

## D RELATED WORK

Reasoning and Test-Time Compute. Recent work demonstrates that scaling test-time compute through longer reasoning chains improves model capabilities ( Guo et al., 2025; Anthropic, 2025b; OpenAI, 2025a; Team, 2025a;b; Team et al., 2025). Multi- ple approaches have been proposed to scale reasoning at inference (

2025; Muennighoff et al., 2025). However, recent studies challenge this assumption, reporting inverse scaling trends where longer reasoning chains degrade performance ( et al., 2025; Su et al., 2025; Wu et al., 2025; Hassid et al., 2025), occurring across diverse contexts: reinforcement learning makes models greedier and less capable ( reward models reinforce incorrect reasoning ( rides (Jang et al., 2025). These effects are particularly pronounced at certain problem complexity levels (Shojaee et al., 2025; Yang et al. on reasoning structure: Wang et al. (2025) show that removing reflection tokens (e.g., "Wait") improves efficiency, Lee et al. (2025) identify length-accuracy tradeoffs through "token complexity,"

and Feng et al. (2025) find that failed reasoning branches systematically bias subsequent reasoning steps. However, existing work does not distinguish systematic reasoning errors from inconsistent failures-a critical distinction for AI safety. Most relevant to our work, tribute overthinking failures to increased output variance; they artificially inject "Wait" tokens to extend reasoning, which may not reflect natural overthinking. Parallel Sampling and Variance Reduction. 

(2025) formalize self-improvement through a sharpening mechanism that concentrates probability on high-quality responses, essentially reducing variance. However, we find that high variance and incoherence naturally remain in reasoning models. 

across random seeds (Bui et al., 2025), and this instability persists even in scaled systems. Errica et al. (2025) formalize this through sensitivity (how outputs change under semantically-equivalent 

variability into user articulation, prompt variation, and internal model factors (

2025), but these studies focus on single-step responses rather than extended reasoning. Variance can even increase with model size before eventually declining ( tions about scale and stability. Our work extends these analyses to long reasoning tasks through bias-variance decompositions. We find that as reasoning chains extend, variance grows-revealing that scale reduces bias but fails to control variance-driven failures. Understanding Scaling Behavior and Model Performance. scaling shapes model behavior. Scaling has been shown to drive convergence in representations across architectures and modalities, suggesting a shared geometry of learned features (

2024). Other studies find that larger models tend to make more correlated errors, even across providers and architectures (Kim et al. tings where one model evaluates another ( similarity, scaling also alters performance in long-horizon tasks: small improvements in stepwise reliability translate into large differences in longer execution ( ments these findings by focusing on how models fail. Rather than studying aggregate error alone, we decompose it into bias and variance to measure incoherence in model behavior.

## E LLM USE STATEMENT

We used LLMs to assist with polishing and smoothing the writing throughout this paper, as well as

Snell et al., 2025; Jaech et al., 2024; Jaech et al., 2024; Guo et al.,

Gema et al., 2025; Ghosal Schmied et al., 2025), step-level

Ma et al., 2025), and models resist instruction over- , 2025). Recent work provides complementary perspectives

 Ghosal et al. (2025) at-

 Parallel sampling and selection strategies are widely

 Huang et al.

Kunievsky & Evans,

Yang et al., 2020), complicating assumpfor coding assistance during low-level implementation. We take full responsibility for all content, ideas, experimental design, results, and conclusions presented in this work.

 Recent work has investigated how

Huh et al.,

, 2025), and that this similarity undermines oversight set- Goel et al., 2025). Beyond representational and error

Sinha et al., 2025). Our work comple-
