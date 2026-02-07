**Title: Beyond Transformers**


**Subtitle: Architectures for the Next Generation ML and GenAI Systems**


**V_1_1**

### **ABOUT THE AUTHOR** **Raphaël MANSUY**


Raphaël Mansuy is a French AI entrepreneur and strategist based in Hong Kong, specializing in
Agentic AI, data engineering, and systems that bridge software engineering with autonomous
intelligence. With over two decades of global technology leadership, he has been recognized
among the world's top 100 most influential people in AI.

As CTO of ELITIZON, a global AI consulting firm based in Hong Kong, Mansuy leads teams
across Europe and Asia in deep learning, NLP, MLOps, and computer vision. He founded
Quantalogic, a sovereign AI Agent platform for Europe, and StudentCentral.ai, an initiative
reimagining education through AI. A proponent of building in public, he advises major
corporations on data strategy and AI transformation from his unique East-West vantage point.

### **Annaëlle MANSUY**


Annaëlle Mansuy is a computer science student pursuing a Master of Engineering at University
College London (UCL), with a focus on machine learning, generative AI, and software
development. She has hands-on experience building applications powered by large language
models and vision AI.

As an AI Engineer at ELITIZON, she has contributed to projects including structured content
extraction, audio-to-text transcription systems featuring diarization and retrieval-augmented
generation (RAG) for fact-checking. Her work spans the full software development
lifecycle—from requirements and testing to backend, frontend, and integration.

With a foundation in mathematical finance, including financial optimization and econometrics,
Annaëlle brings an interdisciplinary perspective to her technical work. She is currently preparing
to pursue research in machine learning.


|The Book’s Goal|Col2|
|---|---|
|What kind of individual would<br>be interested in this book?|**_ Senior Machine Learning Engineers, AI Researchers, Systems_**<br>**_Architects, and CTOs who are hitting the efficiency and_**<br>**_reasoning limits of standard Transformers._**|
|What knowledge do they<br>need**before**they start<br>reading?|**_ A solid understanding of Deep Learning fundamentals_**<br>**_(PyTorch/JAX), familiarity with the Transformer architecture_**<br>**_(Attention, KV-Cache), and experience deploying or fine-tuning_**<br>**_LLMs._**|
|Why should they buy this<br>book?|**_ To future-proof their career and their tech stack. This is the first_**<br>**_book to move beyond "Prompt Engineering" and teach the_**<br>**_engineering physics of the next wave of AI (Mamba, RWKV,_**<br>**_JEPA, and Neuro-symbolic systems)._**|
|What is the product approach<br>and USP of the book?|**_ USP: It is an "Architectural Intuition Builder." Instead of just_**<br>**_surveying papers, it defines a taxonomy (Generative vs._**<br>**_Predictive vs. Energy-Based) and uses 4 consistent "Running_**<br>**_Examples" to prove which architecture solves which problem._**|
|Product Breakdown: In two<br>sentences, describe the<br>“journey” the book takes the<br>reader on. Look at your<br>section headings for help|**_ The reader starts by mathematically dissecting why_**<br>**_Transformers fail at scale (the "Wall"), then systematically_**<br>**_builds competence in every major alternative (Recurrent,_**<br>**_State-Space, Hybrid, Energy-Based, and JEPA). Finally, they learn_**<br>**_to combine these into unified, agentic systems that are efficient_**<br>**_enough to run on the edge and smart enough to reason/plan._**|
|By the end of this book, you<br>will...|**_ Be able to select, architect, and optimize the correct_**<br>**_post-Transformer model (SSM, Hybrid, or World Model) for any_**|


**COMPETITIVE BOOK TITLES**


**List the books here:**






|No.|Title|Opportunities for Packt (gaps, poor reviews,<br>less-popular authors)|
|---|---|---|
|**1 **|**_"Generative AI with LangChain"_**<br>**_(Packt/Competitor)_  **|**Gap: Focuses entirely on tooling/orchestration**<br>**of****_existing_ APIs (OpenAI), not the underlying**<br>**_model architecture_. Our book targets the**<br>**builders of the****_next_ models, not just the users**<br>**of current ones.**|
|**2 **|**_"Build a Large Language Model (From Scratch)"_**<br>**_(Manning)_ **|**Gap: Excellent for understanding the****_baseline_ **<br>**(Transformers), but stops there. It does not**<br>**cover Mamba, JEPA, or Neuro-symbolic**<br>**hybrids, leaving readers unprepared for 2026.**|
|**3 **|**"Designing Machine Learning Systems"**<br>**(O'Reilly)**|**Gap: High-level systems focus. It lacks the**<br>**mathematical depth on****_specifc_ new**<br>**architectures (like Selective Scan or WKV**<br>**kernels) that engineers need to implement**<br>**these systems.**|


### **LEARNING OUTCOME - WHAT WILL THE READER LEARN AND DO?**

Decide what the key learning objectives will be for your book. Please, list them below.​


Also, consider the competing books; in particular the **description**, **table of contents** and **book reviews**,


how do the learning objectives of this book make it unique?


|1|Diagnose Transformer Limits: Mathematically calculate the memory bandwidth and FLOPs<br>bottlenecks of Attention for any given sequence length to determine when to switch<br>architectures.|
|---|---|
|2|**Master Linear Architectures:** Implement and train Gated Linear Recurrent models (xLSTM,<br>RWKV) and State Space Models (Mamba) from scratch using PyTorch.|
|3|**Design Hybrid Systems:** Architect "Best of Both Worlds" models by interleaving SSM layers (for<br>bulk context) with Attention layers (for recall) to optimize the accuracy-latency curve.|
|4|**Implement Neuro-Symbolic Reasoning:** Build "System 2" agents that use Graph Neural<br>Networks (GNNs) and Energy-Based Models (EBMs) to verify facts and constrain hallucinations.<br>|
|5|**Deploy World Models:** Construct Joint Embedding Predictive Architectures (JEPA) that plan in<br>latent space, achieving efciency gains for robotic or video tasks.|
|6|**Optimize for Hardware:** Write custom CUDA/Triton kernels for Selective Scan and<br>WKV operations to maximize inference throughput on consumer GPUs.|
|7|**Evaluate Beyond Perplexity:** Apply a new suite of benchmarks to measure "Long-Context<br>Degradation" and "Agentic Planning Success" rather than just next-token prediction.<br>|
|8||

## **Parts and Chapters** **Chapter Outline**


|Part 1: The Baseline and the Bottleneck|Col2|
|---|---|
|1.|The Transformer Paradigm (2017–2023)<br>|
|2.|The Architectural Taxonomy of 2026|
|3.||


|Part 2: Linear-Time & Recurrent Architectures|Col2|
|---|---|
|1.|Modern Recurrent Neural Networks (The Gated Linear Pattern)|
|2.|State Space Models (SSMs)|
|3.|Comparative Dynamics|


|Part 3: Compositional & Hybrid Architectures|Col2|
|---|---|
|1.|The Hybrid Pattern (Attention + State)|


|2.|Multimodal Hybrids (Vision & Video)|
|---|---|
|3.||


|Part 4: Neuro-Symbolic, World Models, and Reasoning|Col2|
|---|---|
|1.|Graph Neural Networks (GNNs) & Structured Reasoning|
|2.|Energy-Based Models (EBMs) & Dynamic Inference|
|3|World Models & Joint Embedding Architectures (JEPA)|
|4|Agentic Loops & Hierarchical Planning|


|Part 5: Efficiency, Training, & Production|Col2|
|---|---|
|1.|Extreme Compression & Quantization|
|2.|Hardware-Aware Design|
|3|Evaluation & Benchmarks (2025-2026)|


|Part 6: Future Directions|Col2|
|---|---|
|1.|Toward Unified Architectures (2026–2030)|
|2.|Societal & Ethical Implications|

# **Detailed Outline**

Now it’s time to plan out your chapters in more detail. In the following section you will have the
opportunity to list what each chapter does and what the user is gaining from it. The first one has been
filled out as an example. Use this as a template and add as many additional chapters as you need.


**Chapter 1:** The Transformer Paradigm (2017–2023)


40 pages


**Description** :This chapter establishes the engineering baseline by rigorously analyzing the Transformer
architecture. It moves beyond a basic introduction to focus on the "Physics of Attention"—specifically
why the O(N^2) complexity and KV-cache growth create a hard wall for long-context applications. It
introduces the first Running Example, The Efficient Scalar.


**Chapter Subheadings**


●​ **The Attention Mechanism:** Inductive bias and global correlation.
●​ **The Hardware Lottery:** Why GPUs favored Transformers over RNNs.
●​ **The Inference Asymmetry:** Analyzing the "Prefill vs. Decode" compute discrepancy.
●​ **The Wall:** Case Study—Calculating memory bandwidth for 128k context on _The Efficient_

_Scalar_ .


**Chapter 2:** The Architectural Taxonomy of 2026


30 pages


**Description** : A high-level guide to the post-Transformer landscape. This chapter teaches readers how to
classify novel architectures into three buckets: Generative (Explicit), Predictive (Latent), or Energy-Based
(Implicit). It provides a decision framework for choosing the right tool.


**Chapter Subheadings**


●​ **Mapping the Landscape:** Recurrent vs. State-Space vs. Energy-Based.
●​ **Benchmarking the Shift:** 2025 Comparative analysis (Throughput vs. Perplexity).
●​ **Design Intuition:** When to choose State (Compression) vs. Attention (Retrieval).
●​ **The 4 Canonical Examples:** Defining the running use-cases (Scalar, Agent, Reasoner,

Streamer).


**Chapter 3:** Modern Recurrent Neural Networks


45 pages


**Description** : This chapter explores the resurgence of RNNs through the "Gated Linear Recurrence"
pattern. It deep dives into xLSTM and RWKV-7, showing how they solve the vanishing gradient problem
and achieve parallelized training via WKV kernels.


**Chapter Subheadings**


●​ **xLSTM:** sLSTM/mLSTM blocks and exponential gating.
●​ **RWKV-7 "Goose":** Expressive dynamic state evolution.
●​ **The WKV Kernel:** Parallelizing RNN training on GPUs
●​ **Implementation:** Building _The Efficient Scalar_ with RWKV to reduce VRAM by 40%.


**Chapter 4:** State Space Models (SSMs)


50 pages


**Description** : A comprehensive guide to Mamba and its descendants. It explains the transition from
Continuous Time Control (S4) to Discrete Deep Learning, focusing heavily on the "Selectivity Mechanism"
that allows Mamba to filter noise and scale linearly.


**Chapter Subheadings**


●​ **From S4 to Mamba:** The discretization process explained.
●​ **The Selectivity Breakthrough:** How to learn _what to forget_ .
●​ **Scaling SSMs:** Routing Mamba (MoE) and Mixture-of-Mamba.
●​ **Case Study:** Benchmarking _The Long-Context Agent_ on "Needle in a Haystack".


**Chapter 5:** Comparative Dynamics


30 pages


**Description** : This chapter unifies the math, showing how SSMs, Linear Attention, and RNNs are cousins.
It crucially analyzes the "Trade-offs"—specifically proving why state-based models struggle with
"Associative Recall" compared to Global Attention.


**Chapter Subheadings**


●​ **The Unification:** Mathematical equivalence of Linear Attention and SSMs.
●​ **The Copying Problem:** Why State models struggle with exact recall.
●​ **Trade-off Analysis:** Latency vs. Accuracy curves.
●​ **RetNet:** The Exponential Moving Average (EMA) family.


**Chapter 6:** The Hybrid Pattern (Attention + State)


40 pages


**Description** : This chapter teaches the practical engineering of Hybrid models (like Samba and Griffin).
Readers learn how to interleave SSM layers (for bulk processing) with sliding-window Attention (for
precision) to build systems that handle infinite context without losing detail.


**Chapter Subheadings**


●​ **Samba & Griffin:** The "Sliding Window" architecture.
●​ **Architectural Placement:** Early, Middle, or Late Attention?
●​ **Memory Management:** Balancing KV-cache with Recurrent State.
●​ **Implementation:** Hybridizing _The Long-Context Agent_ .


**Chapter 7:** Multimodal Hybrids (Vision & Video)


35 pages


**Description** : Moving beyond text, this chapter explores how linear architectures are revolutionizing
Vision. It covers MambaVision and VAMBA, showing how 1D sequence models can process 2D/3D data
more efficiently than ViTs or 3D-UNets.


**Chapter Subheadings**


●​ **MambaVision:** Replacing ViT with Hybrid Stacks.
●​ **VAMBA:** Temporal coherence in video generation.
●​ **The 2D Scan:** Serialization strategies for images.
●​ **Case Study:** _The Dynamic Streamer_ at 60fps.


**Chapter 8:** Graph Neural Networks (GNNs) & Structured Reasoning


40 pages


**Description** : This chapter addresses the "Structure Gap." It explains why sequence models fail at
non-linear relationships and how to use GNNs as a "Relational Inductive Bias." It introduces the
_ConstraintLLM_ framework for Neuro-Symbolic integration.


**Chapter Subheadings**


●​ **The Structure Gap:** Why LLMs fail at topology.
●​ **Graph-LLM Hybrids:** Encoding Knowledge Graphs (KGs) into Latent Space.
●​ **ConstraintLLM:** Enforcing symbolic logic on neural outputs.
●​ **Implementation:** Grounding _The Structural Reasoner_ in an Enterprise KG.


**Chapter 9:** Energy-Based Models (EBMs) & Dynamic Inference


45 pages


**Description** : A shift from "System 1" (Reflexive) to "System 2" (Reflective). This chapter introduces
Energy-Based Models, where generation is an optimization process. It covers Energy-Based Transformers
(EBTs) that "ponder" via internal gradient descent before generating.


**Chapter Subheadings**


●​ **Implicit Generation:** P(x) vs. Energy Functions.
●​ **Energy-Based Transformers (EBTs):** The "Think Loop" at inference.
●​ **Neuro-Symbolic EBMs:** Logic as Energy Constraints.
●​ **Case Study:** Self-correcting math errors in _The Efficient Scalar_ .


**Chapter 10:** World Models & Joint Embedding Architectures (JEPA)


50 pages


**Description** : The "Predictive" paradigm. This chapter explains why "Prediction is not Generation." It
details LeCun’s JEPA architecture, which predicts latent state evolution rather than pixels, enabling
efficient planning and reasoning.


**Chapter Subheadings**


●​ **Prediction vs. Generation:** The Pixel-Level fallacy.
●​ **LeJEPA & LLM-JEPA:** Learning abstract representations.
●​ **The Planning Engine:** Using V-JEPA for robotic simulation.
●​ **Implementation:** _The Dynamic Streamer_ with a Latent Predictor.


**Chapter 11:** Agentic Loops & Hierarchical Planning


50 pages


**Description** : Bringing it all together into autonomous agents. This chapter focuses on "Hierarchical
Decision Mamba" and other architectures designed for long-horizon loops, contrasting them with the
brittleness of Transformer-based agents.


**Chapter Subheadings**


●​ **Hierarchical State:** Managing long time horizons.
●​ **Nemotron Nano:** Efficient on-device agency.
●​ **The Agent Stack:** JEPA (Planner) + EBM (Verifier) + Mamba (Actor).
●​ **Case Study:** An autonomous coding loop for _The Long-Context Agent_ .


**Chapter 12:** Extreme Compression & Quantization


30 pages


**Description** : Addressing the unique challenges of compressing state-based models. This chapter covers
Bi-Mamba (1-bit architectures) and the sensitivity of recurrent weights to quantization noise.


**Chapter Subheadings**


●​ **State Sensitivity:** Why RNNs are harder to quantize than Transformers.
●​ **Bi-Mamba: 1-Bit State Space Models.**
●​ **Sparse Routing:** Pruning connections in MoE Mambas.
●​ **Implementation:** _The Dynamic Streamer_ with a Latent Predictor.


●​ **Practical Guide:** Quantizing RWKV for mobile deployment.


**Chapter 13:** Evaluation & Benchmarks (2025-2026)


30 pages


**Description** : How to measure success when "Perplexity" is no longer enough. This chapter introduces
new metrics for Long-Context recall, Agentic success rates, and recursive extrapolation.


**Chapter Subheadings**


●​ **Beyond Perplexity:** The reasoning gap.
●​ **The Needle & The Haystack:** Measuring context degradation.
●​ **Agentic Benchmarks:** Evaluating planning vs. luck.
●​ **Beyond GPUs:** Neuromorphic hardware trends.


●​ **Reference:** The 2026 Leaderboard.


**Chapter 14:** Toward Unified Architectures (2026–2030)


25 pages


**Description** : A forward-looking chapter on the convergence of these architectures. It speculates on "The
Grand Unification" of SSMs and Attention, and the role of Recursive "Tiny Networks" in self-improving AI.


**Chapter Subheadings**


●​ **Grand Unification:** Mathematical convergence.
●​ **Recursive Reasoning:** "Less is More" with Tiny Networks.
●​ **Self-Improvement:** Bootstrapping intelligence.
●​ **The Road to 2030:** Predictions.


**Chapter 15:** Societal & Ethical Implications


20 pages


**Description** : A sober analysis of the risks and benefits. It focuses on the "Democratization via Efficiency"
(running powerful models on cheap hardware) and the specific safety risks of autonomous agentic loops.


**Chapter Subheadings**


●​ **Democratization:** AI on the Edge.
●​ **Agentic Risks:** When loops go wrong.
●​ **Neuro-Symbolic Safety:** Verifiability and Trust.
●​ **Conclusion:** Building a responsible future.


                        - end of outline –

# **Community outreach (optional)**


**Technical Reviewers**


Can you recommend peers and members of your community to become technical reviewers?


**Technical Reviewers** _(Ideally, select experts from the RWKV, Mamba, or LeCun Lab_


_communities)_

|Full name|Email Address|LinkedIn Profile|
|---|---|---|
|**Tri Dao** (Mamba Author)|||
|**Bo Peng** (RWKV Author)|||
|**Yann LeCun** (JEPA<br>Advocate)|||



**Amazon Reviewers**


|Full name|Email Address|LinkedIn Profile|
|---|---|---|
||||
||||
||||


**Influencers**


Can you recommend any influential community members or organizations for Packt to collaborate with on


the marketing campaign of your title?








|Full name|Email Address|LinkedIn Profile|
|---|---|---|
|Anthony Alcaraz<br> <br> <br>Author of Agentic Graph<br>RAG (O’Reilly)<br>|alcarazanthony1@gmail.com <br>h<br>n|ttps://www.linkedin.com/in/antho<br>y-alcaraz-b80763155/|
||||
||||


