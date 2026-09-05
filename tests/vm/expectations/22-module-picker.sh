#!/usr/bin/env bash
source "$(dirname "$0")/lib.sh"

require_guest
layout_path=$(field bladeLayoutPath)
original_slots=$(guest "jq -c .blades.right.slots $(printf '%q' "$layout_path")")
original_open=$("$OVM" ipc "$PLUGIN" blades | jq -r .blades.right.open)
original_focus=$(field focusedBlade)
cleanup() {
  ctl setBladeSlots right "base64:$(printf '%s' "$original_slots" | base64 -w0)"
  [[ $original_open == true ]] || ctl closeBlade right
  if [[ -n $original_focus ]]; then ctl focusBlade "$original_focus"; else ctl releaseBladeFocus; fi
}
trap cleanup EXIT

picker_text() {
  local shot crop
  shot=$("$OVM" shot module-picker-ocr | tail -1)
  crop=$(mktemp --suffix=.png)
  magick "$shot" -crop "160x24+$(( $1 + 7 ))+63" +repage -colorspace gray -negate -resize 400% "$crop"
  tesseract "$crop" - --psm 7 2>/dev/null | tr -s '[:space:]' ' '
  rm -f -- "$crop" "$shot"
}

ctl setBladeSlots right "base64:$(printf '%s' '[{"id":"picker-notes","modules":[{"module":"notes","state":{"text":"picker note preserved"}}]}]' | base64 -w0)"
ctl focusBlade right
"$OVM" ipc notifications dismissAll >/dev/null
sleep 1
notes_x=$((1920 - $(field propertiesBladeWidth) + 9))
"$OVM" mouse click $((notes_x + 79)) 42
"$OVM" mouse move $((notes_x + 160)) 233
sleep 1
expect_contains E-22-01 "a single Notes section opens its module picker" "$(picker_text $((notes_x + 64)))" "Add module"
"$OVM" key esc
sleep 1
expect_missing E-22-01 "Escape dismisses the picker after hovering its rows" "$(picker_text $((notes_x + 64)))" "Add module"
expect_true E-22-01 "Escape leaves the right blade open" '[[ $("$OVM" ipc "$PLUGIN" blades | jq -r .blades.right.open) == true ]]'

"$OVM" mouse click $((notes_x + 79)) 42
sleep 1
"$OVM" shot module-picker-right >/dev/null
"$OVM" mouse click $((notes_x + 270)) 77
sleep 1
expect_missing E-22-01 "the close X dismisses the module picker" "$(picker_text $((notes_x + 64)))" "Add module"

"$OVM" mouse click $((notes_x + 79)) 42
sleep 1
"$OVM" type skills
"$OVM" key ret
sleep 2
expect_true E-22-01 "keyboard selection adds Skills to the right Notes section" '[[ $("$OVM" ipc "$PLUGIN" blades | jq -r ".blades.right.slots[0].modules[1].module") == data-goblin.fileblade-skills/skills ]]'
expect_out E-22-01 "adding a module preserves Notes" "jq -r '.blades.right.slots[0].modules[0].state.text.items[0].text' $(printf '%q' "$layout_path")" "picker note preserved"
"$OVM" shot module-picker-added-right >/dev/null

ctl focusBlade left
"$OVM" mouse click 111 42
"$OVM" mouse move 230 233
sleep 1
expect_contains E-22-01 "the left FileBlade header still opens the picker" "$(picker_text 93)" "Add module"
"$OVM" key esc
sleep 1
expect_missing E-22-01 "Escape also dismisses the left picker" "$(picker_text 93)" "Add module"

summary
