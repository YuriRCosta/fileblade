#!/usr/bin/env bash
source "$(dirname "$0")/expectations/lib.sh"
require_guest
fixture >/dev/null
open_left
goto_root "$ROOT_DIR"
ctl closeBlade right
guest 'pkill -x nvim; pkill -x foot; pkill -x nautilus' >/dev/null 2>&1
trap 'ctl hideActions; ctl dockBlade left' EXIT
click_row alpha.txt

"$OVM" key f2
sleep 2
expect menu-keys "F2 opens Rename" actionMenuMode rename
expect menu-keys "and the dialog is visible" actionMenuOpen true
"$OVM" key esc
sleep 1
"$OVM" key f2
sleep 2
expect menu-keys "a fresh F2 still opens Rename" actionMenuOpen true
"$OVM" key esc
sleep 1

held_copy() {
  ctl clearClipboard
  "$OVM" key m
  sleep 2
  "$OVM" type copy
  sleep 1
  "$OVM" ssh 'wtype -P Return -s 1000 -p Return'
  sleep 2
  expect menu-keys "held Enter runs Copy" clipboardCount 1
  expect menu-keys "and closes the menu" actionMenuOpen false
}

click_row alpha.txt
held_copy
expect_out menu-keys "held confirmation cannot open the file underneath" 'hyprctl -j clients | jq length' 0
"$OVM" shot menu-held-enter >/dev/null
"$OVM" key ret
wait_for '[[ $("$OVM" hypr clients | jq length) == 1 ]]' 15
expect_out menu-keys "a fresh Enter still opens the file" 'hyprctl -j clients | jq length' 1
guest 'pkill -x nvim; pkill -x foot' >/dev/null 2>&1
wait_for '[[ $("$OVM" hypr clients | jq length) == 0 ]]' 15

ctl focusTree
"$OVM" key m
sleep 2
"$OVM" ssh 'wtype -P Escape -s 1000 -p Escape'
sleep 1
expect menu-keys "held Escape closes only the menu" actionMenuOpen false
expect menu-keys "and keeps the blade focused" focusedBlade left

ctl windowToggle
wait_for '[[ $(blade_mode left) == window ]]' 15
ctl focusTree
navigation='wtype -k Home'
target_row=$(row_index alpha.txt)
for ((step=0; step<target_row; step++)); do navigation+=' -k Down'; done
guest "$navigation"
sleep 1
expect menu-keys "window-mode precondition selects the file" selectedPath "$ROOT_DIR/alpha.txt"
held_copy
expect_out menu-keys "window-mode confirmation leaves only the blade window" 'hyprctl -j clients | jq length' 1
"$OVM" shot window-menu-held-enter >/dev/null
summary
