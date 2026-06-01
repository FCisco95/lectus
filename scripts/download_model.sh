#!/bin/bash
set -e
MODEL_DIR="$(dirname "$0")/../models"
mkdir -p "$MODEL_DIR"
MODEL_FILE="$MODEL_DIR/ggml-tiny.en.bin"
if [ -f "$MODEL_FILE" ]; then echo "Model already exists"; exit 0; fi
echo "Downloading ggml-tiny.en.bin..."
curl -L -o "$MODEL_FILE" "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
echo "Done: $MODEL_FILE"
