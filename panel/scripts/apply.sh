#!/usr/bin/env bash
# Runs `omahoy gui apply` detached from the panel, and reports on it. The
# shell reloads every panel when a plugin is added (`omarchy plugin add`
# rescans), and a reload kills the panel's own processes, so apply runs in a
# transient user unit of its own (`systemd-run --user`), or in a session of
# its own (`setsid`) where there is no user manager to ask. Its output goes
# to files the panel reads back, after a reload too.
#
#   apply.sh start DIR BIN   boat text on stdin; exit 3 while one runs
#   apply.sh status DIR      a state line (none, running, lost, code N),
#                            then out, err and in, a prefixed line each
#   apply.sh clear DIR       forgets a finished apply
#
# DIR holds `in` (the boat text), `out` and `err` (apply's stdout and
# stderr), `pid` (the worker's) and `code` (its exit status, written last).
set -euo pipefail

cmd=${1:-}
d=${2:?usage: apply.sh start|status|clear DIR [BIN]}

state() {
  [[ -e $d/in ]] || { echo none; return; }
  [[ -e $d/code ]] && { echo "code $(<"$d/code")"; return; }
  local pid
  pid=$(cat -- "$d/pid" 2>/dev/null || true)
  if [[ -n $pid ]]; then
    if kill -0 "$pid" 2>/dev/null; then echo running; else echo lost; fi
  elif (( $(date +%s) - $(stat -c %Y -- "$d/in") < 10 )); then
    # Started, and the worker has not written its pid yet.
    echo running
  else
    echo lost
  fi
}

case $cmd in
  start)
    bin=${3:?usage: apply.sh start DIR BIN}
    mkdir -p -m 700 -- "$d"
    if [[ $(state) == running ]]; then
      echo "An apply is already running." >&2
      exit 3
    fi
    rm -f -- "$d/code" "$d/pid"
    : > "$d/out"
    : > "$d/err"
    cat > "$d/in"
    self=$(readlink -f -- "$0")
    # The unit gets this environment (PATH, the session's display and
    # OMARCHY_PATH), which the user manager's own lacks.
    env=()
    while IFS= read -r name; do env+=(-E "$name"); done < <(compgen -e)
    if ! systemd-run --user --quiet --collect "${env[@]}" -- bash "$self" work "$d" "$bin" 2>/dev/null; then
      setsid -f bash "$self" work "$d" "$bin" </dev/null >/dev/null 2>&1
    fi
    ;;
  work)
    echo $$ > "$d/pid"
    set +e
    "$3" gui apply < "$d/in" > "$d/out" 2> "$d/err"
    echo $? > "$d/code.new"
    mv -- "$d/code.new" "$d/code"
    ;;
  status)
    state
    [[ -e $d/in ]] || exit 0
    sed 's/^/o /' -- "$d/out" 2>/dev/null || true
    sed 's/^/e /' -- "$d/err" 2>/dev/null || true
    sed 's/^/i /' -- "$d/in"
    ;;
  clear)
    [[ $(state) == running ]] && exit 3
    rm -f -- "$d/in" "$d/out" "$d/err" "$d/pid" "$d/code"
    ;;
  *)
    echo "usage: apply.sh start|status|clear DIR [BIN]" >&2
    exit 2
    ;;
esac
