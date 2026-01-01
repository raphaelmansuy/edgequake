# Roadblocks and Mitigation

## 1. `pdfium-render` Thread Safety

**Risk**: `pdfium-render` might not be `Send` or `Sync` depending on how it's initialized and used. The `PdfBackend` trait requires `Send + Sync`.
**Mitigation**:

- Check `pdfium-render` documentation.
- If `Pdfium` struct is not thread-safe, we might need to wrap it in a `Mutex` or instantiate it per request (which might be expensive).
- Alternatively, the `extract` method takes `&self`, so the backend needs to be shared. If `Pdfium` is not thread-safe, we might need a pool of workers or a single-threaded actor.
- _Investigation_: `pdfium_render::Pdfium` seems to be a handle to the library. `PdfDocument` is the per-document handle.

## 2. Async Trait Overhead

**Risk**: Using `#[async_trait]` adds boxing overhead.
**Mitigation**:

- For PDF extraction, the I/O and CPU work dwarfs the allocation overhead. It's negligible.

## 3. Breaking Changes

**Risk**: Changing `PdfExtractor`'s API will break existing code (e.g., `examples/`, `tests/`).
**Mitigation**:

- Keep `PdfExtractor::new` as a convenience wrapper that sets up the default backend (Pdfium).
- Mark old methods as deprecated if necessary, or just update them to use the new internal structure while keeping the signature compatible where possible.

## 4. Feature Flags Complexity

**Risk**: Managing `cfg(feature = "pdfium")` across the codebase can get messy.
**Mitigation**:

- Isolate all `pdfium` specific code in `src/backend/pdfium.rs`.
- The rest of the code should only interact with the `PdfBackend` trait.
- Only the factory/builder method in `PdfExtractor` needs the `cfg` check to decide which backend to instantiate by default.

## 5. Testing without Pdfium

**Risk**: CI environments might not have the `pdfium` dynamic library installed.
**Mitigation**:

- The `MockBackend` is crucial here.
- Ensure `cargo test` runs with the mock backend by default or when the `pdfium` feature is disabled.
- Add a specific test suite that requires `pdfium` and is skipped if the library is missing.
