"""OpenVLA inference server for Black Splat's robot-policy integration.

Implements the model-agnostic `/act` contract that the Rust/WASM client
calls over HTTP:

    POST /act
    {
        "image": "<base64-encoded PNG/JPEG>",
        "instruction": "<free-text task instruction>",
        "proprioception": [<float>, ...],   # optional, model-specific
        "unnorm_key": "<dataset name>"      # optional, OpenVLA-specific
    }
    ->
    {
        "actions": [[<float>, ...], ...],   # one or more action-chunk steps
        "model": "openvla/openvla-7b"
    }

`actions` is always a list of steps (each a flat vector), even for models
like OpenVLA that only ever return one step per call -- this is what lets a
chunk-predicting model (Octo, pi0, ...) be swapped in later without changing
the response shape the Rust client parses. Each step's vector is whatever
the model's own action space is (OpenVLA/Franka: 6-DoF end-effector delta +
1 gripper value); mapping that into MuJoCo joint positions is the client's
job, not this server's.

Weights are NOT baked into the image -- `from_pretrained` below downloads
them on first startup and caches them under `HF_HOME` (see the Dockerfile,
which mounts that path as a volume so re-runs skip the download).
"""
import base64
import io
import json
import os
from pathlib import Path

import torch
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from PIL import Image
from pydantic import BaseModel
from peft import PeftModel
from transformers import AutoModelForVision2Seq, AutoProcessor, BitsAndBytesConfig

MODEL_ID = os.environ.get("MODEL_ID", "openvla/openvla-7b")
# Which dataset's action statistics OpenVLA should use to de-normalize its
# output back into real units -- required by `predict_action`. Defaults to
# the Bridge dataset (matches the model card's zero-shot example); override
# per-deployment or per-request if fine-tuned on something else.
DEFAULT_UNNORM_KEY = os.environ.get("UNNORM_KEY", "bridge_orig")
# "eager" needs no extra compiled deps; set to "flash_attention_2" if that's
# installed in the image for faster inference.
ATTN_IMPLEMENTATION = os.environ.get("ATTN_IMPLEMENTATION", "eager")
# Load the 7B weights 4-bit (NF4) instead of bf16: ~5GB instead of ~15GB.
# Default-on because bf16 does not leave workable headroom on a 16GB card --
# the weights alone reach ~14.9/16.4GB, and rather than OOM outright, Windows'
# WDDM driver silently spills the overflow to system RAM over PCIe, so
# inference still "works" but takes minutes per call instead of seconds.
# Quantized inference also matches how the local checkpoints were fine-tuned
# (QLoRA, same NF4 config). Set QUANTIZE_4BIT=0 on a big-VRAM machine, where
# bf16 is both feasible and slightly higher-fidelity.
QUANTIZE_4BIT = os.environ.get("QUANTIZE_4BIT", "1") not in ("0", "false", "False")
# Optional LoRA adapter to apply on top of MODEL_ID, instead of pointing
# MODEL_ID at an already-merged checkpoint. Serving the adapter directly is
# much cheaper to iterate on: an adapter is ~1GB against a merged model's
# ~15GB, so trying a different training checkpoint means moving 1GB rather
# than merging and copying a whole new model. `finetune.py` writes one of
# these to its --adapter_tmp_dir at every save_steps interval, so mid-run
# checkpoints can be served without waiting for a run to finish and merge.
ADAPTER_DIR = os.environ.get("ADAPTER_DIR", "").strip()
# Explicit path to a `dataset_statistics.json`. Needed when serving an adapter,
# since MODEL_ID then points at the *base* model, whose directory has no such
# file (and whose config.json carries only the base OXE norm_stats). Defaults
# to looking next to MODEL_ID, which is right for a merged local checkpoint.
DATASET_STATS = os.environ.get("DATASET_STATS", "").strip()
ALLOWED_ORIGINS = os.environ.get("ALLOWED_ORIGINS", "*").split(",")

app = FastAPI(title="Black Splat OpenVLA server")

app.add_middleware(
    CORSMiddleware,
    allow_origins=ALLOWED_ORIGINS,
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)

_device = "cuda" if torch.cuda.is_available() else "cpu"
_processor = None
_model = None


class ActRequest(BaseModel):
    image: str
    instruction: str
    proprioception: list[float] | None = None
    unnorm_key: str | None = None


class ActResponse(BaseModel):
    actions: list[list[float]]
    model: str


@app.on_event("startup")
def load_model():
    global _processor, _model
    if _device == "cpu":
        # Not a hard error -- useful for smoke-testing the HTTP contract
        # without a GPU -- but OpenVLA-7b inference on CPU is impractically
        # slow for anything beyond that.
        print("WARNING: no CUDA device found, loading OpenVLA on CPU.")
    _processor = AutoProcessor.from_pretrained(MODEL_ID, trust_remote_code=True)

    quantization_config = None
    if QUANTIZE_4BIT and _device == "cuda":
        quantization_config = BitsAndBytesConfig(
            load_in_4bit=True, bnb_4bit_compute_dtype=torch.bfloat16, bnb_4bit_quant_type="nf4"
        )
    _model = AutoModelForVision2Seq.from_pretrained(
        MODEL_ID,
        attn_implementation=ATTN_IMPLEMENTATION,
        torch_dtype=torch.bfloat16 if _device == "cuda" else torch.float32,
        quantization_config=quantization_config,
        low_cpu_mem_usage=True,
        trust_remote_code=True,
    )
    # bitsandbytes places quantized weights on the GPU itself during loading,
    # and transformers rejects a later .to() on a 4-/8-bit model outright.
    if quantization_config is None:
        _model = _model.to(_device)

    if ADAPTER_DIR:
        # PEFT injects the LoRA layers into the base model's modules in place, so
        # the unwrapped inner model still runs adapted weights -- and `predict_action`
        # (defined on OpenVLAForActionPrediction, not exposed through the PeftModel
        # wrapper) stays directly callable on it.
        print(f"applying LoRA adapter: {ADAPTER_DIR}")
        _model = PeftModel.from_pretrained(_model, ADAPTER_DIR).base_model.model
    _model.eval()

    # A locally fine-tuned checkpoint's config.json still carries the *base*
    # model's norm_stats (whatever OXE datasets it originally shipped with) --
    # `finetune.py` never merges the new dataset's stats into it, it only
    # writes a sibling `dataset_statistics.json`. Without this, `unnorm_key`
    # for anything fine-tuned here (e.g. "black_splat_tool_hang") would raise
    # "not in the set of available dataset statistics". Matches the same
    # "[Hacky]" load OpenVLA's own `vla-scripts/deploy.py` does.
    stats_path = Path(DATASET_STATS) if DATASET_STATS else Path(MODEL_ID) / "dataset_statistics.json"
    if stats_path.is_file():
        print(f"loading action norm_stats from: {stats_path}")
        with open(stats_path) as f:
            _model.norm_stats = json.load(f)
    elif DATASET_STATS:
        raise RuntimeError(f"DATASET_STATS points at a missing file: {stats_path}")


@app.get("/health")
def health():
    return {"status": "ok", "model": MODEL_ID, "device": _device, "model_loaded": _model is not None}


@app.post("/act", response_model=ActResponse)
def act(req: ActRequest):
    if _model is None or _processor is None:
        raise HTTPException(status_code=503, detail="model not loaded yet")

    try:
        image_bytes = base64.b64decode(req.image)
        image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"invalid image: {exc}") from exc

    prompt = f"In: What action should the robot take to {req.instruction}?\nOut:"
    inputs = _processor(prompt, image).to(_device, dtype=torch.bfloat16 if _device == "cuda" else torch.float32)
    # OpenVLA's own predict_action() conditionally appends a training-time
    # marker token to input_ids (to match its expected "Out:" formatting) but
    # never extends attention_mask to match, so a stale attention_mask here
    # causes a shape mismatch a token later inside the language model
    # ("size of tensor a (N) must match tensor b (N-1)"). We're always doing
    # unpadded batch-size-1 inference, so just drop it -- generate() builds a
    # correct all-ones mask itself from the (already fixed-up) input_ids.
    inputs.pop("attention_mask", None)

    unnorm_key = req.unnorm_key or DEFAULT_UNNORM_KEY
    try:
        action = _model.predict_action(**inputs, unnorm_key=unnorm_key, do_sample=False)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"inference failed: {exc}") from exc

    # OpenVLA predicts a single next-step action; wrap it as a one-step chunk
    # so the response shape matches chunk-predicting models too.
    return ActResponse(actions=[action.tolist()], model=MODEL_ID)


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=int(os.environ.get("PORT", "8000")))
