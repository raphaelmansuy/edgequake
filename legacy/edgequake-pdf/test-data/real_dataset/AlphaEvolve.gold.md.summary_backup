# AlphaEvolve: A coding agent for scientific and algorithmic discovery

**Google DeepMind** 

**Authors:** Alexander Novikov, Ngân Vũ, Marvin Eisenberger, Emilien Dupont, Po-Sen Huang, Adam Zsolt Wagner, Sergey Shirobokov, Borislav Kozlovskii, Francisco J. R. Ruiz, Abbas Mehrabian, M. Pawan Kumar, Abigail See, Swarat Chaudhuri, George Holland, Alex Davies, Sebastian Nowozin, Pushmeet Kohli and Matej Balog 

---

### Abstract

In this white paper, we present AlphaEvolve, an evolutionary coding agent that substantially enhances capabilities of state-of-the-art LLMs on highly challenging tasks such as tackling open scientific problems or optimizing critical pieces of computational infrastructure. AlphaEvolve orchestrates an autonomous pipeline of LLMs, whose task is to improve an algorithm by making direct changes to the code. Using an evolutionary approach, continuously receiving feedback from one or more evaluators, AlphaEvolve iteratively improves the algorithm, potentially leading to new scientific and practical discoveries. We demonstrate the broad applicability of this approach by applying it to a number of important computational problems. When applied to optimizing critical components of large-scale computational stacks at Google, AlphaEvolve developed a more efficient scheduling algorithm for data centers, found a functionally equivalent simplification in the circuit design of hardware accelerators, and accelerated the training of the LLM underpinning AlphaEvolve itself. Furthermore, AlphaEvolve discovered novel, provably correct algorithms that surpass state-of-the-art solutions on a spectrum of problems in mathematics and computer science, significantly expanding the scope of prior automated discovery methods (Romera-Paredes et al., 2023). Notably, AlphaEvolve developed a search algorithm that found a procedure to multiply two  complex-valued matrices using 48 scalar multiplications; offering the first improvement, after 56 years, over Strassen's algorithm in this setting. We believe AlphaEvolve and coding agents like it can have a significant impact in improving solutions of problems across many areas of science and computation. 

---

## 1. Introduction

Discovering new high-value knowledge, such as making a novel scientific discovery or developing a commercially valuable algorithm, generally requires a prolonged process of ideation, exploration, backtracking on unpromising hypotheses, experimentation, and validation. There has been much recent interest in using large language models (LLMs) to automate significant parts of this process. Hopes of success here are driven by the breathtaking power of recent LLMs, which can enhance their capabilities using test-time compute, and the rise of agents that combine language generation and action. These advances have improved performance across a range of established benchmarks and accelerated discovery-oriented tasks like hypothesis generation and experiment design. However, getting LLM pipelines all the way to making entirely new scientific or practical discoveries remains challenging. 

In this white paper, we present an LLM code superoptimization agent, called AlphaEvolve, that takes on this challenge using a combination of evolutionary computation and LLM-based code generation. AlphaEvolve focuses on the broad spectrum of scientific and engineering discovery problems in which the candidates of discovery can be automatically evaluated. It represents the candidates (for example, new mathematical objects or practical heuristics) as algorithms and uses a set of LLMs to generate, critique, and evolve a pool of such algorithms. The LLM-directed evolution process is grounded using code execution and automatic evaluation. This evaluation mechanism allows AlphaEvolve to avoid any incorrect suggestions from the base LLM. 

The evolutionary process in AlphaEvolve leverages modern LLMs' ability to respond to feedback, enabling the discovery of candidates that are substantially different from the initial candidate pool in syntax and function. It is applicable both to problems where discovering new algorithms is the intrinsic goal, as well as to the broad range of problems where the solution of interest is not an algorithm itself but an algorithm can describe how that solution is to be constructed or found. In the latter case, discovering the algorithm is only an instrumental goal, but it turns out to be a surprisingly effective strategy compared to searching for the solution directly. 

The idea of combining evolutionary methods with coding LLMs has been previously explored in various specialized settings. In particular, AlphaEvolve is a substantial enhancement of FunSearch (see Table 1), which used LLM-guided evolution to discover heuristics in order to construct novel mathematical objects or to drive the operation of online algorithms. Also, related approaches have been used in tasks such as discovering policies for simulated robots, symbolic regression, and the synthesis of heuristic functions for combinatorial optimization. In contrast to these systems, AlphaEvolve leverages state-of-the-art (SOTA) LLMs to evolve large pieces of code that implement complex algorithms spanning multiple functions and components. As a result, it is able to go significantly beyond its predecessors in scale and generality. 

**Table 1: Capabilities and typical behaviours of AlphaEvolve and our previous agent.** 

| FunSearch [83] | AlphaEvolve |
| --- | --- |
| evolves single function | evolves entire code file |
| evolves up to 10-20 lines of code | evolves up to hundreds of lines of code |
| evolves code in Python | evolves any language |
| needs fast evaluation (~20min on 1 CPU) | can evaluate for hours, in parallel, on accelerators |
| millions of LLM samples used | thousands of LLM samples suffice |
| small LLMs used; no benefit from larger | benefits from SOTA LLMs |
| minimal context (only previous solutions) | rich context and feedback in prompts |
| optimizes single metric | can simultaneously optimize multiple metrics |
| 

 |  |

While the use of an automated evaluation metric offers AlphaEvolve a key advantage, it is also a limitation—in particular, it puts tasks that require manual experimentation out of our scope. Because problems in mathematics, computer science, and system optimization typically permit automated evaluation metrics, our efforts on AlphaEvolve focus on these domains. Specifically, we use AlphaEvolve to make progress on several well-known open problems in algorithm design and constructive mathematics, as well as the optimization of critical layers in the large-scale computation stacks at Google. 

Within algorithm design, we consider the fundamental problem of discovering fast algorithms for multiplying matrices, a problem to which a more specialized AI approach had been applied previously. Despite being general-purpose, AlphaEvolve goes beyond previous work, improving the SOTA for 14 matrix multiplication algorithms; notably, for  matrices, AlphaEvolve improves Strassen (1969)'s algorithm by discovering an algorithm using 48 multiplications to multiply  complex-valued matrices. 

In mathematics, we consider a broad range of open problems on which one can make progress by discovering constructions (objects) with better properties than all previously known constructions, according to given mathematical definitions. We apply AlphaEvolve to a large number (over 50) of such problems and match the best known constructions on ~75% of them (in many cases these constructions are likely to already be optimal). On ~20% of the problems, AlphaEvolve surpasses the SOTA and discovers new, provably better constructions. This includes an improvement on the Minimum Overlap Problem set by Erdős and an improved construction on the Kissing Numbers problem in 11 dimensions. 

Finally, we use AlphaEvolve in four engineering problems spanning different layers of Google's compute stack: discovering scheduling heuristics for Google's cluster management system, optimizing matrix-multiplication kernels used to train LLMs, optimizing arithmetic circuits used within TPUs, and optimizing the runtime of attention in Transformers. Because these components are run repeatedly over a long period of time, any improvements are highly valuable. 

---

## 2. AlphaEvolve

AlphaEvolve is a coding agent that orchestrates an autonomous pipeline of computations including queries to LLMs, and produces algorithms that address a user-specified task. At a high level, the orchestrating procedure is an evolutionary algorithm that gradually develops programs that improve the score on the automated evaluation metrics associated with the task. A high-level overview of AlphaEvolve is shown in Figure 1, and Figure 2 gives an expanded view. 

Figure 1 | AlphaEvolve high-level overview.  The process involves a human defining "What?" (evaluation criteria, initial solution) and AlphaEvolve figuring out "How?" through a loop involving a Prompt Sampler, LLMs ensemble, Evaluators pool, and a Program database. 

### 2.1. Task specification

**Evaluation.** Since AlphaEvolve tackles problems with machine-gradeable solutions, the user must provide a mechanism for automatically assessing generated solutions. This mechanism takes the form of a function  mapping a solution to a set of scalar evaluation metrics.  By convention, these metrics are maximized. In our current setup,  is typically implemented as a Python function, called `evaluate`, with a fixed input/output signature, returning a dictionary of scalars. Depending on the application, executing this function may take only seconds on a single device or spawn extensive computations. For mathematical problems, the function  is typically very simple. For example, when wishing to find largest possible graphs satisfying a given property,  invokes the evolved code to generate a graph, checks whether the property holds, and then simply returns the size of the graph as the score. In more complicated cases, the function  might involve performing an evolved search algorithm, or training and evaluating a machine learning model. 

Figure 2 | Expanded view of the AlphaEvolve discovery process. The user provides an initial program (with components to evolve marked), evaluation code, and optional configurations (Section 2.1).  AlphaEvolve then initiates an evolutionary loop. The Prompt sampler uses programs from the Program database to construct rich prompts (Section 2.2). Given these prompts, the LLMs generate code modifications (diffs), which are applied to create new programs (Section 2.3). These are then scored by Evaluators (Section 2.4), and promising solutions are registered back into the Program database (Section 2.5), driving the iterative discovery of better and better programs. 

**API.** To support evolving multiple components across a codebase, AlphaEvolve exposes an input API where blocks of code can be annotated as to-be-evolved-by-the-system; see Figure 3a for an illustration. This design facilitates integrating it with existing codebases while requiring only minimal changes, simply by adding special markers (`# EVOLVE-BLOCK-START` and `# EVOLVE-BLOCK-END`) as comments into the code. Any user-provided code inside such evolution blocks serves as the initial solution to be improved by AlphaEvolve, and the rest of the code forms a skeleton that ties the evolved pieces together, so that they can be invoked from `evaluate`. While this initial implementation must be complete, it can be rudimentary—for instance, consisting of single-line functions that return constants of the appropriate types. 

**Flexibility in choosing the abstraction.** AlphaEvolve can be applied to the same problem in very different ways—especially when the evolved programs are not the final output but a means to discover solutions. For example, AlphaEvolve can evolve the solution in raw string representation (as in classical evolutionary algorithms); evolve a function of a definite form that specifies how to construct the solution from scratch (the approach taken in FunSearch); evolve a bespoke search algorithm to find the solution within some fixed compute budget; or even co-evolve intermediate solutions and search algorithms together, such that each search algorithm is specifically tailored to further improve upon a particular intermediate solution. We find that different levels of abstraction work better for different problems. For example, we hypothesize that for problems with highly symmetric solutions it is advantageous to evolve constructor functions as these tend to be more concise, whereas for problems with non-symmetric solutions it works better to evolve customized search algorithms. 

### 2.2. Prompt sampling

As AlphaEvolve leverages SOTA LLMs, it supports various types of customization and providing long contexts as part of the primary evolution prompt. This prompt comprises multiple previously discovered solutions sampled from the program database, as well as system instructions on how to propose changes to a particular solution. Beyond these key ingredients, users can further tailor prompts to their specific needs in different ways, such as the following. 

* 
**Explicit context:** details about the problem being solved, such as fixed human-written instructions, equations, code snippets, or relevant literature (e.g., pdf files). 


* 
**Stochastic formatting:** template placeholders with human-provided alternatives for increased diversity, instantiated using probability distributions provided in a separate config file. 


* 
**Rendered evaluation results:** usually this will include a program, the result of executing that program, and the scores assigned by the evaluate function. 


* 
**Meta prompt evolution:** instructions and context suggested by the LLM itself in an additional prompt-generation step, co-evolved in a separate database analogous to the solution programs. 



### 2.3. Creative generation

To drive the evolutionary procedure, AlphaEvolve leverages the capabilities of SOTA LLMs, whose principal role is to digest information about previously developed solutions and propose new, diverse ways to improve the solutions. Although AlphaEvolve is model-agnostic, in ablations we observe that AlphaEvolve performs increasingly better as the underlying LLM improves (see Section 4). 

Figure 3 | Illustrative example of applying AlphaEvolve to evolving a supervised learning pipeline.  (a) The user-provided file with blocks marked for evolution (`# EVOLVE-BLOCK-START/END`), and the special evaluate function that can be invoked to score the current version of the code.  (b) Example of an assembled prompt to be provided to the LLMs, including prior programs and their scores.  (c) Example output generated by the LLM using `SEARCH`/`REPLACE` blocks to modify the model architecture and optimizer. The proposed diffs in (c) will be applied to the "current program" shown in the prompt (b), and the resulting modified program will then be sent to the evaluators. The evaluators will invoke the evaluate function from (a) in order to obtain the scores of the newly proposed program. 

**Output format.** When AlphaEvolve asks an LLM to modify existing code, especially within larger codebases, it requests the changes to be provided as a sequence of diff blocks in a specific format: 

```python
<<<<<<< SEARCH
# Original code block to be found and replaced
=======
# New code block to replace the original
>>>>>>> REPLACE

```



Here, the code between `<<<<<<< SEARCH` and `=======` is the exact segment to match in the current program version. The code between `=======` and `>>>>>>> REPLACE` is the new segment that will replace the original one. This allows for targeted updates to specific parts of the code. In cases where the code being evolved is very short, or when a complete rewrite is more appropriate than a small modification, AlphaEvolve can be configured to instruct the LLM to output the entire code block directly, rather than using the diff format. 

**Models used.** AlphaEvolve employs an ensemble of large language models. Specifically, we utilize a combination of Gemini 2.0 Flash and Gemini 2.0 Pro. This ensemble approach allows us to balance computational throughput with the quality of generated solutions. Gemini 2.0 Flash, with its lower latency, enables a higher rate of candidate generation, increasing the number of ideas explored per unit of time. Concurrently, Gemini 2.0 Pro, possessing greater capabilities, provides occasional, higher-quality suggestions that can significantly advance the evolutionary search and potentially lead to breakthroughs. This strategic mix optimizes the overall discovery process by maximizing the volume of evaluated ideas while retaining the potential for substantial improvements driven by the more powerful model. 

### 2.4. Evaluation

To track AlphaEvolve's progress and to select which ideas to propagate in future generations, each new solution proposed by the LLMs is automatically evaluated. In principle, this process amounts to simply executing the user-provided evaluation function  on the generated solution. In practice, AlphaEvolve supports optional mechanisms to make this evaluation more flexible and more efficient: 

* 
**Evaluation cascade (hypothesis testing):** the user can specify ensembles of test cases of increasing difficulty, such that new solutions are evaluated on the next stage only if they achieve sufficiently promising results in all earlier stages. This helps to prune out less promising solutions more quickly. Moreover, new solutions are initially evaluated on a small scale before being subjected to the main test cases, to filter out faulty programs early. 


* 
**LLM-generated feedback:** in some applications, desirable solutions have certain characteristics that are difficult to capture precisely in the user-provided evaluation function ; for example, simplicity of the discovered program. These properties can be graded using separate LLM calls and added to the dictionary of scores to steer evolution, or they can be used to discard solutions when a criterion is not fulfilled. 


* 
**Parallelized evaluation:** the sample efficiency of AlphaEvolve makes it feasible to spend on the order of 100 compute-hours to evaluate any new solution. However, unless individual evaluations are parallelized to reduce their wall-clock duration, this can slow down the rate at which new generations appear, limiting the ability of the evolutionary algorithm to apply several consecutive mutations. In many applications, evaluation is embarrassingly parallel (for example, running a search algorithm from multiple randomized initializations), allowing AlphaEvolve to distribute this work through asynchronous calls to an evaluation cluster. 



**Multiple scores.** AlphaEvolve allows for optimizing multiple user-provided scores, i.e., evolving objects that achieve a high score under one or multiple evaluation metrics.  This has both an intrinsic and instrumental value. While in multiple applications we genuinely care about developing solutions for multiple evaluation metrics (or one solution that is strong on all of them simultaneously), we find that even if one metric is of particular interest, optimizing for multiple metrics often improves results for the single target metric. Perhaps this occurs because programs excelling under different evaluation criteria often possess distinct structures or logic and, by incorporating examples of these diverse, high-performing programs into the prompts provided to the language model, we can stimulate the generation of more varied candidate solutions. 

### 2.5. Evolution

During its evolutionary procedure, AlphaEvolve continually generates a growing number of solutions with evaluation results (scores and program outputs) attached to them. These solutions are stored in an evolutionary database, the primary goal of which is to optimally resurface previously explored ideas in future generations. A key challenge in designing such databases is balancing exploration and exploitation, to continuously improve the best programs while maintaining diversity to encourage exploration of the entire search space. In AlphaEvolve, the evolutionary database implements an algorithm that is inspired by a combination of the MAP elites algorithm and island-based population models. 

### 2.6. Distributed pipeline

AlphaEvolve is implemented as an asynchronous computational pipeline (using the asyncio Python library) in which many computations are run concurrently, with each computation blocking (waiting) whenever its next step relies on the result of another, yet unfinished computation. More specifically, the asynchronous pipeline comprises a controller, LLM samplers, and evaluation nodes. The entire pipeline is optimized for throughput (rather than the speed of any one particular computation), in order to maximize the number of ideas that can be proposed and evaluated within a specific overall computation budget. 

---

## 3. Results

### 3.1. Faster matrix multiplication via finding novel algorithms for tensor decomposition

From accelerating machine learning computations to enabling realistic computer graphics, matrix multiplication serves as a fundamental operation underpinning numerous critical algorithms and applications within computer science. Since the pioneering work of Strassen, it has been known that a rich space of algorithms for multiplying two matrices can be represented as decompositions of a given 3D tensor into rank-one tensors. The rank (number of terms) of the decomposition exactly specifies the number of scalar multiplications needed to compute the matrix product. Hence, to develop faster matrix multiplication algorithms one needs to find low-rank decompositions of particular tensors. This problem has been tackled with many approaches, yet despite decades of effort, even for the simple case of multiplying two  matrices, the minimum achievable rank is not known. 

**Table 2: Upper bounds on the rank of the tensor (m, n, p) representing the product of an  matrix and an  matrix.** 

| (m, n, p) | best known [reference] | AlphaEvolve |  | (m, n, p) | best known [reference] | AlphaEvolve |
| --- | --- | --- | --- | --- | --- | --- |
| (2, 4, 5) | 33 [42] | **32** |  | (3, 5, 6) | 70 [48] | **68** |
| (2, 4, 7) | 46 [93] | **45** |  | (3, 5, 7) | 82 [91] | **80** |
| (2, 4, 8) | 52 [93] | **51** |  | (4, 4, 4) | 49 [95] | **48** |
| (2, 5, 6) | 48 [93] | **47** |  | (4, 4, 5) | 62 [47] | **61** |
| (3, 3, 3) | 23 [52] | 23 |  | (4, 4, 7) | 87 [93] | **85** |
| (3, 4, 6) | 56 [48] | **54** |  | (4, 4, 8) | 98 [95] | **96** |
| (3, 4, 7) | 66 [91] | **63** |  | (4, 5, 6) | 93 [48] | **90** |
| (3, 4, 8) | 75 [91] | **74** |  | (5, 5, 5) | 93 [72] | 93 |
| 

 |  |  |  |  |  |  |

Starting from the problem description and a standard gradient-based algorithm, AlphaEvolve is able to develop sophisticated tensor decomposition algorithms that outperform existing approaches. To evaluate each evolved program, we choose a set of matrix multiplication targets and run the algorithm, initialized with multiple random seeds. The performance is then measured as the best (lowest) rank achieved on each target as well as the fraction of seeds that achieved this rank. To ensure the exactness of the decomposition, when evaluating, we round each element to the nearest integer or the nearest half-integer; and, to encourage the algorithm to generate near-integral solutions, we include this request in natural language in the LLM's prompt. 

In Table 2, one can see that the various algorithms developed by AlphaEvolve improve the state of the art for 14 different matrix multiplication targets. Notably, for multiplying two  matrices, applying the algorithm of Strassen recursively results in an algorithm with rank 49. AlphaEvolve is the first method to find a rank-48 algorithm to multiply two  complex-valued matrices. 

Figure 4 | Changes proposed by AlphaEvolve to discover faster matrix multiplication algorithms. The full diff is outlined on the left (see magnified version in Figures 9a to 9c) and some excerpts are highlighted on the right. In this example, AlphaEvolve proposes extensive changes across several components, including the optimizer and weight initialization (top right), the loss function (middle right), and hyperparameter sweep (bottom right). These changes are highly non-trivial, requiring 15 mutations during the evolutionary process. 

### 3.2. Finding tailored search algorithms for a wide range of open mathematical problems

A significant frontier in mathematical research involves discovering objects or constructions that possess optimal, or near-optimal, properties according to some measure. Examples range from finding dense packings of geometric shapes to identifying functions or sets satisfying specific combinatorial or analytic constraints. Progress often relies on finding a single construction that surpasses all previously known examples, thereby establishing new lower or upper bounds for the optimal value. We demonstrate that AlphaEvolve serves as a powerful tool for exploring the vast search space inherent in these problems. 

To assess its capabilities, we apply AlphaEvolve to a curated set of over 50 mathematical problems, spanning more than five different branches of mathematics, including analysis, combinatorics, number theory, and geometry. In 75% of the cases AlphaEvolve rediscovered the best known constructions, and in 20% of the cases it discovered a new object that is better than a previously known best construction, thereby improving the SOTA. 

Figure 5 | Examples of SOTA-breaking mathematical constructions discovered with AlphaEvolve. The versatility of AlphaEvolve allows us to tackle problems in **Analysis** (autocorrelation and uncertainty inequalities), **Geometry** (packing and minimum/maximum distance problems) and **Combinatorics** (Erdős's minimum overlap problem and sums and differences of finite sets). 

A significant advantage of the AlphaEvolve configuration used here is its versatility and speed of application. The core methodology, focused on evolving heuristic search programs, can be rapidly deployed across a diverse range of mathematical construction problems and conjectures, often requiring less initial problem-specific expert tailoring compared to traditional bespoke approaches. The key methodological innovation enabling these discoveries is AlphaEvolve's ability to evolve heuristic search algorithms rather than directly evolving the constructions themselves. For many problems, particularly those with fast objective function evaluations, we employed an iterative refinement strategy. Each generation of AlphaEvolve was tasked with evolving a program representing a search heuristic. This program was given a fixed time budget and was shown the best construction found by the previous best heuristic. Its goal was to leverage this starting point and the allotted time to find an even better construction. The final constructions were often the result of a sequence of different, specialized heuristics discovered by AlphaEvolve. 

Below are high-level descriptions of some of the problems where AlphaEvolve yielded new results. 

* **Analysis:**
* 
**Autocorrelation inequalities.** AlphaEvolve was able to improve the best known bounds on several autocorrelation inequalities. 


* 
**Uncertainty principles.** AlphaEvolve was able to produce a refined configuration for a problem arising in Fourier analysis, by polishing an uncertainty principle construction leading to a slightly better upper bound. 




* **Combinatorics and number theory:**
* 
**Erdős's minimum overlap problem.** AlphaEvolve established a new upper bound for the minimum overlap problem, slightly improving upon the previous record. 




* **Geometry and packing:**
* 
**Kissing number problem.** In 11 dimensions, AlphaEvolve improved the lower bound on the kissing number, finding a configuration of 593 non-overlapping unit spheres that can simultaneously touch a central unit sphere, surpassing the previous record of 592. 


* 
**Packing problems.** AlphaEvolve achieved several new results in packing problems, such as packing N points in a shape to minimize the ratio of the maximum and minimum distance, packing various polygons in other polygons in the most efficient way, and variants of the Heilbronn problem concerning point sets avoiding small-area triangles. 





### 3.3. Optimizing Google's computing ecosystem

#### 3.3.1. Improving data center scheduling

Efficiently scheduling compute jobs onto a cluster of machines is a critical optimization problem, particularly at the scale of Google's data centers, orchestrated by Borg. This task involves assigning jobs to available machines based on job resource requirements and machine capacity. Inefficient assignments can result in stranded resources: when a machine can no longer accept jobs because it has run out of one kind of resource (e.g., memory) but still has other resources free (e.g., CPU). We address this challenge by framing the online job scheduling problem as a vector bin-packing problem with two variables. 

An early version of AlphaEvolve was used to discover a remarkably simple yet effective heuristic function (shown in Figure 6), evolving from the existing one in production. We use a simulator of our data centers to provide feedback to AlphaEvolve based on historical snapshots of workloads and capacity across Google's fleet. Post-deployment measurements across Google's fleet confirmed the simulator results, revealing that this heuristic function continuously recovers on average 0.7% of Google's fleet-wide compute resources, which would otherwise be stranded. 

**Figure 6 | Left:** The heuristic function discovered by AlphaEvolve, tailored to Google's workloads and capacity. **Right:** Visualization of the heuristic scoring function. Yellow regions represent high scores, while purple regions represent low scores. 

#### 3.3.2. Enhancing Gemini kernel engineering

Training large models like Gemini requires substantial computational resources. A critical aspect of kernel optimization is tuning the tiling strategy for matrix multiplication operations (see Figure 7). This technique involves dividing a large matrix multiplication computation into smaller subproblems to better balance computation with data movement. We address this challenge by employing AlphaEvolve to optimize tiling heuristics for an important matrix multiplication kernel used to train Gemini. 

Figure 7 | Visualization of the tiling heuristic problem for a matrix product . Creating a heuristic that automatically chooses the right tile size (M, N, P) for all input shapes is difficult because one has to know the matrix multiplication unit's optimal shapes and memory capacity, among other details. 

This automated approach enables AlphaEvolve to discover a heuristic that yields an average 23% kernel speedup across all kernels over the existing expert-designed heuristic, and a corresponding 1% reduction in Gemini's overall training time. In addition, the use of AlphaEvolve significantly reduced the kernel optimization time, from several months of dedicated engineering effort to just days of automated experimentation. The tiling heuristic discovered by AlphaEvolve has been deployed in production, directly enhancing Gemini's training efficiency. 

#### 3.3.3. Assisting in hardware circuit design

Specialized hardware, such as Google's Tensor Processing Units (TPUs), is crucial for achieving the resource efficiency required to run modern AI systems at scale. Register-Transfer Level (RTL) optimization is a critical step in this process. In this work, AlphaEvolve was challenged to optimize an already highly optimized Verilog implementation of a key TPU arithmetic circuit within the matrix multiplication unit. The optimization objectives were to reduce both area and power consumption while preserving the component's core functionality. AlphaEvolve was able to find a simple code rewrite that removed unnecessary bits, a change validated by TPU designers for correctness. Integrated into an upcoming TPU, this improvement represents Gemini's first direct contribution to TPU arithmetic circuits, achieved via AlphaEvolve. 

#### 3.3.4. Directly optimizing compiler-generated code

The transformer architecture is used in the majority of modern neural networks. The core computation of transformers is the attention mechanism. We challenged AlphaEvolve to directly optimize the XLA-generated IRs encapsulating the FlashAttention kernel along with pre- and postprocessing code. We optimized a configuration corresponding to a highly impactful transformer model used for inference at scale on GPUs. AlphaEvolve was able to provide meaningful optimizations: the FlashAttention kernel for the configuration of interest was sped up by 32%, and improvements in pre- and postprocessing resulted in a 15% speed up in that part. 

---

## 4. Ablations

We carried out ablations on two tasks: finding tensor decompositions for faster matrix multiplication and computing lower bounds on kissing numbers, aiming to understand the efficacy of the following components of AlphaEvolve: 

* 
**Evolutionary approach.** To analyze the importance of evolution, we consider an alternative approach ("No evolution"), which repeatedly feeds the same initial program to the language model. 


* 
**Context in prompts.** We consider an alternative approach where no explicit context is added to the prompt ("No context in the prompt"). 


* 
**Meta prompts.** To test the efficacy of meta prompting, we disable it for the task of tensor decomposition ("No meta prompt evolution"). 


* 
**Full-file evolution.** We consider an alternative in the context of tensor decomposition where only the loss function is evolved ("No full-file evolution"). 


* 
**Powerful language models.** We consider an alternative where only a single small base model is used ("Small base LLM only"). 



**Figure 8 | Left:** Ablations of AlphaEvolve on the problem of finding low-rank tensor decomposition for faster matrix multiplication. **Right:** Ablations of AlphaEvolve on the problem of finding sphere packings for improving kissing numbers.  Each curve shows the performance of an individual setting with increasing compute budget. As can be seen, each of the components is responsible for a significant improvement in the results. 

---

## 5. Related work

**Evolutionary methods.** AlphaEvolve extends a long tradition of research on evolutionary or genetic programming. In contrast to classical methods using handwritten evolution operators, AlphaEvolve uses LLMs to automate the construction of these operators. AlphaEvolve extends the FunSearch system in three key ways: it evolves entire codebases rather than single functions; it provides the ability to perform multiobjective optimization; and it uses frontier LLMs with rich forms of natural-language context and feedback. 

**Superoptimization and algorithm discovery.** AlphaEvolve can be viewed as a method for code superoptimization in that it iteratively improves an initial program using execution feedback. Previous work on using LLMs for algorithm discovery provided promising results, but AlphaEvolve's approach to leverage it for evolutionary algorithms allows us to address significantly more challenging problems. 

**AI for scientific and mathematical discovery.** Many recent LLM-based methods target scientific problems in multiple disciplines. AlphaEvolve differs from most of these methods in its use of programmatic hypothesis representations and evaluation metrics. In the context of pure mathematics, the FunSearch approach established LLM-guided evolution as a powerful tool for discovering witnesses for, and counterexamples to, mathematical statements. 

---

## 6. Discussion

AlphaEvolve demonstrates the surprising power of combining state-of-the-art LLMs with automated evaluation metrics within an evolutionary framework. AlphaEvolve can also be seen as a test-time compute agent that, through its evolutionary procedure, significantly enhances the capability of the base LLM. A natural next step will be to consider distilling the AlphaEvolve-augmented performance of the base LLMs into the next generation of the base models. Beyond distillation, it is also intriguing that AlphaEvolve can make practical discoveries that increase the efficiency of its own infrastructure and of (future versions of) its base LLMs. The main limitation of AlphaEvolve is that it handles problems for which it is possible to devise an automated evaluator. 

---

## Acknowledgements

We thank Michael Figurnov for reviewing this white paper; Alhussein Fawzi, Bernardino Romera-Paredes, and Ankit Anand for early explorations; and Stig Petersen and Demis Hassabis for support and advice. We specifically acknowledge contributions from Terence Tao, Javier Gomez Serrano, and Jordan Ellenberg for suggesting specific open mathematical problems. We also thank the many contributors to the data center scheduling, Gemini kernel engineering, TPU circuit design, and compiler optimization applications. 

---

## Author information

**Contributions.** A.N. and M.B. designed and implemented the initial version of AlphaEvolve. M.B., A.N., N.V. and P.K. developed project vision and scoped problems. N.V. and P.-S.H. oversaw the practical applications. E.D. and M.E. implemented the first benchmark problem. A.N. and M.E. developed the final version of AlphaEvolve. Detailed contributions for applications and paper writing are listed in the full text. 

**Corresponding authors:** Matej Balog, Alexander Novikov and Pushmeet Kohli. 

---

## Appendix A. Faster matrix multiplication: Full results

Figures 9a to 9c | Magnified version of Figure 4 (left), giving the program that discovers a faster algorithm to multiply  matrices. The code shows changes to the optimizer, initializer, loss function (including reconstruction, discretization, and hallucination losses), and hyperparameter sweeps. 

**Table 3: Full version of Table 2, showing the best ranks obtained by AlphaEvolve for tensor decomposition for all considered parameters.** Of the 54 targets, AlphaEvolve matches the state of the art in 38 cases, surpasses it in 14 cases, and falls behind in 2 cases. 

| (m,n,p) | best known | AlphaEvolve |  | (m,n,p) | best known | AlphaEvolve |
| --- | --- | --- | --- | --- | --- | --- |
| (2,2,2) | 7 | 7 |  | (3,4,4) | 38 | 38 |
| (2,2,3) | 11 | 11 |  | (3,4,5) | 47 | 47 |
| (2,2,4) | 14 | 14 |  | (3,4,6) | 56 | **54** |
| (2,2,5) | 18 | 18 |  | (3,4,7) | 66 | **63** |
| (2,2,6) | 21 | 21 |  | (3,4,8) | 75 | **74** |
| (2,2,7) | 25 | 25 |  | (3,5,5) | 58 | 58 |
| (2,2,8) | 28 | 28 |  | (3,5,6) | 70 | **68** |
| (2,2,9) | 32 | 32 |  | (3,5,7) | 82 | **80** |
| (2,2,10) | 35 | 35 |  | (4,4,4) | 49 | **48** |
| (2,2,11) | 39 | 39 |  | (4,4,5) | 62 | **61** |
| (2,2,12) | 42 | 42 |  | (4,4,6) | 73 | 73 |
| (2,2,13) | 46 | 46 |  | (4,4,7) | 87 | **85** |
| (2,2,14) | 49 | 49 |  | (4,4,8) | 98 | **96** |
| (2,2,15) | 53 | 53 |  | (4,4,9) | 104 | 108 |
| (2,2,16) | 56 | 56 |  | (4,5,5) | 76 | 76 |
| (2,3,3) | 15 | 15 |  | (4,5,6) | 93 | **90** |
| (2,3,4) | 20 | 20 |  | (5,5,5) | 93 | 93 |
| (2,3,5) | 25 | 25 |  | (6,6,6) | 153 | 156 |
| 

 |  |  |  |  |  |  |

---

## Appendix B. Details of mathematical discoveries of AlphaEvolve

B.1. First autocorrelation inequality. AlphaEvolve found a step function with 600 equally-spaced intervals on  that gives a slightly better upper bound . 

B.2. Second autocorrelation inequality. AlphaEvolve found a step function with 50 equally-spaced intervals on  that gives a slightly better lower bound . 

B.3. Third autocorrelation inequality. AlphaEvolve found a step function with 400 equally-spaced intervals on  that gives a slightly better upper bound . 

B.4. An uncertainty inequality. AlphaEvolve improved the upper bound to  (refined to 0.3216 in later experiments) with a similar linear combination as in previous work, but with refined constants. 

**B.5. Erdős' minimum overlap problem.**
Figure 10 | Construction found by AlphaEvolve for the minimum overlap problem of Erdős. AlphaEvolve found a step function that gives the upper bound of . 

B.6. Sums and differences of finite sets. AlphaEvolve found a set  of size 2003 improving the lower bound to , and another set  of size 54265 further improving the lower bound to . 

**B.7. Packing unit regular hexagons inside a regular hexagon.**
**Figure 11 | Constructions of the packing problems found by AlphaEvolve.** Left: Packing 11 unit hexagons into a regular hexagon of side length 3.931. Right: Packing 12 unit hexagons into a regular hexagon of side length 3.942. 

**B.8. Minimizing the ratio of maximum to minimum distance.**
**Figure 12 | Left:** 16 points in 2 dimensions achieving a ratio of maximum distance to minimum distance of . **Right:** 14 points in 3 dimensions achieving a ratio of . Both constructions improve the best known bounds. 

**B.9 - B.10. The Heilbronn problem for triangles and convex regions.**
**Figure 13 | New constructions found by AlphaEvolve improving the best known bounds on two variants of the Heilbronn problem.** Left: 11 points in a unit-area triangle with all formed triangles having area . Middle: 13 points inside a convex region with unit area with all formed triangles having area . Right: 14 points inside a unit convex region with minimum area . 

B.11. Kissing number in dimension 11. AlphaEvolve improved the best known lower bound to 593 by finding 593 many 11-dimensional non-zero points with integral coordinates such that the maximum norm of these points is smaller than their minimum pairwise distance. 

**B.12 - B.13. Packing circles.**
**Figure 14 | New constructions found by AlphaEvolve improving the best known bounds on packing circles to maximize their sum of radii.** Left: 26 circles in a unit square with sum of radii . Middle: 32 circles in a unit square with sum of radii . Right: 21 circles in a rectangle with perimeter 4, with sum of radii .