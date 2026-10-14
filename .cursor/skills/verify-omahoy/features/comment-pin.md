# Comment pin

Downstream repos copy `scripts/check-comments.sh` byte for byte. The sha256
line in `AGENTS.md` is the pin.

## Sub-features

- `comment-pin` the file contains exactly one pin line, and `sha256sum -c` accepts that line.

## How to get to it (user POV)

- From the repo root, pipe the pin line in `AGENTS.md` to `sha256sum -c`.

## Driving it with bash

Preconditions:

- Launch passed. Doctor may already have run this command. Drive it again when `scripts/check-comments.sh` or the pin line changed.

- **Match.** Run `drive_gate comment-pin bash -c 'test "$(grep -cE "^[0-9a-f]{64}  scripts/check-comments.sh$" AGENTS.md)" -eq 1 && grep -E "^[0-9a-f]{64}  scripts/check-comments.sh$" AGENTS.md | sha256sum -c'`. `exit.txt` is `0`. `stdout.txt` is `scripts/check-comments.sh: OK`. `stderr.txt` is empty.

## Gotchas

- The pin line has two spaces between the hash and `scripts/check-comments.sh`. That is the text-mode line `sha256sum` prints. GNU coreutils `sha256sum -c` also accepts one space. The grep in Doctor counts only the two-space line.
- Do not copy the hash into a second file. `AGENTS.md` is the pin.
- A missing line makes `sha256sum` exit 1 and write `no properly formatted checksum lines found` to stderr.
