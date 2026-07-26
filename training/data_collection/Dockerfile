FROM python:3.11-slim

WORKDIR /app

COPY pyproject.toml requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY finbert_sft/ finbert_sft/
COPY data_collection/ data_collection/

ENTRYPOINT ["python", "-m", "data_collection.collect"]
CMD ["--watch"]
