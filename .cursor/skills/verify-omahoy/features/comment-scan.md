# Comment scan

`scripts/check-comments.sh` with no arguments scans the tracked files the
rule names. Arguments replace that set.

## Sub-features

- `scan-default` exits 0 with empty stdout and stderr.
- `scan-path` one path replaces the default set.
- `scan-empty` a pathspec that lists no file exits 0.

## How to get to it (user POV)

- Run `scripts/check-comments.sh`.
- Run `scripts/check-comments.sh scripts/verify.py`.
- Run `scripts/check-comments.sh 'no-such-*.qqq'`.

## Driving it with bash

Preconditions:

- Launch and Doctor passed.

- **Default.** Run `drive comment-scan scripts/check-comments.sh`. `exit.txt` is `0`. `stdout.txt` and `stderr.txt` are empty.
- **One file.** Run `drive comment-scan-file scripts/check-comments.sh scripts/verify.py`. `exit.txt` is `0`. Both streams are empty.
- **No file.** Run `drive comment-scan-empty scripts/check-comments.sh 'no-such-*.qqq'`. `exit.txt` is `0`. Both streams are empty.

## Gotchas

- Arguments replace the default set. They do not add to it.
- The script excludes its own path. A listed path that is exactly `-` is read as `./-`.
- A search error fails the run. Empty output and exit 0 is a pass when the listing had no file to read, or no hit.
- HTML, CSS, and Markdown are outside the default set. The comment rule still applies to them.
- Do not plant a banned comment in this checkout to prove a hit. `scripts/test_check_comments.py` does that in a temp repo, and `scripts/verify.sh` runs it.
