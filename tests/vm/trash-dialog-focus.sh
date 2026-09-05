#!/usr/bin/env bash
source "$(dirname "$0")/expectations/lib.sh"
require_guest
fixture >/dev/null
open_left
goto_root "$ROOT_DIR"

trash_file() {
  click_row "$1"
  "$OVM" key delete
  sleep 1
  "$OVM" key tab
  "$OVM" key ret
  wait_for "[[ \$(field operationBusy) == false ]]" 15
  sleep 1
}
row_action() {
  "$OVM" mouse move 200 152
  sleep 1
  "$OVM" mouse click "$1" 152
  sleep 1
}

trash_file alpha.txt
ctl navigate trash:///
sleep 2
row_action 350
expect_contains dialog "permanent-delete confirmation opens" "$(screen_text)" "permanently"
"$OVM" key tab
"$OVM" key esc
sleep 1
expect_missing dialog "Tab cannot strand Escape outside the dialog" "$(screen_text)" "permanently"
expect dialog "cancel preserves the trashed item" trashCount 1
"$OVM" shot trash-dialog-tab-escape >/dev/null

row_action 322
"$OVM" key tab
"$OVM" key tab
"$OVM" key ret
sleep 1
expect_missing dialog "restore destination tabs reach Cancel" "$(screen_text)" "another folder"
expect dialog "cancel does not restore the item" trashCount 1

row_action 322
"$OVM" key ctrl-a
"$OVM" type "$ROOT_DIR/dest"
"$OVM" key tab
"$OVM" key tab
"$OVM" key tab
"$OVM" key ret
wait_for '[[ $(field trashCount) == 0 ]]' 15
expect_out dialog "restore destination is editable and keyboard-confirmable" "test -f $ROOT_DIR/dest/alpha.txt && echo yes" yes

goto_root "$ROOT_DIR/dest"
trash_file alpha.txt
goto_root "$ROOT_DIR"
trash_file bravo.txt
ctl navigate trash:///
sleep 2
expect dialog "precondition: two entries" trashCount 2
row_action 350
expect_contains dialog "held-key test starts inside a confirmation" "$(screen_text)" "permanently"
"$OVM" ssh 'wtype -P Return -s 1000 -p Return'
sleep 2
expect dialog "held Enter purges only the confirmed entry" trashCount 1
expect_out dialog "repeat cannot restore the next entry underneath" "test ! -f $ROOT_DIR/dest/alpha.txt && test ! -f $ROOT_DIR/bravo.txt && echo yes" yes
"$OVM" shot trash-dialog-held-enter >/dev/null
"$OVM" key ret
wait_for '[[ $(field trashCount) == 0 ]]' 15
expect dialog "a fresh Enter still restores the remaining entry" trashCount 0
summary
