## Fundamentals of Building Autonomous LLM

## Agents *⋆*

Victor de Lamo Castrillo1, Habtom Kahsay Gidey2, Alexander Lenz2, and

Alois Knoll2

1 Universitat Politècnica de Catalunya, Barcelona, Spain

victor.de.lamo@estudiantat.upc.edu

2 Technische Universität München, München, Germany {habtom.gidey, alex.lenz, knoll}@tum.de

 the limitations of traditional LLMs in real-world tasks, the research aims to explore patterns to develop "agentic" LLMs that can automate complex 

into meaningful representations; a reasoning system that formulates plans, adapts to feedback, and evaluates actions through different techniques like Chain-of-Thought and Tree-of-Thought; a memory system that retains knowledge through both short-term and long-term mechanisms; and an execution system that translates internal decisions into concrete actions. This paper shows how integrating these systems leads to more capable and generalized software bots that mimic human cognitive processes for autonomous and intelligent behavior.

**Keywords:** Autonomous LLM Agents · Perception · Reasoning and Planning · Memory Systems · Action Systems · Multi-agent Systems

## 1 Introduction

### 1.1 Motivation

Artificial intelligence (AI) is a powerful technology that is transforming cognitive automation and fundamentally reshaping the way tasks are performed [13, 14, 37]. Today, one can develop remarkable systems without the need to write complex algorithms or master low-level code. We are closer than ever to realizing the idea that "if you can think it, you can build it." Instead of relying solely on programming skills, what increasingly matters is understanding how a human would reason through a problem, since LLM agents can learn and mimic human problem solving by externalizing intermediate reasoning and refining it through self-feedback [26, 38, 49, 58, 60, 65, 66]. 

arXiv:2510.09244v1 [cs.AI] 10 Oct 2025


---

LLM agents represent a new paradigm that breaks traditional barriers. They enable the execution of tasks that were previously costly, time-consuming, or even infeasible. More than tools, agents act as collaborators, assisting humans in dynamic environments and automating decision-making in critical systems. However, this transformation is still in its early stages. Engaging with LLM agents is comparable to engaging with a new species, one that we are only beginning to understand, train, and guide [3]. This raises a crucial question: How can we build agents who think and act intelligently? How should we structure their 'minds' so that they can interpret information, reason, plan effectively, and make decisions that we can trust? Building on this vision of LLM agents as intelligent collaborators, this review explores and defines the architectural foundations that enable their autonomous and effective performance in complex tasks [20].

### 1.2 Review Ob jective

The primary ob jective of this research is to review the design and implementation of intelligent agents powered by large language models (LLMs) to improve the execution of complex automation tasks [13, 14]. Specifically, the review focuses on the agents' perception, memory, reasoning, planning, and execution capabilities. The review aims to accomplish this by pursuing the following particular goals:

1. Explore the options for perception systems, including multimodal LLMs and

image processing tools, analyzing their contributions to interpreting visual inputs for task execution.

2. Examine reasoning architectures, such as Chain-of-Thought (CoT) and Tree-

of-Thought (ToT), and their contributions to generating structured plans for complex tasks, including how reflection enhances iterative problem solving.

3. Explore and evaluate memory-augmented architectures, such as Retrieval-

Augmented Generation (RAG) and long-term memory systems, investigating effective methods for information storage to enable practical and useful applications.

4. Examine the available execution architectures, such as tool-based frameworks,

and code generation approaches, exploring their contributions to automating tasks.

5. Finally, evaluate the complexity of implementation of each system solution

proposed. To achieve these ob jectives, some challenges need to be overcome.

### 1.3 Problem Statement

Building LLM agents to automate complex tasks can offer useful opportunities but also pose complex challenges [13, 23, 61]. Despite all the advances in LLMs, developing agents that perform well in various scenarios remains a significant


---

Building Autonomous LLM Agents 3

 contrasting various strategies. Benchmarks such as OSworld [71], alongside studies on autonomous software agents [13, 15, 16], reveal key limitations in multimodal agents, highlighting the following issues:

1. **Difficulties in GUI grounding and operational knowledge:** Agents

 interactions and application-specific features.

2. **Repetitive actions:** Agents frequently predict repetitive actions, indicating

a lack of progress or an inability to break out of loops.

3. **Inability to handle unexpected window noise:** Agents are not robust to

unexpected elements or changes in UI layout, such as unanticipated pop-up windows or dialog boxes.

4. **Limitations in exploration and adaptability:** Particularly for agents

equipped with modules like "Set-of-Mark" (SoM), it has been observed that 

5. **Significant performance gap with human capabilities:** As reported on

the OSworld website [43], humans achieve a task completion rate of more than

72.36%. In contrast, leading models reach approximately 42.9% completion (as of June 2025), indicating a substantial gap with human performance. To address these challenges and guide the investigation of agent design, this research presents a set of questions to explore the architectural components, integration strategies, and generalization capabilities of LLM-based agents.

### 1.4 Research Questions

To guide this survey, we formulate the following research questions that structure the analysis of architectural foundations, subsystem design, and evaluation of LLM based agents.

1. **RQ1, Design space,** What architectural options exist for the core subsys-

tems of LLM-based agents, perception, reasoning and planning, memory, and execution, and how can they be systematically organized for practitioner use?

2. **RQ2, Integration,** Which subsystem integration patterns enable reliable

closed-loop autonomy in realistic software environments, for example, GUI and web tasks that combine visual grounding with structured signals such as DOM or accessibility trees [30, 56]?

3. **RQ3, Reasoning efficacy,** How do reasoning strategies, for example, CoT,

ToT, Re Act, and parallel planning, such as DPPM or MCTS-based approaches, affect task success rate, efficiency, and cost?


---

4. **RQ4, Memory impact,** How do long-term and short-term memory mech-

anisms, for example, RAG and context management, influence accuracy, robustness to context length limits, and adaptation in long-horizon tasks?

5. **RQ5, Failures and mitigation,** What are the principal failure modes

 reflection, anticipatory reflection, SoM, and guardrails, are most effective?

6. **RQ6, Evaluation and generalization,** Which benchmarks and metrics are

appropriate for assessing these systems, for example, OSWorld, Web Arena, and Mind2Web [8, 70, 71], and to what extent do agents generalize across tasks, applications, and interfaces? Before delving into these research questions, let us first explore the origins of LLM-based agents.

## 2 Fundamentals

### 2.1 Background of LLMs

The introduction of machine learning methods, particularly deep learning, brought a significant shift by laying the groundwork for advanced modern AI models. Large language models (LLMs) are among the most significant developments. Their appearance represents a ma jor breakthrough in AI's ability to understand and produce complex language, influencing the state of LLM-based agents today and their future course. A key technological advance in the development of LLMs has been the transformer architecture, distinguished by its "attention mechanism" [52]. This mechanism allows LLMs to attend to different words in the input enabling them to understand long-range dependencies [52]. This architectural shift, alongside their training on vast datasets and the principles of generative AI, has enabled LLMs to perform a wide range of tasks, including natural language processing (NLP), machine translation, vision applications, and question-answering.

### 2.2 From LLMs to LLM Agents

LLMs in their standard form have significant limitations due to their chatbot nature. This restricts their effectiveness in real-world tasks. These models lack long-term memory, cannot autonomously interact with external tools, and struggle 

To overcome these constraints, LLMs are guided to follow a reasoning path and are provided with tools to interact with the environment that enables them to function as autonomous agents. They are well-suited for dynamic tasks because they exhibit good planning skills, context adaptability, and they minimize human intervention. Such agents offer a scalable and flexible solution by simulating human-like team strategies and leveraging external tools [29].


---

Building Autonomous LLM Agents 5

However, simply augmenting an LLM with modules, tools, or predefined steps does not make it an agent, in any case, that would make it a workflow.

### 2.3 Workflows vs. Agents

 Workflows are structured systems that enhance LLMs by enabling tool use, environmental interaction, or access to long-term memory. However, they are not agents. Workflows perform well in controlled and predictable environments where tasks are well defined and follow a fixed sequence of steps. In a workflow, the LLM follows a pre-established plan created by its designer, broken down into specific, sequential actions. This rigidity makes workflows highly effective for repetitive and structured tasks but limits their adaptability. If, during the workflow, the LLM faces an error, it often struggle to adjust, as they lack the ability to dynamically re-plan or adapt based on new information. In contrast, agents are far more versatile and autonomous. Agents are designed to act according to the feedback from its environment. Rather than relying on a pre-set plan, agents generate their own strategies tailored to the task and context, often using techniques like Chain-of-Thought reasoning or iterative refinement to break down complex problems. This adaptability allows agents to deal with unexpected challenges, bounce back from mistakes, and function well in unpredictable environments [3]. To understand how these agents achieve autonomy, we first explore their core components and their interconnections.

### 2.4 Constitution of an Agent Perception System An agent begins its interaction with the world through its

perception system. This component is responsible for capturing and processing data from the environment, such as images, sounds, or any other form of informa-tion. Its task is to transform this information into meaningful representations that the LLM can understand and utilize, such as identifying ob jects or recognizing patterns.

## Reasoning System The reasoning system receives the task instructions along

with the data from the perception system and formulates a plan that is broken down into distinct steps. It is also responsible for adjusting this plan based on environmental feedback and evaluating its own actions to correct errors or improve execution efficiency.

## Memory System The memory system keeps the knowledge that is not embedded

in the model's weights. This includes everything from past experiences to relevant documents and structured data stored in relational databases. The LLM uses this information to enhance the accuracy of its responses.


---

**Action System** Finally, the action system is responsible for translating abstract decisions into concrete actions that impact the environment. This module ensures that the agent's instructions are carried out in the real or simulated world, completing the interaction cycle by executing what has been decided. This can involve using a set of tools, such as calling APIs or writing code to execute mouse movements in a software environment [39].

*Fig. 1. Key Components of an Agent's LLM Architecture*

Having outlined the core components that enable an LLM agent's autonomy, we now delve into a detailed exploration of the perception system.

## 3 Perception System

The perception system of an LLM agent essentially acts as its "eyes and ears," converting environmental stimuli into a format that the LLM can understand and process. The complexity of the environment and the kinds of information required determine the architecture. This challenge can be approached in four ways: text-based, multimodal, information tree/structured data, and tool-based.

### 3.1 Text-Based Perception (Pure LLM)

The simplest form in which the environment is described is purely in text. The LLM receives and processes this text description. In this mode, the environment provides textual observations directly to the LLM's prompt. This could be a description of the current state, recent events, or results of actions taken. In this environment, the perception system does not need to intervene.


---

Building Autonomous LLM Agents 7

This approach offers low computational overhead for perception and integrates directly with the LLM's core capabilities. However, it is limited to environments that give the response to LLM interactions in text. This is practical for chats or text-driven simulations.

### 3.2 Multimodal Perception

Agents can process and integrate information from a variety of sources, mainly textual and visual (images, videos), thanks to multimodal perception. For agents functioning in real-world or graphical user interfaces (GUIs), this capability is 

Language Models (VLMs) and their more advanced successors, Multimodal Large Language Models (MM-LLMs). These models aim to bridge the gap between both modalities. Although significant progress has been made in the extension of LLMs to vision, it still has some challenges. For instance, most models still struggle with precise spatial relationships or accurate ob ject counting without external aid [9]. Regardless of the specific training paradigm, a fundamental principle is the learning of a unified embedding space for vision and language. This means that both visual and textual data are converted into numerical representations (embeddings) that can be processed and compared together by the model [34]. 

or outputs. Unlike VLMs, which primarily aim to align visual and linguistic representations, MM-LLMs leverage the inherent reasoning capabilities of a large language model as their central processing unit. This enables them not only to process and connect modalities but also to perform complex reasoning, planning, and generation across a diverse range of multimodal tasks. The general architecture of MM-LLMs typically comprises a structured pipeline with distinct components [67]:

**- Modality Encoder (ME):** This component is responsible for encoding inputs from various modalities, such as images, videos, or even audio and

3D data, to obtain corresponding features or embeddings. For visual inputs, specialized encoders like Convolutional Neural Networks (CNNs) or Vision Transformers (ViT) are used to extract rich visual representations [34, 45]. 

LLM. It acts as a bridge, transforming the visual embeddings into a format that the LLM can comprehend and integrate alongside textual inputs. This processing ensures that the visual embeddings are effectively supplied to the LLM, enabling the LLM to leverage its pre-trained linguistic knowledge for multimodal reasoning [34, 50].

**- LLM Backbone:** This is the core reasoning engine. The processed and aligned multimodal representations (visual embeddings and textual features)


---

are fed to the LLM. The LLM processes these representations, answering using the semantic understanding of the inputs. 

token representations from the LLM Backbone into features understandable by a Modality Generator.

**- Modality Generator (for multimodal generation):** This component is tasked with producing outputs in distinct modalities, such as synthesizing images using models like Latent Diffusion Models.

*Fig. 2. Architecture of Multimodal Large Language Models (MM-LLMs) for Under-*

standing and Generation [67] While the architectural components of MM-LLMs enable multimodal process-ing, their perceptual capabilities often require further enhancement to address limitations in visual understanding, as explored in the following subsection.

**Enhancing Perception in MM-LLMs** As outlined in the paper "VCoder: Versatile Vision Encoders for Multimodal Large Language Models" by Jain et al. (2023) [28], traditional MM-LLM systems often face limitations in fundamental visual perception, such as accurately identifying or counting ob jects, and a tendency to hallucinate non-existent entities. A faster and more cost-effective way to enhance perception (rather than improving each individual component of an MM-LLM) is to use visual encoders. These encoders, which can be separate models, extract relevant information from images to help the MM-LLM interpret them more effectively. While this approach doesn't match the performance gains of directly improving each component of the MM-LLM, it offers a practical trade-off by significantly improving results at a much lower computational and developmental cost. These are different ways to enhance visual perception with visual encoders:

**- Segmentation and Depth Maps:** VCoder enhances MM-LLM capabilities through a specialized adaptive architecture and the integration of additional


---

Building Autonomous LLM Agents 9

perception modalities. It functions as an adapter to a base MM-LLM, enabling the model to process "control inputs" such as segmentation maps (offering fine-grained ob ject and background information) and depth maps (providing spatial relationship details). Information from these inputs is pro jected into the LLM's embedding space via additional vision encoders [45].

*Fig. 3. Usage of segmentation and depth maps for MM-LLM perception [28]*

**- Set-of-Mark Operation:** To enhance the model's ability to handle complex visual tasks, Set-of-Mark (SoM) operation provides a structured approach to guide MM-LLMs in processing visual inputs. As seen in Fig. 4 set-of-mark process consists in annotating images with explicit markers (e.g., bounding boxes or labels) that highlight key regions or ob jects, enabling the model to focus on specific areas during reasoning. This technique improves the model's understanding of the image and task-specific performance [64]. 

LLMs adapted with VCoder and SoM significantly outperform baseline models reduced hallucination. This highlights the ongoing efforts to enhance the granular perception capabilities of LLM-based agents. While techniques like Set-of-Mark and VCoder enhance visual perception through targeted annotations and prompting, structured data approaches, such as Accessibility Tree and HTML utilization, offer alternative methods for robust environmental interpretation, as explored in the following subsection.


---

*Fig. 4. Image with Set-of-Mark [64]*

### 3.3 Information Tree/Structured Data Perception

 descriptive labels to facilitate semantic grounding.

**- HTML Utilization:** Meanwhile, DUALVCR [30] captures both the visual features of the screenshot and the descriptions of associated HTML elements to obtain a robust representation of the visual screenshot.

### 3.4 Tool-based Perception

Beyond direct multimodal inputs and structured data retrieval, LLM-based agents can significantly enhance their perception capabilities through tool augmentation. This means utilizing external tools and APIs to enable the agent to gather, process, and interpret data from a wider variety of sources, including real-world sensors and specialized databases. The mechanism of integration typically involves the LLM generating specific tool calls based on its current understanding and goals, with the results from these tools being "fed back" into the LLM [44, 47].

**Categorizing Tools for Perception** The diverse landscape of external tools 

**- Web Search and Information Retrieval APIs:** These tools allow agents to access vast amounts of up-to-date information, facts, and specific data points from the internet. By issuing queries to search engines (e.g., Google Search API) or structured knowledge bases (e.g., Wikipedia API), agents can
