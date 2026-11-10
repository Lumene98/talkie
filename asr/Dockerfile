FROM ollama/ollama:latest

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        python3 python3-venv python3-pip && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN python3 -m venv /opt/venv

ENV PATH="/opt/venv/bin:${PATH}"

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py .
COPY start.sh .

RUN chmod +x start.sh

EXPOSE 8001

ENTRYPOINT ["/app/start.sh"]
