#!/usr/bin/env bash
# One-shot project setup: fetches large assets that aren't checked into git
# and does any other local setup needed before a first build. Safe to re-run.
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TRAJ_DIR="$DIR/source_assets/mujoco_trajectories"
TRAJ_FOLDER_URL="https://drive.google.com/drive/folders/1u40p4fdvgRfVgqDOMejafBekzL-plcjZ"
TOOL_HANG_OUT="$DIR/examples/splat/game_assets/mujoco/trajectories/tool_hang"

fetch_trajectories() {
  for f in lift_ph_low_dim.hdf5 square_ph_low_dim.hdf5 tool_hang_ph_low_dim.hdf5; do
    if [ ! -f "$TRAJ_DIR/$f" ]; then
      echo "==> Fetching robomimic trajectory datasets into $TRAJ_DIR"
      python3 -m pip show gdown >/dev/null 2>&1 || python3 -m pip install --quiet gdown
      mkdir -p "$TRAJ_DIR"
      python3 -m gdown --folder "$TRAJ_FOLDER_URL" -O "$TRAJ_DIR"
      return
    fi
  done
  echo "==> Trajectory datasets already present, skipping"
}

# Bulk-converts every ToolHang demo (the task actually in use) to this
# engine's trajectory JSON schema, gitignored since it's deterministically
# regenerable from the fetched HDF5 (see .gitignore's note on
# examples/splat/game_assets/mujoco/trajectories/tool_hang/). Lift/square
# aren't bulk-converted here -- only fetched above -- since nothing uses their
# full demo sets yet; run robomimic_to_trajectory.py --all by hand for those
# if that changes.
convert_tool_hang() {
  if [ -n "$(ls -A "$TOOL_HANG_OUT"/*.json 2>/dev/null)" ]; then
    echo "==> ToolHang trajectory JSON already present, skipping"
    return
  fi
  echo "==> Converting ToolHang demos into $TOOL_HANG_OUT"
  python3 -m pip show h5py >/dev/null 2>&1 || python3 -m pip install --quiet h5py
  mkdir -p "$TOOL_HANG_OUT"
  python3 "$DIR/tools/robomimic_to_trajectory.py" \
    "$TRAJ_DIR/tool_hang_ph_low_dim.hdf5" --all -o "$TOOL_HANG_OUT"
}

fetch_trajectories
convert_tool_hang

echo "==> Setup complete"
