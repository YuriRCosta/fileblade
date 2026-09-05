#!/usr/bin/env bash
# Hold Space once the pointer of the nested session comes within reach of a
# screen point, for takes that open the drop wheel mid-drag. Run it through the
# session before the take, e.g.
#
#   demos/session.sh exec demos/hold-space-near.sh 1150 520 &
#   demos/session.sh run demos/drop-wheel.toml
#
#   hold-space-near.sh X Y [REACH_PX] [HOLD_MS]
set -euo pipefail
x=${1:?x}; y=${2:?y}; reach=${3:-10}; hold=${4:-700}
limit=$((reach * reach))
for _ in $(seq 1 12000); do
  read -r cx cy < <(hyprctl cursorpos | tr -d ',')
  dx=$((cx - x)); dy=$((cy - y))
  if (( dx * dx + dy * dy <= limit )); then
    wtype -P space -s "$hold" -p space
    exit 0
  fi
  sleep 0.005
done
echo "hold-space-near: the pointer never reached $x,$y" >&2
exit 1
