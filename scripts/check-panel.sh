#!/usr/bin/env bash
# Drives the panel headless against scripts/fake-omahoy through its IPC
# handler and checks the behaviours that went wrong before: stale answers,
# serialization, hardware choices, closing during an apply. Never touches the
# real omahoy or the live shell.
set -euo pipefail
cd "$(dirname "$0")/.."

export QT_QPA_PLATFORM=offscreen
export OMAHOY_BIN=$PWD/scripts/fake-omahoy
tmp=$(mktemp -d)
pid=
trap '[[ -z $pid ]] || kill "$pid" 2>/dev/null || true; rm -rf "$tmp"' EXIT

fail() { echo "check-panel: $*" >&2; exit 1; }
ipc() { quickshell ipc -p ui/shell.qml call omahoysetup "$@" >/dev/null; }
st() { quickshell ipc -p ui/shell.qml call omahoysetup status | jq -c "$1"; }
# until EXPR WANT: poll the status until jq EXPR prints WANT.
until_is() {
  for _ in $(seq 100); do
    [[ $(st "$1" 2>/dev/null || true) == "$2" ]] && return 0
    sleep 0.1
  done
  fail "$1 is $(st "$1"), wanted $2 (${3:-}); $(st .)"
}
settled() { until_is '.busy' false; sleep 0.5; until_is '.busy' false; }
start() {
  [[ -z $pid ]] || { kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
  rm -f "$tmp/member"
  FAKE_OMAHOY_STATE=$tmp/member "$@" quickshell -p ui/shell.qml >"$tmp/qs.log" 2>&1 &
  pid=$!
  until_is '.loaded' true "panel did not load"
}

echo "panel: stale check does not convert units"
start env FAKE_OMAHOY_FAST=1 FAKE_OMAHOY_CHECK_SLEEP=1
settled
ipc set draft 8
ipc set units meters
until_is '.units' '"meters"'
sleep 2
until_is '.draft' '"2.44"' "8 ft in meters"

echo "panel: refuses control characters in every field, including units"
start env FAKE_OMAHOY_FAST=1
for bad in $'feet\nsource = tcp:x:1' $'feet\u0085x' $'feet x' 'fe"et'; do
  ipc set units "$bad"
  until_is '.refusing' true "units $(printf %q "$bad")"
  ipc set units feet
  until_is '.refusing' false
done

echo "panel: hardware choices survive a rescan and uucp is remembered"
ipc probe; settled
ipc chooseDevice 0 use
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
ipc chooseNet use
until_is '.sources|map(select(test("garbage")))|length' 0

echo "panel: go needs a current plan, and edits clear a finished apply"
ipc goTo review
until_is '.ready' true
ipc go
until_is '.applyState' '"done"'
ipc set name Other
until_is '.applyState' '""'
until_is '.ready' true

echo "panel: closing during an apply is refused"
start env
ipc goTo review
until_is '.ready' true
ipc go
until_is '.applyState' '"running"'
ipc close
until_is '.closeRefused' true
until_is '.opened' true
until_is '.applyState' '"done"'
echo "ok"
