from fastapi import FastAPI, File, UploadFile, HTTPException
import io
import wave
import os
import base64
import httpx

OLLAMA_BASE_URL = os.getenv(
    "OLLAMA_BASE_URL",
    "http://127.0.0.1:11434"
)

OLLAMA_MODEL = os.getenv(
    "OLLAMA_MODEL",
    "frozenlab/qwen3-asr:0.6b"
)

app = FastAPI()


@app.get("/health")
async def health():
    return {"status": "ok"}

@app.post("/transcribe")
async def transcribe(file: UploadFile = File(...)):
    audio_bytes = await file.read()

    try:
        with wave.open(io.BytesIO(audio_bytes), "rb") as audio:
            sample_rate = audio.getframerate()
            channels = audio.getnchannels()
            sample_width = audio.getsampwidth()
            duration = audio.getnframes() / sample_rate

    except (wave.Error, EOFError):
        raise HTTPException(
            status_code=422,
            detail="Invalid WAV file"
        )

    encoded_audio = base64.b64encode(audio_bytes).decode("utf-8")

    payload = {
        "model": OLLAMA_MODEL,
        "stream": False,
        "messages": [
            {
                "role": "user",
                "content": "",
                "images": [encoded_audio]
            }
        ]
    }

    async with httpx.AsyncClient(timeout=120.0) as client:
        response = await client.post(
            f"{OLLAMA_BASE_URL}/api/chat",
            json=payload
        )

        response.raise_for_status()

        result = response.json()

    return result
