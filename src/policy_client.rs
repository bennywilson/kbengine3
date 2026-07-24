//! HTTP client for the model-agnostic `/act` policy-server contract (see
//! `policy_server/openvla/server.py`'s module doc for the server side of
//! this same contract). Any model server implementing that contract --
//! OpenVLA today, SmolVLA/Octo/pi0 later -- works here unchanged; only
//! `base_url` and whatever `unnorm_key`/model-specific knobs you pass
//! change per deployment.
//!
//! This module only sends one frame and parses the response -- turning a
//! rendered game-camera frame into `image_png_bytes` (see the cubemap
//! readback-to-PNG path in `renderer.rs` for the shape of that GPU
//! readback) and turning `PolicyResponse::actions` into a MuJoCo qpos step
//! is the caller's job (see `trajectory.rs`'s `RetargetedClip` for the
//! existing joint-vector format this should eventually feed into).

use anyhow::Context;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ActRequest<'a> {
    image: String,
    instruction: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    proprioception: Option<&'a [f64]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unnorm_key: Option<&'a str>,
}

#[derive(Deserialize, Debug)]
pub struct PolicyResponse {
    /// One or more action-chunk steps, each a flat model-specific vector
    /// (OpenVLA: 6-DoF end-effector delta + 1 gripper value). Always a list
    /// even for single-step models, so a future chunk-predicting model
    /// doesn't change this shape -- see the server-side contract doc.
    pub actions: Vec<Vec<f32>>,
    pub model: String,
}

impl PolicyResponse {
    /// The next action to act on. Errors on an empty chunk rather than
    /// panicking, since that's a malformed-but-200 response from the model
    /// server, not a Rust-side bug.
    pub fn first_step(&self) -> anyhow::Result<&[f32]> {
        self.actions
            .first()
            .map(Vec::as_slice)
            .ok_or_else(|| anyhow::anyhow!("policy server returned zero action steps"))
    }
}

/// Sends one rendered frame + task instruction to a running policy server
/// and returns its predicted action(s).
///
/// `base_url` looks like `http://localhost:8000` (no trailing slash, no
/// path). `image_png_bytes` is a single already-encoded frame (PNG/JPEG),
/// base64-encoded here to match the server's `/act` contract.
///
/// On wasm, reaching `localhost` from a hosted page is a real browser-policy
/// obstacle course, not just a CORS problem:
///   - Mixed content isn't actually an issue -- browsers treat `localhost`/
///     `127.0.0.1`/`[::1]` as "potentially trustworthy" regardless of the
///     page's own scheme, so an HTTPS page fetching plain `http://localhost`
///     was never blocked on that basis.
///   - Chrome/Edge 142+ gate it behind Local Network Access instead: the
///     first request to a loopback/private address prompts the user
///     ("...wants to look for and connect to devices on your local
///     network"), and it's a hard fail until they click Allow. This is the
///     same mechanism Plex's web app (app.plex.tv) and Synology/UniFi's
///     hosted consoles already rely on to reach local hardware, so it's a
///     supported long-term pattern, not a loophole about to close.
///   - Safari deviates from the mixed-content spec and blocks this outright
///     over HTTPS with no permission prompt to grant. If the page itself is
///     served over plain HTTP instead, Safari's block doesn't apply.
/// None of this is something `query_policy` can detect or route around --
/// see the `.send()` error context below for what callers actually see when
/// any of these block the request.
pub async fn query_policy(
    base_url: &str,
    image_png_bytes: &[u8],
    instruction: &str,
    proprioception: Option<&[f64]>,
    unnorm_key: Option<&str>,
) -> anyhow::Result<PolicyResponse> {
    let body = ActRequest {
        image: BASE64.encode(image_png_bytes),
        instruction,
        proprioception,
        unnorm_key,
    };
    let url = format!("{base_url}/act");

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // Backed by the browser's own fetch() under wasm-bindgen; no
            // ambient async runtime required, unlike the native branch below.
            //
            // Browsers deliberately collapse every failure mode -- server
            // offline, wrong port, a denied/never-shown Local Network Access
            // prompt, missing CORS headers, Safari's outright block -- into
            // one opaque "TypeError: Failed to fetch", so reqwest can't tell
            // us which one happened. The context below just enumerates the
            // suspects instead of leaving callers with that raw message.
            let resp = reqwest::Client::new().post(&url).json(&body).send().await.with_context(|| {
                format!(
                    "couldn't reach policy server at {url}. Check that it's running, \
                     and if a browser prompt asking to access your local network \
                     appeared, make sure to allow it (Safari blocks this outright \
                     when the page is served over HTTPS)"
                )
            })?;
            if !resp.status().is_success() {
                anyhow::bail!("policy server {url} returned HTTP {}", resp.status());
            }
            let parsed = resp.json::<PolicyResponse>().await?;
        } else {
            // Nothing in this engine runs a tokio runtime (async work is
            // driven ad hoc via pollster::block_on on a spawned OS thread,
            // see example_game.rs's model/texture loads) -- reqwest's async
            // client needs one to poll timers/sockets and would panic
            // reaching for it. The blocking client sidesteps that by
            // spinning its own throwaway runtime per call, which is exactly
            // as expensive as it sounds and fine for a once-per-frame
            // policy query but not a tight inner loop.
            let resp = reqwest::blocking::Client::new()
                .post(&url)
                .json(&body)
                .send()
                .with_context(|| format!("couldn't reach policy server at {url} -- is it running?"))?;
            if !resp.status().is_success() {
                anyhow::bail!("policy server {url} returned HTTP {}", resp.status());
            }
            let parsed = resp.json::<PolicyResponse>()?;
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercises the actual HTTP request/response wiring (JSON shape, base64
    // encoding, CORS-adjacent headers) against `policy_server/stub_server.py`
    // -- a canned-response FastAPI server with no model/GPU dependency, kept
    // separate from OpenVLA's own server.py precisely so this path can be
    // tested without paying for weights + CUDA. Ignored by default since it
    // needs that server already running:
    //   python policy_server/stub_server.py
    //   cargo test -- --ignored
    #[test]
    #[ignore = "requires `python policy_server/stub_server.py` running on localhost:8000"]
    fn act_round_trip_against_stub_server() {
        let resp = pollster::block_on(query_policy(
            "http://localhost:8000",
            b"not a real png, just bytes",
            "pick up the red block",
            None,
            None,
        ))
        .expect("stub server should respond");
        assert_eq!(resp.model, "stub");
        let step = resp.first_step().expect("stub always returns one step");
        assert_eq!(step.len(), 7);
    }
}
