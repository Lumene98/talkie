#!/bin/sh
set -eu

echo "Starting Ollama..."
ollama serve &

echo "Waiting for Ollama..."
until ollama list >/dev/null 2>&1; do
    sleep 1
done

MODEL="${OLLAMA_MODEL:-frozenlab/qwen3-asr:0.6b}"

if ! ollama show "$MODEL" >/dev/null 2>&1; then
    echo "Downloading $MODEL..."
    ollama pull "$MODEL"
fi

echo "Starting FastAPI..."
exec uvicorn main:app --host 0.0.0.0 --port 8001

