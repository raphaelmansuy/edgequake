# *Alpha Evolve*: A coding agent for scientific and algorithmic discovery

**Alexander Novikov*****, Ngân Vu˜*****, Marvin Eisenberger** **See , Swarat Chaudhuri , George Holland , Alex Davies , Sebastian Nowozin , Pushmeet Kohli and Matej Balog**

Google Deep Mind

**In this white paper, we present*****Alpha Evolve*** **capabilities of state-of-the-art LLMs on highly challenging tasks such as tackling open scientific problems** **or optimizing critical pieces of computational infrastructure.** **pipeline of LLMs, whose task is to improve an algorithm by making direct changes to the code. Using** **an evolutionary approach, continuously receiving feedback from one or more evaluators,** **iteratively improves the algorithm, potentially leading to new scientific and practical discoveries. We** 

**stacks at Google,*****Alpha Evolve*** **developed a more efficient scheduling algorithm for data centers, found** 

**novel, provably correct algorithms that surpass state-of-the-art solutions on a spectrum of problems** **in mathematics and computer science, significantly expanding the scope of prior automated discovery** **methods (Romera-Paredes et al., 2023). Notably,** **procedure to multiply two**4 × 4 **complex-valued matrices using** **first improvement, after 56 years, over Strassen's algorithm in this setting. We believe** **coding agents like it can have a significant impact in improving solutions of problems across many areas** **of science and computation.**

***, Emilien Dupont*****, Po-Sen Huang*****, Adam Zsolt Wagner*****,**

**, an evolutionary coding agent that substantially enhances**

***Alpha Evolve*** **orchestrates an autonomous**

***Alpha Evolve***

***Alpha Evolve*** **itself. Furthermore,*****Alpha Evolve*** **discovered** ***Alpha Evolve*** **developed a search algorithm that found a**

48 **scalar multiplications; offering the**

***Alpha Evolve*** **and**

1

**1.**

114

1 © 2025 Google Deep Mind. All rights reserved

## Introduction

 exploration, backtracking on unpromising hypotheses, experimentation, and validation. There has been much recent interest in using large language models (LLMs) to automate significant parts of this process. Hopes of success here are driven by the breathtaking power of recent LLMs [32,76], which can enhance their capabilities using test-time compute, and the rise of*agents*that combine language generation and action [ 

getting LLM pipelines all the way to making entirely new scientific or practical discoveries remains challenging. In this white paper, we present an LLM code superoptimization agent, called that takes on this challenge using a combination of evolutionary computation and LLM-based code generation.*Alpha Evolve*focuses on the broad spectrum of scientific and engineering

See Acknowledgments and Author information section.

88,]. These advances have

34] and experiment design [7,43]. However,

*Alpha Evolve*,

∗Equal contributions.


---

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

discovery problems in which the candidates of discovery can be automatically evaluated. It represents the candidates (for example, new mathematical objects or practical heuristics) as algorithms and uses a set of LLMs to generate, critique, and evolve a pool of such algorithms. The LLM-directed evolution process is grounded using code execution and automatic evalua-tion. This evaluation mechanism allows*Alpha Evolve*to avoid any incorrect suggestions from the base LLM [44]. The evolutionary process in*Alpha Evolve*leverages modern LLMs' ability to respond to feedback, enabling the discovery of candidates that are substantially different from the initial candidate pool in syntax and function. It is applicable both to problems where discovering new algorithms is the intrinsic goal, as well as to the broad range of problems where the solution of interest is not an algorithm itself but an algorithm can*describe*how that solution is to be constructed or found. In the latter case, discovering the algorithm is only an instrumental goal, but it turns out to be a surprisingly effective strategy compared to searching for the solution directly [83]. 

of*Fun Search* [83] (see Table 1), which used LLM-guided evolution to discover heuristics in order to construct novel mathematical objects or to drive the operation of online algorithms. Also, related approaches have been used in tasks such as discovering policies for simulated 

(SOTA) LLMs to evolve large pieces of code that implement complex algorithms spanning multiple functions and components. As a result, it is able to go significantly beyond its predecessors in scale and generality. *Fun Search* [83] *Alpha Evolve* evolves single function evolves entire code file evolves up to 10-20 lines of code evolves up to hundreds of lines of code evolves code in Python evolves any language needs fast evaluation (≤ 20min on 1 CPU) can evaluate for hours, in parallel, on accelerators millions of LLM samples used thousands of LLM samples suffice small LLMs used; no benefit from larger benefits from SOTA LLMs minimal context (only previous solutions)rich context and feedback in prompts optimizes single metric can simultaneously optimize multiple metrics

**Table 1** |Capabilities and typical behaviours of*Alpha Evolve*and our previous agent.

| Value |  |  |  |  |  |  |  |  |  |  |  |  |  |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| constructed or found. In the latter case, discovering the algorithm is only an instrumental |  |  |  |  |  |  |  |  |  |  |  |  |  |
| goal, but it turns out to be a surprisingly effective strategy compared to searching for the |  |  |  |  |  |  |  |  |  |  |  |  |  |
| solution directly [83]. |  |  |  |  |  |  |  |  |  |  |  |  |  |
| The idea of combining evolutionary methods with coding LLMs has been previously ex- |  |  |  |  |  |  |  |  |  |  |  |  |  |
| plored in various specialized settings. In particular,Alpha Evolveis a substantial enhancement |  |  |  |  |  |  |  |  |  |  |  |  |  |
| of Fun Search [83] (see Table 1), which used LLM-guided evolution to discover heuristics in |  |  |  |  |  |  |  |  |  |  |  |  |  |
| order to construct novel mathematical objects or to drive the operation of online algorithms. |  |  |  |  |  |  |  |  |  |  |  |  |  |
| Also, related approaches have been used in tasks such as discovering policies for simulated |  |  |  |  |  |  |  |  |  |  |  |  |  |
| robots [57], symbolic regression [35,89], and the synthesis of heuristic functions for combi- |  |  |  |  |  |  |  |  |  |  |  |  |  |
| natorial optimization [63]. In contrast to these systems,Alpha Evolveleverages state-of-the-art |  |  |  |  |  |  |  |  |  |  |  |  |  |
| (SOTA) LLMs to evolve large pieces of code that implement complex algorithms spanning |  |  |  |  |  |  |  |  |  |  |  |  |  |
| multiple functions and components. As a result, it is able to go significantly beyond its |  |  |  |  |  |  |  |  |  |  |  |  |  |
| predecessors in scale and generality. |  |  |  |  |  |  |  |  |  |  |  |  |  |
| Fun Search [83] Alpha Evolve |  |  |  |  |  |  |  |  |  |  |  |  |  |
| evolves single function evolves entire code file |  |  |  |  |  |  |  |  |  |  |  |  |  |
| evolves up to 10-20 lines of code evolves up to hundreds of lines of code |  |  |  |  |  |  |  |  |  |  |  |  |  |
| evolves code in Python evolves any language |  |  |  |  |  |  |  |  |  |  |  |  |  |
| needs fast evaluation (≤ 20min on 1 CPU) can evaluate for hours, in parallel, on accelerators |  |  |  |  |  |  |  |  |  |  |  |  |  |
| millions of LLM samples used thousands of LLM samples suffice |  |  |  |  |  |  |  |  |  |  |  |  |  |
| small LLMs used; no benefit from larger benefits from SOTA LLMs |  |  |  |  |  |  |  |  |  |  |  |  |  |
| minimal context (only previous solutions)rich context and feedback in prompts |  |  |  |  |  |  |  |  |  |  |  |  |  |
| optimizes single metric can simultaneously optimize multiple metrics |  |  |  |  |  |  |  |  |  |  |  |  |  |

While the use of an automated evaluation metric offers*Alpha Evolve*a key advantage, it is also a limitation-in particular, it puts tasks that require manual experimentation out of our scope. Because problems in mathematics, computer science, and system optimization typically permit automated evaluation metrics, our efforts on*Alpha Evolve*focus on these domains. Specifically, we use*Alpha Evolve*to make progress on several well-known open problems in algorithm design and constructive mathematics, as well as the optimization of critical layers in the large-scale computation stacks at Google.

2


---

Within algorithm design, we consider the fundamental problem of discovering fast algorithms for multiplying matrices, a problem to which a more specialized AI approach had been applied previously [26]. Despite being general-purpose,*Alpha Evolve*goes beyond [26], improving the SOTA for 14 matrix multiplication algorithms; notably, for4 × 4 matrices, *Alpha Evolve*improves Strassen (1969)'s algorithm by discovering an algorithm using 48 multiplications to multiply4 × 4 complex-valued matrices. In mathematics, we consider a broad range of open problems on which one can make progress by discovering constructions (objects) with better properties than all previously known constructions, according to given mathematical definitions. We apply large number (over 50) of such problems and match the best known constructions on of them (in many cases these constructions are likely to already be optimal). On problems,*Alpha Evolve*surpasses the SOTA and discovers new, provably better constructions. This includes an improvement on the Minimum Overlap Problem set by Erdős improved construction on the Kissing Numbers problem in Finally, we use*Alpha Evolve*in four engineering problems spanning different layers of Google's compute stack: discovering scheduling heuristics for Google's cluster management system, optimizing matrix-multiplication kernels used to train LLMs, optimizing arithmetic circuits used within TPUs, and optimizing the runtime of attention in Transformers. Because these components are run repeatedly over a long period of time, any improvements are highly valuable.

*Alpha Evolve*to a

∼75%

∼20% of the [] and an

dimensions [8, 31].

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

2

25

11

**2.**

**2.1.**

2

3

***Alpha Evolve*** 

high-level overview of*Alpha Evolve*is shown in Figure 1, and Figure 2 gives an expanded view.

## Task specification

**Evaluation.** Since*Alpha Evolve*tackles problems with machine-gradeable solutions, the user must provide a mechanism for automatically assessing generated solutions. This mechanism takes the form of a functionℎ mapping a solution to a set of scalar evaluation metrics. By convention, these metrics are maximized. In our current setup,

These discovered algorithms as well as our other new mathematical results can be found at

```
//colab.research.google.com/github/google- deepmind/alphaevolve_results/blob/maste
```

`r/mathematical_results.ipynb`.

**Figure 1** |*Alpha Evolve*high-level overview.

ℎ is typically implemented

```
https:
```


---

**Figure 2**|Expanded view of the*Alpha Evolve*discovery process. The user provides an initial program (with components to evolve marked), evaluation code, and optional configurations (Section 2.1). *Alpha Evolve*then initiates an evolutionary loop. The programs from the*Program database*to construct rich prompts (Section 2.2). Given these prompts, the*LLMs* generate code modifications (diffs), which are applied to create new programs (Section 2.3). These are then scored by solutions are registered back into the*Program database* discovery of better and better programs. as a Python function, called`evaluate`, with a fixed input/output signature, returning a dictionary of scalars. Depending on the application, executing this function may take only seconds on a single device or spawn extensive computations. For mathematical problems, the function typically very simple. For example, when wishing to find largest possible graphs satisf ying a given property,ℎ invokes the evolved code to generate a graph, checks whether the property holds, and then simply returns the size of the graph as the score. In more complicated cases, the functionℎ might involve performing an evolved search algorithm, or training and evaluating a machine learning model.

*Prompt sampler* uses

*Evaluators*(Section 2.4), and promising

(Section 2.5), driving the iterative

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

4

**API.** To support evolving multiple components across a codebase, input API where blocks of code can be annotated as to-be-evolved-by-the-system; see Figure 3a for an illustration. This design facilitates integrating it with existing codebases while requiring only minimal changes, simply by adding special markers ( `EVOLVE-BLOCK-END`) as comments into the code.

*Alpha Evolve*exposes an


---

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

Any user-provided code inside such evolution blocks serves as the initial solution to be improved by*Alpha Evolve*, and the rest of the code forms a skeleton that ties the evolved pieces together, so that they can be invoked from`evaluate`. While this initial implementation must be complete, it can be rudimentary-for instance, consisting of single-line functions that return constants of the appropriate types.

**Flexibility in choosing the abstraction.***Alpha Evolve*can be applied to the same problem in very different ways-especially when the evolved programs are not the final output but a means to discover solutions. For example,*Alpha Evolve*can evolve the solution in raw string representation (as in classical evolutionary algorithms); evolve a function of a definite form that specifies how to construct the solution from scratch (the approach taken in [83]); evolve a bespoke search algorithm to find the solution within some fixed compute budget; or even co-evolve intermediate solutions and search algorithms together, such that each search algorithm is specifically tailored to further improve upon a particular intermediate solution. We find that different levels of abstraction work better for different problems. For example, we hypothesize that for problems with highly symmetric solutions it is advantageous to evolve constructor functions as these tend to be more concise [83], whereas for problems with non-symmetric solutions it works better to evolve customized search algorithms.

### 2.2. Prompt sampling

As *Alpha Evolve*leverages SOTA LLMs, it supports various types of customization and providing long contexts as part of the primary evolution prompt. This prompt comprises multiple previously discovered solutions sampled from the program database, as well as system instructions on how to propose changes to a particular solution. Beyond these key ingredients, users can further tailor prompts to their specific needs in different ways, such as the following.

 • *Explicit context*: details about the problem being solved, such as fixed human-written

instructions, equations, code snippets, or relevant literature (e.g., pdf files).

 • *Stochastic formatting*: template placeholders with human-provided alternatives for

increased diversity, instantiated using probability distributions provided in a separate config file.

 • *Rendered evaluation results*: usually this will include a program, the result of executing

that program, and the scores assigned by the`evaluate `function.

 • *Meta prompt evolution*: instructions and context suggested by the LLM itself in an

additional prompt-generation step, co-evolved in a separate database analogous to the solution programs.

### 2.3. Creative generation

To drive the evolutionary procedure,*Alpha Evolve*leverages the capabilities of SOTA LLMs, whose principal role is to digest information about previously developed solutions and propose new, diverse ways to improve the solutions. Although*Alpha Evolve*is model-agnostic, in ablations we observe that*Alpha Evolve*performs increasingly better as the underlying LLM improves (see Section 4).

5


---

```
# EVOLVE-BLOCK START
```

```
import jax
```

```
# EVOLVE-BLOCK-END
```

```
# EVOLVE-BLOCK-START
```

```
def __init__(self, num_classes): ...
def __call__(self, inputs, is_training): ...
def sweep():
return hyper.zipit([...])
# EVOLVE-BLOCK-END
```

```
def evaluate(eval_inputs) -> dict[str, float]:
```

```
return metrics
The current model uses a simple Res Net architecture with only
three Res Net blocks. We can improve its performance by
increasing the model capacity and adding regularization. This
will allow the model to learn more complex features and
generalize better to unseen data. We also add weight decay to
the optimizer to further regularize the model and prevent
overfitting. AdamW is generally a better choice than Adam,
especially with weight decay.
<<<<<<< SEARCH
```

```
=======
```

```
self._block2 = Res Net Block(num_channels, stride=)
>>>>>>> REPLACE
<<<<<<< SEARCH
def optimizer(self, learning_rate):
return optax.adam(learning_rate)
=======
def optimizer(self, learning_rate):
return optax.adamw(learning_rate, weight_decay=)
>>>>>>> REPLACE
Act as an expert software developer. Your task is to iteratively
improve the provided codebase. [...]
```

 - `- Prior programs`

```
Previously we found that the following programs performed well
on the task at hand:
top_1_acc: 0.796; neg_eval_log_loss: 0.230; average_score: 0.513
```

```
"""Network."""
def __init__(self, num_channels=32, num_output_classess=):
super().__init__()
self._conv1 = hk.Conv2D(num_channels, kernel_shape=3)
self._logits_module = hk.Linear(num_output_classes)
```

 - `- Current program`

```
Here is the current program we are trying to improve (you will
need to propose a modification to it below).
top_1_acc: 0.862; neg_eval_log_loss: 0.387; average_score: 0.624
```

```
"""Network."""
def __init__(self, num_channels=32, num_output_classes=):
super().__init__()
self._conv1 = hk.Conv2D(num_channels, kernel_shape=3)
```

```
self._logits_module = hk.Linear(num_output_classes)
```

```
SEARCH/REPLACE block rules:
```

```
Make sure that the changes you propose are consistent with each
other. For example, if you refer to a new config variable
somewhere, you should also propose a change to add that
variable.
Example:
```

```
Task
Suggest a new idea to improve the code that is inspired by your
expert knowledge of optimization and machine learning.
Describe each change with a SEARCH/REPLACE block.
```

*Alpha Evolve*to evolving a supervised learning

`evaluate `function

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

```
10
10
```

```
1e-4
```

6

**Figure 3** |Illustrative example of applying pipeline. All snippets are abbreviated, with ellipsis (...) indicating skipped lines. (a) The user-provided file with blocks marked for evolution, and the special that can be invoked to score the current version of the code. (b) Example of an assembled prompt to be provided to the LLMs. (c) Example output generated by the LLM. The proposed diffs in (c) will be applied to the "current program" shown in the prompt (b), and the resulting modified program will then be sent to the evaluators. The evaluators will invoke the `evaluate `function from (a) in order to obtain the scores of the newly proposed program.


---

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

**Output format.** When *Alpha Evolve*asks an LLM to modif y existing code, especially within larger codebases, it requests the changes to be provided as a sequence of diff blocks in a specific format:

```
<<<<<<< SEARCH
# Original code block to be found and replaced
=======
# New code block to replace the original
>>>>>>> REPLACE
```

Here, the code between`<<<<<<< SEARCH `and `======= `is the exact segment to match in the current program version. The code between`======= `and `>>>>>>> REPLACE `is the new segment that will replace the original one. This allows for targeted updates to specific parts of the code. In cases where the code being evolved is very short, or when a complete rewrite is more appropriate than a small modification,*Alpha Evolve*can be configured to instruct the LLM to output the entire code block directly, rather than using the diff format.

**Models used.** *Alpha Evolve*employs an ensemble of large language models. Specifically, we utilize a combination of Gemini 2.0 Flash and Gemini 2.0 Pro. This ensemble approach allows us to balance computational throughput with the quality of generated solutions. Gemini 2.0 Flash, with its lower latency, enables a higher rate of candidate generation, increasing the number of ideas explored per unit of time. Concurrently, Gemini 2.0 Pro, possessing greater capabilities, provides occasional, higher-quality suggestions that can significantly advance the evolutionary search and potentially lead to breakthroughs. This strategic mix optimizes the overall discovery process by maximizing the volume of evaluated ideas while retaining the potential for substantial improvements driven by the more powerful model.

### 2.4. Evaluation

To track*Alpha Evolve*'s progress and to select which ideas to propagate in future generations, each new solution proposed by the LLMs is automatically evaluated. In principle, this process amounts to simply executing the user-provided evaluation functionℎ on the generated solution. In practice,*Alpha Evolve*supports optional mechanisms to make this evaluation more flexible and more efficient:

 • *Evaluation cascade (hypothesis testing)*: the user can specif y ensembles of test cases of

increasing difficulty, such that new solutions are evaluated on the next stage only if they achieve sufficiently promising results in all earlier stages. This helps to prune out less promising solutions more quickly. Moreover, new solutions are initially evaluated on a small scale before being subjected to the main test cases, to filter out faulty programs early.

 • *LLM-generated feedback*: in some applications, desirable solutions have certain charac-

teristics that are difficult to capture precisely in the user-provided evaluation function

7


---

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

ℎ; for example, simplicity of the discovered program. These properties can be graded using separate LLM calls and added to the dictionary of scores to steer evolution, or they can be used to discard solutions when a criterion is not fulfilled.

 • *Parallelized evaluation*: the sample efficiency of*Alpha Evolve*makes it feasible to spend

on the order of 100 compute-hours to evaluate any new solution. However, unless individual evaluations are parallelized to reduce their wall-clock duration, this can slow down the rate at which new generations appear, limiting the ability of the evolutionary algorithm to apply several consecutive mutations. In many applications, evaluation is 

calls to an evaluation cluster.

**Multiple scores.** *Alpha Evolve*allows for optimizing multiple user-provided scores, i.e., evolving objects that achieve a high score under one or multiple evaluation metrics. This has both an intrinsic and instrumental value. While in multiple applications we genuinely care about developing solutions for multiple evaluation metrics (or one solution that is strong on all of them simultaneously), we find that even if one metric is of particular interest, optimizing for multiple metrics often improves results for the single target metric. Perhaps this occurs because programs excelling under different evaluation criteria often possess distinct structures or logic and, by incorporating examples of these diverse, high-performing programs-each representing a different definition of "good"-into the prompts provided to the language model, we can stimulate the generation of more varied candidate solutions, increasing the chances of discovering novel approaches that are highly effective for the target metric.

### 2.5. Evolution

During its evolutionary procedure,*Alpha Evolve*continually generates a growing number of solutions with evaluation results (scores and program outputs) attached to them. These solutions are stored in an evolutionary database, the primary goal of which is to optimally resurface previously explored ideas in future generations. A key challenge in designing such databases is balancing exploration and exploitation, to continuously improve the best programs while maintaining diversity to encourage exploration of the entire search space. In *Alpha Evolve*, the evolutionary database implements an algorithm that is inspired by a combination of the MAP elites algorithm [74] and island-based population models [83,97].

### 2.6. Distributed pipeline

*Alpha Evolve*is implemented as an asynchronous computational pipeline (using the`asyncio` Python library) in which many computations are run concurrently, with each computation blocking (waiting) whenever its next step relies on the result of another, yet unfinished computation. More specifically, the asynchronous pipeline comprises a controller, LLM samplers, and evaluation nodes. The entire pipeline is optimized for throughput (rather than the speed of any one particular computation), in order to maximize the number of ideas that can be proposed and evaluated within a specific overall computation budget.

8


---

⟨ ⟩ best known [reference]*Alpha Evolve* ⟨2 4 5⟩ 33 [42] **32**

⟨2 4 7⟩ 46 [93] **45**

⟨2 4 8⟩ 52 [93] **51**

⟨2 5 6⟩ 48 [93] **47**

⟨3 3 3⟩ 23 [52] 23 ⟨3 4 6⟩ 56 [48] **54**

⟨3,4,7⟩ ⟨3,4,8⟩ ⟨3,5,6⟩ ⟨3,5,7⟩ ⟨4,4,4⟩ ⟨4,4,5⟩ ⟨4,4,7⟩ ⟨4,4,8⟩ ⟨4,5,6⟩ ⟨5,5,5⟩

66 [91] **63**

75 [91] **74**

70 [48] **68**

82 [91] **80**

49 [95] **48**

62 [47] **61**

87 [93] **85**

98 [95] **96**

93 [48] **90**

93 [72] 93

⟨, , ⟩representing the product of an×

⟨3,4,7⟩, ⟨4,4,4⟩, and ⟨4,4,8⟩, the algorithms

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

26

9

**Table 2**|Upper bounds on the rank of the tensor matrix and an × matrix, i.e. the number of scalar multiplications required to compute this matrix product. Beyond the examples shown here, for all parameters either matched or surpassed the best known solutions, and provided exact algorithms (see Table 3 in appendix for full results). For

| Value |  |  |  |  |  |  |  |  |  |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| discovered by Alpha Evolveuse complex-valued multiplications which can be used for exact |  |  |  |  |  |  |  |  |  |
| multiplication of complex or real-valued matrices. The decompositions shown in this table |  |  |  |  |  |  |  |  |  |
| can be found in the accompanying Google Colab. |  |  |  |  |  |  |  |  |  |

## 3. Results

**3.1. Faster matrix multiplication via finding novel algorithms for tensor decomposition**

From accelerating machine learning computations to enabling realistic computer graphics, matrix multiplication serves as a fundamental operation underpinning numerous critical algorithms and applications within computer science. Since the pioneering work of Strassen [95], it has been known that a rich space of algorithms for multiplying two matrices can be represented as decompositions of a given 3D tensor into rank-one tensors. The rank (number of terms) of the decomposition exactly specifies the number of scalar multiplications needed to compute the matrix product. Hence, to develop faster matrix multiplication algorithms one needs to find low-rank decompositions of particular tensors. This problem has been tackled with many approaches, from specialized alternating least squares solvers [ deep reinforcement learning [] and custom search algorithms [ of effort, even for the simple case of multiplying two rank is not known, showcasing the difficulty of the problem. Starting from the problem description and a standard gradient-based algorithm (including an initializer, a reconstruction loss function, and an Adam optimizer [

47]; yet, despite decades

3 × 3 matrices, the minimum achievable

50]),*Alpha Evolve*is


---

able to develop sophisticated tensor decomposition algorithms that outperform existing approaches. To evaluate each evolved program, we choose a set of matrix multiplication targets and run the algorithm, initialized with multiple random seeds using the evaluation cascade described in Section 2.4. The performance is then measured as the best (lowest) rank achieved on each target as well as the fraction of seeds that achieved this rank, providing a signal for*Alpha Evolve*to hill-climb. To ensure the exactness of the decomposition and avoid any potential numerical error, when evaluating, we round each element to the nearest integer or the nearest half-integer; and, to encourage the algorithm to generate near-integral solutions, we include this request in natural language in the LLM's prompt. In Table 2, one can see that the various algorithms developed by state of the art for 14 different matrix multiplication targets. Notably, for multiplying two 4 × 4 matrices, applying the algorithm of Strassen with rank (number of scalar multiplications) equal to 49, which works over any field. For the very specific case of multiplying in the field with 2 elements, Fawzi et al. algorithm with rank 47. For 56 years, designing an algorithm with rank less than 49 over any field with characteristic 0 was an open problem. a rank-48 algorithm to multiply two4 × 4 complex-valued matrices. As shown in Figure 4, *Alpha Evolve*makes significant changes to the initial program, introducing several original ideas to design increasingly better algorithms. While most results in Table 2 (including ⟨4,4,4⟩) were obtained from a simple initial program, we found that for some parameters, seeding the initial program with our own ideas (such as adding stochasticity to the evaluation function or using evolutionary approaches) could further boost performance, highlighting the possibility of scientific collaboration between researchers and *Alpha Evolve*.

*Alpha Evolve*improve the

[95] recursively results in an algorithm

[] found an

3 *Alpha Evolve*is the first method to find

*Alpha Evolve*: A coding agent for scientific and algorithmic discovery

26

29

104

of the matrix multiplication tensor, and they cannot be applied recursively to multiplying larger matrices.

10

**3.2. Finding tailored search algorithms for a wide range of open mathematical problems**

A significant frontier in mathematical research involves discovering objects or that possess optimal, or near-optimal, properties according to some measure. Examples range from finding dense packings of geometric shapes [ satisf ying specific combinatorial or analytic constraints (e.g., [ often relies on finding a single construction that surpasses all previously known examples, thereby establishing new lower or upper bounds for the optimal value. We demonstrate that *Alpha Evolve*serves as a powerful tool for exploring the vast search space inherent in these problems, successfully tackling a diverse array of open mathematical challenges. To assess its capabilities, we apply*Alpha Evolve* problems, spanning more than five different branches of mathematics, including analysis, combinatorics, number theory, and geometry, evaluated across numerous specific parameter settings (e.g., different dimensions or sizes). In 75% of the cases the best known constructions, and in 20% of the cases it discovered a new object that is better than a previously known best construction, thereby improving the SOTA. In all these cases, the initial starting point was a simple or a random construction. These results underscore *Alpha Evolve*'s broad potential as a versatile tool for mathematical research.

3There exist algorithms using fewer than 49 multiplications, but they do not correspond to decompositions

*constructions*

] to identif ying functions or sets

39,40,70,]). Progress

to a curated set of over 50 mathematical

*Alpha Evolve*rediscovered
