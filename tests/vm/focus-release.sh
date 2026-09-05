#!/usr/bin/env bash
source "$(dirname "$0")/expectations/lib.sh"
require_guest
fixture >/dev/null
ctl closeBlade left
ctl closeBlade right
guest 'pkill -x foot; setsid foot >/dev/null 2>&1 < /dev/null &' >/dev/null 2>&1
wait_for '[[ $("$OVM" hypr clients | jq length) == 1 ]]' 15
"$OVM" mouse move 130 400
for delay in 0.15 0.30 0.40; do
  ctl closeBlade left
  sleep 0.8
  guest "omarchy-shell -q $PLUGIN.control toggleBladeFocus left; sleep $delay; omarchy-shell -q $PLUGIN.control releaseBladeFocus"
  sleep 1
  expect focus-release "release during opening ($delay s) stays released under a stationary pointer" focusedBlade ""
done
"$OVM" mouse move 145 420
wait_for '[[ $(field focusedBlade) == left ]]' 8
expect focus-release "real pointer movement still refocuses the blade" focusedBlade left
"$OVM" mouse move 130 1060
ctl releaseBladeFocus
sleep 1
expect focus-release "release in the footer stays released" focusedBlade ""
"$OVM" mouse move 145 1060
wait_for '[[ $(field focusedBlade) == left ]]' 8
expect focus-release "the first real movement in the footer refocuses" focusedBlade left
"$OVM" shot focus-released-during-opening >/dev/null
guest 'pkill -x foot' >/dev/null 2>&1
summary
