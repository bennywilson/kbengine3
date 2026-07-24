# NVIDIA-only: requires --gpus all on Linux, WSL2 + the CUDA-on-WSL driver
# on Windows. No Mac support -- Apple Silicon has no CUDA, and OpenVLA's
# from_pretrained example hard-codes .to("cuda:0").
FROM nvidia/cuda:12.1.1-cudnn8-runtime-ubuntu22.04

RUN apt-get update && apt-get install -y --no-install-recommends \
        python3.10 python3-pip python3.10-dev build-essential \
    && rm -rf /var/lib/apt/lists/*
RUN ln -s /usr/bin/python3.10 /usr/bin/python

WORKDIR /app

COPY requirements.txt .
# torch is pinned to the exact version OpenVLA was tested against (see
# requirements.txt), which is old enough to only ship CUDA wheels on this
# separate index rather than resolving them via PyPI directly. build-essential
# (gcc) + python3.10-dev (Python.h) are required at runtime too, not just
# build time -- Triton JIT-compiles a CUDA kernel on first inference call and
# fails without a C compiler and Python headers present.
RUN pip install --no-cache-dir --extra-index-url https://download.pytorch.org/whl/cu121 \
        -r requirements.txt

COPY server.py .

# Weights are NOT baked into the image. AutoProcessor/AutoModelForVision2Seq
# .from_pretrained() in server.py downloads them from the Hugging Face Hub
# the first time the container starts, caching under HF_HOME -- mount that
# path as a volume (see docker-compose.yml) so a re-run doesn't re-download
# several GB of weights.
ENV HF_HOME=/root/.cache/huggingface

EXPOSE 8000

CMD ["python", "-m", "uvicorn", "server:app", "--host", "0.0.0.0", "--port", "8000"]
