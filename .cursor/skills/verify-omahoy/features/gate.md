# Suite gate

`scripts/verify.sh` is the done command. It runs every check, then reports
whether that run created or changed a file.

## Sub-features

- `gate-all` runs every check, then `clean`.
- `gate-pass` exits 0 and prints the pass line.
- `gate-unchanged` the `clean` section names no path.

## How to get to it (user POV)

- From the repo root, run `scripts/verify.sh`.
- CI runs that command, then `git status --porcelain --untracked-files=all`.

## Driving it with bash

Preconditions:

- Launch and Doctor passed.
- No other command is writing this work tree.

- **All checks.** Run `drive gate scripts/verify.sh`. `exit.txt` is `0`. `stdout.txt` lists each check as `== <name>` with no finding under it, then a blank line, then `All verification steps passed.`
- **Unchanged tree.** Same transcript. The `== clean` section has no path under it. `stderr.txt` is empty.
- **Proof.** Keep `command.txt`, `stdout.txt`, `stderr.txt`, and `exit.txt`.

## Gotchas

- A file that was already dirty is not a `clean` finding. CI still fails if `git status` is dirty after the run, including dirt that was already there.
- `feature-map` reads `.cursor/skills/verify/features/` only. Do not retarget that check at this directory. That change belongs in a PR whose purpose is the gate.
- The run reads the checkout. A second writer in the same tree can make `clean` fail.
