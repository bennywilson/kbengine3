//! Converts a replayed [`crate::trajectory::RetargetedClip`] into OpenVLA
//! fine-tuning tuples: per-frame `(image, instruction, action)` records,
//! where `action` is the `[dx,dy,dz,drx,dry,drz,gripper]` vector
//! [`MujocoScene::apply_policy_action`](crate::mujoco::MujocoScene::apply_policy_action)
//! expects. The caller (see `examples/splat/src/example_game.rs`'s dataset
//! export state machine) supplies the actual body poses/gripper qpos and
//! rendered frame -- this module only holds the pure math and the on-disk
//! record shape, so it stays testable without a live MuJoCo sim or renderer.
//!
//! This is deliberately *not* RLDS/TFDS: that conversion is a separate later
//! step (see the `openvla-policy-finetune-project` memory's 4-step plan) once
//! there's more than one recorded episode to combine.

use serde::Serialize;

/// One `(image, instruction, action)` training tuple, serialized as one line
/// of a `manifest.jsonl` (see [`ManifestWriter`]). `image` is a path relative
/// to the manifest's own directory, not absolute -- so the whole export
/// directory can be moved/copied without invalidating it.
#[derive(Serialize)]
pub struct ActionRecord {
    pub image: String,
    pub instruction: String,
    /// `[dx, dy, dz, drx, dry, drz, gripper]`, see [`action_from_poses`].
    pub action: [f64; 7],
}

/// Builds one frame's action label from that frame's and the next frame's
/// `hand`-body world pose (position + `[w,x,y,z]` quaternion, e.g. from
/// [`MujocoScene::body_world_pose`](crate::mujoco::MujocoScene::body_world_pose))
/// plus that frame's gripper joint reading (see [`gripper_closedness`]).
///
/// `dx,dy,dz` is the plain world-frame translation delta. `drx,dry,drz` is
/// the world-frame rotation delta as an axis-angle vector -- matching
/// `apply_policy_action`'s documented "world-frame extrinsic composition"
/// convention exactly, since that's the function a fine-tune trained on
/// these labels will eventually be driving: a mismatched convention here
/// would silently teach the wrong rotation sense.
pub fn action_from_poses(
    curr: ([f64; 3], [f64; 4]),
    next: ([f64; 3], [f64; 4]),
    gripper: f64,
) -> [f64; 7] {
    let (p0, q0) = curr;
    let (p1, q1) = next;
    let d = axis_angle_delta(q0, q1);
    [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2], d[0], d[1], d[2], gripper]
}

/// Normalizes a finger joint's `qpos` (MJCF range `0..=0.04` on
/// `panda.xml`'s `finger` joint class -- 0 = closed, 0.04 = open per finger)
/// to `apply_policy_action`'s own gripper convention: `0.0` = open, `1.0` =
/// closed. Both finger joints move together (coupled by an equality
/// constraint + shared tendon in `panda.xml`), so reading just one is enough.
pub fn gripper_closedness(finger_qpos: f64, finger_open_qpos: f64) -> f64 {
    (1.0 - finger_qpos / finger_open_qpos).clamp(0.0, 1.0)
}

/// World-frame rotation delta from `q_curr` to `q_next` (both `[w,x,y,z]`
/// unit quaternions) as an axis-angle vector (axis * radians) -- the
/// quaternion log map, computed as `q_next * conjugate(q_curr)`.
///
/// Canonicalizes the delta quaternion to a non-negative scalar part before
/// extracting the angle. Skipping this is a real trap: `q` and `-q` represent
/// the identical rotation, but naively taking `2*atan2(|v|, w)` on whichever
/// sign happened to come out of the Hamilton product picks up the *long* way
/// around (e.g. a genuine 10 deg per-frame step can come back as 350 deg)
/// depending on an arbitrary sign flip in the source data. Forcing `w >= 0`
/// first guarantees the short-way-around angle, which is what a smooth
/// per-frame demo delta should always be.
fn axis_angle_delta(q_curr: [f64; 4], q_next: [f64; 4]) -> [f64; 3] {
    let conj = [q_curr[0], -q_curr[1], -q_curr[2], -q_curr[3]];
    let mut d = quat_mul(q_next, conj);
    let norm = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2] + d[3] * d[3]).sqrt();
    if norm > 0.0 {
        for c in &mut d {
            *c /= norm;
        }
    }
    if d[0] < 0.0 {
        for c in &mut d {
            *c = -*c;
        }
    }
    let (w, v) = (d[0], [d[1], d[2], d[3]]);
    let v_norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if v_norm < 1e-12 {
        return [0.0, 0.0, 0.0];
    }
    let angle = 2.0 * v_norm.atan2(w);
    [v[0] / v_norm * angle, v[1] / v_norm * angle, v[2] / v_norm * angle]
}

/// Hamilton product of two `[w,x,y,z]` quaternions.
fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let (w1, x1, y1, z1) = (a[0], a[1], a[2], a[3]);
    let (w2, x2, y2, z2) = (b[0], b[1], b[2], b[3]);
    [
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
    ]
}

// ---- Disk output -----------------------------------------------------------
//
// Native only: this always runs from the editor's own dataset-export button
// (see example_game.rs), never in a wasm build, same split as
// `trajectory::cache`.
#[cfg(not(target_arch = "wasm32"))]
mod disk {
    use super::ActionRecord;
    use std::io::Write;

    /// Where a clip's exported frames + manifest land:
    /// `resources/openvla_datasets/<clip-file-stem>/`. Mirrors the
    /// `resources/trajectory_cache` convention `trajectory::cache` already
    /// uses for native-only derived output.
    pub fn export_dir_for(clip_path: &str) -> std::path::PathBuf {
        let stem = std::path::Path::new(clip_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("clip");
        std::path::Path::new("resources/openvla_datasets").join(stem)
    }

    /// Appends one JSONL line per [`ActionRecord`] to `<dir>/manifest.jsonl`.
    /// Opened once per export (truncating any previous run for the same
    /// clip) and reused across frames rather than reopening per record.
    pub struct ManifestWriter {
        file: std::io::BufWriter<std::fs::File>,
    }

    impl ManifestWriter {
        pub fn create(dir: &std::path::Path) -> anyhow::Result<Self> {
            std::fs::create_dir_all(dir)?;
            let file = std::fs::File::create(dir.join("manifest.jsonl"))?;
            Ok(Self { file: std::io::BufWriter::new(file) })
        }

        pub fn append(&mut self, record: &ActionRecord) -> anyhow::Result<()> {
            serde_json::to_writer(&mut self.file, record)?;
            self.file.write_all(b"\n")?;
            Ok(())
        }

        pub fn flush(&mut self) -> anyhow::Result<()> {
            self.file.flush()?;
            Ok(())
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use disk::{export_dir_for, ManifestWriter};

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

    fn axis_angle_quat(axis: [f64; 3], angle: f64) -> [f64; 4] {
        let half = angle / 2.0;
        [half.cos(), axis[0] * half.sin(), axis[1] * half.sin(), axis[2] * half.sin()]
    }

    #[test]
    fn zero_delta_between_identical_poses() {
        let pose = ([1.0, 2.0, 3.0], IDENTITY);
        let action = action_from_poses(pose, pose, 0.5);
        assert_eq!(action, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5]);
    }

    #[test]
    fn recovers_a_small_known_rotation() {
        let q_next = axis_angle_quat([0.0, 0.0, 1.0], 10f64.to_radians());
        let d = axis_angle_delta(IDENTITY, q_next);
        assert!((d[2] - 10f64.to_radians()).abs() < 1e-9, "got {:?}", d);
        assert!(d[0].abs() < 1e-9 && d[1].abs() < 1e-9);
    }

    #[test]
    fn sign_flipped_quaternion_still_recovers_the_short_way_around() {
        // q and -q are the same rotation; a naive atan2(|v|, w) without
        // canonicalizing w >= 0 would read this as a ~350 degree turn
        // instead of the genuine 10 degree one.
        let q_next = axis_angle_quat([0.0, 0.0, 1.0], 10f64.to_radians());
        let q_next_flipped = [-q_next[0], -q_next[1], -q_next[2], -q_next[3]];
        let d = axis_angle_delta(IDENTITY, q_next_flipped);
        assert!((d[2] - 10f64.to_radians()).abs() < 1e-9, "got {:?}", d);
    }

    #[test]
    fn gripper_closedness_matches_apply_policy_action_convention() {
        assert_eq!(gripper_closedness(0.04, 0.04), 0.0); // fully open
        assert_eq!(gripper_closedness(0.0, 0.04), 1.0); // fully closed
        assert!((gripper_closedness(0.02, 0.04) - 0.5).abs() < 1e-9);
    }
}
