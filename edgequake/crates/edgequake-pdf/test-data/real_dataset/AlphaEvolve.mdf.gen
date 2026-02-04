# *AlphaEvolve: A coding agent for scientific and*algorithmic discovery

**Alexander Novikov, Ngân Vu, Marvin Eisenberger, Emilien Dupont** * **˜***

**Sergey Shirobokov, Borislav Kozlovskii, Francisco J. R. Ruiz, Abbas Mehrabian, M. Pawan Kumar, Abigail** *

**See, Swarat Chaudhuri, George Holland, Alex Davies, Sebastian Nowozin, Pushmeet Kohli and Matej Balog**

Google DeepMind

**In this white paper, we present AlphaEvolve, an evolutionary coding agent that substantially enhances capabilities of state-of-the-art LLMs on highly challenging tasks such as tackling open scientific problems or optimizing critical pieces of computational infrastructure. pipeline of LLMs, whose task is to improve an algorithm by making direct changes to the code. Using an evolutionary approach, continuously receiving feedback from one or more evaluators, iteratively improves the algorithm, potentially leading to new scientific and practical discoveries. We** 

**stacks at Google,AlphaEvolvedeveloped a more efficient scheduling algorithm for data centers, found a functionally equivalent simplification in the circuit design of hardware accelerators, and acceler- ated the training of the LLM underpinning AlphaEvolve novel, provably correct algorithms that surpass state-of-the-art solutions on a spectrum of problems in mathematics and computer science, significantly expanding the scope of prior automated discovery methods (Romera-Paredes et al., 2023). Notably,** ***AlphaEvolvedeveloped a search algorithm that found a*** **procedure to multiply two4×4complex-valued matrices using first improvement, after 56 years, over Strassen's algorithm in this setting. We believe coding agents like it can have a significant impact in improving solutions of problems across many areas of science and computation.**

1. Introduction

Discovering new high-value knowledge, such as making a novel scientific discovery or developing a commercially valuable algorithm, generally requires a prolonged process of ideation, exploration, backtracking on unpromising hypotheses, experimentation, and validation. There has been much recent interest in using large language models (LLMs) to automate significant parts of this process. Hopes of success here are driven by the breathtaking power of recent LLMs [32,76], which can enhance their capabilities using test-time compute, and the rise ofagentsthat combine language generation and action [ improved performance across a range of established benchmarks and accelerated discoveryoriented tasks like hypothesis generation [ 34] and experiment design [7,43]. However,

getting LLM pipelines all the way to making entirely new scientific or practical discoveries remains challenging. In this white paper, we present an LLM code superoptimization agent, called that takes on this challenge using a combination of evolutionary computation and LLM-based

***, Po-Sen Huang, Adam Zsolt Wagner,** *

***AlphaEvolveorchestrates an autonomous***

***AlphaEvolve***

**itself. Furthermore,AlphaEvolvediscovered**

48scalar multiplications; offering the

***AlphaEvolveand***

code generation.AlphaEvolvefocuses on the broad spectrum of scientific and engineering

1See Acknowledgments and Author information section.

©2025 Google DeepMind. All rights reserved

∗Equal contributions.

88,114]. These advances have

*AlphaEvolve,*


---

discovery problems in which the candidates of discovery can be automatically evaluated. It represents the candidates (for example, new mathematical objects or practical heuristics) as algorithms and uses a set of LLMs to generate, critique, and evolve a pool of such algorithms. The LLM-directed evolution process is grounded using code execution and automatic evalua- tion. This evaluation mechanism allows AlphaEvolve the base LLM [44]. The evolutionary process in AlphaEvolve feedback, enabling the discovery of candidates that are substantially different from the initial candidate pool in syntax and function. It is applicable both to problems where discovering new algorithms is the intrinsic goal, as well as to the broad range of problems where the solution of interest is not an algorithm itself but an algorithm can constructed or found. In the latter case, discovering the algorithm is only an instrumental goal, but it turns out to be a surprisingly effective strategy compared to searching for the solution directly [83]. The idea of combining evolutionary methods with coding LLMs has been previously explored in various specialized settings. In particular, of FunSearch[83] (see Table 1), which used LLM-guided evolution to discover heuristics in order to construct novel mathematical objects or to drive the operation of online algorithms. Also, related approaches have been used in tasks such as discovering policies for simulated robots [57], symbolic regression [35,89], and the synthesis of heuristic functions for combi- natorial optimization [63]. In contrast to these systems, (SOTA) LLMs to evolve large pieces of code that implement complex algorithms spanning multiple functions and components. As a result, it is able to go significantly beyond its predecessors in scale and generality. FunSearch[83]

*AlphaEvolve*

evolves single function

evolves entire code file

evolves up to 10-20 lines of code evolves up to hundreds of lines of code evolves code in Python

evolves any language

needs fast evaluation (≤20min on 1 CPU) can evaluate for hours, in parallel, on accelerators millions of LLM samples used

thousands of LLM samples suffice

small LLMs used; no benefit from larger benefits from SOTA LLMs minimal context (only previous solutions) rich context and feedback in prompts optimizes single metric

can simultaneously optimize multiple metrics to avoid any incorrect suggestions from leverages modern LLMs' ability to respond to

*describehow that solution is to be*

*AlphaEvolveis a substantial enhancement*

*AlphaEvolveleverages state-of-the-art*

problems in algorithm design and constructive mathematics, as well as the optimization of critical layers in the large-scale computation stacks at Google.

**Table 1|Capabilities and typical behaviours of**

While the use of an automated evaluation metric offers is also a limitation-in particular, it puts tasks that require manual experimentation out of our scope. Because problems in mathematics, computer science, and system optimization typically permit automated evaluation metrics, our efforts on domains. Specifically, we use AlphaEvolve to make progress on several well-known open

*AlphaEvolveand our previous agent.*

*AlphaEvolvea key advantage, it*

*AlphaEvolvefocus on these*


---

Within algorithm design, we consider the fundamental problem of discovering fast algorithms for multiplying matrices, a problem to which a more specialized AI approach had been applied previously [26]. Despite being general-purpose, improving the SOTA for 14 matrix multiplication algorithms; notably, for *AlphaEvolveimproves Strassen (1969)'s algorithm by discovering an algorithm using 48* multiplications to multiply4×4complex-valued matrices. In mathematics, we consider a broad range of open problems on which one can make progress by discovering constructions (objects) with better properties than all previously known constructions, according to given mathematical definitions. We apply large number (over 50) of such problems and match the best known constructions on of them (in many cases these constructions are likely to already be optimal). On problems,AlphaEvolvesurpasses the SOTA and discovers new, provably better constructions. This includes an improvement on the Minimum Overlap Problem set by Erdős improved construction on the Kissing Numbers problem in Finally, we use AlphaEvolvein four engineering problems spanning different layers of Google's compute stack: discovering scheduling heuristics for Google's cluster management system, optimizing matrix-multiplication kernels used to train LLMs, optimizing arithmetic circuits used within TPUs, and optimizing the runtime of attention in Transformers. Because these components are run repeatedly over a long period of time, any improvements are highly valuable.

2. AlphaEvolve

AlphaEvolveis a coding agent that orchestrates an autonomous pipeline of computations including queries to LLMs, and produces algorithms that address a user-specified task. At a high level, the orchestrating procedure is an evolutionary algorithm that gradually develops programs that improve the score on the automated evaluation metrics associated with the task. A high-level overview of AlphaEvolveis shown in Figure 1, and Figure 2 gives an expanded view.

*AlphaEvolvegoes beyond [26],*

4×4matrices,

*AlphaEvolveto a* ∼75% ∼20% of the

                                      - [25]and an

11dimensions [8, 31].

2These discovered algorithms as well as our other new mathematical results can be found at

```
//colab.research.google.com/github/google-deepmind/alphaevolve_results/blob/maste
r/mathematical_results.ipynb.
```

2

```
https:
```

### 2.1. Task specification

**Evaluation.SinceAlphaEvolvetackles problems with machine-gradeable solutions, the user**

must provide a mechanism for automatically assessing generated solutions. This mechanism takes the form of a functionℎmapping a solution to a set of scalar evaluation metrics. By convention, these metrics are maximized. In our current setup,

**Figure 1|AlphaEvolvehigh-level overview.**

ℎis typically implemented


---

**Figure 2|Expanded view of the AlphaEvolve** discovery process. The user provides an initial program (with components to evolve marked), evaluation code, and optional configurations (Section 2.1).AlphaEvolvethen initiates an evolutionary loop. The programs from the Program databaseto construct rich prompts ( prompts, theLLMsgenerate code modifications (diffs), which are applied to create new programs (Section 2.3). These are then scored by *Evaluators(Section 2.4), and promising* solutions are registered back into the Program database (Section 2.5), driving the iterative discovery of better and better programs. as a Python function, calledevaluate, with a fixed input/output signature, returning a dictionary of scalars. Depending on the application, executing this function may take only seconds on a single device or spawn extensive computations. For mathematical problems, the function typically very simple. For example, when wishing to find largest possible graphs satisfying a given property,ℎinvokes the evolved code to generate a graph, checks whether the property holds, and then simply returns the size of the graph as the score. In more complicated cases, the functionℎmight involve performing an evolved search algorithm, or training and evaluating a machine learning model.

*Prompt sampleruses* Section 2.2). Given these

only minimal changes, simply by adding special markers ( `# EVOLVE-BLOCK-STARTand#`

```
EVOLVE-BLOCK-END) as comments into the code.
```

**API.To support evolving multiple components across a codebase,**

input API where blocks of code can be annotated as to-be-evolved-by-the-system; see Figure for an illustration. This design facilitates integrating it with existing codebases while requiring

ℎis

*AlphaEvolveexposes an*


---

Any user-provided code inside such evolution blocks serves as the initial solution to be improved by AlphaEvolve, and the rest of the code forms a skeleton that ties the evolved pieces together, so that they can be invoked from `evaluate. While this initial implementation` must be complete, it can be rudimentary-for instance, consisting of single-line functions that return constants of the appropriate types.

**Flexibility in choosing the abstraction.** *AlphaEvolvecan be applied to the same problem* in very different ways-especially when the evolved programs are not the final output but a means to discover solutions. For example, string representation (as in classical evolutionary algorithms); evolve a function of a definite form that specifies how to construct the solution from scratch (the approach taken in [ evolve a bespoke search algorithm to find the solution within some fixed compute budget; or even co-evolve intermediate solutions and search algorithms together, such that each search algorithm is specifically tailored to further improve upon a particular intermediate solution. We find that different levels of abstraction work better for different problems. For example, we hypothesize that for problems with highly symmetric solutions it is advantageous to evolve constructor functions as these tend to be more concise [ with non-symmetric solutions it works better to evolve customized search algorithms.

### 2.2. Prompt sampling

AsAlphaEvolveleverages SOTA LLMs, it supports various types of customization and providing long contexts as part of the primary evolution prompt. This prompt comprises multiple previously discovered solutions sampled from the program database, as well as system instructions on how to propose changes to a particular solution. Beyond these key ingredients, users can further tailor prompts to their specific needs in different ways, such as the following. •Explicit context: details about the problem being solved, such as fixed human-written instructions, equations, code snippets, or relevant literature (e.g., pdf files).

- Stochastic formatting: template placeholders with human-provided alternatives for

increased diversity, instantiated using probability distributions provided in a separate config file. •Rendered evaluation results: usually this will include a program, the result of executing that program, and the scores assigned by the •Meta prompt evolution: instructions and context suggested by the LLM itself in an additional prompt-generation step, co-evolved in a separate database analogous to the solution programs.

*AlphaEvolvecan evolve the solution in raw*

83]);

83], whereas for problems

in ablations we observe that AlphaEvolveperforms increasingly better as the underlying LLM improves (see Section 4).

### 2.3. Creative generation

To drive the evolutionary procedure,AlphaEvolve whose principal role is to digest information about previously developed solutions and propose new, diverse ways to improve the solutions. Although

```
evaluatefunction.
```

leverages the capabilities of SOTA LLMs, *AlphaEvolveis model-agnostic,*


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
def__init__(self, num_classes): ...
def__call__(self, inputs, is_training): ...
defsweep():
returnhyper.zipit([...])
# EVOLVE-BLOCK-END
```

```
defevaluate(eval_inputs) -> dict[str, float]:
```

```
returnmetrics
The current model uses a simple ResNet architecture with only
three ResNet blocks. We can improve its performance by
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
self._block2 = ResNetBlock(num_channels, stride=1)
>>>>>>> REPLACE
<<<<<<< SEARCH
defoptimizer(self, learning_rate):
returnoptax.adam(learning_rate)
=======
defoptimizer(self, learning_rate):
returnoptax.adamw(learning_rate, weight_decay=1e-4)
>>>>>>> REPLACE
Act as an expert software developer. Your task is to iteratively
improve the provided codebase. [...]
```

                      - Prior programs

```
Previously we found that the following programs performed well
on the task at hand:
top_1_acc: 0.796; neg_eval_log_loss: 0.230; average_score: 0.513
```

```
"""Network."""
def__init__(self, num_channels=32, num_output_classess=10):
super().__init__()
self._conv1 = hk.Conv2D(num_channels, kernel_shape=3)
self._logits_module = hk.Linear(num_output_classes)
```

                      - Current program

```
Here is the current program we are trying to improve (you will
need to propose a modification to it below).
top_1_acc: 0.862; neg_eval_log_loss: 0.387; average_score: 0.624
```

```
"""Network."""
def__init__(self, num_channels=32, num_output_classes=10):
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

**Figure 3|Illustrative example of applying** *AlphaEvolveto evolving a supervised learning* pipeline. All snippets are abbreviated, with ellipsis (...) indicating skipped lines. (a) The user-provided file with blocks marked for evolution, and the special that can be invoked to score the current version of the code. (b) Example of an assembled prompt to be provided to the LLMs. (c) Example output generated by the LLM. The proposed diffs in (c) will be applied to the "current program" shown in the prompt (b), and the resulting modified program will then be sent to the evaluators. The evaluators will invoke the

```
evaluatefunction from (a) in order to obtain the scores of the newly proposed program.
evaluatefunction
```


---

**Output format.WhenAlphaEvolveasks an LLM to modify existing code, especially within**

larger codebases, it requests the changes to be provided as a sequence of diff blocks in a

specific format:

```
<<<<<<< SEARCH
# Original code block to be found and replaced
=======
# New code block to replace the original
>>>>>>> REPLACE
```

Here, the code between<<<<<<< SEARCH in the current program version. The code between new segment that will replace the original one. This allows for targeted updates to specific parts of the code. In cases where the code being evolved is very short, or when a complete rewrite is more appropriate than a small modification,AlphaEvolve output the entire code block directly, rather than using the diff format.

**Models used.AlphaEvolveemploys an ensemble of large language models. Specifically, we**

utilize a combination of Gemini 2.0 Flash and Gemini 2.0 Pro. This ensemble approach allows us to balance computational throughput with the quality of generated solutions. Gemini 2.0 Flash, with its lower latency, enables a higher rate of candidate generation, increasing the number of ideas explored per unit of time. Concurrently, Gemini 2.0 Pro, possessing greater capabilities, provides occasional, higher-quality suggestions that can significantly advance the evolutionary search and potentially lead to breakthroughs. This strategic mix optimizes the overall discovery process by maximizing the volume of evaluated ideas while retaining the potential for substantial improvements driven by the more powerful model.

### 2.4. Evaluation

To track AlphaEvolve's progress and to select which ideas to propagate in future generations, each new solution proposed by the LLMs is automatically evaluated. In principle, this process amounts to simply executing the user-provided evaluation function solution. In practice,AlphaEvolvesupports optional mechanisms to make this evaluation more flexible and more efficient:

and=======is the exact segment to match

```
=======and>>>>>>> REPLACEis the
```

can be configured to instruct the LLM to

•LLM-generated feedback: in some applications, desirable solutions have certain characteristics that are difficult to capture precisely in the user-provided evaluation function •Evaluation cascade (hypothesis testing): the user can specify ensembles of test cases of increasing difficulty, such that new solutions are evaluated on the next stage only if they achieve sufficiently promising results in all earlier stages. This helps to prune out less promising solutions more quickly. Moreover, new solutions are initially evaluated on a small scale before being subjected to the main test cases, to filter out faulty programs early.

ℎon the generated


---

ℎ; for example, simplicity of the discovered program. These properties can be graded using separate LLM calls and added to the dictionary of scores to steer evolution, or they can be used to discard solutions when a criterion is not fulfilled. •Parallelized evaluation: the sample efficiency of on the order of 100 compute-hours to evaluate any new solution. However, unless individual evaluations are parallelized to reduce their wall-clock duration, this can slow down the rate at which new generations appear, limiting the ability of the evolutionary algorithm to apply several consecutive mutations. In many applications, evaluation is embarrassingly parallel (for example, running a search algorithm from multiple random- ized initializations), allowing AlphaEvolve calls to an evaluation cluster.

**Multiple scores.AlphaEvolveallows for optimizing multiple user-provided scores, i.e.,**

evolving objects that achieve a high score under one or multiple evaluation metrics. This has both an intrinsic and instrumental value. While in multiple applications we genuinely care about developing solutions for multiple evaluation metrics (or one solution that is strong on all of them simultaneously), we find that even if one metric is of particular interest, optimizing for multiple metrics often improves results for the single target metric. Perhaps this occurs because programs excelling under different evaluation criteria often possess distinct structures or logic and, by incorporating examples of these diverse, high-performing programs-each representing a different definition of "good"-into the prompts provided to the language model, we can stimulate the generation of more varied candidate solutions, increasing the chances of discovering novel approaches that are highly effective for the target metric.

### 2.5. Evolution

During its evolutionary procedure,AlphaEvolve solutions with evaluation results (scores and program outputs) attached to them. These solutions are stored in an evolutionary database, the primary goal of which is to optimally resurface previously explored ideas in future generations. A key challenge in designing such databases is balancing exploration and exploitation, to continuously improve the best programs while maintaining diversity to encourage exploration of the entire search space. InAlphaEvolve, the evolutionary database implements an algorithm that is inspired by a combination of the MAP elites algorithm [74] and island-based population models [83,97].

*AlphaEvolvemakes it feasible to spend*

to distribute this work through asynchronous

continually generates a growing number of

the speed of any one particular computation), in order to maximize the number of ideas that can be proposed and evaluated within a specific overall computation budget.

### 2.6. Distributed pipeline

*AlphaEvolveis implemented as an asynchronous computational pipeline (using the* Python library) in which many computations are run concurrently, with each computation blocking (waiting) whenever its next step relies on the result of another, yet unfinished computation. More specifically, the asynchronous pipeline comprises a controller, LLM samplers, and evaluation nodes. The entire pipeline is optimized for throughput (rather than

```
asyncio
```


---

⟨, , ⟩ best known [reference] ⟨2,4,5⟩ 33 [42] ⟨2,4,7⟩ 46 [93] ⟨2,4,8⟩ 52 [93] ⟨2,5,6⟩ 48 [93] ⟨3,3,3⟩ 23 [52] ⟨3,4,6⟩ 56 [48] ⟨3,4,7⟩ 66 [91] ⟨3,4,8⟩ 75 [91] ⟨3,5,6⟩ 70 [48] ⟨3,5,7⟩ 82 [91] ⟨4,4,4⟩ 49 [95] ⟨4,4,5⟩ 62 [47] ⟨4,4,7⟩ 87 [93] ⟨4,4,8⟩ 98 [95] ⟨4,5,6⟩ 93 [48] ⟨5,5,5⟩ 93 [72]

**Table 2|Upper bounds on the rank of the tensor**

matrix and an× matrix, i.e. the number of scalar multiplications required to compute this matrix product. Beyond the examples shown here, for all parameters either matched or surpassed the best known solutions, and provided exact algorithms (see Table 3 in appendix for full results). For ⟨3,4,7⟩,⟨4,4,4⟩, and⟨4,4,8⟩, the algorithms discovered by AlphaEvolveuse complex-valued multiplications which can be used for exact multiplication of complex or real-valued matrices. The decompositions shown in this table can be found in the accompanying Google Colab

3. Results

**3.1.Faster matrix multiplication via finding novel algorithms for tensor decomposition**

From accelerating machine learning computations to enabling realistic computer graphics, matrix multiplication serves as a fundamental operation underpinning numerous critical algorithms and applications within computer science. Since the pioneering work of Strassen

- [95], it has been known that a rich space of algorithms for multiplying two matrices can be

represented as decompositions of a given 3D tensor into rank-one tensors. The rank (number of terms) of the decomposition exactly specifies the number of scalar multiplications needed to compute the matrix product. Hence, to develop faster matrix multiplication algorithms one needs to find low-rank decompositions of particular tensors. This problem has been tackled with many approaches, from specialized alternating least squares solvers [ deep reinforcement learning [26] and custom search algorithms [ of effort, even for the simple case of multiplying two rank is not known, showcasing the difficulty of the problem.

*AlphaEvolve*

**45 51 47 54 63 74 68 80 48 61 85 96 90**

93

⟨, , ⟩representing the product of an×

, , ≤5,AlphaEvolve

.

Starting from the problem description and a standard gradient-based algorithm (including an initializer, a reconstruction loss function, and an Adam optimizer [

**32**

23

50]),AlphaEvolveis 47]; yet, despite decades

3×3matrices, the minimum achievable


---

able to develop sophisticated tensor decomposition algorithms that outperform existing approaches. To evaluate each evolved program, we choose a set of matrix multiplication targets and run the algorithm, initialized with multiple random seeds using the evaluation cascade described in Section 2.4. The performance is then measured as the best (lowest) rank achieved on each target as well as the fraction of seeds that achieved this rank, providing a signal for AlphaEvolveto hill-climb. To ensure the exactness of the decomposition and avoid any potential numerical error, when evaluating, we round each element to the nearest integer or the nearest half-integer; and, to encourage the algorithm to generate near-integral solutions, we include this request in natural language in the LLM's prompt. In Table 2, one can see that the various algorithms developed by state of the art for 14 different matrix multiplication targets. Notably, for multiplying two 4×4matrices, applying the algorithm of Strassen with rank (number of scalar multiplications) equal to 49, which works over any field. For the very specific case of multiplying in the field with 2 elements, Fawzi et al. algorithm with rank 47. For 56 years, designing an algorithm with rank less than 49 over any field with characteristic 0 was an open problem. a rank-48algorithm to multiply two4×4 complex-valued matrices. As shown in Figure 4,AlphaEvolvemakes significant changes to the initial program, introducing several original ideas to design increasingly better algorithms. While most results in Table 2 (including⟨4,4,4⟩) were obtained from a simple initial program, we found that for some parameters, seeding the initial program with our own ideas (such as adding stochasticity to the evaluation function or using evolutionary approaches) could further boost performance, highlighting the possibility of scientific collaboration between researchers and *AlphaEvolve.*

**3.2.Finding tailored search algorithms for a wide range of open mathematical problems**

A significant frontier in mathematical research involves discovering objects or that possess optimal, or near-optimal, properties according to some measure. Examples range from finding dense packings of geometric shapes [ satisfying specific combinatorial or analytic constraints (e.g., [ often relies on finding a single construction that surpasses all previously known examples, thereby establishing new lower or upper bounds for the optimal value. We demonstrate that *AlphaEvolveserves as a powerful tool for exploring the vast search space inherent in these* problems, successfully tackling a diverse array of open mathematical challenges. To assess its capabilities, we apply AlphaEvolve problems, spanning more than five different branches of mathematics, including analysis, combinatorics, number theory, and geometry, evaluated across numerous specific parameter settings (e.g., different dimensions or sizes). In 75% of the cases the best known constructions, and in 20% of the cases it discovered a new object that is better than a previously known best construction, thereby improving the SOTA. In all these cases, the initial starting point was a simple or a random construction. These results underscore *AlphaEvolve's broad potential as a versatile tool for mathematical research.*

*AlphaEvolveimprove the*

                        - [95]recursively results in an algorithm
                                      - [26]found an

3*AlphaEvolveis the first method to find*

*constructions*

29] to identifying functions or sets

39,40,70,104]). Progress

3There exist algorithms using fewer than 49 multiplications, but they do not correspond to decompositions of the matrix multiplication tensor, and they cannot be applied recursively to multiplying larger matrices.

10

to a curated set of over 50 mathematical

*AlphaEvolverediscovered*


---

```
# EVOLVE -BLOCK -START
def _get_optimizer (self) -> optax. GradientTransformation :
""" Returns optimizer ."""
+ return optax.adamw(
```

7 `+ self`

```
weight_decay
```

8 `+ )`

```
def _get_init_fn (self) -> jax.nn. initializers .Initializer:
""" Returns initializer
```

                - return initializers

```
complex64)
+ # Initialize
solutions.
+ # Increase scale slightly for better exploration
+ return initializers
""" Computes ( batched ) loss on learned decomposition
```

3 `# Compute reconstruction`

```
, M, P)
```

5`...`

6`+ # Discretization`

```
integer).
```

7`+ def dist_to_half_ints`

8`...`

```
+ def dist_to_ints
...
```

```
+ for factor in decomposition
```

```
+ ) # average across all factors and loss components
+ discretization_weight
+ global_step
+ # Cosine annealing for half -integer loss.
+ cycle_length
per cycle
+ cycle_progress
+ global_step
[0, 1)
```

30 `+ half_int_multiplier`

31 `+ half_int_multiplier`

32+ 1 self.hypers. half_int_start

35 `+ total_loss`

36 `+ rec_loss`

37`+ + discretization_weight`

```
half_int_multiplier
```

38 `+ )`

39`...`

```
return hyper.zipit ([
```

3 hyper

4 hyper

5 `+ hyper`

6 `+ hyper`

7 `+ hyper`

8 `+ hyper`

9 `+ hyper`

```
0.01)),
+ # Add noise to the gradient to aid in exploration
```

```
# EVOLVE -BLOCK -END
.hypers.learning_rate , weight_decay =self.hypers.
function ."""
.normal (0.0 , self.hypers.init_scale , jnp.
with a smaller scale to encourage finding low -rank
.
."""
loss.
# (B, N
loss (encourage entries to be multiples of 1/2 or
(x):
(x):
= 0.0
+= jnp.mean( dist_to_half_ints (factor))
+= jnp.mean( dist_to_ints (factor))
, start =0.0 , end=self.hypers. discretization_weight
= self.config. training_steps // 4 # Number of steps
% cycle_length
# Normalized progress within the current cycle
+ self.hypers. half_int_start
```

                              - `* discretization_loss *`

```
.uniform('init_scale ', hyper.interval (0.2 , 1.5)),
.uniform('learning_rate ', hyper.interval (0.05 , 0.3)),
.uniform('init_scale ', hyper.interval (0.1 , 1.0)),
.uniform('learning_rate ', hyper.interval (0.01 , 0.2)),
.uniform('discretization_weight ', hyper.interval (0.0 , 0.1))
.uniform('hallucination_prob ', hyper.interval (0.0 , 0.2)),
.uniform('hallucination_scale ', hyper.interval (0.0 , 0.2)),
.uniform('noise_std', hyper.interval (0.0 , 0.01)),
.uniform('target_noise_std ', hyper.interval (0.0 , 0.01)),
.uniform('weight_decay ', hyper.interval (0.00001 , 0.001)),
.uniform('clip_min', hyper.interval (0.0 , 0.5)),
.uniform('clip_max', hyper.interval (1.0 , 3.0)),
.
.uniform('grad_noise_std ', hyper.interval (0.0 , 0.001)),
.uniform('half_int_start ', hyper.interval (0.0 , 1.0)),
```

1 `@@ -45,9 +45 ,14 @@`

1 `@@ -91,13 +156 ,86 @@`

17 18 19 20 21 22 23 24 25 26 27 28 29

1 `@@ -117,6 +255 ,18 @@`

17 18 19 20

changes across several components, including the optimizer and weight initialization (top right), the loss function (middle right), and hyperparameter sweep (bottom right). These changes are highly non-trivial, requiring 15 mutations during the evolutionary process.

**Figure 4|Changes proposed by AlphaEvolve**

rithms. The full diff is outlined on the left (see magnified version in some excerpts are highlighted on the right. In this example,

to discover faster matrix multiplication algo-

 Figures 9a to 9c) and

*AlphaEvolveproposes extensive*


---

**Figure 5|Examples of SOTA-breaking mathematical constructions discovered with**

*volve. The versatility of AlphaEvolveallows us to tackle problems in analysis (autocorrelation* 

finite sets). A significant advantage of the AlphaEvolve speed of application. The core methodology, focused on evolving heuristic search programs (detailed below), can be rapidly deployed across a diverse range of mathematical construction 

problem formulation and search space definition, to autonomously discover effective search patterns and attack strategies by identifying subtle structures within the problem landscape. This allows for efficient, large-scale exploration across many different problems. The key methodological innovation enabling these discoveries is evolveheuristic search algorithmsrather than directly evolving the constructions themselves. For many problems, particularly those with fast objective function evaluations-which are common in mathematics-we employed an iterative refinement strategy. Each generation of *AlphaEvolvewas tasked with evolving a program representing a search heuristic. This program* was given a fixed time budget (e.g., 1000 seconds) and was shown the best construction found by the previous best heuristic. Its goal was to leverage this starting point and the allotted time to find an even better construction. The evolutionary process thus selects for heuristics that are effective at improving already high-quality solutions. The final constructions were often the result of a sequence of different, specialized heuristics discovered by heuristics proficient at making large gains from random or simple initial states, and later heuristics adept at fine-tuning near-optimal configurations. This automated discovery of multi-stage, adaptive search strategies is challenging to replicate manually and proved crucial for surpassing the SOTA. Below are high-level descriptions of some of the problems where

AlphaEconfiguration used here is its versatility and

*AlphaEvolveoften demonstrates a capacity*

*AlphaEvolve's ability to*

### results

Full list of problems and details are provided in Appendix

12

*AlphaEvolve-early*

*AlphaEvolveyielded new*


---

•Analysis

**-Autocorrelation inequalities.** *AlphaEvolvewas able to improve the best known* bounds on several autocorrelation inequalities.

**-Uncertainty principles.AlphaEvolve**

for a problem arising in Fourier analysis, by polishing an uncertainty principle construction [33] leading to a slightly better upper bound. •Combinatorics and number theory

**-Erdős's minimum overlap problem.**

for the minimum overlap problem [ record [40]. •Geometry and packing

**-Kissing number problem.In 11 dimensions,**

bound on the kissing number, finding a configuration of 593 non-overlapping unit spheres that can simultaneously touch a central unit sphere, surpassing the previous record of 592 [31].

-Packing problems.AlphaEvolve achieved several new results in packing problems, such as packingpoints in a shape to minimize the ratio of the maximum and minimum distance, packing various polygons in other polygons in the most efficient way, and variants of the Heilbronn problem concerning point sets avoiding small-area triangles [29]. The full list of problems appears in Appendix AlphaEvolvecan be found in the accompanying on these problems and the methods used will be provided in an upcoming paper. Most of these discoveries are on open problems suggested to us by external mathematicians Javier Gomez Serrano and Terence Tao, who also advised on how to best formulate them as inputs to AlphaEvolve. This highlights the potential for synergistic partnerships between AI-driven discovery engines like AlphaEvolveand human mathematical expertise.

### 3.3. Optimizing Google's computing ecosystem

In addition to the scientific applications presented in preceding sections, here we demonstrate how AlphaEvolvehas been used to improve performance of mission-critical infrastructure and deliver real-world impact.

was able to produce a refined configuration

*AlphaEvolveestablished a new upper bound* 25], slightly improving upon the previous

*AlphaEvolveimproved the lower*

 B and the new constructions found by Google Colab. More examples and details

but still has other resources free (e.g., CPU). Improvements in scheduling efficiency can recover these stranded resources, allowing more jobs to be completed on the same amount

13

### *3.3.1. Improving data center scheduling*

Efficiently scheduling compute jobs onto a cluster of machines is a critical optimization problem, particularly at the scale of Google's data centers, orchestrated by Borg [ task involves assigning jobs to available machines based on job resource requirements and machine capacity. Inefficient assignments can result in stranded resources: when a machine can no longer accept jobs because it has run out of one kind of resource (e.g., memory)

102]. This


---

```
defalpha_evolve_score(required, free):
cpu_residual = required.cpu / free. cpu
```

3 `mem_residual = required.mem / free. mem`

5 `return-1.0 * (cpu_residual + mem_residual`

```
mem_residual / cpu_residual
cpu_residual / mem_residual)
```

**Figure 6|Left: The heuristic function discovered by**

workloads and capacity. Right: Visualization of the heuristic scoring function. Yellow regions represent high scores, while purple regions represent low scores. of computational footprint. This recovery is essential to accommodate growing compute needs without a proportional increase in resource consumption. Furthermore, this problem is challenging since it combines typical engineering difficulties, such as debuggability and scale, on top of the classically difficult bin-packing problem. We address this challenge by framing the online job scheduling problem as a vector bin-packing problem with two variables. In this context, machines represent bins with defined capacities for CPU and memory, and incoming jobs are items with specific resource demands. A heuristic function takes as input a pending job's CPU and memory requirements and a potential machine's CPU and memory availability. This function then outputs a priority score for the machine. The Borg scheduler subsequently assigns the pending job to the machine with the highest priority score as determined by the heuristic function, among other objectives. Because this heuristic only influences the ranking of machines already determined by Borg to be available and capable of running each pending job, the resulting scheduling decisions are effectively correct by construction. An early version of AlphaEvolvewas used to discover a remarkably simple yet effective heuristic function (shown in Figure 6), evolving from the existing one in production. We use a simulator of our data centers to provide feedback to snapshots of workloads and capacity across Google's fleet. We measure the performance of *AlphaEvolve's heuristic function on an unseen test dataset of recent workloads and capacity* to ensure generalization. Observing that AlphaEvolve one in production, we rolled out AlphaEvolve deployment measurements across Google's fleet confirmed the simulator results, revealing that this heuristic function continuously recovers on average 0.7% of Google's fleet-wide compute resources, which would otherwise be stranded. deep reinforcement learning approach because its code solution not only leads to better performance, but also offers clear advantages in interpretability, debuggability, predictability,

*AlphaEvolve, tailored to Google's*

and ease of deployment-essential qualities for a mission-critical system.

14

*AlphaEvolvebased on historical*

's heuristic function outperforms the 's heuristic function to the entire fleet. Post-

*AlphaEvolvewas chosen over a*


---

**Figure 7|Visualization of the tiling heuristic problem for a matrix product**

a heuristic that automatically chooses the right tile size ( difficult because one has to know the matrix multiplication unit's optimal shapes and memory capacity, the memory requirements of surrounding operations, extra operations that are fused into the kernel, and low-level compiler intricacies, among other details.

### *3.3.2. Enhancing Gemini kernel engineering*

Training large models like Gemini requires substantial computational resources. Gemini is built on JAX [9], and Pallas is an extension to JAX that enables writing custom, highly specialized programs (kernels) tailored for optimal execution on hardware accelerators. Therefore, efficient Pallas kernels are crucial for optimizing Gemini's training performance. A critical aspect of kernel optimization is tuning the tiling strategy for matrix multiplication operations (see Figure 7). This technique involves dividing a large matrix multiplication computation into smaller subproblems to better balance computation with data movement, which is key to accelerating the overall computation. Traditionally, kernel engineers rely on either search-based autotuning or manually crafted heuristics to determine near-optimal tiling configurations for various input shapes. Search-based tuning interrupts the research workflow, necessitating retuning for every input shape change. Conversely, manually crafting effective tiling heuristics is a major engineering bottleneck due to its complexity, demanding a deep understanding of both kernel functionality and hardware intricacies. The key advantage of a performant heuristic is its ability to deliver high performance across arbitrary input shapes. Consequently, to expedite the design of performant kernels for emerging hardware and to simplify their utilization by model developers, we aim to facilitate the heuristic generation process. We address this challenge by employing *AlphaEvolveto optimize tiling heuristics for an* important matrix multiplication kernel used to train Gemini. The objective is to minimize the kernel's actual runtime.AlphaEvolveiteratively explores and refines tiling heuristics for this kernel by proposing candidate code, aiming to minimize this runtime on various input

= . Creating

,,) for all input shapes is

shapes on real TPU accelerators. The kernel's correctness is maintained by construction because AlphaEvolveis optimizing the tiling strategy for this kernel rather than altering

15


---

its underlying mathematical operation. To build the training and evaluation datasets for AlphaEvolve, we automatically collect realistic kernel input shapes from kernel users. Half of these input shapes form the training set, providing the optimization targets during the evolutionary process. The remaining input shapes form the evaluation set, used to test the general applicability of the resulting heuristic. This automated approach enables AlphaEvolve average 23% kernel speedup across all kernels over the existing expert-designed heuristic, and a corresponding 1% reduction in Gemini's overall training time. In addition, the use of AlphaEvolvesignificantly reduced the kernel optimization time, from several months of dedicated engineering effort to just days of automated experimentation. This acceleration speeds up the deployment of optimized kernels, allowing kernel engineers to dedicate their expertise to more strategic, higher-level optimization problems. Furthermore, offers a path towards automating the manual tuning process and improving the ergonomics of Gemini kernel usage. The tiling heuristic discovered by production, directly enhancing Gemini's training efficiency and the Gemini team's research and engineering velocity. This deployment also marks a novel instance where Gemini, through the capabilities of AlphaEvolve, optimizes its own training process.

#### *3.3.3. Assisting in hardware circuit design*

Specialized hardware, such as Google's Tensor Processing Units (TPUs), is crucial for achieving the resource efficiency required to run modern AI systems at scale. However, designing new computer chips is a complex and time-consuming process, often spanning years. Register- Transfer Level (RTL) optimization, a critical step in this process, involves manually rewriting hardware descriptions to improve metrics like power, performance, and area, demanding months of iteration by highly skilled engineers. In this work,AlphaEvolvewas challenged to optimize an already highly optimized Verilog implementation of a key TPU arithmetic circuit within the matrix multiplication unit. The optimization objectives were to reduce both area and power consumption while preserving the component's core functionality. Crucially, the final proposal must pass robust verification methods to confirm that the modified circuit maintains functional correctness. was able to find a simple code rewrite that removed unnecessary bits, a change validated by TPU designers for correctness. While this specific improvement was also independently caught by downstream synthesis tools,AlphaEvolve 's contribution at the RTL stage demonstrates its capability to refine source RTL and provide optimizations early in the design flow. Integrated into an upcoming TPU, this improvement represents Gemini's first direct contribution to TPU arithmetic circuits, achieved via contributions. A key advantage of AlphaEvolve directly in Verilog, the standard language used by hardware engineers, fostering trust and 

to discover a heuristic that yields an

*AlphaEvolve*

*AlphaEvolvehas been deployed in*

*AlphaEvolve*

16

*AlphaEvolve, paving the way for future* is that it communicates the suggested changes


---

#### *3.3.4. Directly optimizing compiler-generated code*

The transformer architecture [100] is used in the majority of modern neural networks, ranging from LLMs to AlphaFold [1]. The core computation of transformers is the attention mechanism [4], which is most commonly implemented using FlashAttention [ 

these stages, improved decisions on memory access orchestration or computation scheduling can significantly reduce runtime on specific hardware. We challenged AlphaEvolveto directly optimize the XLA-generated IRs encapsulating the FlashAttention kernel along with pre-and postprocessing code. We optimized a configuration corresponding to a highly impactful transformer model used for inference at scale on GPUs, with the goal of minimizing the module's overall execution time. This was a particularly challenging task, because (1) the IR is designed for debugging purposes rather than for direct editing by developers, and (2) it is compiler-generated and already highly optimized. Each modification proposed by AlphaEvolvewas checked against the reference (unmodified) code on randomized inputs in order to ensure numerical correctness throughout optimization. The final version of the code was rigorously confirmed by human experts to be correct for all possible inputs. AlphaEvolvewas able to provide meaningful optimizations for both levels of abstraction exposed by the IR. Firstly, the FlashAttention kernel for the configuration of interest was sped up by 32%. Secondly,AlphaEvolvefound improvements in pre-and postprocessing of kernel inputs and outputs, resulting in a 15% speed up in this part. These results demonstrate the ability of AlphaEvolveto optimize compiler-generated code, offering the potential of incorporating discovered optimizations into existing compilers for specific use cases, or, in the longer term, incorporating AlphaEvolve into the compiler workflow itself.

4. Ablations

We carried out ablations on two tasks: finding tensor decompositions for faster matrix multiplication (Section 3.1) and computing lower bounds on kissing numbers ( aiming to understand the efficacy of the following components of

22]. In our

approach where no explicit context is added to the prompt. We refer to this approach as "No context in the prompt".

17

•Evolutionary approach.AlphaEvolve utilizes an evolutionary approach, where previously generated programs are stored in a database and used to obtain better programs in subsequent iterations. To analyze the importance of evolution, we consider an alternative approach, which repeatedly feeds the same initial program to the language model. We refer to this approach as "No evolution". •Context in prompts.AlphaEvolveuses powerful language models with large context windows, whose output can be improved significantly by providing problem-specific context in the prompt. To test the importance of context, we consider an alternative

Section 3.2),

*AlphaEvolve.*


---

**Figure 8|Left: Ablations of AlphaEvolveon the problem of finding low-rank tensor decom-** relies on a mixture of small and large lan-

position for faster matrix multiplication. Right: Ablations of finding sphere packings for improving kissing numbers. Each curve shows the performance of an individual setting with increasing compute budget, averaged over all considered targets (higher values on the target metric are better). The shades indicate intra-target standard deviation, averaged over three independent runs of random seeds. •Meta prompts.AlphaEvolvealso uses meta prompts in order to improve the prompts that are provided to the language model. This allows it to potentially surpass the performance one can obtain using a human prompter. To test the efficacy of meta prompting, we disable it for the task of tensor decomposition. We refer to this approach as "No meta prompt evolution". •Full-file evolution.Unlike previous approaches such as FunSearch, 

decomposition where only the loss function is evolved. We refer to this approach as "No full-file evolution". •Powerful language models.AlphaEvolve guage models in order to obtain highly diverse samples. To understand the importance of this component, we consider an alternative where only a single small base model is used. We refer to this approach as "Small base LLM only". Figure 8 shows the results of the all-inclusive alternatives listed above. As can be seen, each of the components is responsible for a significant improvement in the results.

*AlphaEvolveon the problem of*

*AlphaEvolve, initialized with different*

*AlphaEvolvecan*

*AlphaEvolveapproach as well as the various*

have succeeded in symbolic regression applications [ algorithmic [16] discovery, and scheduling [ 118] problems. However, a challenge with these

66,87], automated scientific [21] or

18

5. Related work

**Evolutionary methods.AlphaEvolveextends a long tradition of research on**

orgenetic programming[54], where one repeatedly uses a set of mutation and crossover operators to evolve a pool of programs [5,51]. In particular, classical evolutionary techniques

*evolutionary*


---

methods is the use of handwritten evolution operators, which can be hard to design and may fail to capture important properties of the domain. In contrast, automate the construction of these operators-it leverages the LLM's world knowledge to mutate programs without the need to pre-define a set of allowed mutation operations. *AlphaEvolvewas preceded by a body of recent efforts that combine LLMs and evolution;* specifically, it extends the FunSearch system, introduced by Romera-Paredes et al. approach to mathematical discovery. FunSearch was subsequently used in downstream tasks such as learning acquisition functions for Bayesian optimization [ models [13], computing distances between graphs [ programming [101].AlphaEvolvegoes beyond FunSearch and its recent reimplementation

- [24] in three key ways. First, while FunSearch only allowed the evolution of a single Python

function,AlphaEvolveallows evolution over entire codebases written in a wide range of programming languages. Second, FunSearch optimized a single objective function, while *AlphaEvolveprovides the ability to perform multiobjective optimization. Third, the LLMs in* FunSearch were relatively small and solely trained on code. By contrast, 

problems that were not amenable to FunSearch. Other efforts in this category include the approach by Lehman et al. an LLM-guided evolution process to discover programmatic policies for a set of simulated robots; or the approach by Hemberg et al. [41]for code synthesis. Similar approaches have found use in several scientific and mathematical tasks, including symbolic regression [ 89], discovering heuristics for combinatorial optimization [ molecular structures [105]. LLM-guided evolution has also been used to improve AI systems by enhancing LLM prompts [27] and searching over neural architectures [ differs from these approaches in its scale, flexibility, and general applicability to a broad range of domains. Some recent efforts have augmented the basic paradigm of LLM-guided evolution with complementary ideas. For example, Surina et al. by continuously finetuning the LLM through reinforcement learning. Grayeli et al. enhance the evolution process with an LLM-directed concept learning step that summarizes high-performing programs in the pool into natural language. More investigation is required to understand the benefits of these ideas at the scale at which Evolutionary methods have also found use in the recent AI Co-Scientist work [ seeks to automate scientific discovery using distinct agents for tasks like hypothesis discovery, ranking of hypotheses, and literature review. While AI Co-Scientist represents scientific hypotheses and their evaluation criteria in *natural language,AlphaEvolvefocuses on evolving code, and directs evolution using programmatic evaluation functions. This choice enables* us to substantially sidestep LLM hallucinations, which allows evolution process for a large number of time steps. Nevertheless, it is possible in principle to combine the two approaches, leading to a method that allows a flexible combination of natural-language and programmatic idioms.

*AlphaEvolveuses LLMs to*

                                        - [83]as an

2], discovering cognitive 103], or combinatorial competitive

*AlphaEvolveuses*

                                    - [57], which uses

63,115,117], and synthesizing

14,73].AlphaEvolve

                        - [96]complement the evolution process
                                          - [35]

*AlphaEvolveoperates.*

34], which

19

*AlphaEvolveto carry on the*


---

**Superoptimization and algorithm discovery.**

*code superoptimizationin that it iteratively improves an initial program using execution feed-* back. The idea of code superoptimization goes back to the 1980s [ to the problem included systematic enumeration [ sampling [86], and deep reinforcement learning [ focus on a single problem such as matrix multiplication, there have been systems such as AlphaTensor that were also able to discover provably correct algorithms [ More recently, a body of LLM-based approaches to superoptimization and algorithm discovery have emerged. This literature builds on the success of LLMs in coding tasks, perhaps best illustrated by their success in (simulated) programming competitions as in the case of AlphaCode [60]. For instance, LLM agents have been used to optimize certain operations in GPU kernels, such as the attention operation [ operations [56]. There is also work on using LLMs to discover novel evolutionary algorithms

- [55], train language models [58], and optimize warehouse-scale computers [

recent work [108] has also proposed the use of multiple LLM agents that converse with each other to accomplish mathematical and coding tasks. While previous work on using LLMs for algorithm discovery provided promising results, AlphaEvolve's approach to leverage it for evolutionary algorithms allows us to address significantly more challenging problems, as demonstrated in

**AI for scientific and mathematical discovery.**

applied to a wide range of scientific disciplines and tasks, from protein structure prediction

- [46] to quantum physics [6,84] to climate sciences [

recent LLM-based methods that target scientific problems in multiple disciplines, such as materials science [45,71,94,119], chemistry [

- [79], and quantum physics [30, 78] (for surveys on the topic, see [

Many of these methods use LLMs to automate several distinct stages of the scientific discovery process [37,59,106,109,112], e.g., for generating and ranking hypotheses and ideas [38,90]. Of these methods, especially related to use LLM-guided tree search-based algorithms [

[34,113,120]. Other works use LLMs to optimize experimental planning and design

[7,10,43,75] or experiment execution and workflow [ are also works focusing on the data analysis stage [ these methods in its use of programmatic hypothesis representations and evaluation metrics. AI systems have also contributed to advances in pure mathematics [ the FunSearch approach [24,83] established LLM-guided evolution as a powerful tool for discovering witnesses for, and counterexamples to, mathematical statements-a problem that is complementary to that of finding formal and informal proofs of mathematical statements [ 19, 98, 99, 110, 111].

*AlphaEvolvecan be viewed as a method for*

69]; pre-LLM approaches

69], genetic search [20], Monte Carlo

68]. Additionally, in limited settings that

15] or more general user-specified

61]. Other

 Section 3.

Over the last decade, AI systems have been

53]. In particular, there are numerous 12,64], bioinformatics [67,85], geoscience

36, 65, 81]).

*AlphaEvolveare the methods that* 11] or LLM-guided evolutionary algorithms

28,62,82,105,116]. Finally, there 80].AlphaEvolvediffers from most of

23]. In this context,

26].

20


---

6. Discussion

*AlphaEvolvedemonstrates the surprising power of combining state-of-the-art LLMs with* automated evaluation metrics within an evolutionary framework, which can lead to new discoveries on decades-old mathematical problems as well as practical improvements to highly optimized compute stacks. Interestingly,AlphaEvolveoften allows approaching the same problem in different ways: searching for the solution directly, finding a function that constructs it from scratch, or evolving a search algorithm to find it. Applying different biases (for example, finding constructive functions may favor discovering highly symmetric objects [83]) and thus can suit different problems. *AlphaEvolvecan also be seen as a test-time compute agent that, through its evolutionary* procedure, significantly enhances the capability of the base LLM (compared to, e.g., repeated sampling). On one hand, this can be seen as a compelling demonstration of how machine feedback is able to sustain test-time compute scaling up to regimes where new scientific discoveries and highly valuable practical optimizations are made. On the other hand, a natural next step will be to consider distilling the the base LLMs into the next generation of the base models. This can have intrinsic value and also, likely, uplift the next version of AlphaEvolve Beyond distillation, it is also intriguing that that increase the efficiency of its own infrastructure and of (future versions of) its base LLMs. Currently, the gains are moderate and the feedback loops for improving the next version of *AlphaEvolveare on the order of months. However, with these improvements we envision* that the value of setting up more environments (problems) with robust evaluation functions will become more widely recognized, which in turn will result in more high-value practical discoveries going forward. The main limitation of AlphaEvolveis that it handles problems for which it is possible to devise an automated evaluator. While this is true of many problems in the mathematical and computational sciences, there are domains such as the natural sciences where only some experiments can be simulated or automated. While evaluation of ideas, this is not a setting we have optimized for. However, concurrent work shows this is possible [34], and a natural step would be to link the two settings, with LLMs providing feedback on high-level ideas before transitioning to an implementation stage, for which machine feedback is available through code execution.

*AlphaEvolvein different ways comes with*

*AlphaEvolve-augmented performance of* . *AlphaEvolvecan make practical discoveries*

*AlphaEvolvedoes allow for LLM-provided*

versatile tool for a wide range of applications. We gratefully acknowledge the invaluable contributions of these individuals towards the applications highlighted in this white paper:

21

## Acknowledgements

We thank Michael Figurnov for reviewing this white paper; Alhussein Fawzi, Bernardino Romera-Paredes, and Ankit Anand for early explorations and insightful discussions; Stig Petersen and Demis Hassabis for support and advice; JD Velasquez for helpful advice on managing the practical applications; and all early users and collaborators of for their diverse use cases and insightful feedback, which shaped it into a more robust and

*AlphaEvolve*


---

Terence Tao, Javier Gomez Serrano, and Jordan Ellenberg for suggesting specific open mathematical problems and advising on how to best formulate them for Georgiev, Ray Jiang, and Johannes Bausch for their contributions to applying such problems. Mohammadamin Barekatain, Patrick Heisel, Chase Hensel, Robert O'Callahan, and Pengming Wang for co-leading the application to data center scheduling; Federico Piccinini, Sultan Kenjeyev, and Andrea Michi for making significant contributions; Kieran Milan, Daniel Mankowitz, Cosmin Paduraru, Calin Cascaval, Tammo Spalink, and Natasha Antropova for providing helpful advice; Aaron Gentleman, Gaurav Dhiman, Parthasarathy Ranganatha, and Amin Vahdat for reviewing this work. 

advice; Jean-Baptiste Alayrac, Dmitry Lepikhin, Sebastian Borgeaud, Koray Kavukcuoglu and Jeff Dean for reviewing this work. Timur Sitdikov for leading the application to TPU circuit design; Georges Rotival for providing the circuit evaluation infrastructure; Kirk Sanders, Srikanth Dwarakanath, Indranil Chakraborty, Christopher Clark for verifying and validating the results in the TPU design; Vinod Nair, Sergio Guadarrama, Dimitrios Vytiniotis, and Daniel Belov for their helpful advice; Kerry Takenaka, Jeff Dean, Sridhar Lakshmanamurthy, Parthasarathy Ranganathan, and Amin Vahdat for reviewing this work. Benjamin Chetioui, Sergei Lebedev, Alexander Belyaev, Henning Becker, Oleg Shyshkov, and Aliia Khasanova for their help with XLA modifications as well as their helpful advice; Giorgio Arena, Marco Cornero, and Sebastian Bodenstein for reviewing this work.

## Author information

These authors contributed equally: Alexander Novikov, Ngân V Dupont, Po-Sen Huang, Adam Zsolt Wagner, Sergey Shirobokov, Borislav Kozlovskii, and Matej Balog.

*AlphaEvolve; Bogdan AlphaEvolveto*

u, Marvin Eisenberger, Emilien˜

*AlphaEvolve.*

S.S. worked on applying AlphaEvolveto directly optimize compiler-generated code. M.E. performed the ablation experiments. M.B., A.N., M.E., S.S. and P.-S. H. performed the

22

### Contributions.A.N. and M.B. designed and implemented the initial version of

M.B., A.N., N.V. and P.K. developed project vision and scoped problems. N.V. and P.- S.H. oversaw the practical applications. E.D. and M.E. implemented the first benchmark problem used for iterating on AlphaEvolve, with input from F.J.R.R. and M.B. A.N. and M.E. developed the final version of AlphaEvolve, with contributions from S.S., P.-S.H., and input from M.B., E.D., A.Z.W. and N.V. A.N., S.S., P.-S.H. and M.E. maintained the infrastructure underlying AlphaEvolve. M.E. and E.D. used matrix multiplication, with input from F.J.R.R. A.Z.W. worked on the applications to open mathematical problems, with help from A.M., M.E., and A.N. A.N. contributed to the Borg scheduling application. P.-S.H. and N.V. worked on the application to Gemini kernel engineering. P.-S.H. and A.N. contributed to the TPU circuit design application. B.K. and

*AlphaEvolveto discover new algorithms for*


---

majority of code reviews. M.B., E.D., S.C., N.V., A.Z.W., F.J.R.R., M.E., A.N., B.K., S.S., A.M., and M.P.K. wrote the paper, with input from A.S., P.-S.H and P.K. N.V., E.D., M.E., S.C., A.N., and A.Z.W. created the figures. F.J.R.R., A.M., and A.Z.W. assembled the accompanying Google Colab. S.N., A.D. and P.K. advised and enabled multiple strands of this work. M.B., A.N., N.V. and G.H. coordinated the team. P.K. supervised and coordinated the research program.

**Corresponding authors.Matej Balog, Alexander Novikov and Pushmeet Kohli.**

## References

- [1]J. Abramson, J. Adler, J. Dunger, R. Evans, T. Green, A. Pritzel, O. Ronneberger, L. Will- `https://deepmind.g`

more, A. J. Ballard, J. Bambrick, et al. Accurate structure prediction of biomolecular interactions with alphafold 3.Nature, 630(8016):493-500, 2024.

- [2]V. Aglietti, I. Ktena, J. Schrouff, E. Sgouritsa, F. J. R. Ruiz, A. Malek, A. Bellot, and

S. Chiappa. FunBO: Discovering acquisition functions for Bayesian optimization with FunSearch. InInternational Conference on Machine Learning

- [3]AlphaProof and AlphaGeometry teams. AI achieves silver-medal standard solving

International Mathematical Olympiad problems, 2024. URL

```
oogle/discover/blog/ai-solves-imo-problems-at-silver-medal-lev
```

- [4] D. Bahdanau, K. Cho, and Y. Bengio. Neural machine translation by jointly learning

to align and translate.arXiv preprint arXiv:1409.0473

- [5]W. Banzhaf, P. Nordin, R. E. Keller, and F. D. Francone.

*Introduction on the Automatic Evolution of computer programs and its Applications* Morgan Kaufmann Series in Artificial Intelligence, 1998.

- [6]J. Bausch, A. W. Senior, F. J. H. Heras, T. Edlich, A. Davies, M. Newman, C. Jones,

K. Satzinger, M. Y. Niu, S. Blackwell, G. Holland, D. Kafri, J. Atalaya, C. Gidney, D. Hassabis, S. Boixo, H. Neven, and P. Kohli. Learning high-accuracy error decoding for quantum processors.Nature, 635(8040):834-840, 2024. doi: 10.1038/s41586-0 24-08148-8.

- [7]D. A. Boiko, R. MacKnight, B. Kline, and G. Gomes. Autonomous chemical research

with large language models.Nature, 624(7992):570-578, 2023. doi: 10.1038/s415 86-023-06792-0.

- [8]P. Boyvalenkov, S. Dodunekov, and O. Musin. A survey on the kissing numbers.

*Math. J., 38(4):507-522, 2012. ISSN 1310-6600.*

- [9]J. Bradbury, R. Frostig, P. Hawkins, M. J. Johnson, C. Leary, D. Maclaurin, G. Necula,

A. Paszke, J. VanderPlas, S. Wanderman-Milne, and Q. Zhang. JAX: composable

, 2025.

, 2014. *Genetic Programming: An*

transformations of Python+NumPy programs, 2018. URL

```
ax-ml/jax.
http://github.com/j
```

23

*Serdica*


---

- [10]A. M. Bran, S. Cox, O. Schilter, C. Baldassari, A. D. White, and P. Schwaller. Augmenting

large language models with chemistry tools.

535, 2024. doi: 10.1038/s42256-024-00832-8.

- [11]A. M. Bran, T. A. Neukomm, D. P. Armstrong, Z. Jončev, and P. Schwaller. Chemical

reasoning in LLMs unlocks steerable synthesis planning and reaction mechanism elucidation. InarXiv preprint arXiv:2503.08537

- [12]M. Caldas Ramos, C. J. Collison, and A. D. White. A review of large language models

and autonomous agents in chemistry. 10.1039/D4SC03921A.

- [13]P. S. Castro, N. Tomasev, A. Anand, N. Sharma, R. Mohanta, A. Dev, K. Perlin, S. Jain,

K. Levin, N. Éltető, W. Dabney, A. Novikov, G. C. Turner, M. K. Eckstein, N. D. Daw, K. J. Miller, and K. L. Stachenfeld. Discovering symbolic cognitive models from human and animal behavior. InInternational Conference on Machine Learning

- [14]A. Chen, D. M. Dohan, and D. R. So. EvoPrompting: Language models for code-level

neural architecture search. InAdvances in Neural Information Processing Systems 2023.

- [15]T. Chen, B. Xu, and K. Devleker. Automating GPU kernel generation with DeepSeek-R1

and inference time scaling, 2025. URL

```
automating-gpu-kernel-generation-with-deepseek-r1-and-inference
-time-scaling.
```

- [16] X. Chen, C. Liang, D. Huang, E. Real, K. Wang, H. Pham, X. Dong, T. Luong, C.-J.

Hsieh, Y. Lu, and Q. V. Le. Symbolic discovery of optimization algorithms. *in Neural Information Processing Systems*

- [17]A. Cloninger and S. Steinerberger. On suprema of autoconvolutions with an application

to Sidon sets.Proceedings of the American Mathematical Society 2017.

- [18]H. Cohn and F. Gonçalves. An optimal uncertainty principle in twelve dimensions via

modular forms.Inventiones mathematicae

- [19]K. M. Collins, A. Q. Jiang, S. Frieder, L. Wong, M. Zilka, U. Bhatt, T. Lukasiewicz, Y. Wu,

J. B. Tenenbaum, W. Hart, et al. Evaluating language models for mathematics through interactions.Proceedings of the National Academy of Sciences 2024.

- [20]K. D. Cooper, D. Subramanian, and L. Torczon. Adaptive optimizing compilers for the

21st century.The Journal of Supercomputing

- [21]M. Cranmer. Interpretable machine learning for science with pysr and symbolicre- `https://developer.nvidia.com/blog/`

gression. jl.arXiv preprint arXiv:2305.01582

- [22]T. Dao, D. Fu, S. Ermon, A. Rudra, and C. Ré. Flashattention: Fast and memory- `https://developer.nvidia.com/blog/`

*Nature Machine Intelligence, 6(5):525-*

, 2025. *Chemical Science, 16:2514-2572, 2025. doi:*

, 2025.

*Advances*

, 2023.

, 145(8):3191-3200,

, 217:799-831, 2019.

, 121(24):e2318124121,

efficient exact attention with io-awareness.

*systems, 35:16344-16359, 2022.*

*Advances in neural information processing*

24

, 23:7-22, 2002. , 2023.


---

- [23]A. Davies, P. Veličković, L. Buesing, S. Blackwell, D. Zheng, N. Tomašev, R. Tanburn,

P. Battaglia, C. Blundell, A. Juhász, M. Lackenby, G. Williamson, D. Hassabis, and P. Kohli. Advancing mathematics by guiding human intuition with AI. (7887):70-74, 2021. doi: 10.1038/s41586-021-04086-x.

- [24]J. S. Ellenberg, C. S. Fraser-Taliente, T. R. Harvey, K. Srivastava, and A. V. Sutherland.

Generative modelling for mathematical discovery.

2025. Riveon Lematematika, 9:45-48, 1955.

- [25] P. Erdős. Some remarks on number theory.
- [26]A. Fawzi, M. Balog, A. Huang, T. Hubert, B. Romera-Paredes, M. Barekatain, A. Novikov,

F. J. R. Ruiz, J. Schrittwieser, G. Swirszcz, D. Silver, D. Hassabis, and P. Kohli. Discovering faster matrix multiplication algorithms with reinforcement learning.

610(7930):47-53, 2022. doi: 10.1038/s41586-022-05172-4.

- [27]C. Fernando, D. Banarse, H. Michalewski, S. Osindero, and T. Rocktäschel. Prompt- *arXiv preprint arXiv:2503.11061,*

breeder: Self-referential self-improvement via prompt evolution.

*arXiv:2309.16797, 2023.*

- [28]N. Ferruz and B. Höcker. Controllable protein design with language models.

*Machine Intelligence, 4(6):521-532, 2022.*

- [29]E. Friedman. Erich's Packing Center. `https://erich-friedman.github.io/pa`

```
cking/, 2025. Accessed: 2025-04-22.
```

- [30]F. Frohnert, X. Gu, M. Krenn, and E. van Nieuwenburg. Discovering emergent connec- *arXiv preprint arXiv:2503.11061,*

tions in quantum physics research via dynamic word embeddings. *Science and Technology, 6(1):015029, 2025. doi: 10.1088/2632-2153/adb00a.*

- [31] M. Ganzhinov. Highly symmetric lines. In
- [32]Gemini team. Gemini 2.5: Our most intelligent AI model, 2025. URL

```
//blog.google/technology/google-deepmind/gemini-model-thinking-u
pdates-march-2025.
```

- [33] F. Gonçalves, D. O. e Silva, and S. Steinerberger. Hermite polynomials, linear flows

on the torus, and an uncertainty principle for roots. *and Applications, 451(2):678-711, 2017.*

- [34]J. Gottweis, W.-H. Weng, A. Daryin, T. Tu, A. Palepu, P. Sirkovic, A. Myaskovsky,

F. Weissenberger, K. Rong, R. Tanno, K. Saab, D. Popovici, J. Blum, F. Zhang, K. Chou, A. Hassidim, B. Gokturk, A. Vahdat, P. Kohli, Y. Matias, A. Carroll, K. Kulkarni, N. Tomasev, Y. Guan, V. Dhillon, E. D. Vaishnav, B. Lee, T. R. D. Costa, J. R. Penadés, G. Peltz, Y. Xu, A. Pawlosky, A. Karthikesalingam, and V. Natarajan. Towards an AI

co-scientist.arXiv preprint arXiv:2502.18864

- [35]A. Grayeli, A. Sehgal, O. Costilla Reyes, M. Cranmer, and S. Chaudhuri. Symbolic

*Nature, 600*

*Nature,*

*arXiv preprint*

*Nature*

*Machine Learning:*

*arXiv preprint arXiv:2207.08266v1, 2022.*

```
https:
```

*Journal of Mathematical Analysis*

regression with a learned concept library. *Systems, 37:44678-44709, 2024.*

*Advances in Neural Information Processing*

25

, 2025.


---

- [36]M. Gridach, J. Nanavati, C. Mack, K. Z. E. Abidine, and L. Mendes. Agentic AI for

scientific discovery: A survey of progress, challenges, and future directions. In *Workshop: Towards Agentic AI for Science: Hypothesis Generation, Comprehension, Quantification, and Validation, 2025.*

- [37]X. Gu and M. Krenn. Interesting scientific idea generation using knowledge

graphs and LLMs: Evaluations with 100 research group leaders. In

*arXiv:2405.17044, 2024.*

- [38]S. Guo, A. H. Shariatmadari, G. Xiong, and A. Zhang. Embracing foundation models

for advancing scientific discovery. In *Proceedings of the IEEE International Conference on Big Data, pages 1746-1755, 2024. doi: 10.1109/bigdata62323.2024.10825618.*

- [39] K. Gyarmati, F. Hennecart, and I. Z. Ruzsa. Sums and differences of finite sets.

*Functiones et Approximatio Commentarii Mathematici*

- [40]J. K. Haugland. The minimum overlap problem revisited.

*arXiv:1609.08000, 2016.*

- [41]E. Hemberg, S. Moskal, and U.-M. O'Reilly. Evolving code with a large language

model.Genetic Programming and Evolvable Machines s10710-024-09494-2.

- [42]J. E. Hopcroft and L. R. Kerr. On minimizing the number of multiplications necessary

for matrix multiplication.SIAM J. Appl. Math.

1399. doi: 10.1137/0120004.
- [43]K. Huang, Y. Qu, H. Cousins, W. A. Johnson, D. Yin, M. Shah, D. Zhou, R. Altman,

M. Wang, and L. Cong. CRISPR-GPT: An LLM agent for automated design of gene- editing experiments. InarXiv preprint arXiv:2404.18021

- [44]L. Huang, W. Yu, W. Ma, W. Zhong, Z. Feng, H. Wang, Q. Chen, W. Peng, X. Feng,

B. Qin, et al. A survey on hallucination in large language models: Principles, taxonomy, challenges, and open questions.ACM Transactions on Information Systems 1-55, 2025.

- [45]S. Jia, C. Zhang, and V. Fung. LLMatDesign: Autonomous materials discovery with

large language models. InarXiv preprint arXiv:2406.13163

- [46]J. Jumper, R. Evans, A. Pritzel, T. Green, M. Figurnov, O. Ronneberger, K. Tunyasuvu- *arXiv preprint*

nakool, R. Bates, A. Žídek, A. Potapenko, A. Bridgland, C. Meyer, S. A. A. Kohl, A. J. Ballard, A. Cowie, B. Romera-Paredes, S. Nikolov, R. Jain, J. Adler, T. Back, S. Petersen, 

Highly accurate protein structure prediction with AlphaFold.

583-589, 2021. doi: 10.1038/s41586-021-03819-2.

- [47]M. Kauers and J. Moosbauer. Flip graphs for matrix multiplication. In

*ICLR*

*arXiv preprint*

, 37(1):175-186, 2007.

, 25(2):21, 2024. doi: 10.1007/

, 20(1):30-36, Jan. 1971. ISSN 0036-

, 2024.

, 43(2):

, 2024.

*of the 2023 International Symposium on Symbolic and Algebraic Computation* 381-388, 2023.

, pages

26

*Nature, 596(7873):*

*Proceedings*


---

- [48]M. Kauers and J. Moosbauer. Some new non-commutative matrix multiplication

algorithms of size(, ,6).ACM Commun. Comput. Algebra ISSN 1932-2232. doi: 10.1145/3712020.3712021.

- [49]M. Kauers and I. Wood. Consequences of the Moosbauer-Poole algorithms.

*preprint arXiv:2505.05896, 2025.*

- [50] D. P. Kingma and J. Ba. Adam: A method for stochastic optimization. In

*Conference on Learning Representations (ICLR)*

- [51] J. R. Koza. Genetic programming as a means for programming computers by natural

selection.Statistics and Computing, 4(2):87-112, 1994. doi: 10.1007/BF00175355.

- [52]J. D. Laderman. A noncommutative algorithm for multiplying

23 multiplications.Bulletin of the American Mathematical Society 1976.

- [53]R. Lam, A. Sanchez-Gonzalez, M. Willson, P. Wirnsberger, M. Fortunato, F. Alet,

S. Ravuri, T. Ewalds, Z. Eaton-Rosen, W. Hu, A. Merose, S. Hoyer, G. Holland, O. Vinyals, J. Stott, A. Pritzel, S. Mohamed, and P. Battaglia. Learning skillful medium-range global weather forecasting. 10.1126/science.adi2336.

- [54]W. B. Langdon and R. Poli.Foundations of genetic programming

Business Media, 2013.

- [55]R. Lange, Y. Tian, and Y. Tang. Large language models as evolution strategies. In

*Proceedings of the Genetic and Evolutionary Computation Conference Companion* '24 Companion, pages 579-582. Association for Computing Machinery, 2024. doi: 10.1145/3638530.3654238.

- [56]R. T. Lange, A. Prasad, Q. Sun, M. Faldor, Y. Tang, and D. Ha. The AI CUDA engineer:

Agentic CUDA kernel discovery, optimization and composition. Technical report, Sakana AI, 02 2025.

- [57]J. Lehman, J. Gordon, S. Jain, K. Ndousse, C. Yeh, and K. O. Stanley. Evolution through

large models. InHandbook of evolutionary machine learning 2023.

- [58]J. Lehman, J. Gordon, S. Jain, K. Ndousse, C. Yeh, and K. O. Stanley.

*Through Large Models, pages 331-366. Springer Nature Singapore, 2024. doi:* 10.1007/978-981-99-3814-8\_11.

- [59]P.-H. Li, Y.-Y. Sun, H.-F. Juan, C.-Y. Chen, H.-K. Tsai, and J.-H. Huang. A large language

model framework for literature-based disease-gene association prediction. *Bioinformatics, 26(1):bbaf070, 02 2025. ISSN 1477-4054. doi: 10.1093/bib/bbaf070.*

- [60]Y. Li, D. Choi, J. Chung, N. Kushman, J. Schrittwieser, R. Leblond, T. Eccles, J. Keeling,

, 58(1):1-11, Jan. 2025.

*arXiv*

*International*

, 2015.

3×3matrices using , 82(1):126 - 128,

*Science, 382(6677):1416-1421, 2023. doi:*

, GECCO

F. Gimeno, A. D. Lago, T. Hubert, P. Choy, C. de Masson d'Autume, I. Babuschkin, X. Chen, P.-S. Huang, J. Welbl, S. Gowal, A. Cherepanov, J. Molloy, D. J. Mankowitz,

27

, pages 331-366. Springer,

*Evolution*

*Briefings in*


---

E. S. Robson, P. Kohli, N. de Freitas, K. Kavukcuoglu, and O. Vinyals. Competition- level code generation with AlphaCode. 10.1126/science.abq1158.

- [61]H. Lin, M. Maas, M. Roquemore, A. Hasanzadeh, F. Lewis, Y. Simonson, T.-W. Yang,

A. Yazdanbakhsh, D. Altinbüken, F. Papa, et al. ECO: An LLM-driven efficient code optimizer for warehouse scale computers.

- [62]Z. Lin, H. Akin, R. Rao, B. Hie, Z. Zhu, W. Lu, N. Smetanin, R. Verkuil, O. Kabeli,

Y. Shmueli, A. dos Santos Costa, M. Fazel-Zarandi, T. Sercu, S. Candido, and A. Rives. Evolutionary-scale prediction of atomic-level protein structure with a language model. *Science, 379(6637):1123-1130, 2023. doi: 10.1126/science.ade2574.*

- [63] F. Liu, X. Tong, M. Yuan, X. Lin, F. Luo, Z. Wang, Z. Lu, and Q. Zhang. Evolution of

heuristics: Towards efficient automatic algorithm design using large language model. *arXiv preprint arXiv:2401.02051, 2024.*

- [64] F. Luo, J. Zhang, Q. Wang, and C. Yang. Leveraging prompt engineering in large

language models for accelerating chemical research. 10.1021/acscentsci.4c01935.

- [65]Z. Luo, Z. Yang, Z. Xu, W. Yang, and X. Du. LLM4SR: A survey on large language

models for scientific research. InarXiv preprint arXiv:2501.04306

- [66]H. Ma, A. Narayanaswamy, P. Riley, and L. Li. Evolving symbolic density functionals.

*Science Advances, 8(36):eabq0279, 2022. doi: 10.1126/sciadv.abq0279.*

- [67]A. Madani, B. Krause, E. R. Greene, S. Subramanian, B. P. Mohr, J. M. Holton, J. L.

Olmos, C. Xiong, Z. Z. Sun, R. Socher, J. S. Fraser, and N. Naik. Large language models generate functional protein sequences across diverse families. (8):1099-1106, August 2023. ISSN 1087-0156. doi: 10.1038/s41587-022-01618-2.

- [68]D. J. Mankowitz, A. Michi, A. Zhernov, M. Gelmi, M. Selvi, C. Paduraru, E. Leurent,

S. Iqbal, J.-B. Lespiau, A. Ahern, T. Köppe, K. Millikin, S. Gaffney, S. Elster, J. Broshear, C. Gamble, K. Milan, R. Tung, M. Hwang, T. Cemgil, M. Barekatain, Y. Li, A. Mandhane, T. Hubert, J. Schrittwieser, D. Hassabis, P. Kohli, M. Riedmiller, O. Vinyals, and D. Silver. Faster sorting algorithms discovered using deep reinforcement learning. (7964):257-263, 2023. doi: 10.1038/s41586-023-06004-9.

- [69]H. Massalin. Superoptimizer - A look at the smallest program. In R. H. Katz and

M. Freeman, editors,Proceedings of the Second International Conference on Architectural *Support for Programming Languages and Operating Systems (ASPLOS II), Palo Alto, California, USA, October 5-8, 1987, pages 122-126. ACM Press, 1987. doi: 10.1145/* 36206.36194.

- [70]M. Matolcsi and C. Vinuesa. Improved bounds on the supremum of autoconvolutions.

*Journal of mathematical analysis and applications*

*Science, 378(6624):1092-1097, 2022. doi:*

*arXiv preprint arXiv:2503.15669, 2025.*

*ACS Central Science, 2025. doi:*

, 2025.

*Nature Biotechnology, 41*

*Nature, 618*

- [71]S. Miret and N. M. A. Krishnan. Are LLMs ready for real-world materials discovery?

InarXiv preprint arXiv:2402.05200, 2024.

28

, 372(2):439-447, 2010.


---

- [72]J. Moosbauer and M. Poole. Flip graphs with symmetry and new matrix multiplication

schemes.arXiv preprint arXiv:2502.04514

- [73] C. Morris, M. Jurado, and J. Zutty. Llm guided evolution-the automation of mod-*arXiv preprint*

els advancing models. InProceedings of the Genetic and Evolutionary Computation *Conference, pages 377-384, 2024.*

- [74] J.-B. Mouret and J. Clune. Illuminating search spaces by mapping elites.

*arXiv:1504.04909, 2015.*

- [75] V. Naumov, D. Zagirova, S. Lin, Y. Xie, W. Gou, A. Urban, N. Tikhonova, K. Alawi,

M. Durymanov, F. Galkin, S. Chen, D. Sidorenko, M. Korzinkin, M. Scheibye-Knudsen, A. Aspuru-Guzik, E. Izumchenko, D. Gennert, F. W. Pun, M. Zhang, P. Kamya, A. Aliper, F. Ren, and A. Zhavoronkov. DORA AI scientist: Multi-agent virtual research team for scientific exploration discovery and automated report generation. In

*preprint:10.1101/2025.03.06.641840*

10.1101/2025.03.06.641840.

- [76] OpenAI. Introducing OpenAI o3 and o4-mini, 2025. URL

```
ndex/introducing-o3-and-o4-mini/
```

- [77]OpenXLA. XLA: composable transformations of Python+NumPy programs. URL

```
https://github.com/openxla/xla
```

- [78]H. Pan, N. Mudur, W. Taranto, M. Tikhanovskaya, S. Venugopalan, Y. Bahri, M. P.

Brenner, and E.-A. Kim. Quantum many-body physics calculations with large language models.Communications Physics, 8(1):49, 2025. doi: 10.1038/s42005-025-01956-y.

- [79]D. Pantiukhin, B. Shapkin, I. Kuznetsov, A. A. Jost, and N. Koldunov. Accelerating Earth

science discovery via multi-agent LLM systems. In 2025.

- [80]Z. Rasheed, M. Waseem, A. Ahmad, K.-K. Kemell, W. Xiaofeng, A. N. Duc, and P. Abra- *bioRxiv*

hamsson. Can large language models serve as data analysts? a multi-agent assisted approach for qualitative data analysis.

- [81]S. Ren, P. Jian, Z. Ren, C. Leng, C. Xie, and J. Zhang. Towards scientific intelligence:

A survey of LLM-based scientific agents. In

- [82]A. Rives, J. Meier, T. Sercu, S. Goyal, Z. Lin, J. Liu, D. Guo, M. Ott, C. L. Zitnick, J. Ma,

and R. Fergus. Biological structure and function emerge from scaling unsupervised learning to 250 million protein sequences. *Sciences, 118(15):e2016239118, 2021. doi: 10.1073/pnas.2016239118.*

- [83]B. Romera-Paredes, M. Barekatain, A. Novikov, M. Balog, M. P. Kumar, E. Dupont,

F. J. R. Ruiz, J. Ellenberg, P. Wang, O. Fawzi, P. Kohli, and A. Fawzi. Mathematical discoveries from program search with large language models.

468-475, 2023. doi: 10.1038/s41586-023-06924-6.

, 2025.

. Cold Spring Harbor Laboratory, 2025. doi:

```
https://openai.com/i
```

.

.

*arXiv preprint arXiv:2503.05854,*

*arXiv preprint arXiv:2402.01386, 2024.*

*arXiv preprint arXiv:2503.24047, 2025.*

*Proceedings of the National Academy of*

29

*Nature, 625(7995):*


---

- [84]F. J. R. Ruiz, T. Laakkonen, J. Bausch, M. Balog, M. Barekatain, F. J. H. Heras,

A. Novikov, N. Fitzpatrick, B. Romera-Paredes, J. van de Wetering, A. Fawzi, K. Meichanetzidis, and P. Kohli. Quantum circuit optimization with AlphaTensor. Machine Intelligence, 7(3):374-385, 2025. doi: 10.1038/s42256-025-01001-1.

- [85]O. A. Sarumi and D. Heider. Large language models and their applications in bioinfor-

matics.Computational and Structural Biotechnology Journal ISSN 2001-0370. doi: https://doi.org/10.1016/j.csbj.2024.09.031.

- [86]E. Schkufza, R. Sharma, and A. Aiken. Stochastic superoptimization. In V. Sarkar and

R. Bodík, editors,Architectural Support for Programming Languages and Operating *Systems, ASPLOS 2013, Houston, TX, USA, March 16-20, 2013*

2013. doi: 10.1145/2451116.2451150.
- [87]M. Schmidt and H. Lipson. Distilling free-form natural laws from experimental data.

*Science, 324(5923):81-85, 2009. doi: 10.1126/science.1165893.*

- [88] N. Shinn, F. Cassano, A. Gopinath, K. Narasimhan, and S. Yao. Reflexion: Language

agents with verbal reinforcement learning. *Advances in Neural Information Processing Systems, 36:8634-8652, 2023.*

- [89]P. Shojaee, K. Meidani, S. Gupta, A. B. Farimani, and C. K. Reddy. LLM-SR: Scientific

equation discovery via programming with large language models. In *Conference on Learning Representations*, 2025.

- [90]C. Si, D. Yang, and T. Hashimoto. Can LLMs generate novel research ideas? a large- *arXiv*

scale human study with 100+ NLP researchers. In *Representations, 2025.*

- [91]A. Smirnov. Several bilinear algorithms for matrix multiplication problems

```
https://www.researchgate.net/publication/350897049_Several_Bilin
ear_Algorithms_for_Matrix_Multiplication_Problems_3_P_Q
```

- [92]A. Smirnov. Bilinear algorithm for matrix multiplication

irrational solution of the brent system? `https://www.researchgate.net/pub`

```
lication/364167198_Bilinear_Algorithm_for_Matrix_Multiplication_
```

### `4x4x9_104_An_irreducibly_irrational_solution_of_the_Brent_system`

10 2022.

- [93]A. V. Smirnov. The bilinear complexity and practical algorithms for matrix multipli- *arXiv*

cation.Computational Mathematics and Mathematical Physics 2013.

- [94]Z. Song, M. Ju, C. Ren, Q. Li, C. Li, Q. Zhou, and J. Wang. LLM-Feynman: Leveraging

large language models for universal scientific formula and theory discovery. In

*preprint arXiv:2503.06512, 2025.*

- [95]V. Strassen. Gaussian elimination is not optimal.

*Nature*

, 23:3498-3505, 2024.

, pages 305-316. ACM,

*International*

*International Conference on Learning*

⟨3, , ⟩. , 04 2021.

⟨4×4×9; 104⟩. an irreducibly

354-356, 1969.

30

, 53(12):1781-1795,

*Numerische mathematik, 13(4):*


---

- [96]A. Surina, A. Mansouri, L. Quaedvlieg, A. Seddas, M. Viazovska, E. Abbe, and C. Gul-

cehre. Algorithm discovery with LLMs: Evolutionary search meets reinforcement learning. InarXiv preprint arXiv:2504.05108

- [97]R. Tanese.Distributed genetic algorithms for function optimization

Michigan, 1989.

- [98] A. Thakur, G. Tsoukalas, Y. Wen, J. Xin, and S. Chaudhuri. An in-context learning

agent for formal theorem-proving. In *Conference on Language Models, 2024.*

- [99] T. H. Trinh, Y. Wu, Q. V. Le, H. He, and T. Luong. Solving olympiad geometry without

human demonstrations.Nature, 625(7995):476-482, 2024.

- [100]A. Vaswani, N. Shazeer, N. Parmar, J. Uszkoreit, L. Jones, A. N. Gomez, L. Kaiser, and

I. Polosukhin. Attention is all you need. In *Systems, 2017.*

- [101]P. Veličković, A. Vitvitskyi, L. Markeeva, B. Ibarz, L. Buesing, M. Balog, and A. Novikov.

Amplifying human performance in combinatorial competitive programming. 2024.

- [102]A. Verma, L. Pedrosa, M. Korupolu, D. Oppenheimer, E. Tune, and J. Wilkes. Large- *arXiv preprint arXiv:2308.08155,*

scale cluster management at Google with Borg. In *Conference on Computer Systems, EuroSys '15, New York, NY, USA, 2015. Association* for Computing Machinery. ISBN 9781450332385. doi: 10.1145/2741948.2741964.

- [103]S. Verma, A. Goyal, A. Mathur, A. Anand, and S. Ranu. GRAIL: Graph edit distance

and node alignment using llm-generated code. In *Learning, 2025.*

- [104]C. Vinuesa del Rio.Generalized Sidon sets

Madrid, 2010.

- [105]H. Wang, M. Skreta, C. T. Ser, W. Gao, L. Kong, F. Strieth-Kalthoff, C. Duan, Y. Zhuang,

Y. Yu, Y. Zhu, Y. Du, A. Aspuru-Guzik, K. Neklyudov, and C. Zhang. Efficient evolutionary search over chemical space with large language models. In Conference on Learning Representations

- [106]Q. Wang, D. Downey, H. Ji, and T. Hope. SciMON: Scientific inspiration machines

optimized for novelty. InProceedings of the 62nd Annual Meeting of the Association for *Computational Linguistics (Volume 1: Long Papers)*

  2024. Association for Computational Linguistics. doi: 10.18653/v1/2024.acl-long.18.
- [107]E. P. White. A new bound for Erdős' minimum overlap problem.

208:235-255, 2023.

- [108]Q. Wu, G. Bansal, J. Zhang, Y. Wu, B. Li, E. Zhu, L. Jiang, X. Zhang, S. Zhang, J. Liu,

A. H. Awadallah, R. W. White, D. Burger, and C. Wang. AutoGen: Enabling next-gen LLM applications via multi-agent conversation. In

, 2025.

*Advances in Neural Information Processing*

*Proceedings of the Tenth European*

*International Conference on Machine*

. PhD thesis, Universidad Autónoma de

*International*

, 2025.

, pages 279-299, Bangkok, Thailand,

*Acta Arithmetica,*

2023. LLM applications via multi-agent conversation. In

. University of

31


---

- [109]Y. Xia, P. Jin, S. Xie, L. He, C. Cao, R. Luo, G. Liu, Y. Wang, Z. Liu, Y.-J. Chen, Z. Guo,

Y. Bai, P. Deng, Y. Min, Z. Lu, H. Hao, H. Yang, J. Li, C. Liu, J. Zhang, J. Zhu, R. Bi, K. Wu, W. Zhang, K. Gao, Q. Pei, Q. Wang, X. Liu, Y. Li, H. Zhu, Y. Lu, M. Ma, Z. Wang, T. Xie, K. Maziarz, M. Segler, Z. Yang, Z. Chen, Y. Shi, S. Zheng, L. Wu, C. Hu, P. Dai, T.-Y. Liu, H. Liu, and T. Qin. Nature language model: Deciphering the language of nature for scientific discovery. InarXiv preprint arXiv:2502.07527v2

- [110]K. Yang, A. Swope, A. Gu, R. Chalamala, P. Song, S. Yu, S. Godil, R. J. Prenger, and

A. Anandkumar. Leandojo: Theorem proving with retrieval-augmented language models.Advances in Neural Information Processing Systems

- [111]K. Yang, G. Poesia, J. He, W. Li, K. Lauter, S. Chaudhuri, and D. Song. Formal

mathematical reasoning: A new frontier in AI.

- [112]Z. Yang, X. Du, J. Li, J. Zheng, S. Poria, and E. Cambria. Large language models for

automated open-domain scientific hypotheses discovery. In *tion for Computational Linguistics: ACL 2024* Computational Linguistics, 2024. doi: 10.18653/v1/2024.findings-acl.804.

- [113]Z. Yang, W. Liu, B. Gao, T. Xie, Y. Li, W. Ouyang, S. Poria, E. Cambria, and D. Zhou.

MOOSE-Chem: Large language models for rediscovering unseen chemistry scientific hypotheses. InInternational Conference on Learning Representations

- [114]S. Yao, J. Zhao, D. Yu, N. Du, I. Shafran, K. Narasimhan, and Y. Cao. React: Synergizing

reasoning and acting in language models. In *Representations (ICLR), 2023.*

- [115]S. Yao, F. Liu, X. Lin, Z. Lu, Z. Wang, and Q. Zhang. Multi-objective evolution of

heuristic using large language model. In volume 39, pages 27144-27152, 2025. doi: 10.1609/aaai.v39i25.34922.

- [116]G. Ye, X. Cai, H. Lai, X. Wang, J. Huang, L. Wang, W. Liu, and X. Zeng. DrugAssist: A

large language model for molecule optimization. In 2023.

- [117]H. Ye, J. Wang, Z. Cao, F. Berto, C. Hua, H. Kim, J. Park, and G. Song. ReEvo: Large

language models as hyper-heuristics with reflective evolution. In *Information Processing Systems, volume 37, 2024.*

- [118]F. Zhang, S. Nguyen, Y. Mei, and M. Zhang.

*Scheduling. Springer, 2021.*

- [119]H. Zhang, Y. Song, Z. Hou, S. Miret, and B. Liu. HoneyComb: A flexible LLM-based

agent system for materials science. In Y. Al-Onaizan, M. Bansal, and Y.-N. Chen, editors,Findings of the Association for Computational Linguistics: EMNLP 2024

3369-3382. Association for Computational Linguistics, Nov. 2024. doi: 10.18653/v1/ 2024.findings-emnlp.192.

, 2025.

, 36:21573-21612, 2023.

*arXiv preprint arXiv:2412.16075, 2024.*

*Findings of the Associa-*

, pages 13545-13565. Association for

, 2025.

*International Conference on Learning*

*AAAI Conference on Artificial Intelligence,*

*arXiv preprint arXiv:2401.10334,*

*Advances in Neural*

*Genetic Programming for Production*

- [120] Y. Zhou, H. Liu, T. Srivastava, H. Mei, and C. Tan. Hypothesis generation with large

language models. In L. Peled-Cohen, N. Calderon, S. Lissak, and R. Reichart, editors,

32

, pages


---

*Proceedings of the 1st Workshop on NLP for Science (NLP4Science)*, pages 117-139. Association for Computational Linguistics, 2024. doi: 10.18653/v1/2024.nlp4scien ce-1.10.

33


---

## A. Faster matrix multiplication: Full results

**Full table of results.We provide the best ranks obtained by**

we considered 54 matrix multiplication sizes in our experiments. These were chosen roughly representing sizes⟨, , ⟩where2≤, ≤5, with some reasonable cutoff for. Due to symmetries of the underlying matrix multiplication tensor, there exist equivalent algorithms for any permutations of the three axes, hence we focus on sorted sizes In all but two considered sizes,AlphaEvolve surpass the best known rank. Anecdotally, we encountered some difficulty when increasing the problem size: when we run the discovered programs on sizes beyond random seeds on evaluators with a single GPU accelerator, we often run out of memory. Hence, extending our setup to larger matrix sizes requires further optimization.

**Table 3|Full version of Table 2, showing the best ranks obtained by**

| decomposition | for | all | considered | parameters. | Of | the | 54 | targets, |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ⟨2,2,2⟩AlphaEvolveuse complex-valued multiplications which can be used for exact multiplication7 [95⟨2,]3,6⟩30 [7⟨93,4],4⟩ 38 [3093] | 38 |  |  |  |  |  |  |  |
| ⟨2,2,4⟩of complex or real-valued matrices. The decompositions shown in this table can be found in14 [⟨932,]3,8⟩1440 [⟨93,4],6⟩ 56 [4048] | 54 |  |  |  |  |  |  |  |
| ⟨2,2,5⟩the accompanying Google Colab.18 [⟨932,]3,9⟩1845 [⟨93,4],7⟩ 66 [4591] | 63 |  |  |  |  |  |  |  |
| ⟨2,2,8⟩ Note: Concurrent work [49] has also found a rank-28 [⟨932,]4,5⟩ 2833 [⟨423,5],6⟩ 70 [3248] | 68 |  |  |  |  |  |  |  |
| ⟨2,2,13⟩ 46 [⟨932,]5,6⟩ Figure 4 (left), which corresponds to the program that discovers a decomposition of rank 48 4648 [⟨934,4],7⟩87 [4793, 95] | 85 |  |  |  |  |  |  |  |

discovered programs which either match or

⟨5,5,5⟩on 1000

*AlphaEvolvefor tensor AlphaEvolvematches the*

108

⟨2,3,5⟩ 25 [⟨93,]3,8⟩ 2555 [⟨936,6],6⟩ 153 [5572] 156

34

90algorithm for⟨4,5,6⟩.

4×4matrices.


---

```
# EVOLVE -BLOCK -START
def _get_optimizer (self) -> optax. GradientTransformation :
""" Returns optimizer ."""
```

5 return optax.adam(self.hypers. learning_rate )

6 + return `optax.adamw(`

7 + self

```
.hypers.learning_rate , weight_decay
```

8 + )

```
def _get_init_fn (self) -> jax.nn. initializers .Initializer :
""" Returns initializer function ."""
solutions.
decomposition = optax. apply_updates (decomposition , updates)
```

22 + # `Add a small amount of gradient noise to help with exploration`

24 + decomposition 25 + lambda

```
shape),
```

27 + decomposition 28 + ) 30 + # `Add noise to the decomposition parameters`

33 + global_step

```
, start=self.hypers
```

34 + ) 35 + decomposition 36 + lambda 37 + decomposition 38 + ) 41 + cycle_length

```
= 2000 # Number of steps per cycle
```

42 + cycle_progress 43 + global_step

```
% cycle_length
```

46 + # `Map cycle progress to a sinusoidal`

47 + clip_threshold_multiplier 49 + clip_threshold 50 + self

```
.hypers.clip_max - self.hypers.clip_min
```

51 + )

58 + x_re_clipped

```
= jnp.where(
```

59 + x_re 60 + ) 61 + x_re_clipped

```
= jnp.where(
```

62 + x_re_clipped

```
< -threshold ,
```

64 + x_re_clipped 65 + )

```
=self.hypers. weight_decay
to encourage finding low -rank
exploration.
( exploration ).
.noise_std , end =0.0
.random.normal(noise_rng , x.shape),
threshold .
curve. Ranges from 0 to 1.
parts separately.
```

1 @@ -45,9 `+45 ,14 @@`

10 11

17 19 20 21

**Figure 9a|Magnified version of Figure 4**

algorithm to multiply4×4matrices (1/3).

(left), giving the program that discovers a faster

35


---

67 + x_im_clipped

```
= jnp.where(
```

68 + x_im 69 + ) 70 + x_im_clipped

```
= jnp.where(
```

71 + x_im_clipped

```
< -threshold ,
```

73 + x_im_clipped 74 + ) 76 + return 78 + decomposition 79 + lambda

```
x: soft_clip (x, clip_threshold
```

80 + ) 83 84 `def _loss_fn(`

86 `""" Computes ( batched ) loss on learned decomposition`

92 + target_noise

```
= self.hypers. target_noise_std
```

93 + noise_rng

```
, self. target_tensor
```

94 + ) 95 + noisy_target_tensor

```
values)
```

98 + hallucination_prob

```
= self.hypers. hallucination_prob
```

99 + hallucination_scale

```
= self.hypers. hallucination_scale
```

101 + def `hallucinate (x, hallucination_rng`

104 + hallucination_rng

```
, x.shape
```

105 + ) 106 + return 109 + decomposition 110 + lambda

```
x: hallucinate (x, jax.random.split(factor_rng )[0]) ,
```

111 + decomposition 112 + )

```
# Add a batch dimension to`target_tensor
broadcasting .
# Define the loss as the L2 reconstruction
```

116 rec_loss = l2_loss_complex (self. target_tensor [None , ...] , rec_tensor

```
# We must return a real - valued loss.
```

120 return jnp.real(rec_loss)

122 + # `Discretization loss (encourage entries to be multiples of 1/2 or`

```
integer).
```

123 + def `dist_to_half_ints (x):`

126 + return

```
jnp.minimum(
```

127 + jnp 128 + jnp 129 + )

```
), decomposition
."""
(rng)
```

                            - `* jax.random.normal(`

```
.shape
by randomly replacing
`to ensure correct
error.
```

114 115 118 119 121

**Figure 9b|Magnified version of Figure 4**

algorithm to multiply4×4matrices (2/3).

(left), giving the program that discovers a faster

36


---

131 + def `dist_to_ints (x):`

132 + return

```
jnp.abs(x - jnp.round (x))
```

134 + discretization_loss

```
= 0.0
```

135 + for `factor in decomposition :`

136 + discretization_loss

```
+= jnp.mean( dist_to_half_ints
```

137 + discretization_loss

```
+= jnp.mean( dist_to_ints (factor))
```

139 + discretization_loss 140 + len 141 + ) `# average across all factors and loss components`

143 + discretization_weight

```
= self. _linear_schedule
```

144 + global_step

```
, start =0.0 , end =self.hypers. discretization_weight
```

145 + ) 147 + # `Cosine annealing for half -integer`

148 + cycle_length

```
= self.config. training_steps
cycle
```

149 + cycle_progress 150 + global_step

```
% cycle_length
```

151 + ) `/ cycle_length # Normalized progress within the current cycle [0,`

152 + half_int_multiplier 153 + half_int_multiplier

154 + 1 self.hypers. half_int_start

157 + total_loss 158 + rec_loss

```
half_int_multiplier
```

160 + ) 162 + # `Add penalty for large values (stability ).`

163 + large_value_penalty

```
= 0.0
```

164 + for `factor in decomposition :`

165 + large_value_penalty 166 + large_value_penalty 167 + total_loss

```
+= self.hypers. large_value_penalty_weight
large_value_penalty
def l2_loss_complex (x: jnp.ndarray , y: jnp.ndarray) -> jnp.ndarray:
""" Elementwise L2 loss for complex
return hyper.zipit ([
```

180 + hyper`.uniform('discretization_weight`

181 + hyper`.uniform('hallucination_prob`

182 + hyper`.uniform('hallucination_scale`

183 + hyper`.uniform('noise_std', hyper`

184 + hyper`.uniform('target_noise_std`

186 + hyper`.uniform('clip_min', hyper`

187 + hyper`.uniform('clip_max', hyper`

188 + hyper`.uniform('large_value_penalty_weight`

```
0.01)),
```

189 + # `Add noise to the gradient to aid in exploration`

```
# EVOLVE -BLOCK -END
(factor))
loss.
// 4 # Number of steps per
)) / 2
numbers ."""
.interval (0.2 , 1.5)),
.interval (0.1 , 1.0)),
.interval (0.0 , 0.01)),
.interval (0.0 , 0.5)),
.interval (1.0 , 3.0)),
.
```

171 172 173 174 @@ -117,6 `+255 ,18 @@`

175

192 193

**Figure 9c|Magnified version of Figure 4**

algorithm to multiply4×4matrices (3/3). Herehyperis a user-provided library for generating hyperparameter sweeps.

(left), giving the program that discovers a faster

37


---

## B. Details of mathematical discoveries of

The data and verification code for all constructions reported in this section appear in the accompanying Google Colab.

### B.1. First autocorrelation inequality

For any function:ℝ→ℝ, define theautoconvolution

∗()B

Let denote the largest constant satisfying

max ∗() ≥

−1/2≤≤1/2

for all non-negative:ℝ→ℝ. This problem arises in additive combinatorics, relating to the size of Sidon sets. It is currently known that

1.28≤

with the lower bound achieved in [17] and the upper bound achieved in [ function construction.AlphaEvolvefound a step function with 600 equally-spaced intervals on[−1/4,1/4]that gives a slightly better upper bound

### B.2. Second autocorrelation inequality

Let be the smallest constant for which one has

∥∗∥

for all non-negative:ℝ→ℝ. It is known that

0.88922≤

with the lower bound coming from a step function construction [ a step function with 50 equally-spaced intervals on lower bound0.8962≤.2

## *AlphaEvolve*

(− )().

−1/4

≤1.5098,

70] via a step

≤1.5053.

≤ ∥∗∥∥∗∥ ∞

≤1

70].AlphaEvolvefound

[−1/4,1/4]that gives a slightly better

75].AlphaEvolvefound a step function with 400 equally-spaced intervals on gives a slightly better upper bound≤1

3

[−1/4,1/4]that

3.4557.

38

### B.3. Third autocorrelation inequality

Let 3be the largest constant satisfying

max |∗()≥

−1/2≤≤1/2

for any function:ℝ→ ℝ. Clearly≤ negative values. There is a step function that gives the upper bound

| 3 ()

−1/4

3, since we now allowto take positive and

≤1.45810[104, page


---

### B.4. An uncertainty inequality

Given a function:ℝ→ℝ, define the Fourier transform

Let 4be the largest constant for which one has

()() ≥4

for all evenwithmax((0),(0))<0. It is known [ ˆ

0.2025≤

(The upper bound is stated as0.353in the paper, but rounding their solution to the fourth digit gives0.3523). We improved the upper bound to combination as in [33], but with refined constants that were found by To obtain upper bounds for, one constructs a specific "test function" 4

conditions and calculates the value() ()for this function, which provides an upperˆ bound 4≤ ()(). Following the approach in [ ˆ ()= ()−, where()is an even polynomial constructed as a linear combination of Hermite polynomials(). This form is particularly useful because the Fourier transform of4 ()−is ()−. For an even polynomial of()is()= ˆ 4 4() =( 4

to the largest positive root of(), and ()is related to the largest positive root of().ˆ Specifically, if() ≥0for large||,()is the largest positive root of largest positive root of(), implying() = (). The inequality becomes≤ (()).4 The method involves finding coefficients 0

41()+ ()+. . . such that()satisfies certain constraints (related to and being positive for large||) and minimizes the largest positive root of approach, the polynomial()is constructed such that optimization process to simplify constraints), meaning positive rootof()is then the largest positive root of max

derived from this construction is/(2) max. The refined constants found by AlphaEvolve

×10

(), finding its largest positive root max(by finding the largest positive root of()/), and calculating/(2)yields the improved upper bound max

linear combination is very similar to the one found in [ hypothesis the construction is nearly optimal. *Note: After publishing the first version of this manuscript, Henry Cohn pointed out that in* a recent paper [18] they used a similar, but more refined approach to get the better constant

### 0.3284. By incorporating their refined approach into

ˆ():= () −2 and

33] that

4≤0.3523.

4 ≤0.3521with a similar linear

*AlphaEvolve.* satisfying the

33], the test function is sought in the form

()= 44(), the Fourier transform ())−= ()−. Thus,()is related

4

(), and()is the ˆ

(0)<0,(0)<0 ˆ (). In our

(0)=0(a condition used in the ()has a factor of. The largest ()/. The upper bound on 4

−5]. Using these coefficients to construct

4≤0.3521. Qualitatively our 33], thus empirically confirming their

constant further to 0.3216. For details, we refer to the accompanying

 Google Colab.

39

*AlphaEvolve, we improved our reported*


---

**Figure 10|Construction found by AlphaEvolve**

### B.5. Erdős' minimum overlap problem

Let 5be the largest constant for which

sup ()(+)

∈[−2,2] −1

for all non-negative, :[−1,1] → [0,1] with+ =1on[−1,1]and extend, by zero outside of[−1,1]. This constant controls the asymptotics of the Minimum Overlap Problem of [25]. The bounds

0.379005≤

are known, where the lower bound was obtained in [ It is known (see [40]) that this constant is equal to the infimum, over all step functions on[0,2]with values in[0,1]and satisfying

max ℎ()(1−ℎ(+ )).

The upper bound to the Erdős minimum overlap problem was then obtained by using this result, in [40] by a step function construction. The step function depicted in ever so slightly better than the previous bound, giving the upper bound of

for the minimum overlap problem of Erdős.

=1, where we

5≤0.380927 107] via convex programming methods. ℎ() =1of

0

 Figure 10 does

5≤0.380924.

1

1.14465≤

2

4

6≤; 3

40

### B.6. Sums and differences of finite sets

Let 6be the largest constant for which the following statement holds: there exist arbitrarily large finite sets of integers, with|+ | ≪ ||and|− | ≫ |+ |. (Here+ {+ : ∈, ∈ }and −={− : ∈, ∈ }denote the sumset and difference set, respectively. The notation≪means that sets, (for sufficiently large sets, ). The notation positive constantindependent of the sets ′

, (for sufficiently large sets, ).) ≤ for some constantindependent of the

≫ means that≥ ′for some


---

**Figure 11|Constructions of the packing problems found by**

unit hexagons into a regular hexagon of side length 3.931. Right: Packing 12 unit hexagons into a regular hexagon of side length 3.942. see [39, Corollary 3] for the upper bound and [ main tool for the lower bound is the following result of Gyarmati et al. [

6≥1+

for any finite setof non-negative integers containing zero satisfying *AlphaEvolvefound a setof size 2003 improving the lower bound to* another setof size 54265 further improving the lower bound to

### B.7. Packing unit regular hexagons inside a regular hexagon

Consider the problem of packingdisjoint regular hexagons with unit side length into a larger regular hexagon, minimizing the side length of the outer hexagon. For =12, the best known constructions use outer hexagons of side lengths respectively [29].AlphaEvolvefound packing arrangements that improve these bounds to

3.931and3.942, respectively. These arrangements are shown in

*AlphaEvolve. Left: Packing 11*

39, Theorem 1] for the lower bound. The

39]:

log |−|

log2 max() +1(

|− | ≤2max() +1. 1.1479≤ 6, and 1.1584≤ 6.

=11and 3.943and4.0,

> Figure 11.
>

1

2

the best known bound of12.890[29]. (In this reference, instead of the ratio itself, the square of the ratio is reported, and we use the same convention.)

41

### B.8. Minimizing the ratio of maximum to minimum distance

For anyand, the goal of this problem is to find to minimize the ratio between the maximum and minimum pairwise distances. found two new constructions improving the best known bounds. The found constructions are shown in Figure 12. In 2 dimensions,AlphaEvolvefound 16 points with ratio

points in the-dimensional space so as

*AlphaEvolve*

≈ 12.889266112, improving


---

**Figure 12|Left: 16 points in 2 dimensions achieving a ratio of maximum distance to** √ minimum distance of≈12.889266112. Right: 14 points in 3 dimensions achieving a ratio √ of≈ 4.165849767. Both constructions improve the best known bounds. In 3 dimensions,AlphaEvolvefound 14 points with ratio √ best known bound of4.168[29].

### B.9. The Heilbronn problem for triangles

The goal of this problem is to findpoints on or inside a triangle with unit area so that the area of the smallest triangle formed by these points is maximized. For 0.036[29], and AlphaEvolve found a construction with minimum area larger than which is shown in Figure 13 (left).

### B.10. The Heilbronn problem for convex regions

The goal of this problem is to findpoints on or inside a convex region with unit area so that the area of the smallest triangle formed by these points is maximized. two of the best known bounds. For =13, the SOTA was0.0306[29], and AlphaEvolveimproved it to0.0309(see Figure 13 (middle)). For=14, the SOTA was 0.0278(see Figure 13 (right)).

≈ 4.165849767, improving the

=11, the SOTA was

0.0365,

unit sphere. The maximum such number in *number [8]. For=11, the best known lower bound was 592 [*

dimensions is called the-dimensional kissing

31] and AlphaEvolveimproved

42

### B.11. Kissing number in dimension 11

The kissing problem asks how many disjoint unit spheres can be packed tangent to a given

*AlphaEvolveimproved*

0.0277[29] and AlphaEvolveimproved it to


---

**Figure 13|New constructions found by AlphaEvolve**

two variants of the Heilbronn problem. Left: 11 points in a unit-area triangle with all formed triangles having area≥0.0365. Middle: 13 points inside a convex region with unit area with all formed triangles having area≥0.0309. Right: 14 points inside a unit convex region with minimum area≥0.0278. this to 593. To prove the lower bound of 593 for the kissing number in dimension 11, *AlphaEvolvefound 593 many 11-dimensional non-zero points with integral coordinates such* that the maximum norm of these points is smaller than their minimum pairwise distance. By the following lemma, this implies the kissing number in dimension 11 is at least 593.

**Lemma 1.Let⊂ℝ be a set of points satisfying**

min∥− ∥: ≠ {

*Then unit spheres centred at:∈* ∥∥ *form a valid kissing configuration in dimension. In particular, the kissing number in dimension Proof.For any≠* ∈ , the inequality∥ − ∥≥max{∥∥,∥∥}implies 2⟨, ⟩ ≤ ∥∥+ ∥∥−max{∥∥ where the last inequality holds because the minimum of two positive numbers is less than or n equal to their geometric mean. The points at them are tangent to a unit sphere centred at the origin. The last step is to show that these spheres do not overlap. This is equivalent to showing, for all

∥∥

After simplifying, this is equivalent to2⟨, n o unit spheres centred at: ∈ ∥∥ form a valid kissing configuration in dimension, as required.

improving the best known bounds on

0∉ *and* }≥max ∥∥: ∈ {}.

*is at least||.*

,∥∥}=min{∥∥,∥∥} ≤ ∥∥ · ∥∥,

∥∥: ∈have norm 2, so unit spheres centred

≠ ∈ , that

≥2. ∥∥ ⟩ ≤ ∥∥ · ∥∥, which we have proved in(4). Thus

to maximize the sum of their radii.AlphaEvolve state of the art [29].

found two new constructions improving the

43

### B.12. Packing circles inside a unit square to maximize sum of radii

Given a positive integer, the problem is to pack

disjoint circles inside a unit square so as


---

For =26, the SOTA was2.634, and AlphaEvolve improved it to (left). For=32, the SOTA was2.936, and AlphaEvolve improved it to (middle).

### B.13. Packing circles inside a rectangle of perimeter 4 to maximize sum of radii

Given a positive integer, the problem is to pack perimeter 4 so as to maximize the sum of their radii. for=21, improving the state of the art from 2.364[29] to2.3658; see Figure 14 (right).

disjoint circles inside a rectangle of *AlphaEvolvefound a new construction*

of radii≥2.635. Middle:32circles in a unit square with sum of radii circles in a rectangle with perimeter4, with sum of radii

2.635; see Figure 14 2.937; see Figure 14

≥2.937. Right: 21

≥2.365.

44

**Figure 14|New constructions found by AlphaEvolve**

packing circles to maximize their sum of radii. Left:

improving the best known bounds on 26circles in a unit square with sum
