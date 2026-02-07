## **KVzap: Fast, Adaptive, and Faithful KV Cache** **Pruning**

**Simon Jégou** **[*]** **Maximilian Jeblick**


[NVIDIA/KVzap](https://huggingface.co/collections/nvidia/kvzap)                - [NVIDIA/kvpress](https://github.com/NVIDIA/kvpress)


**Abstract.** Growing context lengths in transformer-based language models have made the key-value
(KV) cache a critical inference bottleneck. While many KV cache pruning methods have been proposed,
they have not yet been adopted in major inference engines due to speed–accuracy trade-offs. We
introduce KVzap, a fast, input-adaptive approximation of KVzip that works in both prefilling and
decoding. On Qwen3-8B, Llama-3.1-8B-Instruct, and Qwen3-32B across long-context and reasoning
tasks, KVzap achieves 2–4 _×_ KV cache compression with negligible accuracy loss and achieves state-of-theart performance on the [KVpress Leaderboard. Code and models are available at �](https://huggingface.co/spaces/nvidia/kvpress-leaderboard) [NVIDIA/kvpress.](https://github.com/NVIDIA/kvpress)


Figure 1 _|_ [KVpress Leaderboard for Qwen3-8B (left) and Llama-3.1-8B-Instruct (right) comparing](https://huggingface.co/spaces/nvidia/kvpress-leaderboard)
different KV cache pruning methods. The plots compare the accuracy on the RULER 4k dataset (Hsieh
et al., 2024) (y-axis) against the KV cache compression ratio (x-axis). KVzap achieves state-of-the-art
performance on both models, matching KVzip (Kim et al., 2025) — which it approximates — while
outperforming 15 other methods, including Expected Attention (Devoto et al., 2025), Duo Attention
(Xiao et al., 2024), and Compactor (Chari & Durme, 2025).

### **1. Introduction**


In transformer attention (Vaswani et al., 2017), each input token produces a set of key-value (KV) vector
pairs that are stored in a cache and reused during autoregressive generation. The KV cache has shape
(2 _, 𝐿, 𝐻, 𝑇, 𝐷_ ), where _𝐿_ is the number of layers, _𝐻_ the number of heads, _𝑇_ the sequence length, and _𝐷_
the key/value dimension. For example, in bfloat16 precision, the KV cache for a vanilla transformer like
Llama1-65B (Touvron et al., 2023) ( _𝐿_ = 80, _𝐻_ = 64, _𝐷_ = 128) requires 335 GB of memory at _𝑇_ = 128k.
As sequence lengths grow to tens or hundreds of thousands of tokens, the KV cache becomes a dominant
bottleneck for efficient LLM inference (Fu, 2024), increasing GPU peak memory usage and time to first
token while reducing decoding throughput.


∗Main contributor. Contact: `sjegou@nvidia.com`


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Over the years, several architectural modifications have targeted specific axes of the KV cache to
reduce its size. Along the _𝐻_ -axis, Grouped Query Attention (GQA, (Ainslie et al., 2023)) shares keys
and values across multiple queries, yielding KV cache compression factors of 4 _×_ (Llama3, (Llama Team,
2024)), 12 _×_ (GLM 4.5, (GLM-4.5 Team, 2025)), and up to 16 _×_ (Qwen3-235B-A22B, (Qwen Team,
2025)). Along the _𝐷_ -axis, DeepSeek V2 (DeepSeek-AI, 2024) introduces Multi-head Latent Attention
(MLA) to perform a low-rank decomposition of keys and values, equivalent to a 4 _𝐻/_ 9 compression.
Along the _𝐿_ -axis, recent hybrid models interleave attention layers with sliding window attention (2 _×_
compression for GPT-OSS-120B (OpenAI, 2025), 6 _×_ for Gemma3 (Gemma Team, 2025)) or state space
models (8 _×_ compression for Jamba (Lieber et al., 2024), 4 _×_ compression for Kimi-Linear (Kimi Team,
2025), 4.8 _×_ for Nemotron3 Nano (NVIDIA, 2025)).


Notably, no widely adopted architectural change compresses the KV cache along the _𝑇_ -axis. Sparse
attention mechanisms, such as DSA in DeepSeek V3.2 (DeepSeek-AI, 2025), retrieve only the most
relevant KV pairs at each decoding step and can improve throughput, but they do not reduce the KV
cache memory size.


Most attempts at KV cache compression on the _𝑇_ -axis rely on _ad-hoc_ pruning methods. Sparse
attention is motivated by the idea that, within a head, attending to all past KV pairs is unnecessary for
the _next_ decoding step; KV cache pruning goes further and assumes that some KV pairs will never be
attended to _at all_ . Just as a reader does not pay equal attention to every word when understanding a
sentence, not all tokens are equally important, and some need not occupy _𝐻𝐿_ slots in KV cache memory.
By retaining only KV pairs that are likely to be accessed, pruning methods can substantially reduce
memory while preserving the information needed for faithful generation.


Pioneered by _𝐻_ 2 _𝑂_ [(Zhang et al., 2023), the �Awesome-KV-Cache-Compression repository now lists](https://github.com/October2001/Awesome-KV-Cache-Compression)
dozens of KV cache pruning methods, with 20+ implemented in � [NVIDIA/kvpress. While these](https://github.com/NVIDIA/kvpress)
methods validate the pruning intuition—e.g., KVzip (Kim et al., 2025) can reach 4 _×_ compression with
no accuracy loss on some tasks—none have been integrated into major inference engines such as vLLM
(Kwon et al., 2023b), SGLang (Zheng et al., 2023), or TRT-LLM (NVIDIA, 2023). In retrospect, each
solution fails to meet at least one of the following criteria:


 - **Criterion 1: Fast and lightweight.** The pruning overhead must be negligible.

 - **Criterion 2: Phase-agnostic.** The method must apply to both prefilling (long context) and
decoding (reasoning tasks).

 - **Criterion 3:** **Optimization-friendly.** The method must be compatible with kernels like
FlashAttention2 (Dao, 2023) or PagedAttention (Kwon et al., 2023a)

 - **Criterion 4: Faithful.** The method should cause minimal accuracy degradation on any task.


In this work, we introduce KVzap, a fast approximation of KVzip. Our contributions are the following:


  - We enhance the KVzip scoring with a normalization term inspired by (Devoto et al., 2025), creating
**KVzip+** .

  - We demonstrate that KVzip+ scores can be approximated by a lightweight surrogate model trained
on top of the model’s hidden states.

  - We introduce **KVzap**, a new KV cache pruning technique which applies these surrogate models to
the hidden states to prune KV pairs below a fixed threshold _𝜏_ .

### **2. Method**


**Summary:** In each transformer layer, KVzap applies a lightweight model to the input hidden states
to predict importance scores and discards KV pairs whose score falls below a threshold _𝜏_ . The KVzap
model is trained to approximate the scoring policy of an improved KVzip variant (Kim et al., 2025).


**2.1. KVzip**


KVzip (Kim et al., 2025) currently stands as the state-of-the-art KV cache pruning method on the
[KVpress Leaderboard. While it reaches up to 4](https://huggingface.co/spaces/nvidia/kvpress-leaderboard) _×_ compression with minimal accuracy loss, it has major


2


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


limitations that hinder adoption. First, it requires prefilling on an extended prompt twice as long as
the input, making it prohibitively slow. Second, it cannot be used during decoding, which makes it
unsuitable for reasoning tasks that generate thousands of tokens.


KVzip relies on a copy-and-paste pretext task to score the most important KV pairs. Given an input
context `user: <prompt>` whose KV cache is to be compressed, it starts by building an extended prompt:

```
   user: <prompt>
   Repeat the previous context exactly.
   assistant: <prompt>

```

Then, for each head, the KV pair at position _𝑖_ in the original `<prompt>` is scored as the maximum
attention weight over the repeated `<prompt>` (and over heads in a group when GQA is used):


_𝑠𝑖_ = max (1)
_𝑗∈_ `<prompt>` _[𝑎][𝑗𝑖]_


Finally, the lowest-scoring KV pairs across heads and layers are removed. The intuition is that if,
in a given head, the model pays little attention to position _𝑖_ in the original `<prompt>` when repeating
`<prompt>`, then the KV pair at _𝑖_ carries little information and can be discarded.


**2.2. KVzip+**


We enhance KVzip scoring by incorporating the analysis from (Devoto et al., 2025). For a given
transformer head at decoding step _𝑗_, the hidden-state update is:


∑︁
**h** _𝑗_ _[𝑜𝑢𝑡]_ = **h** _𝑗_ + _𝑎𝑗𝑖𝑊𝑂_ **v** _𝑖_ (2)

_𝑖≤𝑗_


where **h** _𝑗_ is the input hidden state, **h** _𝑗_ _[𝑜𝑢𝑡]_ the output hidden state, _𝑊𝑂_ the output projection matrix,
and **v** _𝑖_ the value vector. The term _𝑎𝑗𝑖𝑊𝑂_ **v** _𝑖_ represents token _𝑖_ ’s contribution to the residual stream **h** _𝑗_ .
We incorporate this normalization into Eq. (1) to define the KVzip+ score:


_‖𝑊𝑂_ **v** _𝑖‖_
_𝑠_ [+] _𝑖_ [=] max (3)
_𝑗∈_ `<prompt>` _[𝑎][𝑗𝑖]_ _‖_ **h** _𝑗‖_


**2.3. KVzap**


To address KVzip’s limitations, we train a per-layer surrogate—either a linear layer or a two-layer
MLP—to predict _𝐻_ scores log( _𝑠_ [+] ) directly from the input hidden states **h** (we use log-space to match
the exponential nature of softmax attention). The model acts independently at each sequence position _𝑡_ :
it maps **h** _𝑡_ _∈_ R _[𝐷][ℎ]_ to scores in R _[𝐻]_, where _𝐷ℎ_ is the hidden dimension and _𝐻_ is the number of KV heads.
Because it uses only one or two matrix multiplications and depends only on hidden states, KVzap is
computationally efficient (see Appendix B) and can be applied during decoding.


For training, we curate 1.2M pairs ( **h** _,_ log( _𝑠_ [+] )) per KV head, sampled from [Nemotron-Pretraining-](https://huggingface.co/datasets/nvidia/Nemotron-Pretraining-Dataset-sample)
[Dataset-sample. The dataset is diverse, covering English, multilingual, code, and mathematical text.](https://huggingface.co/datasets/nvidia/Nemotron-Pretraining-Dataset-sample)


Another key difference lies in the eviction policy. Whereas KVzip enforces a fixed budget (e.g.,
keeping exactly 50% of KV pairs), KVzap uses thresholding, discarding KV pairs whose predicted score
falls below a fixed threshold _𝜏_ . Higher thresholds yield higher compression ratios. This makes KVzap
input-adaptive: it dynamically adapts the compression rate based on the prompt information density,
retaining more tokens for complex inputs and fewer for redundant ones.


Finally, to preserve local context, we keep a sliding window of the most recent _𝑤_ = 128 tokens,
following StreamingLLM (Xiao et al., 2023). The full procedure is detailed in Algorithm 1.


3


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning

```
def compress(hidden_states, keys, values, kvzap_model, threshold, window=128):
  scores = kvzap_model(hidden_states)
  scores[..., -window:] = float("inf")
  indices = torch.where(scores >= threshold)
  return keys[indices], values[indices]

```

Algorithm 1 _|_ PyTorch pseudocode for KV cache pruning using KVzap during prefilling. Decoding is
similar but needs a score buffer to enforce the sliding window

### **3. Experiments**


All experiments were run on Qwen3-8B, Llama-3.1-8B-Instruct, and Qwen3-32B and are fully reproducible
via � [NVIDIA/kvpress. Trained models are available in the](https://github.com/NVIDIA/kvpress) [NVIDIA/KVzap collection, and full](https://huggingface.co/collections/nvidia/kvzap)
evaluation logs are provided in [KVzap predictions.](https://drive.google.com/drive/folders/1iDfEV4RygpG4rCrb_TZ_FLu6Ur4qOdHH?usp=sharing)


**3.1. KVzap training**


To generate training pairs ( **h** _,_ log( _𝑠_ [+] )), we leveraged [Nemotron-Pretraining-Dataset-sample. The](https://huggingface.co/datasets/nvidia/Nemotron-Pretraining-Dataset-sample)
dataset contains 27k prompts split into 9 subsets (common crawl, multilingual, math, code, etc.). We
filtered prompts to a length of 750–1,250 tokens to minimize the impact of sequence lengths on attention
weights and then selected up to 500 prompts per subset for training and 5 for validation, resulting in
roughly 2.4k prompts. We then randomly sampled 500 tokens per prompt to obtain 1.2M training pairs
(per head), with 23k held out for validation.


For each KV head, we trained two types of surrogate models to predict log( _𝑠_ [+] ) from the hidden
state **h** : a linear model ( **KVzap-Linear** ) and a two-layer MLP ( **KVzap-MLP** ). The input dimension
matches the model hidden size ( _𝐷ℎ_ = 4096 or 5120), and the output dimension is the number of KV
heads ( _𝐻_ = 8). For MLPs, we used one hidden layer with width _𝐷ℎ/_ 8 (512 or 640), followed by a GELU
activation. In practice, KVzap-Linear and KVzap-MLP consist of a list of _𝐿_ PyTorch modules (Paszke
et al., 2019), with input size ( _𝑇, 𝐷ℎ_ ) and output size ( _𝑇, 𝐻_ ).



We report the average Squared Pearson correlation
( _𝑅_ [2] ) over the _𝐻𝐿_ KV heads on the validation set in
Table 1. Both surrogates reach _𝑅_ [2] in the 0 _._ 60–0 _._ 80
range, showing that the expensive KVzip+ score can be
approximated from hidden states. Across all models,
KVzap-MLP consistently outperforms KVzap-Linear.
A more detailed analysis is provided in Appendix A.


**3.2. Compute and memory overhead**



Table 1 _|_ Average _𝑅_ [2] between KVzip+ scores
and KVzap predictions on the validation set.


**Model** **Linear** **MLP**
Qwen3-8B 0.671 **0.711**
Llama-3.1-8B-Instruct 0.743 **0.772**
Qwen3-32B 0.629 **0.668**



KVzap adds negligible overhead: across all models, its relative compute cost is bounded by 1 _._ 1% for
KVzap-MLP and 0 _._ 02% for KVzap-Linear when considering linear projections _only_ (Table 3 in Appendix
B). The relative memory overhead matches these bounds, and in long-context regimes the quadratic
attention cost dominates, making KVzap’s overhead negligible. Finally, during decoding—which is
strictly memory-bandwidth bound—KVzap’s additional FLOPs effectively utilize idle GPU cycles that
would otherwise be stalled by KV cache retrieval (Recasens et al., 2025).


**3.3. Prefilling and decoding tasks**


KV cache pruning is most impactful for tasks involving thousands of tokens, during prefilling (long
inputs) or decoding (long outputs). To assess KVzap across these regimes, we evaluate KVzap-Linear and
KVzap-MLP on two long-context benchmarks—RULER (Hsieh et al., 2024) ( _𝑛_ = 6500) and LongBench
(Bai et al., 2024) ( _𝑛_ = 4750)—and one reasoning benchmark, AIME25 (Zhang & Math-AI, 2025) ( _𝑛_ = 30).


4


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 2 _|_ **RULER 4k results** for Qwen3-8B (left), Llama-3.1-8B-Instruct (middle), and Qwen3-32B
(right). Zoomed-in view (y-axis range [90, 100]) of the high-performance region from Figure 1. KVzap
surrogates perform comparably to—and sometimes exceed—the KVzip+ oracle they approximate.


**Experimental Setup** We evaluate KVzap using thresholds _𝜏_ _∈{−_ 6 _, −_ 5 _, −_ 4 _, −_ 3 _}_ for Qwen3-8B and
Qwen3-32B, and _𝜏_ _∈{−_ 9 _, −_ 8 _, −_ 7 _, −_ 6 _}_ for Llama-3.1-8B-Instruct. For RULER and LongBench, we used
greedy decoding and disabled reasoning; for AIME25, we evaluated Qwen3-8B and Qwen3-32B models
with reasoning and sampling parameters recommended in the Qwen3 model card (temperature = 0 _._ 6,
top- _𝑝_ = 0 _._ 95, top- _𝑘_ = 20). In all experiments, KV cache compression was applied after the attention
operation.


**3.4. RULER**


RULER (Hsieh et al., 2024) evaluates long-context capabilities across four task categories—retrieval,
multi-hop tracing, aggregation, and question answering—over 13 subsets with sequence lengths ranging
from 4k to 128k.


On RULER 4k, KVzap achieves state-of-the-art results for both Qwen3-8B and Llama-3.1-8B-Instruct
(Figure 1), significantly outperforming 15 concurrent KV cache pruning methods.


We provide a magnified view in Figure 2, comparing KVzap variants against KVzip, KVzip+, and
Expected Attention (Devoto et al., 2025). A few trends emerge: (1) KVzip+ consistently matches
or exceeds KVzip, validating our normalization; (2) KVzap maintains perfect accuracy up to 3–4 _×_
compression; (3) For Qwen models, KVzap-MLP outperforms KVzap-Linear, which degrades sharply at
high compression; (4) surprisingly, KVzap-Linear excels on Llama-3.1-8B-Instruct despite lower _𝑅_ [2] than
KVzap-MLP and even outperforms the KVzip+ oracle it approximates.


**3.5. LongBench**


LongBench (Bai et al., 2024) evaluates long-context capabilities across six task categories—singledocument QA, multi-document QA, summarization, few-shot learning, synthetic tasks, and code
completion—spanning 21 subsets in English and Chinese. We report the average performance across
subsets in Figure 3.


Mirroring the main RULER results, (1) KVzip+ consistently matches or outperforms KVzip, and (2)
KVzap models maintain near-perfect accuracy up to 2–3 _×_ compression. Notably, the same thresholds _𝜏_
yield lower compression ratios, likely due to data characteristics: RULER samples are synthetic and
repetitive, whereas LongBench consists mostly of real-world data with higher information density.


At first glance, Expected Attention (Devoto et al., 2025) appears to surpass the full KV cache baseline
for Qwen3-8B and Llama-3.1-8B-Instruct at lower compression ratios. A closer look reveals this is largely
driven by outlier accuracies on the TREC subset (see Figure 12, 13 and 14), and that Expected Attention
degrades on several subsets where KVzap stays close to the full KV cache baseline. Higher TREC
accuracy at high compression may be explained by the over-prompting phenomenon (Tang et al., 2025):
in a few-shot learning task like TREC, adding more examples can counter-intuitively reduce accuracy.


5


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 3 _|_ **LongBench results** for Qwen3-8B (left), Llama-3.1-8B-Instruct (middle), and Qwen3-32B
(right). KVzap models again maintain accuracy close to the full KV cache baseline. The elevated scores
for Expected Attention are primarily driven by outliers in TREC, one of the 21 subsets of LongBench;
see Figure 12 for results excluding TREC.


Figure 4 _|_ **AIME25 Reasoning Performance.** Comparison of pass@1 (solid lines) and pass@4 (dashed
lines) accuracy for Qwen3-8B (left) and Qwen3-32B (right). KVzap-MLP maintains robust performance
even when discarding over 50% of the KV cache.


The generally low accuracy on most LongBench subsets, combined with their small size (typically
_𝑛_ = 200), leads to high variance, making results harder to interpret conclusively.


**3.6. AIME25**


The AIME25 benchmark (Zhang & Math-AI, 2025) consists of 30 Olympiad-level, integer-answer problems
from the 2025 American Invitational Mathematics Examination. We evaluated KVzap with 4 rollouts
per question, a generation limit of 32k tokens, and we report average pass@1 and pass@4 in Figure 4.
KVzap-MLP preserves reasoning accuracy even at compression ratios exceeding 2 _×_ .


**3.7. Adaptive compression**


Figures 2, 3, and 4 show that the maximum compression that does not degrade accuracy is task-dependent
(e.g., higher on RULER and lower on LongBench). KVzap’s thresholding captures this automatically:
the same threshold _𝜏_ translates into different compression ratios across benchmarks.


Table 2 reports the best KVzap configuration (Linear/MLP and _𝜏_ ) per model. Overall, KVzap
achieves 2 _._ 7–3 _._ 5 _×_ average KV cache compression while maintaining accuracy across model scales and
tasks.


6


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Table 2 _|_ Performance of KVzap across models and datasets. Arrows ( _→_ ) show the change from full to
compressed KV cache. Values in parentheses indicate the KV cache compression ratio (removed fraction).
For each model, we report the best KVzap configuration (Linear/MLP and threshold _𝜏_ ).


**Qwen3-8B** **Llama-3.1-8B** **Qwen3-32B**


KVzap model MLP Linear MLP
Parameters 76M 1.1M 210M
Threshold _𝜏_ = _−_ 4 _𝜏_ = _−_ 7 _𝜏_ = _−_ 4


RULER 4k 95 _._ 32 _→_ 95 _._ 09 (0 _._ 74) 95 _._ 69 _→_ 95 _._ 55 (0 _._ 68) 95 _._ 65 _→_ 95 _._ 95 (0 _._ 68)


RULER 16k 92 _._ 99 _→_ 92 _._ 78 (0 _._ 72) 93 _._ 42 _→_ 93 _._ 29 (0 _._ 70) 95 _._ 19 _→_ 94 _._ 96 (0 _._ 65)


LongBench 46 _._ 74 _→_ 46 _._ 49 (0 _._ 66) 45 _._ 25 _→_ 44 _._ 65 (0 _._ 62) 50 _._ 56 _→_ 50 _._ 40 (0 _._ 57)


AIME25 (pass@4) 0 _._ 77 _→_ 0 _._ 77 (0 _._ 75)  - 0 _._ 83 _→_ 0 _._ 87 (0 _._ 60)


Average compression ratio 0 _._ 72 (3 _._ 5 _×_ ) 0 _._ 67 (3 _._ 0 _×_ ) 0 _._ 63 (2 _._ 7 _×_ )


Figure 5 _|_ Distribution of compression ratios for Qwen3-8B and KVzap-MLP on RULER 4k, LongBench,
and AIME25 (left), and comparison to an alternative pruning method (right).


**3.8. Ablations**


**Threshold-based pruning** KVzap uses score thresholding rather than fixed top- _𝑘_ selection, allowing
the compression rate to adapt to prompt complexity. Figure 5 (left) highlights this adaptability, showing
up to 20% variation across prompts. As shown in Figure 5 (right), thresholding outperforms fixed-ratio
top- _𝑘_ selection, whether per-head or per-layer (AdaKV, (Feng et al., 2025)).


**Sliding Window** We analyze the impact of sliding-window size _𝑤_ on LongBench-LCC with Qwen3-8B,
KVzap-MLP, and _𝜏_ = _−_ 4. Without a local window ( _𝑤_ = 0), accuracy drops to 28.37% because the input
hidden states do not explicitly encode position information. Enforcing _𝑤_ = 128 restores performance to
62.51%, while increasing to _𝑤_ = 512 yields no additional gain (62.37%).

### **4. Discussion**


Across multiple models (Qwen3-8B, Llama-3.1-8B-Instruct, Qwen3-32B) and benchmarks (RULER,
LongBench, AIME25), we show that KVzap achieves 2–4 _×_ KV cache compression with negligible accuracy
loss. Its design—a lightweight linear or MLP model applied to hidden states—is computationally efficient
and easy to integrate. Still, limitations and future directions remain.


7


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


**Scope and Generalization** First, while results on a 32B model are encouraging, further validation
is needed on larger open-source models (e.g., GLM 4.7 (GLM-4.5 Team, 2025), Qwen3-235B-A22B
(Qwen Team, 2025)) and architectures with sparse attention (e.g., DeepSeek V3.2, (DeepSeek-AI, 2025)).
Evaluation could also be extended to more reasoning benchmarks, agentic tasks, and short-context
knowledge tasks.


**Ad-hoc vs. End-to-End Training** Second, KVzap is not training-free, and like most KV cache
pruning methods, it is a post-hoc addition. In the long run, end-to-end integration often prevails in
deep learning, much as Multi-Token Prediction (Gloeckle et al., 2024) is superseding ad-hoc speculative
decoding techniques such as Medusa (Cai et al., 2024). Although still rare, end-to-end pruning objectives
like DMS (Łańcucki et al., 2025) exist and may eventually yield better performance. Nonetheless, KVzap
provides further evidence that LLMs do not fully exploit the KV cache and that unused KV pairs can be
easily identified from hidden states.


**Implementation Challenges** Third, turning compression into wall-clock speedups and GPU memory
savings requires careful engineering and was not explored here. KVzap introduces non-uniform cache
lengths across heads, requiring PagedAttention kernels (Kwon et al., 2023a) that handle variable-length
blocks. Prior work such as DMS (Łańcucki et al., 2025), Compactor (Chari & Durme, 2025), AdaKV
(Feng et al., 2025), have shown this is feasible, but kernel optimization remains non-trivial. Since KVzap
relies only on hidden states, pruning could also be applied before attention to directly accelerate prefilling.


**Conclusion** Despite these challenges, we believe KVzap’s combination of simplicity, high compression
ratios, and robust performance across tasks and models makes it a prime candidate for production
deployment, potentially bridging the gap between academic pruning research and real-world inference
engines.

### **Acknowledgments**


We thank Alessio Devoto for his careful reading of the manuscript and for providing detailed and
constructive feedback.

### **References**


Joshua Ainslie, James Lee-Thorp, Michiel de Jong, Yury Zemlyanskiy, Federico Lebrón, and Sumit
Sanghai. Gqa: Training generalized multi-query transformer models from multi-head checkpoints,
2023. URL `[https://arxiv.org/abs/2305.13245](https://arxiv.org/abs/2305.13245)` .


Yushi Bai, Xin Lv, Jiajie Zhang, Hongchang Lyu, Jiankai Tang, Zhidian Huang, Zhengxiao Du, Xiao
Liu, Aohan Zeng, Lei Hou, Yuxiao Dong, Jie Tang, and Juanzi Li. Longbench: A bilingual, multitask
benchmark for long context understanding, 2024. URL `[https://arxiv.org/abs/2308.14508](https://arxiv.org/abs/2308.14508)` .


Tianle Cai, Yuhong Li, Zhengyang Geng, Hongwu Peng, Jason D. Lee, Deming Chen, and Tri Dao.
Medusa: Simple llm inference acceleration framework with multiple decoding heads, 2024. URL
`[https://arxiv.org/abs/2401.10774](https://arxiv.org/abs/2401.10774)` .


Vivek Chari and Benjamin Van Durme. Compactor: Calibrated query-agnostic kv cache compression
with approximate leverage scores, 2025. URL `[https://arxiv.org/abs/2507.08143](https://arxiv.org/abs/2507.08143)` .


Tri Dao. Flashattention-2: Faster attention with better parallelism and work partitioning. _arXiv preprint_
_arXiv:2307.08691_, 2023. URL `[https://arxiv.org/abs/2307.08691](https://arxiv.org/abs/2307.08691)` .


DeepSeek-AI. Deepseek-v2: A strong, economical, and efficient mixture-of-experts language model. _arXiv_
_preprint arXiv:2405.04434_, 2024. URL `[https://arxiv.org/abs/2405.04434](https://arxiv.org/abs/2405.04434)` .


8


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


DeepSeek-AI. Deepseek-v3.2: Pushing the frontier of open large language models. _arXiv preprint_
_arXiv:2512.02556_, 2025. URL `[https://arxiv.org/abs/2512.02556](https://arxiv.org/abs/2512.02556)` .


Alessio Devoto, Maximilian Jeblick, and Simon Jégou. Expected attention: Kv cache compression by
estimating attention from future queries distribution. _arXiv preprint arXiv:2510.00636_, 2025. URL
`[https://arxiv.org/abs/2510.00636](https://arxiv.org/abs/2510.00636)` .


Yuan Feng, Junlin Lv, Yukun Cao, Xike Xie, and S. Kevin Zhou. Ada-kv: Optimizing kv cache eviction
by adaptive budget allocation for efficient llm inference, 2025. URL `[https://arxiv.org/abs/2407.](https://arxiv.org/abs/2407.11550)`
`[11550](https://arxiv.org/abs/2407.11550)` .


Yao Fu. Challenges in deploying long-context transformers: A theoretical peak performance analysis,
2024. URL `[https://arxiv.org/abs/2405.08944](https://arxiv.org/abs/2405.08944)` .


Gemma Team. Gemma 3 technical report, 2025. URL `[https://arxiv.org/abs/2503.19786](https://arxiv.org/abs/2503.19786)` .


GLM-4.5 Team. Glm-4.5: Agentic, reasoning, and coding (arc) foundation models. _arXiv preprint_
_arXiv:2508.06471_, 2025. URL `[https://arXiv.org/abs/2508.06471](https://arXiv.org/abs/2508.06471)` .


Fabian Gloeckle, Badr Youbi Idrissi, Baptiste Rozière, David Lopez-Paz, and Gabriel Synnaeve. Better
& faster large language models via multi-token prediction, 2024. URL `[https://arxiv.org/abs/2404.](https://arxiv.org/abs/2404.19737)`
`[19737](https://arxiv.org/abs/2404.19737)` .


Cheng-Ping Hsieh, Simeng Sun, Samuel Kriman, Shantanu Acharya, Dima Rekesh, Fei Jia, Yang Zhang,
and Boris Ginsburg. Ruler: What’s the real context size of your long-context language models?, 2024.
URL `[https://arxiv.org/abs/2404.06654](https://arxiv.org/abs/2404.06654)` .


Jang-Hyun Kim, Jinuk Kim, Sangwoo Kwon, Jae W. Lee, Sangdoo Yun, and Hyun Oh Song. Kvzip:
Query-agnostic kv cache compression with context reconstruction. _arXiv preprint arXiv:2505.23416_,
2025. URL `[https://arxiv.org/abs/2505.23416](https://arxiv.org/abs/2505.23416)` .


Kimi Team. Kimi linear: An expressive, efficient attention architecture, 2025. URL `[https://arxiv.](https://arxiv.org/abs/2510.26692)`
`[org/abs/2510.26692](https://arxiv.org/abs/2510.26692)` .


Woosuk Kwon, Zhuohan Li, Siyuan Zhuang, Ying Sheng, Lianmin Zheng, Cody Hao Yu, Joseph E.
Gonzalez, Hao Zhang, and Ion Stoica. Efficient memory management for large language model serving
with pagedattention, 2023a. URL `[https://arxiv.org/abs/2309.06180](https://arxiv.org/abs/2309.06180)` .


Woosuk Kwon, Zhuohan Yu, Guo Li, Zhiqiang Fan, Zhenhua Chen, Heming Zhang, Cheng-Yu Hsieh,
William Ellis, HyoukJoong Yang, Kurt Keutzer, Joseph E. Gonzalez, and Ion Stoica. vllm: Easy,
fast, and cheap llm serving with paged attention. _arXiv preprint arXiv:2309.06180_, 2023b. URL
`[https://arxiv.org/abs/2309.06180](https://arxiv.org/abs/2309.06180)` .


Opher Lieber, Barak Lenz, Hofit Bata, Gal Cohen, Jhonathan Osin, Itay Dalmedigos, Erez Safahi,
Shaked Meirom, Yonatan Belinkov, Shai Shalev-Shwartz, Omri Abend, Raz Alon, Tomer Asida, Amir
Bergman, Roman Glozman, Michael Gokhman, Avashalom Manevich, Nir Ratner, Noam Rozen, Erez
Shwartz, Mor Zusman, and Yoav Shoham. Jamba: A hybrid transformer-mamba language model,
2024. URL `[https://arxiv.org/abs/2403.19887](https://arxiv.org/abs/2403.19887)` .


Llama Team. The llama 3 herd of models, 2024. URL `[https://arxiv.org/abs/2407.21783](https://arxiv.org/abs/2407.21783)` .


NVIDIA. Tensorrt-llm. `[https://github.com/NVIDIA/TensorRT-LLM](https://github.com/NVIDIA/TensorRT-LLM)`, 2023. Open-source library for
optimized LLM inference.


NVIDIA. Nvidia nemotron 3: Efficient and open intelligence, 2025. URL `[https://arxiv.org/abs/](https://arxiv.org/abs/2512.20856)`
`[2512.20856](https://arxiv.org/abs/2512.20856)` .


OpenAI. Gpt-oss: Open-weight models from openai. `[https://github.com/openai/gpt-oss](https://github.com/openai/gpt-oss)`, 2025.
Accessed: 2025-12-10.


9


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Adam Paszke, Sam Gross, Francisco Massa, Adam Lerer, James Bradbury, Gregory Chanan, Trevor
Killeen, Zeming Lin, Natalia Gimelshein, Luca Antiga, Alban Desmaison, Andreas Köpf, Edward
Yang, Zach DeVito, Martin Raison, Alykhan Tejani, Sasank Chilamkurthy, Benoit Steiner, Lu Fang,
Junjie Bai, and Soumith Chintala. Pytorch: An imperative style, high-performance deep learning
library, 2019. URL `[https://arxiv.org/abs/1912.01703](https://arxiv.org/abs/1912.01703)` .


F. Pedregosa, G. Varoquaux, A. Gramfort, V. Michel, B. Thirion, O. Grisel, M. Blondel, P. Prettenhofer,
R. Weiss, V. Dubourg, J. Vanderplas, A. Passos, D. Cournapeau, M. Brucher, M. Perrot, and
E. Duchesnay. Scikit-learn: Machine learning in Python. _Journal of Machine Learning Research_, 12:
2825–2830, 2011.


Qwen Team. Qwen3 technical report, 2025. URL `[https://arxiv.org/abs/2505.09388](https://arxiv.org/abs/2505.09388)` .


Pol G. Recasens, Ferran Agullo, Yue Zhu, Chen Wang, Eun Kyung Lee, Olivier Tardieu, Jordi Torres,
and Josep Ll. Berral. Mind the memory gap: Unveiling gpu bottlenecks in large-batch llm inference,
2025. URL `[https://arxiv.org/abs/2503.08311](https://arxiv.org/abs/2503.08311)` .


Yongjian Tang, Doruk Tuncel, Christian Koerner, and Thomas Runkler. The few-shot dilemma: Overprompting large language models, 2025. URL `[https://arxiv.org/abs/2509.13196](https://arxiv.org/abs/2509.13196)` .


Marian Tietz, Thomas J. Fan, Daniel Nouri, Benjamin Bossan, and skorch Developers. _skorch: A_
_scikit-learn compatible neural network library that wraps PyTorch_, July 2017. URL `[https://skorch.](https://skorch.readthedocs.io/en/stable/)`
`[readthedocs.io/en/stable/](https://skorch.readthedocs.io/en/stable/)` .


Hugo Touvron, Thibaut Lavril, Gautier Izacard, Xavier Martinet, Marie-Anne Lachaux, Timothée
Lacroix, Baptiste Rozière, Naman Goyal, Eric Hambro, Faisal Azhar, Aurélien Rodriguez, Armand
Joulin, Edouard Grave, and Guillaume Lample. LLaMA: Open and efficient foundation language
models, 2023. URL `[https://arxiv.org/abs/2302.13971](https://arxiv.org/abs/2302.13971)` .


Ashish Vaswani, Noam Shazeer, Niki Parmar, Jakob Uszkoreit, Llion Jones, Aidan N. Gomez, Łukasz
Kaiser, and Illia Polosukhin. Attention is all you need. _Proceedings of the 31st International Conference_
_on Neural Information Processing Systems_, 2017.


Guangxuan Xiao, Jiannan Tian, Huan Bao, Peng Yan, Shangguang Di, and Tao Xie. Streamingllm:
Efficient streaming inference of large language models with attention sinks. _arXiv preprint_
_arXiv:2309.17453_, 2023. URL `[https://arxiv.org/abs/2309.17453](https://arxiv.org/abs/2309.17453)` .


Guangxuan Xiao, Jiaming Tang, Jingwei Zuo, Junxian Guo, Shang Yang, Haotian Tang, Yao Fu, and
Song Han. Duoattention: Efficient long-context llm inference with retrieval and streaming heads, 2024.
URL `[https://arxiv.org/abs/2410.10819](https://arxiv.org/abs/2410.10819)` .


Yifan Zhang and Team Math-AI. American invitational mathematics examination (aime) 2025, 2025.


Zhenyu Zhang, Ying Sheng, Tianyi Zhou, Tianlong Chen, Lianmin Zheng, Ruisi Cai, Zhao Song,
Yuandong Tian, Christopher Ré, Clark Barrett, Zhangyang Wang, and Beidi Chen. H2o: Heavy-hitter
oracle for efficient generative inference of large language models. _arXiv preprint arXiv:2306.14048_,
2023. URL `[https://arxiv.org/abs/2306.14048](https://arxiv.org/abs/2306.14048)` .


Lianmin Zheng, Liangsheng Yin, Zhiqiang Xie, Jeff Huang, Chuyue Sun, Cody Hao Yu, Shiyi Cao,
Christos Kozyrakis, Ion Stoica, Joseph E. Gonzalez, Clark Barrett, and Ying Sheng. Sglang: Efficient
execution of structured language model programs. _arXiv preprint arXiv:2312.07104_, 2023. URL
`[https://arxiv.org/abs/2312.07104](https://arxiv.org/abs/2312.07104)` .


Adrian Łańcucki, Konrad Staniszewski, Piotr Nawrot, and Edoardo M. Ponti. Inference-time hyper-scaling
with kv cache compression, 2025. URL `[https://arxiv.org/abs/2506.05345](https://arxiv.org/abs/2506.05345)` .


10


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 6 _|_ Detailed KVzap evaluation analysis for Qwen3-8B. **Left:** Distribution of KVzip+ scores
computed on the 23k validation pairs. **Middle:** Correlation between the _𝑅_ [2] performance of KVzap-MLP
(x-axis) and KVzap-Linear (y-axis) for each head. **Upper Right:** Fraction of KV pairs falling below the
median KVzip+ score. **Lower Right:** Heatmap of _𝑅_ [2] scores for KVzap-MLP across all heads.


Figure 7 _|_ Detailed KVzap evaluation analysis for Llama-3.1-8B-Instruct. **Left:** Distribution of KVzip+
scores computed on the 23k validation pairs. **Middle:** Correlation between the _𝑅_ [2] performance of
KVzap-MLP (x-axis) and KVzap-Linear (y-axis) for each head. **Upper Right:** Fraction of KV pairs
falling below the median KVzip+ score. **Lower Right:** Heatmap of _𝑅_ [2] scores for KVzap-MLP across
all heads.

### **A. KVzap model training**


We report detailed distributions of KVzip+ and _𝑅_ [2] scores in Figures 6 (Qwen3-8B), 7 (Llama-3.1-8BInstruct), and 8 (Qwen3-32B).


The Llama-3.1-8B-Instruct score distribution is significantly lower than for Qwen3-8B and Qwen3-32B,
motivating lower pruning thresholds in our experiments. Across all models, KVzap-MLP consistently
achieves higher _𝑅_ [2] than KVzap-Linear. Both surrogates perform worse in the first transformer layer,
suggesting that KVzip+ scores are harder to infer from token embeddings alone. Predicting scores
directly from keys and values ( **k** _,_ **v** ) instead of hidden states **h** resulted in strictly lower _𝑅_ [2] . We also
acknowledge a potential train-test distribution shift as KVzap is trained on prompts limited to 1,250
tokens.


We trained KVzap-Linear using scikit-learn (Pedregosa et al., 2011) and KVzap-MLP using skorch
(Tietz et al., 2017). Future work could further improve accuracy through better data selection and


11


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 8 _|_ Detailed KVzap evaluation analysis for Qwen3-32B. **Left:** Distribution of KVzip+ scores
computed on the 23k validation pairs. **Middle:** Correlation between the _𝑅_ [2] performance of KVzap-MLP
(x-axis) and KVzap-Linear (y-axis) for each head. **Upper Right:** Fraction of KV pairs falling below the
median KVzip+ score. **Lower Right:** Heatmap of _𝑅_ [2] scores for KVzap-MLP across all heads.


hyperparameter tuning.

### **B. KVzap compute and memory overhead**


We analyze the compute overhead introduced by KVzap within a single transformer decoder layer,
relative to the cost of all linear projections in the layer: the attention projection matrices ( _𝑊𝑄_, _𝑊𝐾_,
_𝑊𝑉_, _𝑊𝑂_ ) and the feed-forward network (FFN). We ignore the quadratic attention matrix multiplication
and nonlinearities, yielding a conservative upper bound on the relative compute cost.


Assuming a GQA setting with _𝐻𝑄_ query heads, _𝐻_ key-value heads, head dimension _𝐷_, hidden
dimension _𝐷ℎ_, and a SwiGLU FFN intermediate dimension _𝐷_ int, the FLOPs from linear projections are:


_𝐶_ = _𝐶_ attn + _𝐶_ ffn = 4 _𝐷ℎ_ (︀ _𝐻𝑄𝐷_ + _𝐻𝐷_ )︀ + 6 _𝐷ℎ𝐷_ int _,_ (4)


We compare against KVzap-MLP, consisting of two linear layers _𝑊_ 1 _∈_ R _[𝐷][ℎ][×][𝐷][ℎ][/]_ [8] and _𝑊_ 2 _∈_ R _[𝐷][ℎ][/]_ [8] _[×][𝐻]_ :



(5)
4 [(] _[𝐷][ℎ]_ [+] _[ 𝐻]_ [)]



(︂
_𝐶_ KVzap-MLP = 2 _𝐷ℎ_ _·_ _[𝐷][ℎ]_

8



)︂ + 2 (︂ _𝐷ℎ_



)︂

_ℎ_

8 _[·][ 𝐻]_ = _[𝐷]_ 4 _[ℎ]_



and KVzap-Linear, consisting of a single projection from _𝐷ℎ_ to _𝐻_ :


_𝐶_ KVzap-Linear = 2 _𝐷ℎ𝐻_ (6)



Model _𝐻𝑄_ _𝐻_ _𝐷_ _𝐷ℎ_ _𝐷_ int _𝐶_ KVzap-MLP



_𝐶_ KVzap-Linear
_𝐶_ _𝐶_



_𝐶_ _𝐶_


Qwen3-8B 32 8 128 4096 12288 1.09% 0.02%
Llama-3.1-8B-Instruct 32 8 128 4096 14336 0.96% 0.02%
Qwen3-32B 64 8 128 5120 25600 0.67% 0.01%



Table 3 _|_ Relative compute overhead of KVzap compared to a single transformer layer, considering only
linear projections.


Table 3 reports the resulting relative compute overhead for Qwen3-8B, Llama-3.1-8B-Instruct, and
Qwen3-32B, showing a maximum overhead of 1 _._ 1% for KVzap-MLP and 0 _._ 02% for KVzap-Linear. In
long-context regimes, the quadratic cost of attention dominates the overall complexity, making this
overhead effectively negligible.


12


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Table 4 _|_ AIME25 number of correct answers ( _𝑛_ = 30) for Qwen3-8B and Qwen3-32B across four rollouts.
KVzap-Linear with _𝜏_ = _−_ 3 achieved 96% compression for Qwen3-8B and 93% for Qwen3-32B, explaining
the zero scores.


**Method** **Threshold (** _𝜏_ **)** **Qwen3-8B** **Qwen3-32B**


**Full KV Cache**       - 18, 20, 21, 21 19, 20, 22, 22



**KVzap-Linear**


**KVzap-MLP**




_−_ 3 0, 0, 0, 0 0, 0, 0, 0

_−_ 4 10, 12, 13, 16 11, 11, 11, 13

_−_ 5 17, 21, 21, 22 17, 21, 21, 22

_−_ 6 19, 20, 21, 22 21, 22, 22, 23


_−_ 3 7, 9, 11, 11 16, 16, 17, 18

_−_ 4 16, 17, 19, 20 20, 20, 22, 22

_−_ 5 20, 20, 21, 22 20, 21, 22, 23

_−_ 6 18, 20, 20, 21 22, 22, 23, 24



The relative memory overhead (ignoring biases) matches the compute overhead, as the factor of two
introduced in FLOPs counting cancels out in the ratio.


Overall, KVzap’s additional parameters introduce no meaningful memory or compute overhead.

### **C. Detailed benchmark results**


**RULER** Figures 9–11 provide per-subset results for RULER 4k (Qwen3-8B, Llama-3.1-8B-Instruct,
Qwen3-32B).


**LongBench** Figures 13–15 provide per-subset LongBench results (Qwen3-8B, Llama-3.1-8B-Instruct,
Qwen3-32B). We also report the average score excluding TREC in Figure 12.


**AIME25** We report results for each of the four rollouts of AIME25 in Table 4.


13


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 9 _|_ RULER 4k results for Qwen3-8B on each of the 13 subsets



14


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 10 _|_ RULER 4k results for Llama-3.1-8B-Instruct on each of the 13 subsets



15


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 11 _|_ RULER 4k results for Qwen3-32B on each of the 13 subsets



16


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 12 _|_ **LongBench results** for Qwen3-8B (left), Llama-3.1-8B-Instruct (middle), and Qwen3-32B
(right). Average score across 20/21 subsets, after excluding the TREC subset.


17


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 13 _|_ LongBench results for Qwen3-8B on each of the 21 subsets



18


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 14 _|_ LongBench results for Llama-3.1-8B-Instruct on each of the 21 subsets



19


KVzap: Fast, Adaptive, and Faithful KV Cache Pruning


Figure 15 _|_ LongBench results for Qwen3-32B on each of the 21 subsets



20


