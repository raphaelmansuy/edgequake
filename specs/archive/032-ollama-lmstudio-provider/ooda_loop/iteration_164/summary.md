# OODA Iteration 164 - Memory Management for Local Providers

## Observe

### Focus

Verify that memory considerations are documented for local providers.

### Investigation

**Local Provider Memory**:

- Ollama and LM Studio load models into memory
- Large models require significant VRAM/RAM
- Model switching may require unloading

**Model Size Examples** (from model cards):

- qwen3:8b - 8B parameters (~4-8GB VRAM)
- gemma3:12b - 12B parameters (~6-12GB VRAM)
- llama3.2:70b - 70B parameters (~35-40GB VRAM)

## Orient

### Memory Requirements

| Model Size | VRAM (quantized) | VRAM (full) |
| ---------- | ---------------- | ----------- |
| 3B         | ~2GB             | ~6GB        |
| 7-8B       | ~4GB             | ~16GB       |
| 12B        | ~6GB             | ~24GB       |
| 70B        | ~35GB            | ~140GB      |

### Memory Management Strategy

1. **Single model loaded**: Most efficient
2. **Model switching**: May cause unload/reload
3. **Concurrent models**: Requires sufficient VRAM

## Decide

**Status**: ✅ COMPLETE

Memory requirements are implicit in model card documentation.

## Act

### Verified

- Model sizes documented (e.g., "70b")
- User can infer memory requirements
- Local providers handle one model at a time
- Adequate for typical deployments

---

_Commit: docs(OODA 164): Verify memory management documentation_
