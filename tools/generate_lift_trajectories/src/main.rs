//! Generates scripted Lift demonstrations, headlessly, as `TrajectoryClip`
//! JSON that drops straight into the existing pipeline (bind one in the
//! editor's Trajectory field -> Export Training Data -> Rebuild TFDS ->
//! fine-tune, all unchanged).
//!
//! # Why this exists
//!
//! The 200 converted robomimic Lift demos all approach the cube the same
//! way, from near-identical arm start poses. A policy fine-tuned on them
//! learned to react to "this looks like the start of a Lift attempt" rather
//! than to where the cube actually is: in a 20-demo closed-loop eval it
//! closed the gripper on its very first inference call in all 20 runs, ~30cm
//! above the cube, with barely any variation across very different cube
//! positions (see `openvla_policy_finetune_project.md`). Diversity in the
//! *approach*, not just in the object's position, is what the current data
//! can't teach it -- so this randomizes both.
//!
//! # How it works
//!
//! A scripted "oracle" controller that cheats: it reads the cube's true pose
//! straight out of the sim ([`MujocoScene::body_world_pose`]) instead of
//! looking at pixels. That's the point -- it generates *demonstrations* for a
//! vision policy to imitate, and needs no vision itself. Motion goes through
//! the very same [`MujocoScene::apply_policy_action`] the live policy loop
//! drives (Cartesian delta -> damped-least-squares Jacobian IK -> ctrl), so a
//! generated demo can only contain motion the real control path could also
//! produce.
//!
//! Physics runs throughout, unlike training-data *export* (which replays
//! recorded qpos kinematically). Generated demos are therefore internally
//! consistent with contact: the cube rests exactly where the floor actually
//! puts it, rather than at a recorded height physics later disagrees with.
//!
//! Every episode is checked before it's kept -- the cube has to actually end
//! up lifted (`LIFT_SUCCESS_M`). Failures are discarded and retried, so the
//! output is all successful demonstrations however often the scripted
//! controller fumbles a randomized start.
//!
//! # Usage
//!
//! ```text
//! cargo run --release -- --count 200 --out-dir <dir> [--seed 1] [--start-index 0] [--verbose]
//! ```
//!
//! Writes `<out-dir>/demo_<N>.json`. Point it at a *fresh* directory rather
//! than the converted robomimic clips' own, so the two sets stay
//! distinguishable (they are not interchangeable: these have a different
//! approach distribution by design).

use anyhow::{Context, Result};
use black_splat::mujoco::MujocoScene;
use black_splat::trajectory::{JointTrack, TrajectoryClip};

// ---- Model-specific names (panda_robot.xml / panda_lift.xml). A different
// MJCF needs different values here, same caveat as apply_policy_action's own
// docs.

const EE_BODY: &str = "hand";
const OBJECT_BODY: &str = "cube_main";
const ARM_ACTUATORS: [&str; 7] = [
    "actuator1",
    "actuator2",
    "actuator3",
    "actuator4",
    "actuator5",
    "actuator6",
    "actuator7",
];
const GRIPPER_ACTUATOR: &str = "actuator8";
const ARM_JOINTS: [&str; 7] = [
    "joint1", "joint2", "joint3", "joint4", "joint5", "joint6", "joint7",
];
const FINGER_JOINTS: [&str; 2] = ["finger_joint1", "finger_joint2"];
const CUBE_JOINT: &str = "cube_joint0";

/// Starting arm pose: the per-joint median of the converted robomimic demos'
/// own frame 0, measured across them rather than taken from panda_robot.xml's
/// `home` keyframe.
///
/// The keyframe looks like the obvious choice and is wrong for this purpose.
/// Its joint7 is -0.785 where the demos sit at +0.777, a **90 degree wrist
/// roll**, and since control here is translation plus orientation-*hold* (see
/// `drive_to`), whatever roll the episode starts with is the roll every
/// recorded frame has. Generating from the keyframe therefore produced a
/// dataset whose gripper is visibly rotated a quarter turn from every
/// recorded demo -- and since the closed-loop eval resets to a *recorded*
/// demo's pose, that is a train/deploy mismatch of precisely the kind this
/// whole dataset exists to remove.
///
/// Set explicitly either way, since `reset()` restores `qpos0` (arm straight
/// out) rather than any keyframe.
const HOME_ARM_QPOS: [f64; 7] = [-0.002, 0.193, 0.007, -2.616, 0.003, 2.946, 0.777];
/// Per-finger open width, the `finger` joint class's upper limit in panda.xml.
const FINGER_OPEN_QPOS: f64 = 0.04;

// ---- Timing.

/// Seconds per control tick. Matches both the recorded demos' own `dt` and
/// the editor's default `policy_interval_secs`, so a generated clip plays
/// back (and strides down for training) exactly like a converted one.
const CONTROL_DT: f32 = 0.05;
/// Physics steps per control tick. panda_robot.xml's `<option>` sets no
/// `timestep`, so MuJoCo's 0.002s default applies: 0.05 / 0.002 = 25. Must be
/// updated together with an explicit timestep if one is ever added there.
const PHYSICS_STEPS_PER_CONTROL: usize = 25;
/// Physics steps to let the dropped cube come to rest (and the arm settle
/// onto its held pose) before anything else runs. 0.4s, comfortably longer
/// than the fall from `CUBE_SPAWN_Z`.
const SETTLE_STEPS: usize = 200;

// ---- Scripted controller shape.

/// Largest Cartesian step the controller will command per control tick.
/// Note this is the *commanded* step: the position servos lag, so measured
/// end-effector motion comes out around a third of it -- which lands
/// per-frame translation in the same ~2-3mm range as the converted demos,
/// rather than teaching a much faster motion scale.
const MAX_STEP_M: f64 = 0.008;
/// Largest wrist correction commanded per control tick, radians. Same
/// clamping rationale as `MAX_STEP_M`: enough to hold orientation against
/// IK drift without snapping the wrist around in a single tick.
const MAX_ROT_STEP_RAD: f64 = 0.03;
/// Arrival tolerance while flying to the hover pose, where precision doesn't
/// matter yet. Deliberately loose: the commanded step shrinks as the target
/// nears, and with servo lag a tight tolerance costs many ticks to shave off
/// millimetres that the descent re-targets anyway.
const HOVER_TOL_M: f64 = 0.015;
/// Arrival tolerance for the descent, where it does matter. The cube is
/// ~4.2cm across and the fingers open 4cm per side, so this leaves real room
/// while still landing the fingers around it rather than beside it.
const GRASP_TOL_M: f64 = 0.008;
/// How far a stalled leg may still be from its target and count as good
/// enough. The servos lag, so legs routinely stop a few millimetres out (see
/// `Leg::Settled`); these are the real accuracy requirements, while the
/// `*_TOL_M` values above are just where the controller stops trying.
/// Deliberately loose: the align leg re-fixes lateral position at hover
/// height and the descent re-fixes height, so where the hover flight
/// actually stops barely matters -- it only has to be roughly above the cube
/// with the gripper clear. A strict value here rejected over half of all
/// episodes for no benefit.
const HOVER_ACCEPT_M: f64 = 0.080;
const GRASP_ACCEPT_M: f64 = 0.012;
/// Align is the *tightest* leg, not the loosest. It runs at hover height
/// where lateral motion is free of the cube, and whatever error it leaves
/// is error the near-vertical descent converges on poorly and then hands
/// straight to the grasp. The cube is only 4.2cm across, so ~2cm of residual
/// lateral offset puts it at the very edge of the fingers and they shut past
/// it -- which is exactly what a looser value here produced. Episodes that
/// can't make this bar are discarded instead.
/// 0.009 rather than tighter because that is the servos' achievable floor
/// here: align reliably settles at 0.0068-0.0077 and cannot do better, so a
/// stricter bar rejects everything. 7mm still leaves the cube comfortably
/// inside the ~8cm finger span.
const ALIGN_ACCEPT_M: f64 = 0.009;
/// Align gets its own, larger budget for the same reason: it's cheap (no
/// contact risk at hover height) and it's the leg whose accuracy decides
/// whether the grasp can work at all.
const ALIGN_TICK_LIMIT: usize = 500;
/// Height of the hand body's origin above the cube's centre at grasp time
/// (`--grasp-offset` overrides it). Read straight out of panda_robot.xml
/// rather than guessed: `left_finger` sits at hand-local z 0.0584 and the
/// grasp pad (`fingertip_pad_collision_1`) at a further 0.0445, so the
/// contact surface is 0.103 below the hand origin. The pad's own extent
/// makes anything from ~0.073 to ~0.132 geometrically viable against this
/// cube; 0.103 centres it.
const GRASP_OFFSET_Z: f64 = 0.103;
/// How far above the grasp point the lift phase pulls to. Comfortably clear
/// of `LIFT_SUCCESS_M` while not spending 37 frames per episode on a rise
/// nothing needs to see.
const LIFT_HEIGHT_M: f64 = 0.07;

/// Where a `--recovery` episode starts recording, as an offset from the grasp
/// point. Drawn to cover the state a served policy actually gets stuck in:
/// the closed-loop eval parks the hand 2-5cm short along -x with y and z
/// essentially correct, then closes on nothing. A clean approach never labels
/// that state -- it only ever sees "short in x" together with "still high in
/// z", so the model can't tell being short apart from still descending --
/// which is the gap these episodes exist to fill.
///
/// The z range sits at or *above* the grasp point rather than straddling it:
/// the recorded correction is then a diagonal move down-and-forward with the
/// open fingers clearing the top of the cube, instead of a lateral sweep at
/// grasp height, which is the move this generator already found knocks the
/// cube off its resting spot (see the align leg's own comment).
const RECOVERY_X_RANGE: (f64, f64) = (-0.060, -0.020);
const RECOVERY_Y_RANGE: (f64, f64) = (-0.015, 0.015);
const RECOVERY_Z_RANGE: (f64, f64) = (0.005, 0.045);
/// Height above the grasp point the recorded align leg re-centres at in a
/// `--recovery` episode, instead of whatever height the hand happens to be
/// at. A recovery start can sit low enough that the fingertips are level with
/// the cube, where sliding sideways into position would shove it -- so the
/// correction rises as it advances, which is also the only recovery that
/// physically works from there.
const RECOVERY_ALIGN_CLEARANCE_M: f64 = 0.045;
/// Recorded-frame cap for a `--recovery` episode. Without this, a slow align
/// (up to `ALIGN_TICK_LIMIT` ticks on its own) can produce an episode several
/// times longer than anything in the non-recovery set -- measured directly:
/// an eval against the first 10-demo recovery batch found the model
/// confidently predicting a sustained *wrong-direction* dx for ~40 straight
/// steps in its one 137-step episode (548 raw frames), right as the episode
/// ran past the length any training data had ever covered. Longer episodes
/// aren't teaching recovery at that point, they're teaching an episode length
/// the model has to extrapolate blindly past. Rejecting the outliers keeps
/// the recovery set's length distribution close to the existing one instead
/// of introducing a second, unrelated distribution shift alongside the one
/// these episodes exist to fix.
const RECOVERY_MAX_RECORDED_FRAMES: usize = 260;
/// Control ticks spent holding still with the gripper closing, before
/// lifting. `apply_policy_action` smooths the gripper channel (see
/// `smooth_gripper_t`), so a close command needs several ticks to fully take
/// -- lifting sooner would drag the cube out of a half-shut hand.
const CLOSE_TICKS: usize = 12;
/// Tick budget for the unrecorded flight to the hover pose. Generous because
/// it crosses the whole workspace (the robot base sits ~14cm above the floor
/// plane, so the hand starts well over half a metre from the cube) and none
/// of it ends up in the output.
const HOVER_TICK_LIMIT: usize = 600;
/// Tick budget for each recorded phase, so a randomized start the controller
/// can't actually service fails the episode instead of looping forever.
const PHASE_TICK_LIMIT: usize = 300;
/// How far the cube may drift during the descent before the episode is
/// written off. Catches the arm clipping the cube on the way down: without
/// this the grasp closes on empty space and the lift check fails anyway,
/// just after wasting the remaining ticks.
const CUBE_DISTURBED_M: f64 = 0.02;

// ---- Randomization ranges. This is what the generated set has that the
// converted robomimic one doesn't: the cube moves *and* the approach does.

/// Cube spawn x/y, matching the converted demos' own measured spread (see
/// panda_lift.xml's floor comment: x 0.51..0.60, y -0.04..0.05).
const CUBE_X_RANGE: (f64, f64) = (0.51, 0.60);
const CUBE_Y_RANGE: (f64, f64) = (-0.04, 0.05);
/// Spawn height. panda_lift.xml's own default for `cube_main`; the cube
/// simply falls to rest from here during `SETTLE_STEPS`, which avoids
/// hardcoding the floor height and half-extent (and so stays correct if
/// either changes).
const CUBE_SPAWN_Z: f64 = -0.06;
/// Lateral offset of the hover point from directly above the cube, so the
/// descent isn't always perfectly vertical from the same relative spot.
const APPROACH_OFFSET_RANGE: (f64, f64) = (-0.03, 0.03);
/// Hover height above the cube -- also, since recording starts there, how far
/// the recorded episode descends. Floor is well above `GRASP_OFFSET_Z` plus
/// the fingertip reach on purpose: the align leg sweeps laterally at this
/// height, so anything lower drags the open fingers through the cube before
/// the descent even starts.
/// Sized to roughly match how far above the cube the converted demos start
/// (they complete a whole lift in ~48 frames at 2-3mm per frame, so on the
/// order of 7cm of approach), rather than the much higher hover an earlier
/// pass used. Still leaves ~2.6cm of fingertip clearance over the cube's top
/// face at the low end, which is what the align leg needs to sweep laterally
/// without dragging it.
const HOVER_HEIGHT_RANGE: (f64, f64) = (0.15, 0.21);
/// Uniform noise added to each arm joint's home angle at episode start, in
/// radians.
///
/// Small on purpose. Control here is translation-only (see `drive_to`), so
/// whatever wrist tilt this introduces persists for the whole episode -- and
/// the grasp pads sit ~0.103m along the hand's own axis (panda_robot.xml:
/// `left_finger` at 0.0584 plus the pad at 0.0445), so even 15 degrees of
/// tilt swings them ~2.6cm sideways. Alignment is measured at the hand
/// origin, so that displacement is invisible to it and the fingers shut
/// beside the cube while every logged number looks correct. At 0.06 rad
/// across seven joints that was enough to defeat every grasp.
///
/// Approach variety comes from the randomized hover pose instead, which
/// varies arm configuration without tilting the gripper off vertical.
const ARM_NOISE_RAD: f64 = 0.015;

// ---- Success criterion.

/// Minimum `finger_joint1` qpos after closing for the grasp to count as
/// holding something. The fingers travel to ~0 when they meet no resistance,
/// so anything meaningfully above that means the cube is between them.
const GRIPPED_FINGER_QPOS_M: f64 = 0.004;
/// How far the cube must rise above its settled start height to count as
/// lifted. Same shape of check as the editor's own closed-loop eval.
const LIFT_SUCCESS_M: f64 = 0.05;

/// Seeded xorshift64*. Hand-rolled rather than pulling `rand` in: this needs
/// nothing beyond a uniform f64, and a seed that reproduces a dataset exactly
/// is worth more here than distribution quality (these are start poses, not
/// statistics).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Any nonzero state works; xorshift is stuck at zero.
        Rng(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        // Top 53 bits -> f64 mantissa, the standard construction.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
}

/// Hamilton product of two `[w, x, y, z]` quaternions.
fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let (aw, ax, ay, az) = (a[0], a[1], a[2], a[3]);
    let (bw, bx, by, bz) = (b[0], b[1], b[2], b[3]);
    [
        aw * bw - ax * bx - ay * by - az * bz,
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
    ]
}

fn quat_conj(q: [f64; 4]) -> [f64; 4] {
    [q[0], -q[1], -q[2], -q[3]]
}

/// Quaternion log map to a rotation vector (axis * radians), canonicalized to
/// w >= 0 so a small rotation never comes back as its ~2pi complement. Same
/// convention `apply_policy_action` expects for `action[3..6]`, and the same
/// one `policy_dataset` uses when labelling.
fn quat_to_axis_angle(q: [f64; 4]) -> [f64; 3] {
    let mut q = q;
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n > 1e-12 {
        q = [q[0] / n, q[1] / n, q[2] / n, q[3] / n];
    }
    if q[0] < 0.0 {
        q = [-q[0], -q[1], -q[2], -q[3]];
    }
    let vec_norm = (q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if vec_norm < 1e-12 {
        return [0.0, 0.0, 0.0];
    }
    let angle = 2.0 * vec_norm.atan2(q[0]);
    [
        q[1] / vec_norm * angle,
        q[2] / vec_norm * angle,
        q[3] / vec_norm * angle,
    ]
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm([a[0] - b[0], a[1] - b[1], a[2] - b[2]])
}

/// Records every joint's `qpos`, in `joint_tracks()` order -- exactly the
/// layout a `TrajectoryClip` frame wants.
fn record_frame(scene: &MujocoScene, joints: &[JointTrack]) -> Vec<f64> {
    let mut frame = Vec::new();
    for jt in joints {
        match scene.joint_qpos_slice(&jt.name) {
            Some(values) => frame.extend(values),
            // Can't happen: `joints` came from this same model. Pad rather
            // than panic so one odd joint can't kill a long run.
            None => frame.extend(std::iter::repeat(0.0).take(jt.dofs)),
        }
    }
    frame
}

/// Outcome of one `drive_to` leg.
#[derive(PartialEq, Debug)]
enum Leg {
    /// Stopped moving toward the target, carrying the distance still left.
    /// Deliberately not a pass/fail verdict: the position servos lag the
    /// commanded step, and since the step shrinks as the target nears, a leg
    /// routinely stalls a few millimetres short instead of ever satisfying a
    /// tight tolerance. Callers decide what's close enough for their phase
    /// (a hover can be sloppy, a grasp can't).
    Settled(f64),
    /// Physics went non-finite (a randomized start that drove the arm into
    /// itself or the table hard enough). Nothing recoverable to record.
    Diverged,
}

/// Ticks without measurable progress before `drive_to` calls a leg done.
///
/// Deliberately small. Every one of these is a recorded frame of the arm
/// sitting essentially still, and with three legs per episode a generous
/// value was contributing ~75 of ~190 frames -- 39% of the dataset spent
/// teaching the policy to hold position at the end of a move. Trimming it
/// costs no actual motion, just the dead tail of each leg.
const STALL_TICKS: usize = 8;
/// Distance improvement below this counts as no progress.
const STALL_EPSILON_M: f64 = 1e-4;

/// Drives the end effector toward `target` one clamped Cartesian step per
/// control tick, stepping physics in between, until it arrives or runs out
/// of ticks.
///
/// `target` is a fixed world point rather than something re-read per tick:
/// an earlier version aimed at the cube's *live* pose, so the moment the arm
/// clipped the cube the target chased it across the table and dragged the
/// arm along with it. Anything that needs to track a moving object should
/// re-enter this with a fresh target instead.
///
/// Appends to `frames` only when `record` -- the flight to the hover pose is
/// setup, not demonstration, and would otherwise put a big approach from
/// halfway across the workspace at the front of every clip.
#[allow(clippy::too_many_arguments)]
fn drive_to(
    scene: &mut MujocoScene,
    joints: &[JointTrack],
    frames: &mut Vec<Vec<f64>>,
    record: bool,
    target: [f64; 3],
    hold_quat: [f64; 4],
    gripper: f32,
    tol: f64,
    max_ticks: usize,
) -> Result<Leg> {
    let arm_actuators: Vec<&str> = ARM_ACTUATORS.to_vec();
    let mut best_dist = f64::INFINITY;
    let mut stalled_for = 0usize;
    for _ in 0..max_ticks {
        let (ee_pos, _) = scene
            .body_world_pose(EE_BODY)
            .with_context(|| format!("no body named '{EE_BODY}'"))?;
        let to_target = [
            target[0] - ee_pos[0],
            target[1] - ee_pos[1],
            target[2] - ee_pos[2],
        ];
        let dist = norm(to_target);
        if dist < tol {
            return Ok(Leg::Settled(dist));
        }
        if dist < best_dist - STALL_EPSILON_M {
            best_dist = dist;
            stalled_for = 0;
        } else {
            stalled_for += 1;
            if stalled_for >= STALL_TICKS {
                return Ok(Leg::Settled(dist));
            }
        }

        let scale = dist.min(MAX_STEP_M) / dist.max(1e-9);
        // Actively steer the wrist back to `hold_quat` instead of commanding a
        // zero rotation delta. Zero does NOT mean "hold": the damped
        // least-squares IK only approximately satisfies the rotation rows, so
        // a few hundred ticks of translation-only commands let the gripper
        // drift off vertical. The grasp pads sit ~0.103m out along the hand's
        // own axis, so a slow tilt swings them centimetres sideways -- enough
        // to clip the cube on the way down and shut beside it, while the
        // hand-origin alignment this loop measures still reads as perfect.
        let (_, ee_quat) = scene
            .body_world_pose(EE_BODY)
            .with_context(|| format!("no body named '{EE_BODY}'"))?;
        let rot = quat_to_axis_angle(quat_mul(hold_quat, quat_conj(ee_quat)));
        let rot_mag = norm(rot);
        let rot_scale = rot_mag.min(MAX_ROT_STEP_RAD) / rot_mag.max(1e-9);
        let action = [
            (to_target[0] * scale) as f32,
            (to_target[1] * scale) as f32,
            (to_target[2] * scale) as f32,
            (rot[0] * rot_scale) as f32,
            (rot[1] * rot_scale) as f32,
            (rot[2] * rot_scale) as f32,
            gripper,
        ];
        scene.apply_policy_action(&action, EE_BODY, &arm_actuators, GRIPPER_ACTUATOR)?;
        for _ in 0..PHYSICS_STEPS_PER_CONTROL {
            scene.step_once();
        }

        let frame = record_frame(scene, joints);
        if frame.iter().any(|v| !v.is_finite()) {
            return Ok(Leg::Diverged);
        }
        if record {
            frames.push(frame);
        }
    }
    let (ee_pos, _) = scene
        .body_world_pose(EE_BODY)
        .with_context(|| format!("no body named '{EE_BODY}'"))?;
    Ok(Leg::Settled(distance(ee_pos, target)))
}

/// Holds position while the gripper closes.
///
/// `target` must be where the hand *is* as the close begins, not the grasp
/// point aimed at earlier: the cube shifts a few millimetres during the
/// descent, so a target derived from its pre-descent pose steers the hand off
/// the cube by exactly that much while the fingers are shutting. Measured
/// doing it: alignment was [+0.000,+0.001] entering the close and
/// [+0.013,+0.002] leaving it, and the fingers shut past the cube.
///
/// Actively drives back toward `target` rather than commanding a zero delta:
/// a zero delta makes `apply_policy_action` set each actuator's `ctrl` to the
/// joint's *current* qpos, which re-anchors the hold to wherever the arm has
/// already sagged to and ratchets that drift in a little more every tick. Over
/// the dozen ticks a close takes that was walking the hand 1-2cm off the cube
/// -- far enough that the fingers shut on empty air next to it.
fn hold_and_close(
    scene: &mut MujocoScene,
    joints: &[JointTrack],
    frames: &mut Vec<Vec<f64>>,
    target: [f64; 3],
    hold_quat: [f64; 4],
    ticks: usize,
) -> Result<()> {
    let arm_actuators: Vec<&str> = ARM_ACTUATORS.to_vec();
    for _ in 0..ticks {
        let (ee_pos, _) = scene
            .body_world_pose(EE_BODY)
            .with_context(|| format!("no body named '{EE_BODY}'"))?;
        let to_target = [
            target[0] - ee_pos[0],
            target[1] - ee_pos[1],
            target[2] - ee_pos[2],
        ];
        let dist = norm(to_target);
        let scale = dist.min(MAX_STEP_M) / dist.max(1e-9);
        let (_, ee_quat) = scene
            .body_world_pose(EE_BODY)
            .with_context(|| format!("no body named '{EE_BODY}'"))?;
        let rot = quat_to_axis_angle(quat_mul(hold_quat, quat_conj(ee_quat)));
        let rot_mag = norm(rot);
        let rot_scale = rot_mag.min(MAX_ROT_STEP_RAD) / rot_mag.max(1e-9);
        let action = [
            (to_target[0] * scale) as f32,
            (to_target[1] * scale) as f32,
            (to_target[2] * scale) as f32,
            (rot[0] * rot_scale) as f32,
            (rot[1] * rot_scale) as f32,
            (rot[2] * rot_scale) as f32,
            1.0,
        ];
        scene.apply_policy_action(&action, EE_BODY, &arm_actuators, GRIPPER_ACTUATOR)?;
        for _ in 0..PHYSICS_STEPS_PER_CONTROL {
            scene.step_once();
        }
        frames.push(record_frame(scene, joints));
    }
    Ok(())
}

/// Runs one randomized episode. `Ok(None)` means the controller ran but
/// didn't get the cube off the table -- an expected outcome for some random
/// starts, and the caller just retries with the next seed draw.
fn generate_episode(
    xml_path: &str,
    rng: &mut Rng,
    grasp_offset: f64,
    recovery: bool,
    verbose: bool,
) -> Result<Option<TrajectoryClip>> {
    // A fresh scene per episode, not one reset in place: `reset()` clears
    // MuJoCo's own state but not `apply_policy_action`'s gripper smoothing,
    // so a reused scene would start each episode carrying the previous one's
    // closed-gripper state and visibly re-open during the first recorded
    // frames.
    let mut scene = MujocoScene::from_xml_path(xml_path)
        .with_context(|| format!("failed to load {xml_path}"))?;
    let joints = scene.joint_tracks();

    // ---- Randomized start state.
    for (i, name) in ARM_JOINTS.iter().enumerate() {
        let noise = rng.range(-ARM_NOISE_RAD, ARM_NOISE_RAD);
        scene.set_joint_qpos(name, &[HOME_ARM_QPOS[i] + noise]);
    }
    for name in FINGER_JOINTS {
        scene.set_joint_qpos(name, &[FINGER_OPEN_QPOS]);
    }
    let cube_x = rng.range(CUBE_X_RANGE.0, CUBE_X_RANGE.1);
    let cube_y = rng.range(CUBE_Y_RANGE.0, CUBE_Y_RANGE.1);
    // Free joint: [x, y, z, qw, qx, qy, qz], spawned upright.
    scene.set_joint_qpos(
        CUBE_JOINT,
        &[cube_x, cube_y, CUBE_SPAWN_Z, 1.0, 0.0, 0.0, 0.0],
    );

    // Freeze the position servos at the pose just written, or they'd drag the
    // arm back toward ctrl's post-reset zeros the moment physics runs (same
    // fix, and same reason, as the editor's "Reset to Trajectory Start").
    scene.hold_current_pose(&ARM_ACTUATORS.to_vec())?;
    for _ in 0..SETTLE_STEPS {
        scene.step_once();
    }

    let (cube_settled, _) = scene
        .body_world_pose(OBJECT_BODY)
        .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
    // The settled home orientation: gripper already pointing down at the
    // table. Every leg steers back to this, so the wrist can't drift over the
    // course of an episode (see drive_to).
    let (_, hold_quat) = scene
        .body_world_pose(EE_BODY)
        .with_context(|| format!("no body named '{EE_BODY}'"))?;

    // ---- Fly to a randomized hover pose. Unrecorded: recording starts once
    // the arm is already near the cube, which is where the converted demos
    // start too (they run 38-60 frames, not the several hundred it takes to
    // cross the workspace from the home pose).
    let hover = [
        cube_settled[0] + rng.range(APPROACH_OFFSET_RANGE.0, APPROACH_OFFSET_RANGE.1),
        cube_settled[1] + rng.range(APPROACH_OFFSET_RANGE.0, APPROACH_OFFSET_RANGE.1),
        cube_settled[2] + rng.range(HOVER_HEIGHT_RANGE.0, HOVER_HEIGHT_RANGE.1),
    ];
    let mut frames = Vec::new();
    let leg = drive_to(
        &mut scene,
        &joints,
        &mut frames,
        false,
        hover,
        hold_quat,
        0.0,
        HOVER_TOL_M,
        HOVER_TICK_LIMIT,
    )?;
    match leg {
        Leg::Settled(d) if d <= HOVER_ACCEPT_M => {}
        other => {
            if verbose {
                eprintln!("  hover: {other:?}");
            }
            return Ok(None);
        }
    }

    // ---- Recovery episodes detour into the error state first, unrecorded,
    // so the recorded frames start already wrong and every label is a
    // correction. Two legs rather than one diagonal: lateral at hover height,
    // then straight down, for the same reason the align leg below is split
    // out -- a diagonal into the grasp region sweeps the open fingers through
    // the cube.
    if recovery {
        let (cube_now, _) = scene
            .body_world_pose(OBJECT_BODY)
            .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
        let start = [
            cube_now[0] + rng.range(RECOVERY_X_RANGE.0, RECOVERY_X_RANGE.1),
            cube_now[1] + rng.range(RECOVERY_Y_RANGE.0, RECOVERY_Y_RANGE.1),
            cube_now[2] + grasp_offset + rng.range(RECOVERY_Z_RANGE.0, RECOVERY_Z_RANGE.1),
        ];
        let (here, _) = scene
            .body_world_pose(EE_BODY)
            .with_context(|| format!("no body named '{EE_BODY}'"))?;
        for target in [[start[0], start[1], here[2]], start] {
            let leg = drive_to(
                &mut scene,
                &joints,
                &mut frames,
                false,
                target,
                hold_quat,
                0.0,
                HOVER_TOL_M,
                PHASE_TICK_LIMIT,
            )?;
            match leg {
                Leg::Settled(d) if d <= HOVER_ACCEPT_M => {}
                other => {
                    if verbose {
                        eprintln!("  recovery-start: {other:?}");
                    }
                    return Ok(None);
                }
            }
        }
        // Getting into position must not itself move the cube, or the demo
        // teaches recovery from a scene the policy will never see. Cheaper to
        // discard the episode than to record a corrupted one.
        let (cube_after_detour, _) = scene
            .body_world_pose(OBJECT_BODY)
            .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
        if distance(cube_settled, cube_after_detour) > CUBE_DISTURBED_M {
            if verbose {
                eprintln!(
                    "  recovery-start disturbed the cube by {:.3} m",
                    distance(cube_settled, cube_after_detour)
                );
            }
            return Ok(None);
        }
    }

    // ---- Recorded from here.
    frames.push(record_frame(&scene, &joints));
    if verbose {
        let (ee, _) = scene
            .body_world_pose(EE_BODY)
            .unwrap_or(([0.0; 3], [0.0; 4]));
        let finger = scene
            .body_world_pose("left_finger")
            .map(|(p, _)| p[2])
            .unwrap_or(f64::NAN);
        let (cube, _) = scene
            .body_world_pose(OBJECT_BODY)
            .unwrap_or(([0.0; 3], [0.0; 4]));
        eprintln!(
            "  first recorded frame: hand-cube [{:+.3},{:+.3},{:+.3}]  left_finger z {:+.4}  (hand-finger {:+.4})",
            ee[0] - cube[0],
            ee[1] - cube[1],
            ee[2] - cube[2],
            finger,
            ee[2] - finger
        );
    }
    let (cube_before, _) = scene
        .body_world_pose(OBJECT_BODY)
        .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
    // Align laterally *at hover height* before dropping. A single diagonal
    // move to the grasp point sweeps the open fingers sideways through the
    // cube on the way in (the hover point is deliberately offset up to 5cm
    // laterally), which knocks it off its resting spot before the gripper
    // ever closes -- measured as the dominant failure mode when this was one
    // combined leg.
    let (hover_now, _) = scene
        .body_world_pose(EE_BODY)
        .with_context(|| format!("no body named '{EE_BODY}'"))?;
    let align_z = if recovery {
        (cube_before[2] + grasp_offset + RECOVERY_ALIGN_CLEARANCE_M).max(hover_now[2])
    } else {
        hover_now[2]
    };
    let align_target = [cube_before[0], cube_before[1], align_z];
    let leg = drive_to(
        &mut scene,
        &joints,
        &mut frames,
        true,
        align_target,
        hold_quat,
        0.0,
        ALIGN_ACCEPT_M,
        ALIGN_TICK_LIMIT,
    )?;
    match leg {
        Leg::Settled(d) if d <= ALIGN_ACCEPT_M => {}
        other => {
            if verbose {
                eprintln!("  align: {other:?}");
            }
            return Ok(None);
        }
    }

    // Re-read after align: the disturbance check below should attribute damage
    // to the leg that caused it, and aligning laterally is its own chance to
    // clip the cube.
    let (cube_before, _) = scene
        .body_world_pose(OBJECT_BODY)
        .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
    let grasp_target = [
        cube_before[0],
        cube_before[1],
        cube_before[2] + grasp_offset,
    ];
    let leg = drive_to(
        &mut scene,
        &joints,
        &mut frames,
        true,
        grasp_target,
        hold_quat,
        0.0,
        GRASP_TOL_M,
        PHASE_TICK_LIMIT,
    )?;
    match leg {
        Leg::Settled(d) if d <= GRASP_ACCEPT_M => {}
        other => {
            if verbose {
                eprintln!("  descend: {other:?}");
            }
            return Ok(None);
        }
    }

    // Clipped the cube on the way down -- the grasp would close on empty
    // space, so stop here rather than burning the rest of the budget.
    let (cube_after, _) = scene
        .body_world_pose(OBJECT_BODY)
        .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
    if distance(cube_before, cube_after) > CUBE_DISTURBED_M {
        if verbose {
            let (ee, _) = scene
                .body_world_pose(EE_BODY)
                .unwrap_or(([0.0; 3], [0.0; 4]));
            eprintln!(
                "  descend disturbed the cube by {:.3} m -- hand-cube dx {:+.3} dy {:+.3} dz {:+.3}",
                distance(cube_before, cube_after),
                ee[0] - cube_before[0],
                ee[1] - cube_before[1],
                ee[2] - cube_before[2]
            );
        }
        return Ok(None);
    }

    if verbose {
        let (ee, _) = scene
            .body_world_pose(EE_BODY)
            .unwrap_or(([0.0; 3], [0.0; 4]));
        let (cube_now, _) = scene
            .body_world_pose(OBJECT_BODY)
            .unwrap_or(([0.0; 3], [0.0; 4]));
        eprintln!(
            "  before close: hand-cube [{:+.3},{:+.3},{:+.3}]  cube moved {:.4} m during descend",
            ee[0] - cube_now[0],
            ee[1] - cube_now[1],
            ee[2] - cube_now[2],
            distance(cube_before, cube_now)
        );
    }

    // Hold where the hand actually ended up, not where the descent was aimed
    // (see hold_and_close's own docs).
    let (hold_pos, _) = scene
        .body_world_pose(EE_BODY)
        .with_context(|| format!("no body named '{EE_BODY}'"))?;
    hold_and_close(
        &mut scene,
        &joints,
        &mut frames,
        hold_pos,
        hold_quat,
        CLOSE_TICKS,
    )?;

    if verbose {
        let (ee, _) = scene
            .body_world_pose(EE_BODY)
            .unwrap_or(([0.0; 3], [0.0; 4]));
        let (cube, _) = scene
            .body_world_pose(OBJECT_BODY)
            .unwrap_or(([0.0; 3], [0.0; 4]));
        // The decisive one: a finger that stops partway is being *blocked by
        // the cube* (a real grip); one that reaches ~0 shut on empty air.
        let finger = scene.joint_qpos("finger_joint1").unwrap_or(f64::NAN);
        eprintln!(
            "  after close: hand-cube [{:+.3},{:+.3},{:+.3}]  finger_joint1 {finger:.4} ({})",
            ee[0] - cube[0],
            ee[1] - cube[1],
            ee[2] - cube[2],
            if finger > 0.004 {
                "BLOCKED = gripping something"
            } else {
                "shut on nothing"
            }
        );
    }

    // A finger stopped partway is blocked by the cube (a real grip); one that
    // reached ~0 shut on empty air, so the lift can only fail. Bail now rather
    // than recording a doomed episode.
    let finger_qpos = scene.joint_qpos("finger_joint1").unwrap_or(0.0);
    if finger_qpos < GRIPPED_FINGER_QPOS_M {
        if verbose {
            eprintln!("  grip: fingers shut to {finger_qpos:.4}, nothing between them");
        }
        return Ok(None);
    }

    let (grasp_pos, _) = scene
        .body_world_pose(EE_BODY)
        .with_context(|| format!("no body named '{EE_BODY}'"))?;
    let lift_target = [grasp_pos[0], grasp_pos[1], grasp_pos[2] + LIFT_HEIGHT_M];
    let leg = drive_to(
        &mut scene,
        &joints,
        &mut frames,
        true,
        lift_target,
        hold_quat,
        1.0,
        GRASP_TOL_M,
        PHASE_TICK_LIMIT,
    )?;
    if leg == Leg::Diverged {
        if verbose {
            eprintln!("  lift: {leg:?}");
        }
        return Ok(None);
    }

    let (cube_end, _) = scene
        .body_world_pose(OBJECT_BODY)
        .with_context(|| format!("no body named '{OBJECT_BODY}'"))?;
    let rise = cube_end[2] - cube_settled[2];
    if verbose {
        eprintln!("  rise {rise:+.4} m over {} frames", frames.len());
    }
    if rise < LIFT_SUCCESS_M {
        return Ok(None);
    }
    if recovery && frames.len() > RECOVERY_MAX_RECORDED_FRAMES {
        if verbose {
            eprintln!(
                "  recovery episode ran {} frames, over the {RECOVERY_MAX_RECORDED_FRAMES}-frame cap -- discarded",
                frames.len()
            );
        }
        return Ok(None);
    }

    Ok(Some(TrajectoryClip {
        joints,
        dt: CONTROL_DT,
        frames,
    }))
}

struct Args {
    count: usize,
    out_dir: String,
    xml_path: String,
    seed: u64,
    start_index: usize,
    grasp_offset: f64,
    recovery: bool,
    verbose: bool,
    probe_gripper: bool,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        count: 200,
        out_dir: String::new(),
        xml_path: "examples/splat/game_assets/mujoco/franka_emika_panda/panda_lift.xml".to_string(),
        seed: 1,
        start_index: 0,
        grasp_offset: GRASP_OFFSET_Z,
        recovery: false,
        verbose: false,
        probe_gripper: false,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        // Flag with no value: handled before the pair-wise advance below.
        if argv[i] == "--verbose" {
            args.verbose = true;
            i += 1;
            continue;
        }
        if argv[i] == "--probe-gripper" {
            args.probe_gripper = true;
            i += 1;
            continue;
        }
        // Every episode starts already in the error state (see
        // RECOVERY_X_RANGE), so the whole clip is a correction. Keep these in
        // their own output directory: mixed with clean approaches at an
        // unknown ratio they'd change what the training set teaches without
        // that being visible anywhere.
        if argv[i] == "--recovery" {
            args.recovery = true;
            i += 1;
            continue;
        }
        let value = argv
            .get(i + 1)
            .cloned()
            .with_context(|| format!("{} needs a value", argv[i]))?;
        match argv[i].as_str() {
            "--count" => args.count = value.parse()?,
            "--out-dir" => args.out_dir = value,
            "--xml" => args.xml_path = value,
            "--seed" => args.seed = value.parse()?,
            "--start-index" => args.start_index = value.parse()?,
            // Exposed because it's the one constant that has to be measured
            // against a given model's gripper rather than reasoned out (see
            // GRASP_OFFSET_Z) -- sweep it and keep whatever wins.
            "--grasp-offset" => args.grasp_offset = value.parse()?,
            other => anyhow::bail!("unknown argument '{other}'"),
        }
        i += 2;
    }
    anyhow::ensure!(
        !args.out_dir.is_empty() || args.probe_gripper,
        "--out-dir is required"
    );
    Ok(args)
}

/// Drives the gripper channel to one extreme and reports where the fingers
/// physically end up. Answers "does action[6] = 1.0 actually close this
/// gripper?" by measurement rather than by reading the convention off a doc
/// comment -- see `--probe-gripper`.
fn probe_gripper(xml_path: &str) -> Result<()> {
    let arm_actuators: Vec<&str> = ARM_ACTUATORS.to_vec();
    for command in [0.0f32, 1.0f32] {
        let mut scene = MujocoScene::from_xml_path(xml_path)?;
        // Home pose, not qpos0: the reset default leaves the arm straight
        // out, which can leave the hand resting on the floor and jam the
        // fingers against it -- confounding exactly the reading being taken.
        for (i, name) in ARM_JOINTS.iter().enumerate() {
            scene.set_joint_qpos(name, &[HOME_ARM_QPOS[i]]);
        }
        for name in FINGER_JOINTS {
            scene.set_joint_qpos(name, &[FINGER_OPEN_QPOS]);
        }
        scene.hold_current_pose(&arm_actuators)?;
        let action = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, command];
        for _ in 0..40 {
            scene.apply_policy_action(&action, EE_BODY, &arm_actuators, GRIPPER_ACTUATOR)?;
            for _ in 0..PHYSICS_STEPS_PER_CONTROL {
                scene.step_once();
            }
        }
        let qpos = scene.joint_qpos("finger_joint1").unwrap_or(f64::NAN);
        let closedness = black_splat::policy_dataset::gripper_closedness(qpos, FINGER_OPEN_QPOS);
        println!(
            "action[6] = {command:.1}  ->  finger_joint1 qpos {qpos:.4}               (0 = shut, {FINGER_OPEN_QPOS} = open)  ->  closedness {closedness:.2}  ->  fingers {}",
            if closedness > 0.5 { "CLOSED" } else { "OPEN" }
        );
    }
    println!(
        "Convention (policy_dataset::gripper_closedness, and what the training          labels use): action[6] 0.0 = open, 1.0 = closed."
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = parse_args()?;
    if args.probe_gripper {
        return probe_gripper(&args.xml_path);
    }
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("couldn't create {}", args.out_dir))?;

    let mut rng = Rng::new(args.seed);
    let mut kept = 0usize;
    let mut attempts = 0usize;
    // Generous but finite: a controller misconfigured for some other MJCF
    // would otherwise spin forever producing nothing.
    let attempt_limit = args.count * 20 + 50;

    println!(
        "Generating {} lift demo(s) from {} (seed {})",
        args.count, args.xml_path, args.seed
    );

    while kept < args.count && attempts < attempt_limit {
        attempts += 1;
        if args.verbose {
            eprintln!("attempt {attempts}:");
        }
        if let Some(clip) =
            generate_episode(
                &args.xml_path,
                &mut rng,
                args.grasp_offset,
                args.recovery,
                args.verbose,
            )?
        {
            let index = args.start_index + kept;
            let path = std::path::Path::new(&args.out_dir).join(format!("demo_{index}.json"));
            std::fs::write(&path, serde_json::to_string(&clip)?)
                .with_context(|| format!("couldn't write {}", path.display()))?;
            kept += 1;
            println!(
                "  demo_{index}.json  ({} frames, attempt {attempts})",
                clip.frames.len()
            );
        }
    }

    println!(
        "Done: {kept}/{} written to {} ({attempts} attempts, {:.0}% success rate)",
        args.count,
        args.out_dir,
        100.0 * kept as f64 / attempts.max(1) as f64
    );
    if kept < args.count {
        anyhow::bail!(
            "gave up after {attempts} attempts -- the scripted controller is failing most \
             episodes, so its tuning (GRASP_OFFSET_Z, the tick budgets, the randomization \
             ranges) likely doesn't suit this model. --verbose reports which phase fails."
        );
    }
    Ok(())
}
