#!/bin/sh

ollama pull qwen3:1.7b

ollama create fin-expert -f Modelfile

exec ollama serve
