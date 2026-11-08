# EdgeQuake Dockerfile
# Multi-stage build for minimal image size
# Multi-platform: supports linux/amd64, linux/arm64 (macOS Apple Silicon, AWS Graviton, etc.)
# Build: docker buildx build --platform linux/amd64,linux/arm64 -t edgequake .
# Or:    docker compose build  (uses DOCKER_DEFAULT_PLATFORM or host default)
#
# SPEC-040 / SPEC-095: pdfium is embedded at compile time via pdfium-auto
# (chromium/7961). No external libpdfium or legacy PDFIUM_DYNAMIC_LIB_PATH.
# Runtime: atomic extract + size integrity under PDFIUM_AUTO_CACHE_DIR.
# Hardened deploys may set PDFIUM_LIB_PATH to a pre-placed read-only library.
#
# edgeparse-ocr: Tesseract OCR CLI + tessdata language packs are harvested from
# Debian into the distroless runtime (no apt/shell in the final image).
#
# Runtime is distroless (no curl/wget/shell) so Trivy HIGH from those packages
# cannot return. HEALTHCHECK uses `edgequake healthcheck` (GET /live).

# ============================================
# Stage 1: Build
# ============================================
# TARGETPLATFORM / BUILDPLATFORM are populated automatically by Docker BuildKit
# when using `docker buildx build --platform ...`.  In a plain `docker build`
# or `docker compose build` the variables are empty, so we must NOT interpolate
# them into `FROM --platform=` (an empty value is rejected by the daemon with an
# "invalid component" error – issue #92).  We therefore let Docker choose the
# base-image platform from the manifest, which gives native builds for free and
# keeps cross-compilation working via buildx without extra flags.
ARG TARGETPLATFORM
ARG BUILDPLATFORM
FROM rust:bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# WHY: Build context is the monorepo root. edgequake-pdf2md is a crates.io
# dependency (see edgequake/Cargo.toml) — no sibling path checkout required.
# Copy workspace files (edgequake subdirectory)
COPY edgequake/Cargo.toml edgequake/Cargo.lock ./
COPY edgequake/crates/ crates/
COPY edgequake/src/ src/
COPY edgequake/migrations/ migrations/
COPY edgequake/.sqlx .sqlx/
# Cargo parses [[bench]] / [[example]] at manifest load — sources must exist even
# though `cargo build --release` does not compile them (CD flake root-cause).
COPY edgequake/benches/ benches/
COPY edgequake/examples/ examples/
# WHY: models.toml defines available LLM providers/models (gpt-5.4, claude-sonnet-4-6,
# gemma4, etc.). The binary falls back to built-in defaults (edgequake-llm v0.5.1) which
# only include gpt-4o and gpt-4o-mini — the new models are only in models.toml.
# We copy it into the build layer so Stage 2 can retrieve it via --from=builder.
COPY edgequake/models.toml models.toml
# WHY: `crates/edgequake-api/build.rs` and `edgequake/build.rs` both
# `include!("../../build_meta.rs")` / `include!("build_meta.rs")`. Without this
# COPY the build script fails with "couldn't find file" and both Docker arches
# die at ~250s, long after the image layers look healthy. Added after v0.38.4
# introduced the shared include without adding it to the build context.
COPY edgequake/build_meta.rs build_meta.rs

# pdfium-auto 0.3.1 downloads chromium%2F7961. That URL, and the slash-form
# HTML download, return HTTP 504 often enough to fail the image build. The
# release-asset API for chromium/7961 redirects to a copy that downloads.
# Asset ids: 483237356 linux-x64, 483237422 linux-arm64.
RUN arch="$(uname -m)"; \
    case "$arch" in \
      x86_64) asset=483237356 ;; \
      aarch64) asset=483237422 ;; \
      *) echo "unsupported pdfium arch: $arch" >&2; exit 1 ;; \
    esac; \
    mkdir -p /opt/pdfium; \
    curl -fL --retry 5 --retry-all-errors --retry-delay 2 \
      -H "Accept: application/octet-stream" \
      -o /tmp/pdfium.tgz \
      "https://api.github.com/repos/bblanchon/pdfium-binaries/releases/assets/${asset}"; \
    tar -xzf /tmp/pdfium.tgz -C /opt/pdfium lib/libpdfium.so; \
    mv /opt/pdfium/lib/libpdfium.so /opt/pdfium/libpdfium.so; \
    rm -rf /tmp/pdfium.tgz /opt/pdfium/lib
ENV PDFIUM_BUNDLE_LIB=/opt/pdfium/libpdfium.so

# Build release binary with SQLx offline mode (no database needed at build time).
# pdfium-auto embeds the library from PDFIUM_BUNDLE_LIB for TARGETPLATFORM.
# OTLP (`otel`) is on by default (SPEC-124 Langfuse / Jaeger). Set ENABLE_OTEL=false
# only if you need a deliberately slimmer image without the OTEL feature.
ARG ENABLE_OTEL=true
ENV SQLX_OFFLINE=true
RUN if [ "$ENABLE_OTEL" = "false" ]; then \
      cargo build --release --locked --no-default-features --features "postgres,vision"; \
    else \
      cargo build --release --locked; \
    fi

# ============================================
# Stage 2: Shared runtime deps (zlib + Tesseract OCR)
# ============================================
# debian-slim previously provided zlib for free. Distroless cc has glibc +
# libstdc++ only. Copy .so files into /opt/edgequake-libs (not /lib) so we
# never overlay glibc. LD_LIBRARY_PATH is set in the runtime stage.
#
# Tesseract + tessdata (LSTM language models) power `edgeparse-ocr`. We harvest
# the CLI binary, shared libs, and traineddata — no shell/apt in the final image.
# Language packs: eng + osd + fra/deu/spa/por/chi_sim. Vision LLM weights stay
# external (Ollama / OpenAI / …). pdfium remains compile-time embedded.
FROM debian:bookworm-slim AS libs
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        zlib1g \
        tesseract-ocr \
        tesseract-ocr-eng \
        tesseract-ocr-osd \
        tesseract-ocr-fra \
        tesseract-ocr-deu \
        tesseract-ocr-spa \
        tesseract-ocr-por \
        tesseract-ocr-chi-sim \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /runtime-libs /ocr/bin /ocr/lib \
    && cp -a /lib/*/libz.so.1* /runtime-libs/ \
    && cp -a /usr/bin/tesseract /ocr/bin/tesseract \
    && cp -a /usr/share/tesseract-ocr /ocr/share-tesseract-ocr \
    && ldd /usr/bin/tesseract | awk '/=>/ {print $3}' \
      | while read -r lib; do \
          [ -n "$lib" ] && [ -f "$lib" ] || continue; \
          base=$(basename "$lib"); \
          case "$base" in \
            libc.so*|libm.so*|libdl.so*|librt.so*|libpthread.so*|libgcc_s.so*|libstdc++.so*|ld-linux*|libresolv.so*) continue ;; \
          esac; \
          cp -aL "$lib" "/ocr/lib/$base"; \
        done \
    && test -x /ocr/bin/tesseract \
    && test -f /ocr/lib/libtesseract.so.5 \
    && test -f /ocr/share-tesseract-ocr/5/tessdata/eng.traineddata \
    && env -u LD_LIBRARY_PATH LD_LIBRARY_PATH=/ocr/lib /ocr/bin/tesseract --list-langs \
    && test "$(ls -1 /ocr/lib | wc -l)" -ge 5

# ============================================
# Stage 3: Runtime (no shell, no curl, no apt)
# ============================================
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app

# pdfium-auto extract cache (SPEC-095: atomic publish + lock + size check).
# Prefer PDFIUM_LIB_PATH=/path/to/libpdfium.so for hardened read-only mounts.
# /tmp is world-writable on distroless; prime_pdfium mkdir's the cache dir.
ENV PDFIUM_AUTO_CACHE_DIR=/tmp/edgequake-pdfium-cache
ENV LD_LIBRARY_PATH=/opt/edgequake-libs:/opt/edgequake-ocr/lib
# Debian bookworm: point at the directory that contains `*.traineddata`.
ENV TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
ENV PATH=/usr/local/bin:/usr/bin:/bin

COPY --from=libs /runtime-libs /opt/edgequake-libs
COPY --from=libs /ocr/lib /opt/edgequake-ocr/lib
COPY --from=libs /ocr/bin/tesseract /usr/local/bin/tesseract
COPY --from=libs /ocr/share-tesseract-ocr /usr/share/tesseract-ocr
COPY --from=builder /app/target/release/edgequake /usr/local/bin/edgequake
# Copy models.toml so the runtime binary finds up-to-date models at ./models.toml
# (CWD=/app, see WORKDIR above). Without this, the binary falls back to the compiled-in
# defaults which lack the v0.9.8 models (gpt-5.4, claude-sonnet-4-6, gemma4).
COPY --from=builder /app/models.toml /app/models.toml

USER nonroot

EXPOSE 8080

# Exec form: distroless has no shell. Probes GET /live (not deep /health).
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/edgequake", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/edgequake"]
