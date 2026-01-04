# arXiv:2510.09244v1 [cs.AI] 10 Oct 2025

*tonomous Agents: Advances in Architecture and Practice* offered at TUM.

*⋆* This paper is based on a seminar technical report from the course *Trends in Au-*

problem solving by externalizing intermediate reasoning and refining it through

would reason through a problem, since LLM agents can learn and mimic human

programming skills, what increasingly matters is understanding how a human

idea that "if you can think it, you can build it." Instead of relying solely on

algorithms or master low-level code. We are closer than ever to realizing the

Today, one can develop remarkable systems without the need to write complex

automation and fundamentally reshaping the way tasks are performed [13, 14, 37].

Artificial intelligence (AI) is a powerful technology that is transforming cognitive

### 1.1 Motivation

## 1 Introduction

Planning · Memory Systems · Action Systems · Multi-agent Systems

### Keywords: Autonomous LLM Agents · Perception · Reasoning and

autonomous and intelligent behavior.

and generalized software bots that mimic human cognitive processes for

This paper shows how integrating these systems leads to more capable

execution system that translates internal decisions into concrete actions.

knowledge through both short-term and long-term mechanisms; and an

Chain-of-Thought and Tree-of-Thought; a memory system that retains

adapts to feedback, and evaluates actions through different techniques like

into meaningful representations; a reasoning system that formulates plans,

ponents include a perception system that converts environmental percepts

tasks and bridge the performance gap with human capabilities. Key com-

the limitations of traditional LLMs in real-world tasks, the research aims

ods of agents powered by large language models (LLMs). Motivated by

### Abstract. This paper reviews the architecture and implementation meth-

{habtom.gidey, alex.lenz, knoll}@tum.de

2 Technische Universität München, München, Germany

victor.de.lamo@estudiantat.upc.edu

1 Universitat Politècnica de Catalunya, Barcelona, Spain

Alois Knoll2

Victor de Lamo Castrillo1, Habtom Kahsay Gidey2, Alexander Lenz2, and

## Agents *⋆*

# Fundamentals of Building Autonomous LLM


---

enable the execution of tasks that were previously costly, time-consuming, or even infeasible. More than tools, agents act as collaborators, assisting humans in dynamic environments and automating decision-making in critical systems. However, this transformation is still in its early stages. Engaging with LLM agents is comparable to engaging with a new species, one that we are only beginning to understand, train, and guide [3].

intelligently? How should we structure their 'minds' so that they can interpret information, reason, plan effectively, and make decisions that we can trust? Building on this vision of LLM agents as intelligent collaborators, this review explores and defines the architectural foundations that enable their autonomous and effective performance in complex tasks [20].

### 1.2 Review Ob jective

The primary ob jective of this research is to review the design and implementation of intelligent agents powered by large language models (LLMs) to improve the execution of complex automation tasks [13, 14]. Specifically, the review focuses on the agents' perception, memory, reasoning, planning, and execution capabilities. The review aims to accomplish this by pursuing the following particular goals:

1. Explore the options for perception systems, including multimodal LLMs and
2. Examine reasoning architectures, such as Chain-of-Thought (CoT) and Tree-
3. Explore and evaluate memory-augmented architectures, such as Retrieval-
4. Examine the available execution architectures, such as tool-based frameworks,
5. Finally, evaluate the complexity of implementation of each system solution

### 1.3 Problem Statement

Building LLM agents to automate complex tasks can offer useful opportunities but also pose complex challenges [13, 23, 61]. Despite all the advances in LLMs, developing agents that perform well in various scenarios remains a significant

LLM agents represent a new paradigm that breaks traditional barriers. They

This raises a crucial question: How can we build agents who think and act

image processing tools, analyzing their contributions to interpreting visual inputs for task execution.

of-Thought (ToT), and their contributions to generating structured plans for complex tasks, including how reflection enhances iterative problem solving.

Augmented Generation (RAG) and long-term memory systems, investigating effective methods for information storage to enable practical and useful applications.

and code generation approaches, exploring their contributions to automating tasks.

proposed.

To achieve these ob jectives, some challenges need to be overcome.


---

challenge [23]. The purpose of this study is to address these issues by review- contrasting various strategies.

agents [13, 15, 16], reveal key limitations in multimodal agents, highlighting the following issues:

1. Difficulties in GUI grounding and operational knowledge: Agents
2. Repetitive actions: Agents frequently predict repetitive actions, indicating
3. Inability to handle unexpected window noise: Agents are not robust to
4. Limitations in exploration and adaptability: Particularly for agents
5. Significant performance gap with human capabilities: As reported on

To address these challenges and guide the investigation of agent design, this research presents a set of questions to explore the architectural components, integration strategies, and generalization capabilities of LLM-based agents.

### 1.4 Research Questions

To guide this survey, we formulate the following research questions that structure the analysis of architectural foundations, subsystem design, and evaluation of LLM based agents.

Benchmarks such as OSworld [71], alongside studies on autonomous software

struggle to accurately map screenshots to precise coordinates for their ac- interactions and application-specific features.

a lack of progress or an inability to break out of loops.

unexpected elements or changes in UI layout, such as unanticipated pop-up windows or dialog boxes.

equipped with modules like "Set-of-Mark" (SoM), it has been observed that they can constrain the agent's action space, hindering exploration and adapt-

the OSworld website [43], humans achieve a task completion rate of more than

72.36%. In contrast, leading models reach approximately 42.9% completion (as of June 2025), indicating a substantial gap with human performance.

1. RQ1, Design space, What architectural options exist for the core subsys-

tems of LLM-based agents, perception, reasoning and planning, memory, and execution, and how can they be systematically organized for practitioner use?

2. RQ2, Integration, Which subsystem integration patterns enable reliable

closed-loop autonomy in realistic software environments, for example, GUI and web tasks that combine visual grounding with structured signals such as DOM or accessibility trees [30, 56]?

3. RQ3, Reasoning efficacy, How do reasoning strategies, for example, CoT,

ToT, Re Act, and parallel planning, such as DPPM or MCTS-based approaches, affect task success rate, efficiency, and cost?

Building Autonomous LLM Agents 3


---

4. RQ4, Memory impact, How do long-term and short-term memory mech-
5. RQ5, Failures and mitigation, What are the principal failure modes
6. RQ6, Evaluation and generalization, Which benchmarks and metrics are

LLM-based agents.

## 2 Fundamentals

### 2.1 Background of LLMs

The introduction of machine learning methods, particularly deep learning, brought a significant shift by laying the groundwork for advanced modern AI models. Large language models (LLMs) are among the most significant developments. Their appearance represents a ma jor breakthrough in AI's ability to understand and produce complex language, influencing the state of LLM-based agents today and their future course.

transformer architecture, distinguished by its "attention mechanism" [52]. This mechanism allows LLMs to attend to different words in the input enabling them to understand long-range dependencies [52]. This architectural shift, alongside their training on vast datasets and the principles of generative AI, has enabled LLMs to perform a wide range of tasks, including natural language processing (NLP), machine translation, vision applications, and question-answering.

### 2.2 From LLMs to LLM Agents

LLMs in their standard form have significant limitations due to their chatbot nature. This restricts their effectiveness in real-world tasks. These models lack long-term memory, cannot autonomously interact with external tools, and struggle to pursue goals in dynamic environments. Such shortcomings hinder their perfor-

and are provided with tools to interact with the environment that enables them to function as autonomous agents. They are well-suited for dynamic tasks because they exhibit good planning skills, context adaptability, and they minimize human intervention. Such agents offer a scalable and flexible solution by simulating human-like team strategies and leveraging external tools [29].

anisms, for example, RAG and context management, influence accuracy, robustness to context length limits, and adaptation in long-horizon tasks?

in agentic settings, for example, hallucination, GUI misgrounding, repeti- reflection, anticipatory reflection, SoM, and guardrails, are most effective?

appropriate for assessing these systems, for example, OSWorld, Web Arena, and Mind2Web [8, 70, 71], and to what extent do agents generalize across tasks, applications, and interfaces?

Before delving into these research questions, let us first explore the origins of

A key technological advance in the development of LLMs has been the

To overcome these constraints, LLMs are guided to follow a reasoning path


---

does not make it an agent, in any case, that would make it a workflow.

### 2.3 Workflows vs. Agents

Many people confuse workflows with agents, but while both enhance the ca- Workflows are structured systems that enhance LLMs by enabling tool use, environmental interaction, or access to long-term memory. However, they are not agents. Workflows perform well in controlled and predictable environments where tasks are well defined and follow a fixed sequence of steps. In a workflow, the LLM follows a pre-established plan created by its designer, broken down into specific, sequential actions. This rigidity makes workflows highly effective for repetitive and structured tasks but limits their adaptability. If, during the workflow, the LLM faces an error, it often struggle to adjust, as they lack the ability to dynamically re-plan or adapt based on new information.

to act according to the feedback from its environment. Rather than relying on a pre-set plan, agents generate their own strategies tailored to the task and context, often using techniques like Chain-of-Thought reasoning or iterative refinement to break down complex problems. This adaptability allows agents to deal with unexpected challenges, bounce back from mistakes, and function well in unpredictable environments [3].

components and their interconnections.

### 2.4 Constitution of an Agent

### Perception System An agent begins its interaction with the world through its

perception system. This component is responsible for capturing and processing data from the environment, such as images, sounds, or any other form of informa- the LLM can understand and utilize, such as identifying ob jects or recognizing patterns.

### Reasoning System The reasoning system receives the task instructions along

with the data from the perception system and formulates a plan that is broken down into distinct steps. It is also responsible for adjusting this plan based on environmental feedback and evaluating its own actions to correct errors or improve execution efficiency.

### Memory System The memory system keeps the knowledge that is not embedded

in the model's weights. This includes everything from past experiences to relevant documents and structured data stored in relational databases. The LLM uses this information to enhance the accuracy of its responses.

However, simply augmenting an LLM with modules, tools, or predefined steps

In contrast, agents are far more versatile and autonomous. Agents are designed

To understand how these agents achieve autonomy, we first explore their core

Building Autonomous LLM Agents 5


---

environment, the perception system does not need to intervene.

description of the current state, recent events, or results of actions taken. In this

provides textual observations directly to the LLM's prompt. This could be a

LLM receives and processes this text description. In this mode, the environment

The simplest form in which the environment is described is purely in text. The

### 3.1 Text-Based Perception (Pure LLM)

ways: text-based, multimodal, information tree/structured data, and tool-based.

required determine the architecture. This challenge can be approached in four

and process. The complexity of the environment and the kinds of information

converting environmental stimuli into a format that the LLM can understand

The perception system of an LLM agent essentially acts as its "eyes and ears,"

## 3 Perception System

we now delve into a detailed exploration of the perception system.

Having outlined the core components that enable an LLM agent's autonomy,

### Fig. 1. Key Components of an Agent's LLM Architecture

movements in a software environment [39].

involve using a set of tools, such as calling APIs or writing code to execute mouse

completing the interaction cycle by executing what has been decided. This can

that the agent's instructions are carried out in the real or simulated world,

decisions into concrete actions that impact the environment. This module ensures

**Action System** Finally, the action system is responsible for translating abstract


---

aligned multimodal representations (visual embeddings and textual features)

- LLM Backbone: This is the core reasoning engine. The processed and

multimodal reasoning [34, 50].

LLM, enabling the LLM to leverage its pre-trained linguistic knowledge for

processing ensures that the visual embeddings are effectively supplied to the

that the LLM can comprehend and integrate alongside textual inputs. This

## LLM. It acts as a bridge, transforming the visual embeddings into a format

textual modalities (e.g., visual embeddings) with the text feature space of the

**-Input Pro jector:** This component aligns the encoded features from non-

Transformers (ViT) are used to extract rich visual representations [34, 45].

specialized encoders like Convolutional Neural Networks (CNNs) or Vision

3D data, to obtain corresponding features or embeddings. For visual inputs,

inputs from various modalities, such as images, videos, or even audio and

**-Modality Encoder (ME):** This component is responsible for encoding

pipeline with distinct components [67]:

The general architecture of MM-LLMs typically comprises a structured

and generation across a diverse range of multimodal tasks.

process and connect modalities but also to perform complex reasoning, planning,

language model as their central processing unit. This enables them not only to

representations, MM-LLMs leverage the inherent reasoning capabilities of a large

or outputs. Unlike VLMs, which primarily aim to align visual and linguistic

proach of augmenting powerful, off-the-shelf LLMs to support multimodal inputs

MM-LLMs represent a significant advancement, distinguished by their ap-

(embeddings) that can be processed and compared together by the model [34].

that both visual and textual data are converted into numerical representations

learning of a unified embedding space for vision and language. This means

Regardless of the specific training paradigm, a fundamental principle is the

precise spatial relationships or accurate ob ject counting without external aid [9].

vision, it still has some challenges. For instance, most models still struggle with

Although significant progress has been made in the extension of LLMs to

both modalities.

images and words, allowing agents to understand and generate content across

Language Models (MM-LLMs). These models aim to bridge the gap between

Language Models (VLMs) and their more advanced successors, Multimodal Large

crucial. In the context of LLM agents, this is largely achieved through Vision-

textual and visual (images, videos), thanks to multimodal perception. For agents

Agents can process and integrate information from a variety of sources, mainly

### 3.2 Multimodal Perception

text-driven simulations.

that give the response to LLM interactions in text. This is practical for chats or

directly with the LLM's core capabilities. However, it is limited to environments

This approach offers low computational overhead for perception and integrates

Building Autonomous LLM Agents 7


---

through a specialized adaptive architecture and the integration of additional

**-Segmentation and Depth Maps:** VCoder enhances MM-LLM capabilities

enhance visual perception with visual encoders:

a much lower computational and developmental cost. These are different ways to

the MM-LLM, it offers a practical trade-off by significantly improving results at

doesn't match the performance gains of directly improving each component of

images to help the MM-LLM interpret them more effectively. While this approach

These encoders, which can be separate models, extract relevant information from

improving each individual component of an MM-LLM) is to use visual encoders.

A faster and more cost-effective way to enhance perception (rather than

tendency to hallucinate non-existent entities.

visual perception, such as accurately identifying or counting ob jects, and a

(2023) [28], traditional MM-LLM systems often face limitations in fundamental

Versatile Vision Encoders for Multimodal Large Language Models" by Jain et al.

### Enhancing Perception in MM-LLMs As outlined in the paper "VCoder:

limitations in visual understanding, as explored in the following subsection.

ing, their perceptual capabilities often require further enhancement to address

While the architectural components of MM-LLMs enable multimodal process-

### Fig. 2. Architecture of Multimodal Large Language Models (MM-LLMs) for Under-

images using models like Latent Diffusion Models.

tasked with producing outputs in distinct modalities, such as synthesizing

**-Modality Generator (for multimodal generation):** This component is

by a Modality Generator.

token representations from the LLM Backbone into features understandable

puts in other modalities (e.g., generating images), this component maps signal

**-Output Pro jector (for multimodal generation):** For tasks requiring out-

are fed to the LLM. The LLM processes these representations, answering


---

environmental interpretation, as explored in the following subsection.

as Accessibility Tree and HTML utilization, offer alternative methods for robust

through targeted annotations and prompting, structured data approaches, such

While techniques like Set-of-Mark and VCoder enhance visual perception

perception capabilities of LLM-based agents.

reduced hallucination. This highlights the ongoing efforts to enhance the granular

on ob ject-level perception tasks, demonstrating improved counting accuracy and

LLMs adapted with VCoder and SoM significantly outperform baseline models

Experimental evidence presented in the papers [28, 64] indicates that MM-

focus on specific areas during reasoning. This technique improves the model's

boxes or labels) that highlight key regions or ob jects, enabling the model to

process consists in annotating images with explicit markers (e.g., bounding

guide MM-LLMs in processing visual inputs. As seen in Fig. 4 set-of-mark

visual tasks, Set-of-Mark (SoM) operation provides a structured approach to

**-Set-of-Mark Operation:** To enhance the model's ability to handle complex

### Fig. 3. Usage of segmentation and depth maps for MM-LLM perception [28]

the LLM's embedding space via additional vision encoders [45].

spatial relationship details). Information from these inputs is pro jected into

fine-grained ob ject and background information) and depth maps (providing

the model to process "control inputs" such as segmentation maps (offering

perception modalities. It functions as an adapter to a base MM-LLM, enabling

Building Autonomous LLM Agents 9


---

### 3.3 Information Tree/Structured Data Perception

**-Accessibility Tree Utilization:** OSCAR [56] utilizes an A11y tree gener-

- HTML Utilization: Meanwhile, DUALVCR [30] captures both the visual

### 3.4 Tool-based Perception

Beyond direct multimodal inputs and structured data retrieval, LLM-based agents can significantly enhance their perception capabilities through tool augmentation. This means utilizing external tools and APIs to enable the agent to gather, process, and interpret data from a wider variety of sources, including real-world sensors and specialized databases. The mechanism of integration typically involves the LLM generating specific tool calls based on its current understanding and goals, with the results from these tools being "fed back" into the LLM [44, 47].

### Categorizing Tools for Perception The diverse landscape of external tools

available to LLM agents can be broadly categorized based on the type of infor-

ated by the Windows API for representing GUI components, incorporating descriptive labels to facilitate semantic grounding.

features of the screenshot and the descriptions of associated HTML elements to obtain a robust representation of the visual screenshot.

**-Web Search and Information Retrieval APIs:** These tools allow agents

to access vast amounts of up-to-date information, facts, and specific data points from the internet. By issuing queries to search engines (e.g., Google Search API) or structured knowledge bases (e.g., Wikipedia API), agents can

### Fig. 4. Image with Set-of-Mark [64]


---

HTML source of the page [15]. This tree provides a hierarchical representation of

In parallel, the agent retrieves the Accessibility Tree (A11y Tree) or the

its text content (if any), a brief description, and its coordinates.

bounding boxes and a structured list describing each detected element, including

and stores the coordinates of each box. The output consists of the image with the

a box on every interactive element on the screen, such as buttons or checkboxes

then applies a Set-of-Mark operation using a visual encoder. This encoder draws

To achieve this, the agent starts by capturing a screenshot of the email app. It

respond to incoming company emails.

scenario where the agent's ob jective is to identify, classify, and, if necessary,

Although this could be easier to achieve using the email API, imagine a

Interface (GUI), such as managing emails in a web-based application.

Let's consider an LLM agent designed to automate tasks within a Graphical User

### 3.5 Example of a Perception System in an LLM Agent

example.

empowers an LLM agent to effectively handle tasks, as illustrated in a practical

Let's now explore how integrating the diverse perception system approaches

beyond simple text matching [10, 42].

local databases. This allows for dynamic and flexible data interpretation

parsing complex log files, running statistical analyses on datasets, or querying

scripts via an interpreter), agents can perceive insights from raw data, such as

processing and calculations. By generating and executing code (e.g., Python

- Code Execution Tools: These tools enable agents to execute code for data

crucial for tasks in robotics or interactive simulations [2, 7].

perceive physical properties and spatial relationships of its environment,

(textual descriptions, structured data like JSON). This allows the agent to

data) from real-world or simulated environments into a digestible format

sensory data (e.g., temperature readings, GPS coordinates, accelerometer

This is achieved through intermediary tools or services that convert raw

perception system can be augmented to interpret data originating from them.

LLM agent does not directly interface with physical hardware sensors, its

**-Sensor Integration (Conceptual via Intermediary Tools):** While an

as document-centric microservices for knowledge discovery [17].

specific information relevant to niche tasks [32, 44], and can be implemented

research papers and experimental data). These tools enable agents to perceive

data), or scientific databases and literature APIs (for accessing specialized

forecasted climatic conditions), stock market APIs (for real-time financial

data types. Examples include weather APIs (for perceiving current and

**-Specialized APIs:** Agents can use domain-specific APIs designed for specific

44, 47].

is crucial for tasks requiring current affairs knowledge or factual accuracy [40,

data cutoff. This helps the agent fill in missing environmental information and

perceive real-time events, verify facts, or retrieve details beyond their training

Building Autonomous LLM Agents 11


---

capable LLM agents.

component, but fundamental enablers for building more intelligent, reliable, and

vancements in perception technologies are not merely improvements to one

directly affects the reasoning and planning modules. Therefore, continuous ad-

Ultimately, the quality and fidelity of an LLM agent's perception system

environments or for widespread adoption.

and inference. This can be a barrier for execution in resource-constrained

timodal inputs, requires high computational resources for both training

**-Computational Resources:** High-fidelity perception, especially with mul-

timodal or specialized domains, often requires large volumes of high-quality,

**-Data Collection:** Training robust perception systems, particularly for mul-

Encoding and feeding this entire information into the LLM's context window

extensive structured data, can generate a vast amount of tokens or embeddings.

**-Context Window Limits:** Large inputs, such as high-resolution images or

bottlenecks, hindering the agent's responsiveness.

pipelines, from raw data acquisition to final LLM interpretation, can create

demand rapid perceptual updates. The sequential nature of many perception

requiring real-time interaction (e.g., robotics, dynamic GUI automation),

introduce substantial latency. Real-world applications, particularly those

especially those involving multimodal processing or external tool calls, can

**-Latency in Inference Pipelines:** Integrating complex perception modules,

or undesirable behavior [25].

agents making decisions based on incorrect interpretations, resulting in errors

or misinterpret visual cues remains a significant hurdle. This can lead to

**-Hallucination:** The tendency for models to "hallucinate" non-existent ob jects

across all approaches:

advanced perceptual capabilities, several critical challenges and limitations persist

While significant progress has been made in empowering LLM agents with

### 3.6 Perception Challenges and Limitations

and restrictions that can impact its performance and reliability.

Despite the robustness of this perception system, it has a number of drawbacks

of the GUI environment.

LLM, this perception system enables the agent to build a rich, actionable model

structure. When combined with the image understanding capabilities of a MM-

perception system. This system allows the agent to understand the interface: its

The accessibility tree and the visual encoder output combine to create a

browser automation tools.

their roles, labels, states (e.g., "unread"). Such data is typically extracted through

GUI components, such as buttons, text fields, links, and list items-along with


---

### Modality Input Format Tool Dependencies Strengths Limitations

### Table 1. Summary of Perception Approaches for LLM-Based Agents

Text-Based Plain text None (relies on LLM's Low computational Limited to text-only Perception descriptions native text processing)overhead; seamless environments; cannot

Multimodal Text, Vision-Language Models Processes diverse data types;High computational cost,

Building Autonomous LLM Agents 13Perception image/video (e.g., CLIP, ViT), suitable for GUIs and struggles with precise

embeddings, Multimodal LLMs, real-world tasks; leveragesspatial tasks and requires audio transcriptspreprocessing tools (e.g.,advanced VLMs extensive training data

CNNs, ASR)

Information JSON, XML, Parsers, database query Precise semantic Limited to environments Tree/Structured database records,tools, accessibility understanding; efficient forwith structured data and Data Perception A11y trees frameworks structured environments likerequires predefined schemas

Tool-Augmented Tool outputs External APIs, code Extends perception to Dependent on tool Perception (text, JSON, interpreters, sensor real-time and specialized availability and reliability,

numerical data) interfaces, web search data; highly flexible and complex integration and

tools dynamic error handling


---

comprehensive understanding of the GUI environment, as summarized in the preceding table, the next critical component is the reasoning system. This system leverages the processed perceptual input to make informed decisions and execute complex tasks.

## 4 Reasoning System

### 4.1 Task Decomposition

A key tactic for helping LLM agents solve complicated problems is task decom- subtasks. This approach, akin to the "divide and conquer" algorithmic paradigm, simplifies the planning process. The procedure involves two main steps: first, the "decompose" step, where the complex task is broken into a set of subtasks; and second, the "subplan" step, where for each subtask a plan is formulated [26]. This systematic breakdown helps in navigating intricate real-world scenarios that would otherwise be challenging to address with a single-step planning process.

Decomposition first and Interleaved decomposition [26]. Decomposition first methods, as seen in systems like HuggingGPT [48] and Plan-and-Solve [55], initially decompose the entire task into sub-goals and then proceed to plan for each sub-goal sequentially. HuggingGPT, for instance, explicitly instructs the LLM to break down multimodal tasks and define dependencies between subtasks [48]. A slightly modified version of the Decomposition first approach is DPPM (Decompose, Plan in Parallel, and Merge). It addresses the limitations of existing planning methods, such as:

1. Handling heavy constraints
2. Carrying errors from the planning of previous steps
3. Forgetting the main goal
4. Cohesion between subtasks

DPPM tackles these problems with the following methods: First, it decomposes the complex task into subtasks. Second, it generates subplans for each of these subtasks concurrently using individual LLM agents. This parallel planning allows each agent to focus only on its assigned subtask, promoting independent work and avoiding the cascading errors that can occur when subplans are sequentially dependent. Finally, DPPM merges these independently generated local subplans into a coherent global plan [36]. Although this method can struggle to adapt well to unexpected environmental problems, this limitation can be mitigated by reflecting on the plan after each execution step.

(CoT) [60] and Re Act [66], interleave the decomposition and subtask planning process, revealing only one or two subtasks at a time based on the current state. This dynamic adjustment based on environmental feedback enhances fault

Having established how the perception system equips an LLM agent with a

Current methodologies for task decomposition broadly fall into two categories:

In contrast, interleaved decomposition methods, such as Chain-of-Thought


---

There are various strategies:

process of generative models.

diverse set of candidate plans, often by leveraging the uncertainty in the decoding

generation and optimal plan selection [26]. Multi-plan generation aims to create a

for a given task [58]. This methodology involves two main stages: multi-plan

approach, focusing on leading the LLM to explore multiple alternative plans

even infeasible. To address this, multi-plan selection emerges as a more robust

LLMs, a single plan generated by an LLM Agent may often be suboptimal or

Due to the inherent complexity of tasks and the uncertainty associated with

### 4.2 Multi-Plan Generation and Selection

decomposition-planning, interleaved decomposition-planning, and DPPM [36].

*Fig. 5. Comparison of different types of planning frameworks, including sequential*

then combine them to derive final results [63].

agents first generate comprehensive plans and obtain observations independently,

a modular paradigm that decouples reasoning from external observations, where

the LLM to regenerate the plan with corrective actions [35]. ReWOO introduces

due to unmet prerequisites, a precondition error message is introduced, prompting

each step of a plan meets necessary prerequisites before execution. If a step fails

approaches such as Re Prompting and ReWOO. Re Prompting involves checking if

Further advancements in task decomposition and planning strategies include

lead to hallucinations or deviation from original goals [26].

tolerance, although excessively long tra jectories in complex tasks can sometimes

Building Autonomous LLM Agents 15


---

solutions within expansive search spaces. However, this comes with trade-offs like

plan selection is a significant advantage, allowing for a broader exploration of

multi-plan searches using the MCTS algorithm [24]. The scalability of multi-

for expansion and selection, evaluating multiple actions to choose the optimal

such as conventional Breadth-First Search (BFS) and Depth-First Search (DFS)

More advanced methods like Tree-of-Thought leverage tree search algorithms

utilizes a simple ma jority vote strategy to identify the most suitable plan [58].

where different search algorithms are employed [26]. Self-consistency, for instance,

Once a set of candidate plans is generated, the next step is plan selection,

benefits of different plans using MCTS to generate the final plan.

process [68]. RAP [24] specifically builds a world model to simulate potential

(or plans) are obtained through multiple calls to the LLM during the MCTS

function for the Monte Carlo Tree Search (MCTS). Multiple potential actions

- LLM-MCTS and RAP: These methods leverage LLMs as a heuristic policy

*Fig. 6. Schematic illustrating various approaches to problem solving with LLMs [65].*

gies [4].

for transformations of thoughts, leading to more powerful prompting strate-

Graph-of-Thought (GoT) extends the tree-like reasoning structure of ToT

evaluations. Unlike CoT-SC, ToT queries LLMs for each reasoning step [65].

an intermediate "thought." The selection of these steps is based on LLM

ates plans using a tree-like reasoning structure where each node represents

**-Tree-of-Thought (ToT) and Graph of Thoughts (GoT):** ToT gener-

paths and their corresponding answers using Chain of Thought (CoT), then

- Self-consistent CoT (CoT-SC): This approach generates various reasoning


---

allowing an agent to learn from its past mistakes by writing the feedback and

linguistic feedback rather than traditional weight updates. It operates iteratively,

is a framework designed to improve the performance of language agents through

the paper "Reflection: Language Agents with Verbal Reinforcement Learning," [49]

**How to Implement a Reflection System:** A Reflection system, as described in

effective reflection system in LLM agents.

we now explore the practical steps and components required to implement an

Building on the conceptual framework of reflection and its key characteristics,

no explicit error occurred.

efficiency or completeness, aiming to optimize their path to the goal even if

**-Goal-Driven Reflection:** Agents can reflect not just on errors, but also on

actions [6, 38].

its "memory" or state [49], or generating a revised plan or a new set of

correcting its reasoning process, learning better ways to use tools, updating

ates actionable insights. This might involve modifying its planning strategy,

**-Correction and Improvement:** Based on the analysis, the agent gener-

environmental changes. Papers like [49] and [38] exemplify this capability,

derstandings of the prompt, incorrect tool usage, logical inconsistencies, or

why a plan failed, or where the reasoning failed. This can be due to misun-

**-Error Detection and Analysis:** Identifying where things went wrong,

actual and expected outcomes.

generated plans, and the results of its actions. This often involves comparing

**-Self-Evaluation:** The agent examines its completed (or ongoing) task, its

Key characteristics of reflection include:

mistakes or inefficiencies without human intervention.

insights to improve its future performance. This allows agents to learn from their

evaluate its own past actions, reasoning, and outcomes, and then use these

Reflection, in the context of LLM agents, refers to the agent's ability to critically

### 4.3 Reflection

and adaptability in dynamic environments.

tions and outcomes after the execution, encouraging continuous improvement

by the process of reflection. This mechanism allows agents to evaluate their ac-

While multi-plan selection enables LLM agents to explore and evaluate multi-

and the potential for randomness due to the stochastic nature of LLMs, which

evaluation introduces challenges regarding their performance in ranking tasks

increased computational demands. Furthermore, the reliance on LLMs for plan

Building Autonomous LLM Agents 17


---

continues with the next group of steps.

1. Successful execution: The actions produced the expected result, so the agent

which would determine the current scenario:

the environment. This feedback would be processed by a reflection mechanism,

steps. As the agent carries out each group of steps, it would receive feedback from

After the final plan is constructed, it would be divided into groups of executable

toward completing the main task.

resulting plan is logically consistent and that all subplans contribute meaningfully

it would explore various combinations of the subtask options, ensuring that the

subtask plans into a final, coherent plan to accomplish the overall goal. To do this,

Following the Merge step in DPPM, the agent would integrate the different

of the "DEVIL'S ADVOCATE" paper mentioned before.

This process combines ideas from Tree-of-thought and the Anticipatory Reflection

problems, it would propose alternative approaches to either solve or avoid them.

that might arise during the execution of each subtask. Based on these anticipated

subtask. While generating these options, the LLM would consider potential issues

in separate calls to an LLM, different planning options would be generated for each

First, the agent would decompose the main task into smaller subtasks. Then,

and Merge).

tioned above. Its core mechanism could be DPPM (Decompose, Plan in Parallel,

A reasoning system can be developed by integrating some of the features men-

### 4.4 Example of a Reasoning System

mitigate challenges, improving its ability to navigate complex tasks effectively.

enhances consistency and adaptability by allowing the agent to anticipate and

advocate" to challenge its own proposed steps. This front-loaded introspection

alternative remedies before executing an action, essentially acting as a "devil's

consists of the agent proactively reflecting on potential failures and considering

Agents" [53] introduces a distinct perspective: Anticipatory Reflection. This

The paper "DEVIL'S ADVOCATE: Anticipatory Reflection for LLM

specific feedback.

signal (e.g., success/fail) and the current tra jectory, it produces nuanced and

and is responsible for generating verbal self-reflections. Given a sparse reward

**-Self-Reflection Model:** Another LLM serves as the self-reflection model

grading, predefined heuristics, or even another LLM instance.

and computes a reward score. Evaluation can be based on exact match

outputs. It takes a complete tra jectory (sequence of actions and observations)

**-Evaluator:** This component assesses the quality of the Actor's generated

current state observations and its memory.

**-Actor:** This is typically a LLM that generates text and actions based on the

Core Components:

of how to implement such a system:

storing and using these reflections in the next iterations. Here's a brief explanation


---

2. Minor error: The actions were close but not entirely accurate (e.g., the agent
3. Execution failure: The plan cannot be completed as-is (e.g., the button to be

*Fig. 7. Flowchart of a Reasoning System Using Decompose, Plan, and Merge (DPPM)*

approach with a reflection system

missed clicking a button because the coordinates were slightly off ). In this case, the steps would be adjusted and corrected accordingly.

clicked does not exist). Here, the agent must reflect on whether the issue lies within the specific subplan or if the entire plan needs to be reconsidered. If only the subplan is flawed, a new one would be generated. If the problem is more fundamental, the entire planning process would restart from the beginning.

Building Autonomous LLM Agents 19


---

secure practices, and monitoring risks in multi-agent interactions [21, 51].

tools [12, 18]; and a Security Expert for mitigating vulnerabilities, promoting

ous applications [21], who can also leverage existing model-driven verification

for ensuring adherence to predefined rules, constraints, and assurances in vari-

a Human-Computer Interaction (HCI) Expert for optimizing user experience

trieval Expert for efficiently acquiring knowledge from external sources [21, 33];

pert for generating, debugging, and optimizing code [51]; an Information Re-

In addition to the experts mentioned above, there could be other helpful

mouse movements in benchmarks like OSWorld. [21, 33, 71].

other systems. For example, it is responsible for creating the move and click

commands or API calls to interact with external tools, web interfaces, or

interactions with the environment. It's skilled in generating the necessary

**-Action Expert:** This expert knows how to translate plans into concrete

challenge in LLM-based multi-agent systems [23, 33].

and that the agent's context is maintained effectively, which is a critical

memory. This expert ensures that relevant information is retrieved efficiently

**-Memory Management Expert:** Responsible for handling the agent's

support self-healing behaviors in adaptive architectures [19].

propose to scroll down if an item is not found in a webpage [51]. It can also

identify common failure patterns, and propose fixes. For example, it could

and suggesting recovery strategies for errors. This expert could analyze logs,

- Error Handling Expert: Specifically focused on identifying, diagnosing,

the reflection system [33].

overall performance. This aligns with the evaluator component discussed in

**-Reflection Expert:** It is dedicated to evaluating plans, responses, and

complex tasks [33].

reflection system, where agents perform reasoning and planning to undertake

manageable subtasks. This aligns with the actor component discussed in the

decomposition. Its role is to break down complex ob jectives into a series of

**-Planning Expert:** This expert focuses on strategic thinking and task

useful experts that an LLM agent could integrate:

increasing its capabilities and robustness [5]. Here are some examples of such

the interaction or reasoning. This modularity enables specialization at each step,

of different specialized "experts," each of whom focuses on a distinct aspect of

Expanding on the idea of multi-agent systems, a single agent can be made up

### 4.5 Multi-Agent Systems

scalability and efficiency.

tems distribute these processes across specialized components to achieve greater

like DPPM, combined with reflection, we now explore how multi-agent sys-

Having illustrated how a single LLM agent can leverage a reasoning system


---

the environment. If any tools are required, it consults the tool expert to determine

Next, the execution expert generates the specific actions to be performed in

planning.

constraint satisfaction expert to ensure that no constraints are violated during

or repeated attempts if problems occur. Additionally, it collaborates with the

main task into subplans. This expert is also responsible for avoiding infinite loops

**Example of a Multi-agent System** First, the planning expert decomposes the

framework.

ing example illustrates how these components collaborate within a multi-agent

With the methodology for crafting specialized experts established, the follow-

term context and long-term knowledge) which can store past experiences or

**-Memory Integration:** The expert may have access to its memory (short-

databases that provide specific, up-to-date, or proprietary knowledge relevant

- External Knowledge Bases: Integrating the expert with external tools or

performance.

base LLM on a dataset relevant to the expert's domain can enhance its

**-Fine-tuning (if applicable):** For highly specialized tasks, fine-tuning a

techniques such as Chain-of-Thought to enhance its reasoning process.

LLM toward performing as the expert, incorporating specific prompting

- Targeted Prompting: Crafting precise and detailed prompts to steer the

knowledge. This can be achieved by:

### Equip with Knowledge An expert's effectiveness hinges on its specialized

experts be consulted or take over? [33].

**-Boundaries:** What are the limitations of its expertise? When should other

input, and what kind of output does it produce?

**-Input and Output:** What kind of information does this expert take as

will this expert excel at? (e.g., planning, code generation, error handling).

**-Clear Specialization:** What specific task, domain, or reasoning capability

This involves:

step is to precisely define the "distinctive attributes and roles" [51] of your expert.

Define the Expert's Role and Scope (Profile and Specialization). The first

principles and leveraging the capabilities of Large Language Models

Building an "expert" within an LLM agent involves a combination of design

### 4.6 How to Build an Expert

turn to the practical process of designing and building these experts.

Having outlined some possible experts within multi-agent systems, we now

Building Autonomous LLM Agents 21


---

which tools to use and how to use them. If executable code is needed beyond basic actions, the coding expert is called upon to produce it.

processed by the reflection expert, which works together with the error handling expert to diagnose issues and propose solutions. Based on this diagnosis, the reflection expert decides how to proceed.

or successful workflows related to similar tasks. This knowledge is used to inform and enhance the next steps proposed to the planning or execution experts.

Once actions are executed, feedback from the environment is received and

To improve its recommendations, the memory expert retrieves past experiences

### Fig. 8. Example of the communication between agents in a multi-agent system


---

**Component Description Key Techniques/Approaches Advantages Challenges/Limitations**

### Table 2. Key Components and Techniques for the Reasoning System (Part 1)

**Task** Breaks down complex - Sequential Decomposition: Divides - Simplifies complex- DPPM struggles with

**Decomposition** tasks into manageabletasks into sequential subgoals and plansproblem-solving. unexpected environmental

subtasks to simplify (e.g., Divide-and-Conquer). - DPPM reduces changes. planning and - Interleaved Decomposition: cascading errors via- Interleaved methods may execution. Dynamically adjusts subtasks based on parallel planning. lead to hallucinations or

feedback (e.g., Chain-of-Thought [CoT],- Interleaved methodsdeviation in long tasks. Re Act). enhance fault

                - DPPM (Decompose, Plan in tolerance.

Building Autonomous LLM Agents 23 **Parallel, Merge)**: Decomposes tasks,

plans subtasks concurrently, and merges into a coherent global plan.

**Multi-Plan** Generates multiple **Self-consistent CoT (CoT-SC)**: - Explores diverse - High computational

**Generation and** candidate plans and Generates multiple reasoning paths and solutions for robustdemands.

**Selection** selects the optimal oneselects the most frequent answer. planning. - Stochastic nature of LLMs

to address task **Tree-of-Thought (ToT)**: Uses tree-like - Scalable for may affect plan consistency. uncertainty. reasoning structures for plan generation.complex tasks with - Challenges in ranking and

 **Graph-of-Thoughts (GoT)**: Extends large search spaces.evaluating plans. ToT with graph structures for flexible aggregation.

### LLM-MCTS and RAP: Use Monte

Carlo Tree Search for plan generation and selection.


---

**Component Description Key Techniques/Approaches Advantages Challenges/Limitations**

### Table 3. Key Components and Techniques for the Reasoning System (Part 2)

**Reflection** Allows agents to **Self-Evaluation**: Compares actual vs. - Enables learning - Requires robust feedback

evaluate actions expected outcomes. from mistakes mechanisms. post-execution, - Error Detection and Analysis: without human - May be limited by the identify errors, and Identifies and analyzes errors (e.g., intervention. agent's ability to accurately improve future incorrect tool usage, logical flaws). - Enhances self-evaluate. performance. - Correction and Improvement: adaptability and

Adjusts plans or strategies based on efficiency.

                - Anticipatory Reflection (DEVIL'S reflection improves

**ADVOCATE)**: Proactively considers consistency. potential failures before execution.

**Multi-Agent** Distributes reasoning- Planning Expert: Handles task - Enhances - Requires careful

**Systems** tasks across decomposition and strategic planning. modularity and coordination between

specialized "experts"- Reflection Expert: Evaluates plans and robustness. experts. for scalability and suggests improvements. - Leverages - Potential for increased efficiency. - Error Handling Expert: Diagnoses specialized expertisecomplexity in system design.

and proposes fixes for runtime errors. for complex tasks. - Security risks in

                - Others: Includes Memory Management, - Improves scalabilitymulti-agent interactions.

Action, Coding, Information Retrieval, through division of Dialogue Management, HCI, Constraint labor. Satisfaction, and Security Experts.


---

attention mechanism [72].

are especially well-suited for producing intricate SQL queries because of their

techniques facilitate reliable database interaction. Transformer-based models

table. By converting natural language queries into SQL queries, text-to-SQL

as information about employees, orders, or other data that can be stored in a

**-SQL Database:** SQL databases are used to store structured knowledge, such

of "hallucinations" [31].

the response precise for the specific use case and reducing the likelihood

ate responses that are based on company files or personal documents making

alongside the original query. This augmented input enables the LLM to gener-

Once the relevant information is retrieved, it is added to the LLM context

in its training data or within its immediate context window.

LLM access to updated and precise information that might not be encoded

indexed by vector embeddings) to locate relevant documents. This gives the

retriever component first looks through an external knowledge base (often

It operates in two main phases: retrieval and augmentation. Using a query, a

LLMs by using external knowledge to improve the accuracy of its responses.

**-RAG:** Retrieval-Augmented Generation (RAG) is a technique that enhances

to what it has learned from these experiences [62].

directly into its neural network. This causes the model to act in ways similar

data, it adjusts its weights, effectively encoding new "facts" or "experiences"

of memory is build into the model itself. When an LLM is fine-tuned on new

learning processes like fine-tuning. Unlike external memory systems, this type

ingrained directly within its model parameters (weights) through continuous

refers to the idea that an agent's experiences and learned behaviors become

**-Embodied Memory:** In the context of LLMs, "embodied memory" often

different ways of implementing it:

the agent to retain knowledge apart from its pre-trained knowledge. There are

past memories and learn information from previous interactions. It also enables

the models to evolve and adapt over time. It allows agents to store relevant

Long-term memory in LLM agents is crucial for sustained interaction and for

### 5.1 Long-term memory

while short-term memory facilitates immediate contextual awareness.

time scales, with long-term memory anchoring sustained knowledge retention

The memory system empowers LLM agents to manage information across varying

## 5 Memory System

inform and enhance these reasoning processes.

provides the critical foundation for retaining and applying past experiences to

and collaborate on complex tasks, we now consider the memory system, which

Having explored how reasoning systems enable LLM agents to plan, reflect,

Building Autonomous LLM Agents 25


---

ities (e.g., where they spent the last Christmas) or background (e.g., where

information that the user has supplied, such as details about their past activ-

**-User information:** Beyond just user preferences, this includes personal

covery pipelines in microservices architectures [17].

machinery, and internal company rules [11], including document-based dis-

**-Knowledge:** This category encompasses external information received as

to guide subsequent generations [59].

training examples and then selectively provides these workflows to the agent

(AWM) is a method that induces commonly reused routines (workflows) from

riences to guide future actions, similar to humans. Agent Workflow Memory

**-Procedures:** LLM agents can learn reusable task workflows from past expe-

generalized steps, which can then be integrated into the agent's memory to

later use, such as inducing a workflow with a summarized description and

of experiences. This format ensures that the experience is retrievable for

is saved in a storage system like a database or a JSON file within a collection

experience with the instruction and a tra jectory of observation-action pairs,

action performed (e.g., click("126") or stop()). This data, structured as an

of the environment (e.g., "The current page shows order 0130") and the

of steps taken to solve it, where each step includes the agent's observation

language instruction (e.g., "Who ordered order 0130?") and the sequence

ability to adapt [1, 22]. To store an experience, you capture a task's natural

of "invalid action filtering," contributes to the agent's robust development and

This continuous learning from past interactions, including the identification

experience," LLMs can learn to avoid repeating similar mistakes in the future.

logged and distinguished as such, can be valuable. By explicitly noting a "failed

tasks. Research has indicated that even failed experiences, when appropriately

**-Experiences:** It is beneficial to store records of both successful and failed

manner.

accumulate experiences, evolve, and behave in a more consistent and effective

This stored data is then used to make better decisions, enabling the agent to

diverse types of information perceived from its environment and interactions.

The memory module within an LLM agent's architecture is designed to store

### 5.3 What Kind of Data to Store

store.

awareness, the memory module's effectiveness hinges on what kind of data to

Regardless of whether it's for long-term retention or immediate contextual

tained within the context window, which acts as a temporary workspace [54].

Short-term memory in LLM agents is analogous to the input information main-

### 5.2 Short-term memory


---

accumulating counts, thereby avoiding redundant storage [54].

newly generated one. Another method aggregates duplicate information by

solution using LLMs, and the original sequences are then replaced with this

reaches a size of five, all sequences within it are condensed into a unified plan

sequences related to the same sub-goal are stored in a list. Once this list

ory Duplication" problem. For instance, in one approach, successful action

been developed to integrate new and previous records to address this "Mem-

**-Memory Duplication:** When storing information in memory, a potential

overcome this is to truncate large texts or summarize them [57].

integrate or utilize all information in very long sequences. The easiest way to

The primary impact of a limited context window is that LLMs cannot directly

consider at any one time when generating a response or performing a task.

be words, parts of words, or punctuation) that an LLM can process and

refers to the maximum amount of text (measured in "tokens," which can

mental constraint known as the "context window" or "context length." This

- Context Window: Large Language Models (LLMs) operate with a funda-

### 5.4 Limitations

sub ject to several limitations.

effectiveness, the utility and management of this stored information are inherently

While defining what kind of data to store is crucial for an LLM agent's

personal details [69].

previous interactions, which inherently involves storing and utilizing these

and adapt to a user's personality over time by synthesizing information from

their parents are from). Mechanisms like Memory Bank aim to comprehend

Building Autonomous LLM Agents 27


---

**Component Description Key Techniques/Approaches Advantages Challenges/Limitations**

### Table 4. Memory Components for LLM-Based Agents (Part 1)

**Long-term** Stores knowledge for- Embodied Memory: Experiences are - Enables persistent- Fine-tuning for embodied

**Memory** sustained retention,ingrained in the model's parameters knowledge retention.memory is computationally

enabling agents to through continuous learning (e.g., - RAG reduces expensive. recall past fine-tuning). hallucinations by - RAG requires efficient experiences and **Retrieval-Augmented Generation** grounding responses indexing and retrieval synthesize **(RAG)**: Retrieves relevant documents in verifiable sources.systems. information from from an external knowledge base using - SQL databases - Text-to-SQL generation

28 de Lamo et al. previous interactions.vector embeddings to enhance responses.support structured, may struggle with complex

                                - SQL Database: Stores structured data queryable data queries or dependencies.

(e.g., employee or order details) accessibleaccess. via text-to-SQL queries generated by LLMs.

**Short-term** Acts as a temporary - Context Window Management: - Facilitates - Limited by context

**Memory** workspace within the Maintains recent conversational or inputimmediate contextualwindow size, leading to

LLM's context data within the transformer's limited awareness. truncation of older data. window, holding context window. - Chunking and - Summarization may omit immediate contextual- Chunking and Summarization: summarization critical details if not information for Breaks down large inputs into manageableprevent information carefully designed. ongoing tasks. pieces and condenses essential informationloss in long

to fit within the context window. sequences.


---

**Component Description Key Techniques/Approaches Advantages Challenges/Limitations**

### Table 5. Memory Components for LLM-Based Agents (Part 2)

**Data Storage** Defines the types of - Procedures (Agent Workflow - Workflows improve - Managing diverse data

**Types** information stored to**Memory - AWM)**: Stores reusable task efficiency by reusingtypes requires robust

support agent workflows derived from past experiences orsuccessful routines.storage systems. functionality. queries to guide future actions. - External knowledge- Privacy concerns with

                - Knowledge: Includes external facts (e.g., enhances response storing user information.

articles, company rules) for accuracy. - Risk of outdated or

Building Autonomous LLM Agents 29 context-specific responses. - User information irrelevant knowledge

                - User Information: Stores personal user supports affecting performance.

details (e.g., preferences, past activities)personalized via systems like Memory Bank for interactions. personalized responses.

**Memory** Addresses challenges - Memory Duplication: Consolidates - Reduces - Duplication consolidation

**Management** in storing and similar records (e.g., combining successfulredundancy and may lose nuanced details.

**Issues** retrieving informationaction sequences into a unified plan orstorage inefficiency.- FIFO overwriting risks

efficiently. aggregating counts). losing valuable older data.


---

DOM structures [46].

UI automation libraries that can identify elements through accessibility trees or

process screenshots and generate coordinate-based actions, or integration with

technical implementation typically involves vision-language models that can

browsers to desktop applications, even when no programmatic API exists. The

bility allows agents to automate tasks in any software application, from web

mouse clicks, keyboard inputs, and drag-and-drop operations [41]. This capa-

### Visual Interface Automation: LLM agents can control graphical user inter-

interfaces [8, 70]. Here's a deeper exploration:

agent capabilities, enabling them to interact with environments beyond pure text

Multimodal action spaces represent one of the most significant advances in LLM

### 6.2 Multimodal Action Spaces

systems. [61].

emails, generating files, performing computations, or getting data from other

to provide. With this method, agents can carry out specific tasks like sending

outputs (typically JSON) that specify which tool to use and what parameters

correspond to particular actions they can perform. The agent generates structured

like file operations, database queries, web requests, or system commands, that

tool calling or function calling capabilities. Agents are given predefined functions,

The most fundamental way LLM agents execute actions is through structured

### 6.1 Tool and API Integration

include:

language understanding and real-world task automation [21]. These mechanisms

and execute actions through several key mechanisms that bridge the gap between

processing of action outcomes [61]. LLM agents interact with their environment

the mechanisms for tool orchestration, action invocation, and the immediate

This system enables the agent to interact with its environment. It encompasses

## 6 Execution System

environment.

derstanding and knowledge into concrete interactions and actions within its

system. This critical component is responsible for translating that internal un-

With its robust memory system supporting processed observations and for-


---

or inaccurate understanding of the environment.

is not yet as robust as required, with many mistakes stemming from an incomplete

remains limited. Thirdly, despite advancements, visual perception in these agents

precise actions in the real world or within graphical user interfaces (GUIs)

LLMs excel at generating and understanding text, their ability to generate

necessary data for targeted training is also time-consuming. Secondly, while

source, making it difficult to fine-tune this models. Moreover, acquiring the

This challenge is compounded by the fact that many advanced models are closed-

lack of sufficient experience interacting in specific environments. Teaching these

fail at certain operations that humans can easily perform, largely due to a

agents, several limitations warrant consideration. Firstly, these agents currently

While our review sheds light on the foundational elements of intelligent LLM

### 7.1 Limitations

## 7 Discussion

modalities [27].

agement to ensure the agent's understanding remains consistent across different

(perception, planning, execution). State synchronization requires careful man-

cessing and physical actions often require different timing considerations. Error

coordination issues arise when combining different modalities, as visual pro-

Multimodal execution presents several technical challenges [21]. Latency and

### 6.3 Integration Challenges and Solutions

real-time feedback from the physical world.

control commands, coordinate multiple actuators and subsystems, and adapt to

sensors) to understand the physical environment, generate motion plans and

tegrations [61]. They process sensor data (cameras, force sensors, temperature

agents can control physical systems through appropriate APIs and sensor in-

**Robotic and Physical System Control:** In robotics applications, LLM

tions [10, 42].

system administration, or produce HTML/CSS/Java Script for web-based solu-

integration between different systems. Agents can write Python scripts for data

valuable for data manipulation tasks, complex calculations, file processing, and

programming languages to solve specific problems. This approach is especially

bility is dynamic code generation where agents write executable code in various

### Code Generation and Execution: A particularly powerful multimodal capa-

Building Autonomous LLM Agents 31


---

architectural designs and advanced techniques contribute to building more capable

These findings directly address our initial ob jectives by illustrating how specific

and the necessity of action systems for translating decisions into tangible outcomes.

perception system in enabling agents to interpret diverse environmental inputs,

Furthermore, our analysis highlighted the critical role of a well-implemented

learning, and long-term coherence and adaptability.

is that robust memory systems are crucial for personalized responses, continuous

part of the reasoning improves performance. Another conclusion from the review

Moreover, the review showed that using different experts to focus on each

that significantly enhance an agent's problem-solving abilities.

we reviewed reasoning techniques, such as Chain-of-Thought and Tree-of-Thought,

upon specialized components that mimic human cognitive processes. Specifically,

that LLM agents are not merely large language models, but complex systems built

perception, memory, reasoning, planning, and execution. Our exploration revealed

for creating intelligent LLM agents, focusing on their core capabilities across

This paper set out to explore the intricate design and implementation strategies

## 8 Conclusion

as assistants. This would improve productivity by 10x.

An even more ambitious extension could be developing agents where humans act

significantly reduce the cost and effort of training LLM agents in new domains.

quently performing it autonomously. This "learn-from-one-shot" paradigm could

accomplish a task after just a single demonstration with human help, subse-

experiences and rectify errors without extensive human intervention. However,

self-correction in LLM agents, enabling them to continuously learn from new

area is to explore more advanced mechanisms for knowledge acquisition and

Future research can extend this work in several promising directions. One critical

### 7.3 Possible Extensions

aware.

assistants that are not only more helpful but also more reliable and context-

and adaptable AI systems that can learn and evolve. Furthermore, the memory

specialized components suggest a promising path towards building more robust

education, and advanced robotics. The modular design and the integration of

understanding and decision-making, such as scientific discovery, personalized

open doors for their application in highly complex domains requiring nuanced

simple language generation to exhibit capabilities akin to human cognition, we

of artificial intelligence. By demonstrating that LLM agents can move beyond

The review presented in this paper has significant implications for the future

### 7.2 Implications


---

truly autonomous and intelligent entities.

and generalized LLM agents, moving beyond simple workflow automation towards

Building Autonomous LLM Agents 33


---

## References

1. Alazraki, L., Mozes, M., Campos, J.A., Yi-Chern, T., Rei, M., Bartolo, M.: No
2. Anthony Brohan, e.a.: Rt-2: Vision-language-action models transfer web knowledge
3. Anthropic: Building effective agents. https://www.anthropic.com/engineering/
4. Besta, M., Blach, N., Kubicek, A., Gerstenberger, R., Podstawski, M., Gianinazzi,
5. Cai, W., Jiang, J., Wang, F., Tang, J., Kim, S., Huang, J.: A survey on mixture
6. Chen, X., Lin, M., Schärli, N., Zhou, D.: Teaching large language models to self-
7. Chen, Y., Cui, W., Chen, Y., Tan, M., Zhang, X., Zhao, D., Wang, H.: Robogpt:
8. Deng, X., Gu, Y., Zheng, B., Chen, S., Stevens, S., Wang, B., Sun, H., Su, Y.:
9. Florian Bordes, e.a.: An introduction to vision-language models. arXiv preprint
10. Gao, L., Madaan, A., Zhou, S., Alon, U., Liu, P., Yang, Y., Callan, J., Neubig, G.:
11. Gao, Y., Xiong, Y., Gao, X., Jia, K., Pan, J., Bi, Y., Dai, Y., Sun, J., Wang, M.,
12. Gidey, H.K., Collins, A., Marmsoler, D.: Modeling and verifying dynamic architec-
13. Gidey, H.K., Hillmann, P., Karcher, A., Knoll, A.: Towards cognitive bots: Archi-
14. Gidey, H.K., Hillmann, P., Karcher, A., Knoll, A.: User-like bots for cognitive
15. Gidey, H.K., Huber, N., Lenz, A., Knoll, A.: Affordance representation and recogni-

need for explanations: Llms can implicitly learn from mistakes in-context. arXiv preprint (2025), https://arxiv.org/abs/2502.08550

to robotic control. arXiv preprint (2023), https://arxiv.org/abs/2307.15818

building- effective- agents (2024), accessed: June 5 2025

L., Ga jda, J., Lehmann, T., Niewiadomski, H., Nyczyk, P., Hoefler, T.: Graph of Thoughts: Solving Elaborate Problems with Large Language Models. arXiv preprint (2023), https://arxiv.org/abs/2308.09687

of experts in large language models. arXiv preprint (2025), https://arxiv.org/pdf/ 2407.06204.pdf

debug. arXiv preprint (2023), https://arxiv.org/abs/2304.05128

an intelligent agent of making embodied long-term decisions for daily instruction tasks. arXiv preprint (2024), https://arxiv.org/abs/2311.15649

Mind2web: Towards a generalist agent for the web. arXiv preprint (2023), https: //arxiv.org/abs/2306.06070

arXiv:2405.17247 (2024), https://arxiv.org/pdf/2405.17247.pdf

Pal: Program-aided language models. arXiv preprint (2023), https://arxiv.org/abs/

2211.10435Pal: Program-aided language models. arXiv preprint (2023), https://arxiv.org/abs/

Wang, H.: Retrieval-augmented generation for large language models: A survey. arXiv preprint (2024), https://arxiv.org/abs/2312.10997

tures with factum studio. In: Formal Aspects of Component Software,FACS 2019,. Springer (2019). https://doi.org/10.1007/978- 3- 030- 40914- 2_13, https://doi.org/ 10.1007/978- 3- 030- 40914- 2_13

tectural research challenges. In: Artificial General Intelligence, AGI 2023,. Springer (2023). https://doi.org/10.1007/978- 3- 031- 33469- 6_11, https://doi.org/10.1007/ 978- 3- 031- 33469- 6_11

automation: A survey. In: Machine Learning, Optimization, and Data Science, LOD 2023,. Springer (2023). https://doi.org/10.1007/978- 3- 031- 53966- 4_29, https: //doi.org/10.1007/978- 3- 031- 53966- 4_29

tion for Autonomous Agents. In: Proceedings of the Second International Workshop on Hypermedia Multi-Agent Systems (Hyper Agents 2025), in conjunction with the 28th European Conference on Artificial Intelligence (ECAI 2025), Bologna, Italy, October 26, 2025. Bologna, Italy (Oct 2025)


---

16. Gidey, H.K., Hueber, N., Lenz, A., Knoll, A.: Visual perception patterns for software

agents (2025), preprint

17. Gidey, H.K., Kesseler, M., Stangl, P., Hillmann, P., Karcher, A.: Document-based

knowledge discovery with microservices architecture. In: Bennour, A., Ensari, T., Kessentini, Y., Eom, S. (eds.) Intelligent Systems and Pattern Recognition: ISPR

2022. Communications in Computer and Information Science, vol. 1589, pp. 146-
161. Springer, Cham (Mar 2022). https://doi.org/10.1007/978- 3- 031- 08277- 1_13,

https://doi.org/10.1007/978- 3- 031- 08277- 1_13

18. Gidey, H.K., Marmsoler, D.: FACTum Studio. https://habtom.github.io/factum/

(2018)

19. Gidey, H.K., Marmsoler, D., Ascher, D.: Modeling adaptive self-healing systems.

CoRR **abs/2304.12773** (Apr 2023). https://doi.org/10.48550/arXiv.2304.12773, https://arxiv.org/abs/2304.12773

20. Gidey, H.K., Marmsoler, D., Eckhardt, J.: Grounded architectures: Using grounded

theory for the design of software architectures. In: 2017 IEEE International Conference on Software Architecture Workshops (ICSAW). pp. 141-148. IEEE, Gothenburg, Sweden (Apr 2017). https://doi.org/10.1109/ICSAW.2017.41, https: //doi.org/10.1109/ICSAW.2017.41

21. Guo, T., Chen, X., Wang, Y., Chang, R., Pei, S., Chawla, N.V., Wiest, O., Zhang,

X.: Large language model based multi-agents: A survey of progress and challenges. arXiv preprint (2024), https://arxiv.org/abs/2402.01680

22. Hamdan, S., Yuret, D.: How much do llms learn from negative examples? arXiv

preprint (2025), https://arxiv.org/abs/2503.14391

23. Han, S., Zhang, Q., Yao, Y., Jin, W., Xu, Z.: Llm multi-agent systems: Challenges

and open problems. arXiv preprint (2025), https://arxiv.org/abs/2402.03578

24. Hao, S., Gu, Y., Ma, H., Hong, J.J., Wang, Z., Wang, D.Z., Hu, Z.: Reasoning

with language model is planning with world model. arXiv preprint (2023), https: //arxiv.org/abs/2305.14992

25. Huang, L., Yu, W., Ma, W., Zhong, W., Feng, Z., Wang, H., Chen, Q., Peng,

W., Feng, X., Qin, B., Liu, T.: A survey on hallucination in large language models: Principles, taxonomy, challenges, and open questions. ACM Transac- http://dx.doi.org/10.1145/3703155

26. Huang, X., Liu, W., Chen, X., Wang, X., Wang, H., Lian, D., Wang, Y., Tang,

R., Chen, E.: Understanding the planning of llm agents: A survey. arXiv preprint (2024), https://arxiv.org/abs/2402.02716

27. Hwang, J., Tani, J.: Seamless integration and coordination of cognitive skills

in humanoid robots: A deep learning approach. arXiv preprint (2017), https: //arxiv.org/abs/1706.02423

28. Jain, J., Yang, J., Shi, H.: Vcoder: Versatile vision encoders for multimodal large

language models. arXiv preprint arXiv:2312.14233 (2023), https://arxiv.org/pdf/ 2312.14233.pdf

29. Jin, H., Huang, L., Cai, H., Yan, J., Li, B., Chen, H.: From LLMs to LLM-based

agents for software engineering: A survey of current, challenges and future. arXiv preprint (2024), https://arxiv.org/pdf/2408.02479

30. Kil, J., Song, C.H., Zheng, B., Deng, X., Su, Y., Chao, W.L.: Dual-view visual

contextualization for web navigation. arXiv preprint (2024), https://arxiv.org/abs/

2402.04476H., Lewis, M., tau Yih, W., Rocktäschel, T., Riedel, S., Kiela, D.: Retrieval-

31. Lewis, P., Perez, E., Piktus, A., Petroni, F., Karpukhin, V., Goyal, N., Küttler,

Building Autonomous LLM Agents 35


---

32. Li, M., Zhao, Y., Yu, B., Song, F., Li, H., Yu, H., Li, Z., Huang, F., Li, Y.: Api-
33. Li, X., Wang, S., Zeng, S., Wu, Y., Yang, Y.: A survey on LLM-based multi-agent
34. Li, Y., Lai, Z., Bao, W., Tan, Z., Dao, A., Sui, K., Shen, J., Liu, D., Liu, H., Kong,
35. Liu, T., Ren, J., Zhang, C.: Planning with large language models via corrective
36. Lu, Z., Lu, W., Tao, Y., Dai, Y., Chen, Z., Zhuang, H., Chen, C., Peng, H.,
37. Macedo, J., Gidey, H.K., Rebuli, K.B., Machado, P.: Evolving user interfaces: A
38. Madaan, A., Tandon, N., Gupta, P., Hallinan, S., Gao, L., Wiegreffe, S., Alon,
39. Mi, Y., Gao, Z., Ma, X., Li, Q.: Building llm agents by incorporating insights from
40. Nakano, R., Hilton, J., Bala ji, S., Wu, J., Ouyang, L., Kim, C., Hesse, C., Jain,
41. Niu, R., Li, J., Wang, S., Fu, Y., Hu, X., Leng, X., Kong, H., Chang, Y., Wang,
42. OpenAI: Code interpreter. OpenAI Platform (2025), https://platform.openai.com/
43. OSWorld Team: Osworld: Benchmarking multimodal agents for open-ended tasks

augmented generation for knowledge-intensive nlp tasks. arXiv preprint (2021), https://arxiv.org/abs/2005.11401

bank: A comprehensive benchmark for tool-augmented llms. arXiv preprint (2023), https://arxiv.org/abs/2304.08244

systems: workflow, infrastructure, and challenges. Vicinagearth **1**, 9 (2024). https:// doi.org/10.1007/s44336- 024- 00009- 2, https://doi.org/10.1007/s44336- 024- 00009- 2

Y.: Visual large language models for generalized and specialized applications. arXiv preprint arXiv:2501.02765 (2025), https://arxiv.org/abs/2501.02765

re-prompting. arXiv preprint (2023), https://arxiv.org/pdf/2305.018323.pdf

Zeng, Z.: Decompose, plan in parallel, and merge: A novel paradigm for large language models based planning with multiple constraints. arXiv preprint (2025), https://arxiv.org/abs/2506.02683

neuroevolution approach for natural human-machine interaction. In: Johnson, C., Rebelo, S.M., Santos, I. (eds.) Artificial Intelligence in Music, Sound, Art and Design: 13th International Conference, EvoMUSART 2024, Held as Part of Evo Star

2024, Aberystwyth, UK, April 3-5, 2024, Proceedings. Lecture Notes in Computer Science, vol. 14633, pp. 246-264. Springer, Cham (Apr 2024). https://doi.org/10. 1007/978- 3- 031- 56992- 0_16, https://doi.org/10.1007/978- 3- 031- 56992- 0_16

U., Dziri, N., Prabhumoye, S., Yang, Y., Gupta, S., Ma jumder, B.P., Hermann, K., Welleck, S., Yazdanbakhsh, A., Clark, P.: Self-refine: Iterative refinement with self-feedback. arXiv preprint (2023), https://arxiv.org/abs/2303.17651

computer systems. arXiv preprint arXiv:2504.04485 (2025), https://arxiv.org/pdf/ 2504.04485v1.pdf

S., Kosara ju, V., Saunders, W., Jiang, X., Cobbe, K., Eloundou, T., Krueger, G., Button, K., Knight, M., Chess, B., Schulman, J.: Webgpt: Browser-assisted question-answering with human feedback. arXiv preprint (2022), https://arxiv.org/ abs/2112.09332

Q.: Screenagent: A vision language model-driven computer control agent. In: Pro- http://dx.doi.org/10.24963/ijcai.2024/711

docs/assistants/tools/code- interpreter, accessed: 26 July 2025

in real computer environments. https://os- world.github.io/ (2024), accessed: 26 July 2025

44. Patil, S.G., Zhang, T., Wang, X., Gonzalez, J.E.: Gorilla: Large language model

connected with massive apis. arXiv preprint (2023), https://arxiv.org/pdf/2305. 15334


---

45. Radford, A., Kim, J.W., Hallacy, C., Ramesh, A., Goh, G., Agarwal, S., Sastry, G.,

Askell, A., Mishkin, P., Clark, J., Krueger, G., Sutskever, I.: Learning transferable visual models from natural language supervision. arXiv preprint arXiv:2103.00020 (2021), https://arxiv.org/abs/2103.00020

46. Rawles, C., Li, A., Rodriguez, D., Riva, O., Lillicrap, T.: Android in the wild:

A large-scale dataset for android device control. arXiv preprint (2023), https: //arxiv.org/abs/2307.10088

47. Schick, T., Dwivedi-Yu, J., Dessì, R., Raileanu, R., Lomeli, M., Zettlemoyer, L.,

Cancedda, N., Scialom, T.: Toolformer: Language models can teach themselves to use tools. arXiv preprint (2023), https://arxiv.org/pdf/2302.04761

48. Shen, Y., Song, K., Tan, X., Li, D., Lu, W., Zhuang, Y.: Hugginggpt: Solving ai

tasks with chatgpt and its friends in hugging face. arXiv preprint (2023), https: //arxiv.org/abs/2303.17580

49. Shinn, N., Cassano, F., Berman, E., Gopinath, A., Narasimhan, K., Yao, S.: Re-

flexion: Language agents with verbal reinforcement learning. arXiv preprint (2023), https://arxiv.org/abs/2303.11366

50. Song, S., Li, X., Li, S., Zhao, S., Yu, J., Ma, J., Mao, X., Zhang, W.: How to bridge

the gap between modalities: Survey on multimodal large language model. arXiv preprint arXiv:2311.07594 (2025), https://arxiv.org/abs/2311.07594

51. Talebirad, Y., Nadiri, A.: Multi-agent collaboration: Harnessing the power of

intelligent llm agents. arXiv preprint (2023), https://arxiv.org/abs/2306.03314

52. Vaswani, A., Shazeer, N., Parmar, N., Uszkoreit, J., Jones, L., Gomez, A.N., Kaiser,

Ł., Polosukhin, I.: Attention is all you need. arXiv preprint arXiv:1706.03762 (2017), https://arxiv.org/pdf/1706.03762.pdf

53. Wang, H., Li, T., Deng, Z., Roth, D., Li, Y.: Devil's advocate: Anticipatory reflection

for llm agents. arXiv preprint (2024), https://arxiv.org/abs/2405.16334

54. Wang, L., Ma, C., Feng, X., Zhang, Z., Yang, H., Zhang, J., Chen, Z.Y., Tang, J.,

Chen, X., Lin, Y., Zhao, W.X., Wei, Z., Wen, J.R.: A survey on large language model based autonomous agents. arXiv preprint (2025), https://arxiv.org/pdf/2308. 11432.pdf

55. Wang, L., Xu, W., Lan, Y., Hu, Z., Lan, Y., Lee, R.K.W., Lim, E.P.: Plan-and-

solve prompting: Improving zero-shot chain-of-thought reasoning by large language models. arXiv preprint (2023), https://arxiv.org/abs/2305.04091

56. Wang, X., Liu, B.: Oscar: Operating system control via state-aware reasoning and re-

planning. arXiv preprint arXiv:2410.18963 (2024), https://arxiv.org/abs/2410.18963

57. Wang, X., Salmani, M., Omidi, P., Ren, X., Rezagholizadeh, M., Eshaghi, A.:

Beyond the limits: A survey of techniques to extend the context length in large language models. arXiv preprint (2024), https://arxiv.org/abs/2402.02244

58. Wang, X., Wei, J., Schuurmans, D., Le, Q., Chi, E., Narang, S., Chowdhery, A.,

Zhou, D.: Self-consistency improves chain of thought reasoning in language models. arXiv preprint (2023), https://arxiv.org/abs/2203.11171

59. Wang, Z.Z., Mao, J., Fried, D., Neubig, G.: Agent workflow memory. arXiv preprint

(2024), https://arxiv.org/abs/2409.07429

60. Wei, J., Wang, X., Schuurmans, D., Bosma, M., Ichter, B., Xia, F., Chi, E., Le, Q.,

Zhou, D.: Chain-of-thought prompting elicits reasoning in large language models. arXiv preprint (2023), https://arxiv.org/abs/2201.11903

61. Xi, Z., Chen, W., Guo, X., He, W., Ding, Y., Hong, B., Zhang, M., Wang, J., Jin,

S., Zhou, E., Zheng, R., Fan, X., Wang, X., Xiong, L., Zhou, Y., Wang, W., Jiang, C., Zou, Y., Liu, X., Yin, Z., Dou, S., Weng, R., Zhang, Q., Qin, W., Zheng, Y., Qiu, X., Huang, X., Gui, T.: The rise and potential of large language model based agents: A survey. arXiv preprint (2023), https://arxiv.org/abs/2309.07864

Building Autonomous LLM Agents 37


---

62. Xiang, J., Tao, T., Gu, Y., Shu, T., Wang, Z., Yang, Z., Hu, Z.: Language models

meet world models: Embodied experiences enhance language models. arXiv preprint (2023), https://arxiv.org/abs/2305.10626

63. Xu, B., Peng, Z., Lei, B., Mukherjee, S., Liu, Y., Xu, D.: Rewoo: Decoupling

reasoning from observations for efficient augmented language models. arXiv preprint (2023), https://arxiv.org/abs/2305.18323

64. Yang, J., Zhang, H., Li, F., Zou, X., Li, C., Gao, J.: Set-of-mark prompting unleashes

extraordinary visual grounding in gpt-4v. arXiv preprint arXiv:2310.11441 (2023), https://arxiv.org/abs/2310.11441

65. Yao, S., Yu, D., Zhao, J., Shafran, I., Griffiths, T.L., Cao, Y., Narasimhan, K.: Tree

of thoughts: Deliberate problem solving with large language models. arXiv preprint (2023), https://arxiv.org/abs/2305.10601

66. Yao, S., Zhao, J., Yu, D., Du, N., Shafran, I., Narasimhan, K., Cao, Y.: React:

Synergizing reasoning and acting in language models. arXiv preprint (2023), https: //arxiv.org/abs/2210.03629

67. Zhang, D., Yu, Y., Dong, J., Li, C., Su, D., Chu, C., Yu, D.: Mm-llms: Recent

advances in multimodal large language models. arXiv preprint arXiv:2401.13601 (2024), https://arxiv.org/abs/2401.13601

68. Zhao, Z., Lee, W.S., Hsu, D.: Large language models as commonsense knowledge

for large-scale task planning. In: Thirty-seventh Conference on Neural Information Processing Systems (2023), https://openreview.net/forum?id=Wjp1AYB8lH

69. Zhong, W., Guo, L., Gao, Q., Ye, H., Wang, Y.: Memorybank: Enhancing large

language models with long-term memory. arXiv preprint (2023), https://arxiv.org/ abs/2305.10250

70. Zhou, S., Xu, F.F., Zhu, H., Zhou, X., Lo, R., Sridhar, A., Cheng, X., Ou, T., Bisk, Y.,

Fried, D., Alon, U., Neubig, G.: Webarena: A realistic web environment for building autonomous agents. arXiv preprint (2024), https://arxiv.org/abs/2307.13854

71. Zhu, X., Chen, Y., Wang, H., et al.: OSWorld: A realistic benchmark for generalist

agents in operating systems. arXiv preprint (2024), https://arxiv.org/pdf/2404. 07972

72. Zhu, X., Li, Q., Cui, L., Liu, Y.: Large language model enhanced text-to-sql

generation: A survey. arXiv preprint (2024), https://arxiv.org/abs/2410.06011
