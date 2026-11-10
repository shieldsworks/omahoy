#!/usr/bin/env bash
# Runs `omahoy gui apply` detached from the panel, and reports on it. The
# shell reloads every panel when a plugin is added (`omarchy plugin add`
# rescans), and a reload kills the panel's own processes, so apply runs in a
# transient user unit of its own (`systemd-run --user`), or in a session of
# its own (`setsid`) where there is no user manager to ask. Its output goes
# to files the panel reads back, after a reload too.
#
#   apply.sh start DIR BIN   boat text on stdin; prints the run's id; exit 3
#                            while an apply or a probe runs
#   apply.sh status DIR      a state line (none, running, lost, code N),
#                            then id, out, err and in, a prefixed line each
#   apply.sh clear DIR ID    forgets run ID once it has finished
#   apply.sh probe DIR BIN [ARG...]
#                            runs `BIN gui probe ARG...`; exit 3 while an
#                            apply or another probe runs
#
# DIR holds `id` (the run's), `in` (the boat text), `out` and `err`
# (apply's stdout and stderr), `pid` (the worker's) and `code` (its exit
# status, written last). `lock` is flocked by start and clear while they
# look and write, by the worker from its start until `code` is written, and
# by a probe for its whole run, so of every panel only one starts an apply,
# a probe never stops the hub during an apply, and a clear can't remove a
# running apply's files or another run's.
set -euo pipefail

cmd=${1:-}
d=${2:?usage: apply.sh start|status|clear|probe DIR [BIN|ID] [ARG...]}

# state [held]: held when the caller has the lock, so no start can be
# waiting on a worker.
state() {
  [[ -e $d/in ]] || { echo none; return; }
  [[ -e $d/code ]] && { echo "code $(<"$d/code")"; return; }
  local pid
  pid=$(cat -- "$d/pid" 2>/dev/null || true)
  if [[ -n $pid ]]; then
    if kill -0 "$pid" 2>/dev/null; then echo running; else echo lost; fi
  elif [[ -z ${1:-} ]] && ! (exec 8> "$d/lock"; flock -n 8); then
    # No worker yet, and a start holds the lock while it waits for one.
    echo running
  else
    echo lost
  fi
}

case $cmd in
  start)
    bin=${3:?usage: apply.sh start DIR BIN}
    mkdir -p -m 700 -- "$d"
    exec 9> "$d/lock"
    if ! flock -n 9 || [[ $(state held) == running ]]; then
      echo "An apply is already running." >&2
      exit 3
    fi
    rm -f -- "$d/code" "$d/pid"
    : > "$d/out"
    : > "$d/err"
    cat > "$d/in"
    id=$(date +%s%N)-$$
    echo "$id" > "$d/id"
    self=$(readlink -f -- "$0")
    # The unit gets this environment (PATH, the session's display and
    # OMARCHY_PATH), which the user manager's own lacks.
    env=()
    while IFS= read -r name; do env+=(-E "$name"); done < <(compgen -e)
    # The worker writes its pid, then waits for this lock. Start keeps the
    # lock until the pid is there, so the run is never ownerless; a worker
    # that comes later than that finds another id, or none, and does nothing.
    if ! systemd-run --user --quiet --collect "${env[@]}" -- bash "$self" work "$d" "$bin" "$id" 2>/dev/null 9>&-; then
      setsid -f bash "$self" work "$d" "$bin" "$id" </dev/null >/dev/null 2>&1 9>&-
    fi
    for (( i = 0; i < 300; i++ )); do
      [[ -s $d/pid ]] && { echo "$id"; exit 0; }
      sleep 0.1
    done
    rm -f -- "$d/id" "$d/in"
    echo "The apply did not start within 30 s." >&2
    exit 2
    ;;
  probe)
    bin=${3:?usage: apply.sh probe DIR BIN [ARG...]}
    mkdir -p -m 700 -- "$d"
    exec 9> "$d/lock"
    if ! flock -n 9 || [[ $(state held) == running ]]; then
      echo "Setup is applying or listening in another window." >&2
      exit 3
    fi
    # The probe keeps fd 9, so the lock is held until it has started the
    # hub again and exited.
    exec "$bin" gui probe "${@:4}"
    ;;
  work)
    mine=${4:?usage: apply.sh work DIR BIN ID}
    [[ $(cat -- "$d/id" 2>/dev/null) == "$mine" ]] || exit 0
    echo $$ > "$d/pid"
    exec 9> "$d/lock"
    flock 9
    [[ $(cat -- "$d/id" 2>/dev/null) == "$mine" ]] || exit 0
    set +e
    "$3" gui apply < "$d/in" > "$d/out" 2> "$d/err" 9>&-
    echo $? > "$d/code.new"
    mv -- "$d/code.new" "$d/code"
    ;;
  status)
    state
    [[ -e $d/in ]] || exit 0
    sed 's/^/d /' -- "$d/id" 2>/dev/null || true
    sed 's/^/o /' -- "$d/out" 2>/dev/null || true
    sed 's/^/e /' -- "$d/err" 2>/dev/null || true
    sed 's/^/i /' -- "$d/in"
    ;;
  clear)
    want=${3:?usage: apply.sh clear DIR ID}
    [[ -d $d ]] || exit 0
    exec 9> "$d/lock"
    if ! flock -n 9 || [[ $(state held) == running ]]; then exit 3; fi
    [[ $(cat -- "$d/id" 2>/dev/null) == "$want" ]] || exit 3
    rm -f -- "$d/id" "$d/in" "$d/out" "$d/err" "$d/pid" "$d/code"
    ;;
  *)
    echo "usage: apply.sh start|status|clear|probe DIR [BIN|ID] [ARG...]" >&2
    exit 2
    ;;
esac
