#!/bin/sh

OLLAMA_BASE_MODEL=${OLLAMA_BASE_MODEL:-martain7r/finance-llama-8b}
OLLAMA_MODEL_NAME=${OLLAMA_MODEL_NAME:-fin-expert}

sed -i "s|^FROM .*|FROM ${OLLAMA_BASE_MODEL}|" Modelfile

ollama pull "${OLLAMA_BASE_MODEL}"
ollama create "${OLLAMA_MODEL_NAME}" -f Modelfile

exec ollama serve
