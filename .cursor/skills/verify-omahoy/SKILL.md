---
name: verify-omahoy
description: "Drive the omahoy suite gate the way a reviewer does (scripts/verify.sh, the comment-check pin, scripts/check-comments.sh) and keep the transcript. Use before calling the tree done, when reviewing a change to the shared scripts, or when asked to verify omahoy."
---

# Verify omahoy

This skill drives the suite repo from the repo root. A reviewer runs
`scripts/verify.sh`, checks the comment pin, and runs
`scripts/check-comments.sh`. There is no window to open. The pages are a
separate drive, `.cursor/skills/verify/SKILL.md`. Run that when a change
is visible on omahoy.org. The agent that built a change does not sign off
on this run.

## Launch

No server. From the repo root, confirm the toolchain and make a directory
for this run. Each drive later gets its own tmux session.

```sh
python3 -c 'import sys; assert sys.version_info >= (3, 10), sys.version'
test "$(git rev-parse --is-inside-work-tree)" = true
test "$(git rev-parse --show-toplevel)" = "$PWD"
run=$(mktemp -d /tmp/omahoy-gate.XXXXXX)
mkdir -p "$run/artifacts/verify"
```

Ready when `$run` exists, `python3` is 3.10 or newer, and the current
directory is the git root. Nothing here installs packages.

## Doctor

Run this in the current shell before the first drive, and again when a
drive fails. It writes `$run/doctor.txt`.

```sh
(
  set -e
  python3 -c 'import sys; print(".".join(map(str, sys.version_info[:3])))'
  git rev-parse --show-toplevel
  test -x scripts/verify.sh
  test -x scripts/check-comments.sh
  echo "scripts executable"
  test "$(grep -cE '^[0-9a-f]{64}  scripts/check-comments.sh$' AGENTS.md)" -eq 1
  grep -E '^[0-9a-f]{64}  scripts/check-comments.sh$' AGENTS.md | sha256sum -c
) >"$run/doctor.txt" 2>"$run/doctor.err"
echo $? >"$run/doctor.exit"
```

`doctor.exit` is `0` when every check passed. `doctor.txt` prints the
Python version, the repo root, `scripts executable`, and
`scripts/check-comments.sh: OK`. `doctor.err` is empty. Any other exit
means a check failed. Stop. A failed pin means the script bytes and the
line in `AGENTS.md` disagree.

Do not drive a second command that writes this work tree while a drive is
running. `scripts/verify.sh` compares `git status` at the start and the
end. Another writer makes the `clean` check fail.

## Drive

Paste `drive_gate` once per shell. The first argument is the feature id.
The rest is the command. The function records the exit code and returns
success when `exit.txt` exists, including when the command itself failed.
Read `exit.txt` before calling the drive a pass.

```sh
drive_gate() {
  local id=$1
  shift
  local out="$run/artifacts/verify/$id"
  local session="omahoy-${id}-$$"
  mkdir -p "$out"
  tmux kill-session -t "=$session" 2>/dev/null || true
  rm -f "$out/exit.txt" "$out/stdout.txt" "$out/stderr.txt"
  { printf '%q ' "$@"; printf '\n'; } >"$out/command.txt"
  printf '%s\n' "$session" >"$out/session.txt"
  tmux new-session -d -s "$session" -c "$PWD" -- \
    bash -c 'out=$1; shift; "$@" >"$out/stdout.txt" 2>"$out/stderr.txt"; echo $? >"$out/exit.txt"' _ "$out" "$@"
  local i
  for i in $(seq 600); do
    test -f "$out/exit.txt" && break
    sleep 0.5
  done
  tmux kill-session -t "=$session" 2>/dev/null || true
  test -f "$out/exit.txt"
}
```

Follow `features/README.md`, then the feature file. One green entry point
does not cover a feature file you did not run.

## Evidence

`$run/artifacts/verify/<feature-id>/` holds `command.txt`, `stdout.txt`,
`stderr.txt`, `exit.txt`, and `session.txt`. Doctor output is
`$run/doctor.txt`, `$run/doctor.err`, and `$run/doctor.exit`.

Quote the exit code and the last stdout line in the PR. Copy
`$run/artifacts` out of `/tmp` when the proof has to outlive this machine.
Do not commit that copy.

A missing `exit.txt` is an unreachable run. An unmet precondition is
written under the feature id as `unreachable.txt`, with the command and
the missing file. Do not report that path as passed through another
command.

`scripts/site-data.py` writes `site/bay.js` and `site/wordmark.svg`. It is
not a gate. Its feature file says when to refuse it.

## Cleanup

Kill only the tmux session this drive started. Do not use `pkill`.

```sh
while IFS= read -r session; do
  tmux kill-session -t "=$session" 2>/dev/null || true
done < <(find "$run/artifacts/verify" -name session.txt -exec cat {} +)
test -d "$run/artifacts/verify"
```

Evidence stays in `$run/artifacts`. After cleanup, `ls
"$run/artifacts/verify"` still lists the feature id you drove. Do not
`rm -rf "$run"`.

## Helpers

This skill ships no script. The commands are `tmux`, `scripts/verify.sh`,
`scripts/check-comments.sh`, and `sha256sum`.

## Maintenance

When those commands or their output change, run
`/maintain-verification-skill` over this directory.
