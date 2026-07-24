"""Fake model server implementing the `/act` contract with a canned
response -- no torch/transformers/GPU required. Exists purely to test the
contract itself (request/response shape, CORS, the Rust client) end-to-end
without paying for OpenVLA's weights download or a GPU.

Run: python policy_server/stub_server.py
"""
import base64

import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

app = FastAPI(title="Black Splat stub policy server")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)


class ActRequest(BaseModel):
    image: str
    instruction: str
    proprioception: list[float] | None = None
    unnorm_key: str | None = None


class ActResponse(BaseModel):
    actions: list[list[float]]
    model: str


@app.get("/health")
def health():
    return {"status": "ok", "model": "stub", "model_loaded": True}


@app.post("/act", response_model=ActResponse)
def act(req: ActRequest):
    try:
        base64.b64decode(req.image)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"invalid image: {exc}") from exc
    if not req.instruction:
        raise HTTPException(status_code=400, detail="instruction must not be empty")

    # Fixed 7-dof (xyz + rpy + gripper) action -- shape-compatible with what
    # OpenVLA's server.py returns, values are arbitrary.
    return ActResponse(actions=[[0.01, 0.0, -0.02, 0.0, 0.0, 0.0, 1.0]], model="stub")


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
