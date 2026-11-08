#!/usr/bin/env bash
# Drives the panel headless against scripts/fake-omahoy through its IPC
# handler and checks the behaviours that went wrong before: stale answers,
# unit changes, serialization, hardware choices, closing during an apply,
# and an apply that outlives the panel. Every instance runs with HOME and
# XDG_RUNTIME_DIR in a temp dir, so its IPC socket, its apply files and any
# command it runs stay there, and every IPC call names the instance's pid:
# a check never reaches a real instance, the real omahoy or the live shell.
set -euo pipefail
cd "$(dirname "$0")/.."

tmp=$(mktemp -d)
pid=
cleanup() {
  if [[ -n $pid ]]; then kill "$pid" 2>/dev/null || true; fi
  # A detached fake apply may still be writing into the temp dir.
  pkill -f -- "$tmp/run/omahoy-setup" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -m 700 "$tmp/run" "$tmp/home" "$tmp/bin"
# An omahoy on PATH that answers nothing, so only the fake ever runs.
printf '#!/bin/sh\necho "check-panel: the panel ran omahoy from PATH" >&2\nexit 2\n' > "$tmp/bin/omahoy"
chmod +x "$tmp/bin/omahoy"

export QT_QPA_PLATFORM=offscreen
export HOME=$tmp/home XDG_RUNTIME_DIR=$tmp/run PATH=$tmp/bin:$PATH
unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_STATE_HOME XDG_CACHE_HOME WAYLAND_DISPLAY DISPLAY
# The gtk3 platform theme opens a display, and there is none offscreen.
export QT_QPA_PLATFORMTHEME=
# No session bus, so `systemd-run --user` finds no user manager and apply.sh
# detaches with setsid instead, inside the temp dir.
unset DBUS_SESSION_BUS_ADDRESS
fake=$PWD/scripts/fake-omahoy
applydir=$tmp/run/omahoy-setup

fail() {
  echo "check-panel: $*" >&2
  [[ ! -e $tmp/qs.log ]] || tail -n 20 "$tmp/qs.log" >&2
  exit 1
}
alive() { kill -0 "$pid" 2>/dev/null || fail "the panel (pid $pid) exited"; }
ipc() { alive; quickshell ipc --pid "$pid" call omahoysetup "$@" >/dev/null; }
st() { alive; quickshell ipc --pid "$pid" call omahoysetup status | jq -c "$1"; }
# until_is EXPR WANT [WHY]: poll the status until jq EXPR prints WANT.
until_is() {
  local _
  for _ in $(seq 100); do
    alive
    [[ $(st "$1" 2>/dev/null || true) == "$2" ]] && return 0
    sleep 0.1
  done
  fail "$1 is $(st "$1"), wanted $2 (${3:-}); $(st .)"
}
settled() { until_is '.busy' false; sleep 0.5; until_is '.busy' false; }
stop() {
  [[ -z $pid ]] || { kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
  pid=
}
# start [VAR=VALUE...]: a fresh instance on the fake, with that environment.
start() {
  stop
  rm -f "$tmp/member" "$tmp/member.unplugged"
  env FAKE_OMAHOY_STATE="$tmp/member" OMAHOY_BIN="$fake" "$@" quickshell -p ui/shell.qml >"$tmp/qs.log" 2>&1 &
  pid=$!
  until_is '.loaded' true "panel did not load"
}
key() { st ".hardware|map(select(.key|test(\"$1\")))|.[0].key" | jq -r .; }

echo "panel: opened without a binary, it takes the one in the open payload"
stop
env -u OMAHOY_BIN FAKE_OMAHOY_FAST=1 quickshell -p ui/shell.qml >"$tmp/qs.log" 2>&1 &
pid=$!
sleep 1
until_is '.loaded' false
ipc open "$(jq -cn --arg b "$fake" '{bin: $b}')"
until_is '.loaded' true "the payload's bin"

echo "panel: stale check does not convert units"
start FAKE_OMAHOY_FAST=1 FAKE_OMAHOY_CHECK_SLEEP=1
settled
ipc set draft 8
ipc set units meters
until_is '.units' '"meters"'
sleep 2
until_is '.draft' '"2.44"' "8 ft in meters"

echo "panel: a refused field keeps a unit change pending"
start FAKE_OMAHOY_FAST=1
settled
ipc set units meters
until_is '.units' '"meters"'
ipc set draft 2.44
sleep 1
settled
ipc set name 'Da#sh'
until_is '.refusing' true
ipc set units feet
sleep 1
until_is '[.units, .wantUnits, .draft]' '["meters","feet","2.44"]' "nothing converts while refused"
ipc set name Dash
until_is '[.units, .wantUnits, .draft]' '["feet","","8.01"]' "2.44 m in feet"

echo "panel: refuses control characters in every field, including units"
for bad in $'feet\nsource = tcp:x:1' $'feet\u0085x' $'feet\u2028x' 'fe"et'; do
  ipc set units "$bad"
  until_is '.refusing' true "units $(printf %q "$bad")"
  ipc set units feet
  until_is '.refusing' false
done

echo "panel: hardware choices survive a rescan and uucp is remembered"
ipc probe; settled
ipc choose "$(key u-blox)" use
until_is '.sources|length' 1
until_is '.sources[0]|test("u-blox")' true
ipc probe; settled
until_is '.sources[0]|test("u-blox")' true "a rescan kept the choice"
ipc allowSerial
until_is '.useUucp' true
settled
ipc probe; settled
until_is '.useUucp' true

echo "panel: a test of garbage cannot be used"
ipc testNet garbage; settled
until_is '.hardware|map(select(.key|test("garbage")))|length' 0
until_is '.hardwareError|test("garbage")' true

echo "panel: a kept source the scan stops hearing stays"
gps=serial:/dev/serial/by-id/usb-u-blox_GPS-if00
start FAKE_OMAHOY_FAST=1 "FAKE_OMAHOY_SOURCES=$gps:9600"
until_is '.sources' "[\"$gps:9600\"]"
ipc probe; settled
until_is '.hardware|length' 3 "one entry per device, the boat's GPS among them"
until_is ".hardware|map(select(.key == \"$gps\"))|.[0]|[.choice, .offered, .heard]" "[\"keep\",\"$gps:4800\",true]"
until_is '.sources' "[\"$gps:9600\"]" "keeping keeps the boat's baud"
touch "$tmp/member.unplugged"
ipc probe; settled
until_is ".hardware|map(select(.key == \"$gps\"))|.[0]|[.choice, .heard]" '["keep",false]'
until_is '.sources' "[\"$gps:9600\"]" "unplugged, it is still kept"
ipc choose "$gps" use
until_is '.sources' "[\"$gps:4800\"]" "use replaces every baud for the path"

echo "panel: allowing serial ports keeps the other choices"
rm -f "$tmp/member.unplugged"
ipc choose "$(key dAISy)" use
until_is '.sources|map(select(test("dAISy")))' '["serial:/dev/serial/by-id/usb-dAISy_AIS_Receiver-if00:38400"]'
ipc allowSerial
settled
until_is '.hardware|map(select(.key|test("FTDI")))|.[0].verdict' '"ok"' "the reprobe heard the denied port"
until_is '.sources|map(select(test("dAISy")))|length' 1 "the silent receiver is still used"
until_is '.sources|map(select(test("u-blox")))|length' 1

echo "panel: a network source skipped before listening stays skipped"
start FAKE_OMAHOY_FAST=1 'FAKE_OMAHOY_SOURCES=tcp:10.0.0.5:10110'
ipc testNet 10.0.0.5:10110; settled
until_is '.hardware|length' 1 "the test is the boat's source"
ipc choose tcp:10.0.0.5:10110 skip
until_is '.sources' '["demo"]'
ipc probe; settled
until_is '.sources' '["demo"]' "listening did not bring it back"

echo "panel: go needs a current plan, and edits clear a finished apply"
start FAKE_OMAHOY_FAST=1
ipc goTo review
until_is '.ready' true
ipc go
until_is '.applyState' '"done"'
[[ ! -e $applydir/in ]] || fail "a finished apply was not cleared"
ipc set name Other
until_is '.applyState' '""'
until_is '.ready' true

echo "panel: closing during an apply is refused, and the shell can hide it after"
start
ipc goTo review
until_is '.ready' true
ipc go
until_is '.applyState' '"running"'
ipc probe
until_is '.listening' false "no probe while apply runs"
ipc close
until_is '.closeRefused' true
until_is '.opened' true
until_is '.visible' true
until_is '.applyState' '"done"'
ipc hide
until_is '[.opened, .visible]' '[false,false]' "visible still follows opened"

echo "panel: an apply outlives the panel and the next one picks it up"
start
ipc goTo review
until_is '.ready' true
ipc set name Wanderer
until_is '.ready' true
ipc go
until_is '.applyState' '"running"'
stop
[[ -e $applydir/in && ! -e $applydir/code ]] || fail "apply did not keep running without the panel"
start
until_is '[.step, .applyState]' '["review","running"]' "picked up the running apply"
until_is '.name' '"Wanderer"' "the form is the text being applied"
st '.' | jq -e '.applyError == ""' >/dev/null || fail "applyError: $(st .applyError)"
until_is '.applyState' '"done"' "it finished"
[[ $(quickshell ipc --pid "$pid" call omahoysetup status | jq -r .step) == review ]] || fail "left review"
[[ ! -e $applydir/in ]] || fail "the finished apply was not cleared"
echo "ok"
