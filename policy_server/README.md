# Policy server

A model-agnostic HTTP inference server for the splat demo's **Policy Control**
(see `src/policy_client.rs` for the client and the `/act` contract). Each policy
tick, the demo renders its fixed policy camera, POSTs the frame + instruction
here, and drives the MuJoCo arm from the returned action vector.

Two interchangeable servers, both on **port 8000** (the demo's default
`Server URL`), run one at a time:

| Server | Command | Needs | Follows instructions? |
| --- | --- | --- | --- |
| **stub** | `run_server.bat stub` | nothing | No -- returns a fixed action for any prompt |
| **openvla** | `run_server.bat openvla` | NVIDIA GPU + Docker (WSL2 CUDA) | Yes -- real OpenVLA-7b |

```sh
run_server.bat stub       # fast pipeline test, no GPU/Docker
run_server.bat openvla    # real model; first run downloads weights
run_server.bat            # defaults to stub
```

`run_server.bat` frees port 8000 from a leftover python server first (see its
header for why it leaves a Docker-held port alone).

## Why keep the stub

The stub (`stub_server.py`) has no model -- it returns a canned 7-DoF action for
*any* instruction. That's the point: it exercises the whole
capture -> HTTP -> apply -> sim path deterministically, without a GPU or a weights
download, so you can tell a plumbing bug (camera framing, the wasm apply bridge,
CORS, the client) apart from the model simply doing something unhelpful. If the
arm doesn't move under the stub, the problem is in the pipeline, not the policy.

## OpenVLA notes

The real server lives in `openvla/` (Dockerfile + `server.py`, pinned deps in
`requirements.txt`); `docker-compose.yml` wires up GPU passthrough and a named
`hf-cache` volume so weights survive restarts. OpenVLA was trained on real-robot
camera photos, so its output on a synthetic MuJoCo render is out-of-distribution
-- expect the pipeline to work while manipulation quality stays rough without an
in-domain fine-tune.
