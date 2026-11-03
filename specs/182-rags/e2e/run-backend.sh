#!/bin/bash
# SPEC-182 demo backend: a Normal RAG workspace that embeds text and images with Vertex
# `gemini-embedding-2`, on :8090 (the port the WebUI dev proxy probes first).
#
# WHY unset OPENAI_API_KEY: the Makefile shell exports one, and a present key makes the
# server advertise `llm_provider_name=openai` for the *chat* model — which would then be the
# model named in every screenshot instead of the local one the demo uses.
unset OPENAI_API_KEY
export PORT=8090
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic"
export GOOGLE_CLOUD_PROJECT="${GOOGLE_CLOUD_PROJECT:-mobioos-506015}"
export EDGEQUAKE_LLM_PROVIDER=ollama
export EDGEQUAKE_LLM_MODEL=gemma4:latest
export OLLAMA_HOST=http://localhost:11434
export EDGEQUAKE_AUTH_ENABLED=false
export AUTH_ENABLED=false
export EDGEQUAKE_DEV_MODE=true
export RUST_LOG=info
cd "$(dirname "$0")/../../../edgequake" || exit 1
exec ./target/debug/edgequake
