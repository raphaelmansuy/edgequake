# Space Time Pilot: Generative Rendering of Dynamic Scenes Across

## Space and Time

## Zhening Huang Hyeonho Jeong

## Tuanfeng Y. Wang Joan Lasenby

### https://zheninghuang.github.io/Space-Time-Pilot/

## Xuelin Chen Yulia Gryaditskaya

## Chun-Hao Huang

## Adobe Research

1,2 2 2 2

1University of Cambridge 2

# arXiv:2512.25075v1 [cs.CV] 31 Dec 2025

*Figure 1. Space Time Pilot enables unified control over both camera and time within a single diffusion model, producing continuous and*

coherent videos along arbitrary space-time trajectories. Given a source video (odd rows), our model synthesizes new videos (even rows) with retimed motion sequences, including slow motion, reverse motion, and bullet time, while precisely controlling camera movement according to a given camera trajectory.

### Abstract

We present Space Time Pilot, a video diffusion model that

disentangles space and time for controllable generative ren- independently alter the camera viewpoint and the motion sequence within the generative process, re-rendering the

scene for continuous and arbitrary exploration across space and time. To achieve this, we introduce an effective an- sequence with respect to that of the source video. As no datasets provide paired videos of the same dynamic scene with continuous temporal variations, we propose a simple


---

1. Introduction

control, we introduce two additional components: an im- synthetic Space and Time full-coverage rendering dataset that provides fully free space-time video trajectories within a scene. Joint training on the temporal-warping scheme and the Cam×Time dataset yields more precise temporal control. We evaluate Space Time Pilot on both real-world and synthetic data, demonstrating clear space-time disen-

*Figure 2. Space-time controllability across methods. Blue cells*

denote the input video/views, while arrows and dots indicate gen- models [ 33] modify only the camera trajectory while keeping time strictly monotonic. 4D multi-view models [, 43] synthe- not generate continuous video sequences. Space Time Pilot enables free movement along both the camera and time axes with full con- reverse playback, and mixed space-time trajectories.

diffusion models can encode implicit 4D priors. Nonethe- methods still lack full 4D exploration, i.e., the ability to navigate scenes freely across both space and time.

In this work, we introduce Space Time Pilot, the first

video diffusion model that enables joint spatial and tem- "animation time" to capture the temporal status of scene dynamics in the source video. As such, it naturally disen- them as two independent signals. A high-level comparison between our approach and prior methods is illustrated in

*Fig. 2. Unlike previous methods, Space Time Pilot enables*

free navigation along both the camera and time axes. Train- feasible in a controlled studio setups. Although temporal di- e.g. [, 53], as done in [41, 43], this approach remains sub- Existing synthetic datasets [, 2] also do not exhibit such properties.

To address this limitation, we introduce a simple yet

effective temporal-warping training scheme that augments existing multi-view video datasets [, 2] to simulate di- structure. By warping input sequences in time, the model is exposed to varied temporal behaviors without requiring

additional data collection. This simple yet crucial strat-

learn temporal control and achieve robust space-time dis-

21

23

## 1To address this limitation, we introduce a simple yet

## 1To address this limitation, we introduce a simple yet

solely on camera parameters, achieving strong novel-viewegy allows the model to learn temporal control signals, en- Autoregressive models like Genie-3 [29] even enable inter- fects during generation. We further ablate various temporal-

Videos are 2D projections of an evolving 3D world, where the underlying generative factors consist of spatial variation (camera viewpoint) and temporal evolution (dynamic scene motion). Learning to understand and disentangle these fac- as scene understanding, 4D reconstruction, video editing, and generative rendering, to name a few. In this work, we approach this challenge from the perspective of gen- while remaining faithful to the underlying scene dynamic.

A common strategy is to first reconstruct dynamic 3D

content from 2D observations, i.e., perform 4D reconstruc- both spatial and temporal variations using representations such as NeRFs [, 25] or Dynamic Gaussian Splatting [15, 42], often aided by cues like geometry [, 28], op- [17, 37]. However, even full 4D reconstructions typi- work [, 43] uses multi-view video diffusion to gener- Gaussian-splatting optimization, but rendering quality re- 5, 12, 16, 30, 39, 46, 51] further enable camera re-posing with more lightweight point cloud representations, reduc- preserving identity, their reliance on per-frame depth and re- To mitigate this, newer approaches condition generation


---

                        3. Method
2. Related work

ral manipulation may inadvertently affect camera behavior. To further strengthen disentanglement, we introduce a new dataset that spans the full grid of camera-time combinations along a trajectory. Our synthetic Cam×Time dataset con- scenes and three camera paths. Each path provides full- multi-view and full-temporal coverage. This rich supervi- Experimental results show that Space Time Pilot success- from single videos, outperforming adapted state-of-the-art baselines by a significant margin. Our main contributions are summarized as follows:

- We introduce Space Time Pilot, the first video diffusion

model that disentangles spatial and temporal factors to en- well as temporal control from a single video.

- We propose the temporal-warping strategy that repur-

poses multi-view datasets to simulate diverse temporal variations. By training on these warped sequences, the model effectively learns temporal control without the need for explicitly constructed video pairs captured un-

- We propose a more precise camera-time conditioning

mechanism, illustrating how viewpoint and temporal em- to achieve fine-grained spatiotemporal control.

- We construct the Cam×Time Dataset, providing dense

spatiotemporal sampling of dynamic scenes across cam- 4D representations and supports precise camera-time control in generative rendering.

thesize new viewpoints.

For dynamic scenes, inpainting-based methods such as

Trajectory Crafter [48], Re Capture [50], and Reangle [] also adopt warp-and-inpaint pipelines, while GEN3C [31] extends this with an evolving 3D cache and EPiC [40] im- Geometry-free dynamic models [, 2, 33, 35, 36] instead learn camera-conditioned generation from multi-view or 4D datasets (e.g., Kubric-4D [7]), enabling smoother and more stable NVS with minimal 3D inductive bias. Proprietary systems like Genie 3 [] further demonstrate real-time, continuous camera control in dynamic scenes, underscor- viewpoint manipulation.

Disentangling Space and Time. Despite great progress in camera controllability (space), the methods discussed above do not address temporal control (time). Meanwhile, disen- focus in 4D scene generation, recently advanced through diffusion-based models. 4DiM [41] introduces a Masked FiLM mechanism that defaults to identity transformations when conditioning signals (e.g., camera pose or time) are absent, enabling unified representations across both static and dynamic data through multi-modal supervision. Simi- 4D dynamic reconstruction to achieve space-time disen- and controllability. In contrast, our approach builds upon text-to-video diffusion models and introduces a new tempo- achieve fully controllable 4D generative reconstruction.

temporal-control mechanism that enables finer-grained ma-under new viewpoints [13, 31, 44, 49]. Although these ap- as bullet-time at any timestep within the video. While tem-3D preprocessing. Geometry-free approaches [2, 33, 52] poral warping increases temporal diversity, it can still en-bypass explicit geometry and directly condition the diffu-

## 13For dynamic scenes, inpainting-based methods such as

1

29

porate explicit 3D geometry in the generation pipeline.video *V* trgpreserves the scene's underlying dynamics, ge- For static scenes, geometry-based methods reconstructometry, and appearance in *V* src, while adhering to the cam- models to complete or hallucinate regions that are unseen**t . A key feature of our method is the disentanglement**trg

We aim to re-render a video from new viewpoints with tem- (NVS) from monocular video inputs.

Video-based NVS. Prior video-based NVS methods can be broadly characterized along two axes: (i) whether they target static or dynamic scenes, and (ii) whether they incor-

We introduce Space Time Pilot, a method that takes a source video *V* ∈ R *F* ×*C* ×*H* ×*W* as input and synthesizes a tar-

number of color channels, and *H* and *W* are the frame height and width, respectively. Each **c** *f* ∈ R 3×4 represents the camera extrinsic parameters (rotation and translation) at frame *f* , with respect to the 1 st frame of *V*. The target


---

Our framework builds upon recent advances in large-scale text-to-video diffusion models and camera-conditioned video generation. We adopt a latent video diffusion back- [34], consisting of a 3D Variational Auto-Encoder (VAE) for latent compression and a Transformer-based denoising model (DiT) operating over multi-modal tokens.

Additionally, our design draws inspiration from

Re Cam Master [2], which introduces explicit camera conditioning for video synthesis. Given an input camera trajectory **c** ∈ R *F* ×3×4, spatial conditioning is achieved by first projecting the camera sequence to the space of video tokens and adding it to the features:

where *x* is the output of the patchifying module and *x* ′ is the input to self-attention layers. The camera encoder E cam

maps each flattened 3 × 4 camera matrix (12-dimensional) into the target feature space, while also transforming the temporal dimension from *F* to *F* ′.

### 3.2. Disentangling Space and Time

We achieve spatial and temporal disentanglement through a two-fold approach: a dedicated time representation and specialized datasets.

#### 3.2.1. Time representation

Recent video diffusion models include position embeddings for latent frame index′, such as RoPE(*f* ′). However, we found using RoPE(′) for temporal control to be ineffective, as it interferes with camera signals: RoPE(′) often con- To address space and time disentanglement, we introduce a dedicated time control parameter ∈ R *F*. By manipulating

**t , we can control the temporal progression of the synthe-**trg

sized video *V* trg. For example, setting **t** trgto a constant locks *V*trgto a specific timestamp in *V* src, while reversing the frame indices produces a playback of *V* srcin reverse.

(Top) For multi-view dynamic scene datasets, a set of

temporal warping operations, including reverse, playback, zigzag motion, slow motion, and freeze are apppplied with teh source video as standford. This gives explicit supervi- temporally varied training data.

*Figure 3. Temporal Wrapping for Spatiotemporal Disentan-*

glement. (Top) For multi-view dynamic scene datasets [2], a set of temporal warping operations (e.g. reverse playback, zigzag motion, slow motion, and freeze) are applied to the target video, with the source video kept as the standard forward reference, pro- and static-scene videos to demonstrate temporal differences, Tem- temporal variation, leading to disentanglement of space and time.

of spatial and temporal factors in the generative process, enabling effects such as bullet-time and retimed playback from novel viewpoints (see Fig. 1).

### 3.1. Preliminaries

(Bottom) Existing camera-control and joint dataset train-**t , t**src trg ∈ R. Next, we apply two 1D convolution lay- videos, making it difficult for models to understand tem-frame space, e = Conv1De 2(Conv1D 1(**e**)). Finally, we add poral variation. The introduced temporal mappings fromthese time features to the camera features and video tokens

multi-view video data, which provide diverse and clear sig-

Time Embedding. To inject temporal control into the dif- encode time similar to a frame index using RoPE embed- (visual evaluations are provided in Supp. Mat.). Instead, we adopt sinusoidal time embeddings applied at the latent frame *f* ′ level, which provide a stable and continuous rep- favorable trade-off between precision and stability. We fur- original frame indices to support finer granularity of time control. To accomplish this, we introduce a time encod- the sinusoidal time embeddings to represent the temporal sequence, **e** src = SinPE(**t** src), **e** trg = SinPE(**t** trg), where

*F*


---

dings where **t** src*,* **t**trgare directly defined in R, and em- the advantages of our proposed method.

#### 3.2.2. Datasets

To enable temporal manipulation in our approach, we re- remapping. Achieving spatial-temporal disentanglement further requires data containing examples of both camera and temporal controls. To the best of our knowledge, no publicly available datasets satisfy these requirements. Only a few prior works, such as 4DiM [41] and CAT4D [43], have attempted to address spatial-temporal disentanglement. A common strategy is to jointly train on static-scene datasets and multi-view video datasets [, 53]. The limited con- temporal evolution and spatial movement, resulting in en- temporal warping and by proposing a new synthetic dataset.

Temporal Warping Augmentation. We introduce simple augmentations that add controllable temporal variations to

|  |  |  |  |  |  |  |  |  |
|---|---|---|---|---|---|---|---|---|
|  |  |  |  |  |  |  |  |  |
| =  { I | f } trg | F } f  =1 | , | Figure 4.  Cam Time  dataset visualization . (Top) A space-time grid defined by a camera trajectory  = [ c status  t  = [ t pairs, covering the full grid for learning disentangled spatial and | Figure 4.  Cam Time  dataset visualization . (Top) A space-time grid defined by a camera trajectory  = [ c status  t  = [ t pairs, covering the full grid for learning disentangled spatial and | 1 | , ..., t] F | . Cam Time  renders images for all  ( c, t ) |
| trg | = trg | temporal control. Any two sampled sequences of  F  frames from | temporal control. Any two sampled sequences of  F  frames from |  |  |  |  |  |

multi-view video datasets. During training, given a source

to the target sequence, producing a warped video *V*

{Itrg}*f* =1. The source animation timestamps are uni- Fig. 3 top b-e): (i) reversal, (ii) acceleration, (iii) freez- which the animation repeatedly reverses direction. After these augmentations, the paired video sequences (*V* src*, V )*trg

differ in both camera trajectories and temporal dynamics, providing the model with a clear signal for learning disen-

Synthetic Cam×Time Dataset for Precise Spatiotempo- encourage strong disentanglement between spatial and tem-

DL3DV10k [] ✘ Moving Mannequin Challenge [32] ✘ Moving Kubric-4D [33] ✔ Moving Re Cam Master [] ✔ Moving Syn Cam Master [] ✔ Fixed

embeddings, updating Eq. (1) as follows: Table 1. Comparison of existing multi-view datasets for cam-

In Sec. 4.2, we compare our approach with alternative

conditioning strategies, such as using sinusoidal embed-RE10k [53] ✘ 1 1 Moving

1:60 1:60

2 1:80 1:80 1 1:80 1:80

1

1

Cam×Time, a new synthetic spatiotemporal dataset ren- nations of the controls. We designate part of Cam×Time as dered in Blender. Given a camera trajectory and an ani-a test set, aiming for it to serve as a benchmark for control- era-time grid, capturing each dynamic scene across diverseresearch on fine-grained spatiotemporal modeling.

*, ..., c]**F* and animation

the grid can form a source-target pair. (Bottom) One typical choice of source videos is taking the diagonal cells in green.

combinations of camera viewpoints and temporal states (**c***,* **t**), as illustrated in Fig. 4. The source video is obtained by sampling the diagonal frames of the dense grid (Fig. 4 (bottom)), while the target videos are obtained by more free-form sampling of continuous sequences. We com- [, 32, 53] are real videos with complex camera path anno- pairs [32] or only provide pairs of static scenes [, 53]. Synthetic multi-view video datasets [, 2, 33] provide pairs of dynamic videos but do not allow training for time control. In contrast, Cam×Time enables fine-grained manipulation of both camera motion and temporal dynamics, enabling bullet-time effects, motion stabilization, and flexible combi-


---

| Method | Dir. Speed Bullet | Avg | Dir. Speed Bullet | Avg | Dir. Speed Bullet |  |  |  |  |  |  |
|---|---|---|---|---|---|---|---|---|---|---|---|
| ReCamM+preshuffled ReCamM+jointdata SpaceTimePilot  (Ours) | 17.13 14.84 14.61 18.32 17.57 17.69 21.75 20.87 20.85 | 17.13 14.84 14.61 18.32 17.57 17.69 21.75 20.87 20.85 | 15.52 17.86 21.16 | 15.52 17.86 21.16 | 0.6623 0.6050 0.5965 0.7322 0.7220 0.7209 0.7725 0.7645 0.7653 | 0.6623 0.6050 0.5965 0.7322 0.7220 0.7209 0.7725 0.7645 0.7653 | 0.6213 0.7250 0.7674 | 0.6213 0.7250 0.7674 | 0.3930 0.4793 0.4863 0.2972 0.3158 0.3089 0.1697 0.1917 0.1677 | 0.3930 0.4793 0.4863 0.2972 0.3158 0.3089 0.1697 0.1917 0.1677 | 0.4529 0.3073 0.1764 |

                        4. Experiments

† Uses simple frame-rearrangement operators (reversal, repetition, freezing) applied prior to inference to emulate temporal manipulation.

### 3.3. Precise Camera Conditioning

We aim for full camera trajectory control in the target video. In contrast, the previous novel-view synthesis approach [ assumes that the first frame is identical in source and target videos and that the target camera trajectory is defined rel- existing approach ignores the source video trajectory, yield- trajectory for consistency:

src src cam trg trg trg cam

Second, it is trained on datasets where the first frame is al- latter limitation is addressed in our training datasets design. To overcome the former, we devise a source-aware cam- source and target videos using a pretrained pose estimator, and inject them jointly into the diffusion model to provide explicit geometric context. Eq. 2 is therefore extended into:

src src cam src ani src

trg trg cam trg ani trg

trg srcframe-dim

where *x* ′ denotes the input of the DiT model, which is the concatenation of target and source tokens along the frame dimension. This formulation provides the model with both source and target camera context, enabling spatially consis-

### 3.4. Support for Longer Video Segments

Finally, to showcase the full potential of our camera and temporal control, we adopt a simple autoregressive video generation strategy, generating each new segment *V* ditioned on the previously generated segment *V* source video *V* srcto produce longer videos.

To enable this capability during inference, we need to

extend our training scenario to support conditioning on two

using temporal warping augmentations or by sampling from the dense space-time grid of our synthetic dataset. When temporal warping is applied, *V* prv and *V* trg may originate from the same or different multi-view sequences represent- control, we do not enforce any other explicit correlations between *V* prv and *V* trg, apart from specifying camera param-

Note that not constraining the source and target videos

to share the same first frame (as discussed in Sec. 3.3) is

+ E (**c**)  crucial for achieving flexible camera control in longer se-

trg*,*

to 90 ◦ (*V*). Conditioning on two consecutive source seg- ments allows the model to leverage information from newly generated viewpoints. In the bullet-time example, condi- to incorporate information from all newly synthesized view- corresponding moment in the source video.

() *,* (3)

Implementation details. We adopt the Wan-2.1 T2V- 1.3B model [34], which produces *F* ′=21 latent frames and decodes them into *F* =81 RGB frames using a 3D-

## VAE. The network is conditioned on camera and animation-

time controls as defined in Eq. 3. Unless otherwise spec- Syn Cam Master datasets with the temporal warping aug- Please refer to Supp. Mat. for complete network architec-

trgcon- 4.1. Comparison with State-of-the-Art Baselines

prv and a

4.1.1. Time-Control Evaluation.

We first evaluate the retiming capability of our model. To factor out the error induced by camera control, we condition

*Table 2. Quantitative comparison across temporal controls (Direction (forward, backward motion) Speed (slow modes), Bullet Time). We*

| report | PSNR↑, | SSIM↑, | and | LPIPS↓. | Best | results | are | in | bold. | Space Time Method | showcase | best | performance | for | temporal | control | overall. |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| datasets or from our synthetic dataset, as was described pre-on the withheld Cam×Time test split, which contains | 50 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |


---

and complex camera paths (pan, tilt, zoom, and vertical motion).

*Table 3. VBench visual-quality evaluation across six dimensions.*

| Higher | is | better | for | all | metrics. |
| --- | --- | --- | --- | --- | --- |
| Traj-Crafter [48] 0.6389 0.9376 0.9888 0.9463 0.9816 | 0.5172 |  |  |  |  |
| Re CamM [] 0.6302 0.9114 0.9945 0.9181 0.9825 | 0.5332 |  |  |  |  |
| Re CamM+Aug 0.6315 0.9165 0.9946 0.9313 0.9788 | 0.5385 |  |  |  |  |
| STPilot (Ours) 0.6486 0.9199 0.9947 0.9325 0.9781 | 0.5315 |  |  |  |  |

erated videos using VBench []. We report all standard visual quality metrics to provide a comprehensive assess- achieves visual quality comparable to the baselines.

4.1.3. Camera-Control Evaluation.

Finlay, we evaluate the effectiveness of our camera control mechanism detailed in Sec. 3.3. Unlike the retiming evalu- here we construct a real-world 90-video evaluation set from Open VideoHD [], encompassing diverse dynamic human and object motions. Each method is evaluated across 20

*Figure 5. Qualitative results of Space Time Pilot. Our model enables fully disentangled control over camera motion and temporal dy-*

namics. Each row shows a different combination of camera trajectory (left icons) and temporal warping (right icons). Space Time Pilot produces coherent videos under diverse controls, including normal playback, reverse playback, bullet-time, slow-motion, replay motion,

10

26

tency. In contrast, Space Time Pilot consistently outperforms camera trajectories: 10 starting from the same initial pose all baselines across all temporal configurations. as the source video and 10 from different initial poses,

be retimed into arbitrary temporal sequences. For each test case, we take a moving-camera source video but set the tar- bullet-time, zigzag, slow motion, and normal playback, to synthesize the corresponding retimed outputs. Since we have ground-truth frames for all temporal configurations, we report perceptual losses: PSNR, SSIM, and LPIPS.

We consider two baselines: (1) Re CamM+preshuffled:

original Re Cam Master combined with input re-shuffling; and (2) Re CamM+jointdata: following [41, 43], we train Re Cam Master with additional static-scene datasets [, 53] which provide only one single temporal pattern.

While frame shuffling may succeed in simple scenar- shown in Table 2, this approach exhibits the weakest tem- datasets improves performance, particularly in the bullet- remains insufficient for achieving robust temporal consis-


---

Traj-Crafter [48] 5.94 0.50 6.93 0.52 Re CamM [] 4.26 0.32 10.08 0.34 Re CamM+Aug 3.66 0.43 11.74 0.46 Space Time Pilot (ours) 0.33 5.63 0.34

† Evaluation based on first-frame camera accuracy.

9.76 22.96% 25.93% Uniform Sampling 14.10 0.5981 0.5039 7.49 7.61% 10.20% 1D-Conv 14.75 0.6134 0.4878 13.88 3.89% 5.93% 1D-Conv + Joint Data 15.41 0.6252 0.4830 4.09 35.19% 54.44% 1D-Conv +Cam×Time 21.16 0.7674 0.1764

*Table 4. Camera accuracy and first-frame estimation. For camera control,Table 5. Time-embedding compressor ablation. The pro-*

the enhanced camera control mechanism enables the generated video to startposed time-embedding method, trained with temporal warp-

Method Rel Rot↓ Rel Trans↓ Abs Rot↓ Abs Trans↓ Rot† ↓ RTA15 † ↑ RTA30 † ↑ Time Embedding PSNR↑ SSIM↑ LPIPS↓

2

2.71

initial pose, and report Rot Err, RTA@15 and RTA@30, as translation magnitude is scale-ambiguous. we show that only our method correctly synthesizes both

To measure only the impact of source camera condition-the camera motion (red boxes) and the animation-time state

ing, we consider the original Re Cam Master [2] (Re CamM) (green boxes). While Re Cam Master handles camera control and two variants. Since Re Cam Master is originally trained well, it cannot modify the temporal state, such as enabling

*Figure 6. Qualitative comparison of disentangled camera-time*

control. In this example, we apply reverse playback (time) and a pan-right camera motion starting from the first-frame pose to a source video (top), whose original camera motion is dolly-in (red to blue). Space Time Pilot, by explicitly disentangling space and time, achieves correct camera control (red boxes) together with accurate temporal control (green boxes). For Trajectory Crafter, it first reverses the frames and then apply their method for viewpoint control, resulting in incorrect camera motion. Re Cam Master (with joint-dataset training) is unable to perform temporal control, lead-

resulting in a total of 1800 generated videos. We apply Spatial Tracker-v2 [45] to recover camera poses from the generated videos and compare them with the corresponding input camera poses. To ensure consistent scale, we align the magnitude of the first two camera locations. Trajectory ac- Rel Trans) and (2) evaluating after aligning to the estimated pose of the first frame (absolute protocol, Abs Rot, Ab- also compare this DUSt3R pose with the target trajectory's

*Figure 7. Temporal compression ablation. Comparing uniform*

resampling, MLP, and 1D-Conv compressors under tilt-down and

on datasets where the first frame of the source and target videos are identical, the model always copies the first frame regardless of the input camera pose. For fairness, we re- Next, we condition the model additionally with source cam- Finally we also report the results of Trajectory Crafter [48]. In Table 4, we observe that the absolute protocol pro- only match the overall shape (relative protocol) but also align correctly in position and orientation. Interestingly, Re CamM+Aug yields higher errors than the original Re- CamM, whereas incorporating source camerassrc results in the best overall performance. This suggests that, with- videos with differing initial frames can instead confuse the model. The newly introduced conditioning signal on the

 **c** src achieves substantially better camera-control accuracy across all metrics, more reliable first-frame alignment, and more faithful adherence to the full trajectory than all baselines.

4.1.4. Qualitative results.

Besides the quantitative evaluation, we also demonstrate the strength of Space Time Pilot with visual examples. In Fig. 6,


---

                        6. Acknowledgement

## References

5. Conclusion

### 4.2. Ablation Study

To validate the effectiveness of the proposed Time embed- sampling the 81-frame embedding uniformly to a 21-frame sequence, which is equivalent to adopting sinusoidal em-

**t ∈** R, trained with Re Cam Master and Syn Cam Master datasets. (3) 1D-Conv+jointdata: row 2 but including addi- row 2 but instead including the proposed Cam×Time. We observe that applying a 1D convolution to learn a compact representation by compressing the fine-grained *F* -dim em- directly constructing sinusoidal embeddings at the coarse *f level. Incorporating static-scene datasets yields only*′

limited improvements, likely due to their restricted tem- Cam×Time consistently delivers the largest gains across all three metrics, confirming the effectiveness of our newly introduced datasets. Furthermore, as shown in Fig. 7, we present a visual comparison of bullet-time results using uni- sampling produces noticeable artifacts, and the MLP com- smooth camera movement.

We would like to extend our gratitude to Duygu Ceylan, Paul Guerrero, and Zifan Shi for insightful discussions and valuable feedback on the manuscript. We also thank Rudi Wu for helpful discussions on implementation details of CAT4D.

reverse playback. Trajectory Crafter, in contrast, is confusedeffects such as reverse playback, slow motion, and bullet- last source frame (blue boxes) to incorrectly appear in the first frame of the generated video. More visual results can be found in Fig. 5.

Conference Papers

and flexible multi-round generation. Across extensive ex-abling camera control for text-to-video generation. In ICLR, periments, Space Time Pilot consistently surpasses state-of- 2025. 8 the-art baselines, offering significantly improved camera-[9] Hao He, Ceyuan Yang, Shanchuan Lin, Yinghao Xu, Meng control accuracy and reliable execution of complex retimingWei, Liangke Gui, Qi Zhao, Gordon Wetzstein, Lu Jiang, and

We present Space Time Pilot, the first video diffusion model to provide fully disentangled spatial and temporal control, enabling 4D space-time exploration from a single monoc- time" representation together with a source-aware camera- poses. This is supported by the synthetic Cam×Time and a temporal-warping training scheme, which supply dense spatiotemporal supervision. These components allow pre-

[1] Jianhong Bai, Menghan Xia, Xintao Wang, Ziyang Yuan,

Xiao Fu, Zuozhu Liu, Haoji Hu, Pengfei Wan, and Di Zhang. Syn Cam Master: Synchronizing multi-camera

*F* to video generation from diverse viewpoints. arXiv preprint

[2] Jianhong Bai, Menghan Xia, Xiao Fu, Xintao Wang, Lianrui

Mu, Jinwen Cao, Zuozhu Liu, Haoji Hu, Xiang Bai, Pengfei Wan, and Di Zhang. Re Cam Master: Camera-controlled gen-

[3] Omer Bar-Tal, Hila Chefer, Omer Tov, Charles Her-

Guanghui Liu, Amit Raj, et al. Lumiere: A space-time diffu- , pages 1-11, 2024.

[4] Andreas Blattmann, Tim Dockhorn, Sumith Kulal, Daniel

Mendelevitch, Maciej Kilian, Dominik Lorenz, Yam Levi, Zion English, Vikram Voleti, Adam Letts, et al. Stable video diffusion: Scaling latent video diffusion models to large datasets. arXiv preprint arXiv:2311.15127, 2023.

[5] Tim Brooks, Bill Peebles, Connor Holmes, Will De Pue,

Yufei Guo, Li Jing, David Schnurr, Joe Taylor, Troy Luh- Ramesh. Video generation models as world simulators.

                          2024. 2

[6] Chen Gao, Ayush Saraf, Johannes Kopf, and Jia-Bin Huang.

Dynamic view synthesis from dynamic monocular video. In ICCV, pages 5712-5721, 2021.

[7] Klaus Greff, Francois Belletti, Lucas Beyer, Carl Doersch,

Yilun Du, Daniel Duckworth, David J Fleet, Dan Gnanapra- Abhijit Kundu, Dmitry Lagun, Issam Laradji, Hsueh- Ti (Derek) Liu, Henning Meyer, Yishu Miao, Derek Nowrouzezahrai, Cengiz Oztireli, Etienne Pot, Noha Rad- Matan Sela, Vincent Sitzmann, Austin Stone, Deqing Sun, Suhani Vora, Ziyu Wang, Tianhao Wu, Kwang Moo Yi, Fangcheng Zhong, and Andrea Tagliasacchi. Kubric: a scal-

[8] Hao He, Yinghao Xu, Yuwei Guo, Gordon Wetzstein, Bo

Dai, Hongsheng Li, and Ceyuan Yang. Camera Ctrl: En-


---

Nattapol Chanpaisit, Yaohui Wang, Xinyuan Chen, Limin Wang, Dahua Lin, Yu Qiao, and Ziwei Liu. VBench: Com- CVPR, 2024. 7

[11] Adobe Systems Inc. Mixamo, 2018. Accessed: 2025-03-07.

[12] Hyeonho Jeong, Chun-Hao Paul Huang, Jong Chul Ye, Niloy

Mitra, and Duygu Ceylan. Track4Gen: Teaching video dif- CVPR, 2025.

[13] Hyeonho Jeong, Suhyeon Lee, and Jong Chul Ye. Reangle-

A-Video: 4d video generation as video-to-video translation. In ICCV, 2025. 3

[14] Haian Jin, Hanwen Jiang, Hao Tan, Kai Zhang, Sai Bi,

Tianyuan Zhang, Fujun Luan, Noah Snavely, and Zexiang Xu. LVSM: A large view synthesis model with minimal 3d inductive bias. In ICLR, 2025.

[15] Bernhard Kerbl, Georgios Kopanas, Thomas Leimk uhler,¨

and George Drettakis. 3d gaussian splatting for real-time radiance field rendering. ACM Trans. Graph., 42(4):139-1,

2023. 2

[16] Weijie Kong, Qi Tian, Zijian Zhang, Rox Min, Zuozhuo Dai,

Jin Zhou, Jiangfeng Xiong, Xin Li, Bo Wu, Jianwei Zhang, et al. Hunyuanvideo: A systematic framework for large video generative models. arXiv preprint arXiv:2412.03603, 2024.

[17] Jiahui Lei, Yijia Weng, Adam Harley, Leonidas Guibas, and

Kostas Daniilidis. Mo Sca: Dynamic gaussian fusion from casual videos via 4d motion scaffolds. In CVPR, 2025.

[18] Zhengqi Li, Tali Dekel, Forrester Cole, Richard Tucker,

Noah Snavely, Ce Liu, and William T Freeman. Learning the depths of moving people by watching frozen people. In

[19] Zhengqi Li, Simon Niklaus, Noah Snavely, and Oliver Wang.

Neural scene flow fields for space-time view synthesis of dy-

[20] Zhengqi Li, Qianqian Wang, Forrester Cole, Richard Tucker,

and Noah Snavely. Dynibar: Neural dynamic image-based rendering. In CVPR, 2023.

[21] Hanwen Liang, Yuyang Yin, Dejia Xu, Hanxue Liang,

Zhangyang Wang, Konstantinos N Plataniotis, Yao Zhao, and Yunchao Wei. Diffusion4D: Fast spatial-temporal con-

2024. 2

[22] Jinwei Lin. Dynamic NeRF: A review. arXiv preprint

arXiv:2405.08609, 2024.

[23] Lu Ling, Yichen Sheng, Zhi Tu, Wentian Zhao, Cheng Xin,

[25] Ben Mildenhall, Pratul P Srinivasan, Matthew Tancik,

Jonathan T Barron, Ravi Ramamoorthi, and Ren Ng. NeRF: Representing scenes as neural radiance fields for view syn-

                          2021. 2

[26] Kepan Nan, Rui Xie, Penghao Zhou, Tiehan Fan, Zhen-

Open Vid-1M: A large-scale high-quality dataset for text-to- 7

[27] Keunhong Park, Utkarsh Sinha, Jonathan T. Barron, Sofien

Bouaziz, Dan B. Goldman, Steven M. Seitz, and Ricardo Martin-Brualla. Nerfies: Deformable neural radiance fields. In ICCV, pages 5865-5874, 2021.

[28] Keunhong Park, Utkarsh Sinha, Peter Hedman, Jonathan T.

Barron, Sofien Bouaziz, Dan B. Goldman, Ricardo Martin- Brualla, and Steven M. Seitz. Hyper NeRF: A higher- radiance fields. ACM Transactions on Graphics (TOG), 40 (6):238:1-238:12, 2021.

[29] Jack Parker-Holder and Shlomi Fruchter. Genie 3: A new

frontier for world models. Google Deep Mind Blog, 2025. Accessed: ¡insert date you retrieved¿. 2, 3

[30] Adam Polyak, Amit Zohar, Andrew Brown, Andros Tjandra,

Animesh Sinha, Ann Lee, Apoorv Vyas, Bowen Shi, Chih- Yao Ma, Ching-Yao Chuang, David Yan, Dhruv Choudhary, Dingkang Wang, Geet Sethi, Guan Pang, Haoyu Ma, Ishan Misra, Ji Hou, Jialiang Wang, Kiran Jagadeesh, Kunpeng Li, Luxin Zhang, Mannat Singh, Mary Williamson, Matt Le, Matthew Yu, Mitesh Kumar Singh, Peizhao Zhang, Pe- Sai Saketh Rambhatla, Sam Tsai, Samaneh Azadi, Samyak Datta, Sanyuan Chen, Sean Bell, Sharadh Ramaswamy, Shelly Sheynin, Siddharth Bhattacharya, Simran Motwani, Tao Xu, Tianhe Li, Tingbo Hou, Wei-Ning Hsu, Xi Yin, Xi- Yi-Chiao Wu, Yue Zhao, Yuval Kirstain, Zecheng He, Zijian He, Albert Pumarola, Ali Thabet, Artsiom Sanakoyeu, Arun Mallya, Baishan Guo, Boris Araya, Breena Kerr, Carleigh Wood, Ce Liu, Cen Peng, Dimitry Vengertsev, Edgar Schon- Liang, John Hoffman, Jonas Kohler, Kaolin Fire, Karthik Sivakumar, Lawrence Chen, Licheng Yu, Luya Gao, Markos Georgopoulos, Rashel Moritz, Sara K. Sampson, Shikai Li, Simone Parmeggiani, Steve Fine, Tara Fowler, Vladan Petro- models, 2024.

[31] Xuanchi Ren, Tianchang Shen, Jiahui Huang, Huan Ling,

Hongsheng Li. Camera Ctrl II: Dynamic scene exploration[24] Jiaxin Lu, Chun-Hao Paul Huang, Uttaran Bhattacharya, via camera-controlled video diffusion models. arXiv preprint Qixing Huang, and Yi Zhou. Humoto: A 4d dataset of mocap arXiv:2503.10592, 2025. human object interactions. In Proceedings of the IEEE/CVF

[10] Ziqi Huang, Yinan He, Jiashuo Yu, Fan Zhang, Chenyang Si, International Conference on Computer Vision (ICCV), pages

Yuming Jiang, Yuanhan Zhang, Tianxing Wu, Qingyang Jin, 10886-10897, 2025. 12

## 12Open Vid-1M: A large-scale high-quality dataset for text-to-

## 2Animesh Sinha, Ann Lee, Apoorv Vyas, Bowen Shi, Chih-

Kun Wan, Lantao Yu, Qianyu Guo, Zixun Yu, Yawen Lu, Yifan Lu, Merlin Nimier-David, Thomas Muller, Alexander¨ et al. DL3DV-10K: A large-scale scene dataset for deep Keller, Sanja Fidler, and Jun Gao. GEN3C: 3d-informed learning-based 3d vision. In CVPR, pages 22160-22169, world-consistent video generation with precise camera con-

2024. 2, 5 trol. In CVPR, 2025. 3


---

Zheng, and Carl Vondrick. Generative camera dolly: Ex-

[34] Team Wan, Ang Wang, Baole Ai, Bin Wen, Chaojie Mao,

Chen-Wei Xie, Di Chen, Feiwu Yu, Haiming Zhao, Jianx- Keyu Yan, Lianghua Huang, Mengyang Feng, Ningyi Zhang, Pandeng Li, Pingyu Wu, Ruihang Chu, Ruili Feng, Shiwei Zhang, Siyang Sun, Tao Fang, Tianxing Wang, Tianyi Gui, Tingyu Weng, Tong Shen, Wei Lin, Wei Wang, Wei Wang, Wenmeng Zhou, Wente Wang, Wenting Shen, Wenyuan Yu, Xianzhong Shi, Xiaoming Huang, Xin Xu, Yan Kou, Yangyu Lv, Yifei Li, Yijing Liu, Yiming Wang, Yingya Zhang, Yi- Zheng, Yuntao Hong, Yupeng Shi, Yutong Feng, Zeyinzi Jiang, Zhen Han, Zhi-Fan Wu, and Ziyu Liu. Wan: Open and advanced large-scale video generative models. arXiv preprint arXiv:2503.20314, 2025. 4, 6

[35] Chaoyang Wang, Ashkan Mirzaei, Vidit Goel, Willi

Menapace, Aliaksandr Siarohin, Avalon Vinella, Michael Vasilkovsky, Ivan Skorokhodov, Vladislav Shakhrai, Sergey Korolev, Sergey Tulyakov, and Peter Wonka. 4real-video- Syst., 2025. 3

[36] Chaoyang Wang, Peiye Zhuang, Tuan Duc Ngo, Willi Mena-

Lee. 4real-video: Learning generalizable photo-realistic 4d video diffusion. In CVPR, 2025. 3

[37] Qianqian Wang, Vickie Ye, Hang Gao, Jake Austin, Zhengqi

Li, and Angjoo Kanazawa. Shape of motion: 4d reconstruc-

[38] Shuzhe Wang, Vincent Leroy, Yohann Cabon, Boris

Chidlovskii, and Jerome Revaud. DUSt3R: Geometric 3d vision made easy. In CVPR, 2024. 8

[39] Yaohui Wang, Xinyuan Chen, Xin Ma, Shangchen Zhou,

Ziqi Huang, Yi Wang, Ceyuan Yang, Yinan He, Jiashuo Yu, Peiqing Yang, et al. Lavie: High-quality video generation with cascaded latent diffusion models. IJCV, pages 1-20,

2024. 2

[40] Zun Wang, Jaemin Cho, Jialu Li, Han Lin, Jaehong Yoon,

Yue Zhang, and Mohit Bansal. EPiC: Efficient Video Camera Control Learning with Precise Anchor-Video. arXiv preprint arXiv:2505.21876, 2025. 3

[41] Daniel Watson, Saurabh Saxena, Lala Li, Andrea Tagliasac-

sion models. In CVPR, 2024. 3, 4, 5, 7

[44] Tong Wu, Shuai Yang, Ryan Po, Yinghao Xu, Ziwei Liu,

Dahua Lin, and Gordon Wetzstein. Video world models with long-term spatial memory. arXiv preprint arXiv:2506.05284,

                          2025. 3

[45] Yuxi Xiao, Jianyuan Wang, Nan Xue, Nikita Karaev, Iurii

Makarov, Bingyi Kang, Xin Zhu, Hujun Bao, Yujun Shen, and Xiaowei Zhou. Spatial TrackerV2: 3d point tracking made easy. In ICCV, 2025. 8

[46] Zhuoyi Yang, Jiayan Teng, Wendi Zheng, Ming Ding, Shiyu

Huang, Jiazheng Xu, Yuanming Yang, Wenyi Hong, Xiao- diffusion models with an expert transformer. arXiv preprint arXiv:2408.06072, 2024.

[47] Jae Shin Yoon, Kihwan Kim, Orazio Gallo, Hyun Soo Park,

and Jan Kautz. Novel view synthesis of dynamic scenes with globally coherent depths from a monocular camera. In CVPR, pages 5339-5348, 2020.

[48] Mark YU, Wenbo Hu, Jinbo Xing, and Ying Shan. Tra-

videos via diffusion models. In ICCV, 2025. 3, 7, 8

[49] Wangbo Yu, Jinbo Xing, Li Yuan, Wenbo Hu, Xiaoyu Li,

Zhipeng Huang, Xiangjun Gao, Tien-Tsin Wong, Ying Shan, and Yonghong Tian. Viewcrafter: Taming video diffusion models for high-fidelity novel view synthesis. arXiv preprint arXiv:2409.02048, 2024. 3

[50] David Junhao Zhang, Roni Paiss, Shiran Zada, Nikhil Kar-

Shou, Neal Wadhwa, and Nataniel Ruiz. Re Capture: Gen- masked video fine-tuning. In CVPR, 2024. 3

[51] David Junhao Zhang, Jay Zhangjie Wu, Jia-Wei Liu, Rui

Zhao, Lingmin Ran, Yuchao Gu, Difei Gao, and Mike Zheng Shou. Show-1: Marrying pixel and latent diffusion models for text-to-video generation. IJCV, pages 1-15, 2024.

[52] Jensen (Jinghao) Zhou, Hang Gao, Vikram Voleti, Aaryaman

Vasishta, Chun-Han Yao, Mark Boss, Philip Torr, Christian Rupprecht, and Varun Jampani. Stable Virtual Camera: Gen-

                          2025. 3

[53] Tinghui Zhou, Richard Tucker, John Flynn, Graham Fyffe,

and Noah Snavely. Stereo magnification: Learning view syn-

[32] Chris Rockwell, Joseph Tung, Tsung-Yi Lin, Ming-Yu Liu, In Proceedings of the IEEE/CVF conference on computer vi-

David F. Fouhey, and Chen-Hsuan Lin. Dynamic camera sion and pattern recognition, pages 20310-20320, 2024. poses and where to find them. In CVPR, 2025. 5 [43] Rundi Wu, Ruiqi Gao, Ben Poole, Alex Trevithick, Changxi

[33] Basile Van Hoorick, Rundi Wu, Ege Ozguroglu, Kyle Sar- Zheng, Jonathan T Barron, and Aleksander Holynski.

gent, Ruoshi Liu, Pavel Tokmakov, Achal Dave, Changxi Cat4D: Create anything in 4d with multi-view video diffu-

[42] Guanjun Wu, Taoran Yi, Jiemin Fang, Lingxi Xie, Xiaopeng

Zhang, Wei Wei, Wenyu Liu, Qi Tian, and Xinggang Wang. 4d gaussian splatting for real-time dynamic scene rendering.


---

## A. Network Architecture

## C. Additional Details on the Proposed

Cam×Time Dataset.

## B. Longer Space-Time Exploration Video with

## Disentangled Controls

into tensors matching the shapes of *x* srcand *x* trg, which are then added to them respectively. During training, we train only the camera embedder E cam, the animation-time embed-

model can generate video segments whose camera poses do not need to start at the first frame. This allows precise con- every generated chunk, ensuring smooth, consistent motion over extended sequences.

To maintain contextual coherence across iterations, we

introduce a lightweight memory mechanism. During train- which enables consistent chaining during inference. Specif-

                        - At iteration = 1, the model is conditioned only on the

original source video.

                        - At iteration = 2, it is conditioned on both the source

video and the previously generated 81-frame segment.

                        - This process repeats, with each iteration conditioning on

the source video as well as the most recent generated seg-

This simple yet effective strategy allows Space Time Pilot

to generate arbitrarily long, smoothly connected sequences with continuous and precise control over both temporal ma-

Here, we showcase how this can be used to conduct large

viewpoint changes, as demonstrated in Fig. 10.

of the previous output, while temporally traversing the re-

The network architecture of Space Time Pilot is depicted in of viewpoint-controlled video segments that together create

*Fig. 8. The newly introduced animation-time embedder E ani a continuous long-range space-time trajectory.*

encodes the source and target animation times,srcand **t** trg,

A key property that enables this behavior is that our

11 24

a 0.5× slow-motion sequence covering frames 0-40 with with clear visibility of the main character, (2) maintains a new camera trajectory. Then, continuing both the visualnon-intersecting movement with the environment through- duce the next segment starting from the final camera poseall viewpoints.

*Figure 8. Architecture of Space Time Pilot. Our model jointly*

conditions on camera trajectories and temporal control signals via space-time attention, enabling non-monotonic motion generation such as reversals, repeats, accelerations, and zigzag time.

One of the central advantages of Space Time Pilot is its abil- fully customizable trajectories through them. Although each individual generation is limited to an 81-frame win- this window indefinitely through a multi-turn autoregres- overall pipeline is illustrated in Fig. 9. The core idea is to generate the final video in autore- given a source video of 81 frames, we may first generate

The Cam×Time dataset is built using high-quality, com- populate the space with multiple animated human charac- and HUMOTO [], and each character is manually tex- and material quality. The animations span a diverse range of human motions, including locomotion, gestures, and human-object interactions. Examples of scenes are shown in Fig. 11. Please refer to the complementary website for the video examples. To capture rich spatial coverage, we generate four dis- include rotational orbits, linear tracking motions, and smoothly curved arcs. A dedicated validity module ensures that each trajectory: (1) begins at a collision-free location


---

to 180k videos.

imations, each with 120 videos full grid rendering, leading

geometry. Overall, we rendered 1500 videos from 500 an-in Fig. 12.

full motion duration with consistent lighting, textures, andcillation. These augmented temporal signals are illustrated

multi-view video sequences per scene, each covering thenon-monotonic time patterns such as forward-backward os-

a resolution of 1080 × 1080 pixels, providing dense tem- variants from these sequences, including slow motion, re-

Each trajectory is rendered into a 120-frame sequence at For temporal-control training, we could sample any time

bird's-eye view-while preserving visual and motion coherence. Please refer to section "AR Demos" in the website for videos.

viewpoint changes far beyond the input video-such as rotating to the rear of the tiger or transitioning from a low-angle shot to a high

chunk, ensuring temporal continuity, stable motion progression, and consistent camera geometry. This dual-conditioning design enables

and Turn-3 generations. At each turn, Space Time Pilot jointly conditions on (1) the original source video and (2) the previously generated

*Figure 10. Multi-turn autoregressive generation with Space Time Pilot Top row: source video frames. Rows 2-4: Turn-1, Turn-2,*

produces a long, coherent video that follows an arbitrary space-time path.

iterations, each with its own camera and temporal trajectory. By chaining these iterations and stitching the outputs, Space Time Pilot

on the source video and a chosen space-time trajectory. The resulting output is then reused as a secondary source video for subsequent

*Figure 9. Overview of the multi-turn autoregressive inference scheme. The model first generates an 81-frame segment conditioned*


---

## E. Additional Qualitative Visualizations

## D. Additional Ablation Studies

*Figure 11. Example of Cam×Time. Multi-view, densely sam-*

pled sequences from the Cam×Time dataset. Each row shows frames from one camera trajectory, and each column samples dif- 120-frame temporal coverage.

In Fig. 14 (bottom), we further show that freezing tempo- without freezing it. Please refer to section "Freeze Warping Ablations" in the website for more videos.

### D.2. Significance of Cam×Time Dataset

Besides the quantitative results in the main paper (Table 5), in Fig. 15 (top), we provide visual comparisons demonstrat- Clear artifacts appear in baselines trained without additional data or with only static-scene augmentation (highlighted in red boxes), whereas incorporating Cam×Time removes these artifacts, demonstrating its significance. Please re- videos.

### D.3. Time Embedding Ablation

As promised in Sec. 3.2.1 in the main paper, we compare several time-embedding strategies. RoPE(*f* ′) can freeze the scene dynamics at=40, but it also undesirably locks the camera motion. Using MLP, by contrast, fails to lock the temporal state at all (red boxes). Conditioning on the la- scene dynamics while still generating accurate camera mo- results. Please refer to section "Time-Embedding Method Ablation" in the website for more examples.

scene datasets naturally support bullet-time effects, they do not provide enough diversity of temporal control configura- as shown in Fig. 14 (top). Please refer to section "Effective Temporal Warping" in the website for more videos.

Using [1, 2] as our default datasets, we compare training jointly with static-scene datasets [18, 53] with applying only temporal warping (TW) augmentation on the default datasets (Sec. 3.2.2 in the main paper). Although static-

*Figure 12. Sampling from Cam×Time. By sampling from the*

Cam×Time dataset, we can extract frames corresponding to arbi- forming source-target pairs with rich camera and temporal control signals.

### D.1. Temporal Warping Augmentation

We show more qualitative results of Space Time Pilot in

*Fig. 13. Our model provides fully disentangled control over*

camera motion and temporal dynamics. Each row presents a different pairing of temporal control inputs (top-left icon) and camera trajectories. Space Time Pilot reliably generates coherent videos under diverse conditions, including normal and reverse playback, bullet-time, slow motion, replay mo-


---

and complex camera paths such as panning, tilting, zooming, and vertical motion.

produces coherent videos across a wide range of controls, including normal and reverse playback, bullet-time, slow motion, replay motion,

row illustrates a different combination of temporal control inputs (top-left icon) and camera trajectories. Space Time Pilot consistently

*Figure 13. More Qualitative results. Our model provides fully disentangled control over camera motion and temporal dynamics. Each*


---

warping, where we show freezing temporal warping (3rd row) leads to better results than those trained without freezing temporal warping.

control signals, allowing models to learn better camera-time disentanglement. (Bottom) We further compare different configurations of

doing temporal warping (TW) augmentation (Sec. 3.2.2 in the main paper). Temporal warping definitely provide more variety of time

*Figure 14. Ablation study. (Top) Using [1, 2] as default datasets, we compare the influence of adding static-scene datasets [18, 53] vs. just*


---

scene dynamics at **t**=40 and produce intended camera motion. Incorporating Cam×Time during training further improves performance.

uniform sampling) introduces noticeable artifacts. In contrast, the proposed 1D-Conv embedding allows Space Time Pilot to both freeze the

correctly freezes the scene dynamics at=40 but unintentionally locks the camera motion too. Conditioning on the latent frame ′ (with

dataset. (Bottom) We compare several time-embedding strategies. The MLP fails to lock the temporal state (red boxes), while RoPE(′)

augmented with static-scene data, whereas training additionally with Cam×Time leads to no artifacts, confirming the usefulness of our

we compare the impact of different datasets on the generated videos. One can clearly see artifacts in baselines without any extra data or

*Figure 15. Ablation study. (Top) We verify the efficacy of the proposed Cam×Time dataset. Considering [1, 2] as default datasets,*
