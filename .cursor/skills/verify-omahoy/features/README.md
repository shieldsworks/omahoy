# omahoy gate verification map

This directory is the source for verifying what a reviewer runs in the
omahoy repo. Read this index, then use the matching feature file. A change
that adds or alters one of these commands updates its feature file in the
same PR.

`scripts/verify.sh` does not read this directory. The `feature-map` check
reads `.cursor/skills/verify/features/` only.

## Baseline preconditions

- The repo root, with `python3` 3.10 or newer, `git`, `tmux`, and `sha256sum`.
- Launch and Doctor from `../SKILL.md` passed for this `$run`.
- No other command is writing this work tree.
- Never drive a checkout whose pin check failed.

## Driving conventions

- Start every recipe from the baseline unless its preconditions say otherwise.
- Commands are literal. Keep the quoted text unchanged.
- Call `drive_gate` from `../SKILL.md`. The first argument is the feature id.
- Do not delete proof during cleanup.

## Proof and skip reporting

- Command proof is `command.txt`, `stdout.txt`, `stderr.txt`, and `exit.txt`.
- A second look is the next read-only command the feature names, or the
  `clean` section of a `scripts/verify.sh` transcript.
- Record the feature id with every artifact.
- An unreachable path is `unreachable.txt` with the command and the unmet
  precondition. It is not a pass by another path.

## Feature entry contract

Each feature file starts with an H1 and one paragraph, then these four H2s,
in order.

1. `Sub-features`
2. `How to get to it (user POV)`
3. `Driving it with bash`
4. `Gotchas`

## Features

- [Suite gate](./gate.md) runs `scripts/verify.sh` and checks that the run wrote nothing.
- [Named check](./named-check.md) lists the checks, runs one, and rejects an unknown name.
- [Comment pin](./comment-pin.md) checks `scripts/check-comments.sh` against the sha256 in `AGENTS.md`.
- [Comment scan](./comment-scan.md) runs `scripts/check-comments.sh` with no arguments and with a pathspec.
- [Chart and wordmark](./site-data.md) is `scripts/site-data.py`. Refuse it when `logo.txt` is missing.
