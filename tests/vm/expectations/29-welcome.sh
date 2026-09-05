#!/usr/bin/env bash
# Welcome tab and example extensions. Expectations E-29-01 .. E-29-05.
source "$(dirname "$0")/lib.sh"

require_guest

STATE=$(field bladeLayoutPath)
original_slots=$(guest "jq -c .blades.right.slots $(printf '%q' "$STATE")")
original_open=$("$OVM" ipc "$PLUGIN" blades | jq -r .blades.right.open)
cleanup() {
  ctl setBladeSlots right "base64:$(printf '%s' "$original_slots" | base64 -w0)"
  [[ $original_open == true ]] || ctl closeBlade right
}
trap cleanup EXIT

welcome_state() { "$OVM" ipc "$PLUGIN" status | jq -r '.welcomeState // ""'; }
right_modules() { "$OVM" ipc "$PLUGIN" blades | jq -c '[.blades.right.slots[0].modules[].module]'; }

ctl setWelcomeState ""
ctl setBladeSlots right 'base64:W10='
ctl resetBladeLayout
sleep 1
expect_out E-29-01 "first launch seeds a Welcome tab in front of Notes" right_modules '["welcome","notes"]'
expect_screen E-29-01 "the Welcome tab asks to install agent extensions" "Install agent extensions"

ctl focusBlade right
ctl welcomeDismiss
sleep 1
expect_out E-29-03 "closing for good removes the tab" "\"$OVM\" ipc \"$PLUGIN\" blades | jq -r '[.blades.right.slots[].modules[].module] | index(\"welcome\") // \"absent\"'" absent
expect_out E-29-03 "and records the choice" welcome_state dismissed
ctl resetBladeLayout
sleep 1
expect_out E-29-03 "a layout reset does not bring it back" "\"$OVM\" ipc \"$PLUGIN\" blades | jq -r '[.blades.right.slots[].modules[].module] | index(\"welcome\") // \"absent\"'" absent

ctl setWelcomeState ""
ctl resetBladeLayout
sleep 1
ctl welcomeInstall
sleep 30
expect_out E-29-02 "Install closes the Welcome tab" "\"$OVM\" ipc \"$PLUGIN\" blades | jq -r '[.blades.right.slots[].modules[].module] | index(\"welcome\") // \"absent\"'" absent
expect_out E-29-04 "and records the install" welcome_state installed
expect_out E-29-02 "the example extensions are installed" "guest 'ls ~/.config/omarchy/plugins | grep -c fileblade-'" 4
ctl resetBladeLayout
sleep 1
expect_out E-29-04 "a layout reset after installing does not bring it back" "\"$OVM\" ipc \"$PLUGIN\" blades | jq -r '[.blades.right.slots[].modules[].module] | index(\"welcome\") // \"absent\"'" absent

pend E-29-05 "install failure keeps the tab open with the failed extension named"
summary
