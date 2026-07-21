FROM python:3.11-slim

WORKDIR /app

COPY training/data_collection/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY training/data_collection/ .

CMD ["python", "collect.py", "--watch"]
