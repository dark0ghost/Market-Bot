# https://medium.com/data-science-in-your-pocket/model-context-protocol-mcp-using-ollama-e719b2d9fd7a
FROM ollama/ollama:0.5.12

COPY Modelfile Modelfile
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

