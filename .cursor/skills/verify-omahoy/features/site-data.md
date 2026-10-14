# Chart and wordmark

`python3 scripts/site-data.py` rewrites `site/bay.js` and `site/wordmark.svg`.
It is a maintainer command. It is not a gate.

## Sub-features

- `site-data` when `logo.txt` is missing, record `unreachable.txt` and do not run the script. When `logo.txt` exists, the script writes both files and prints a summary.

## How to get to it (user POV)

- From the repo root, run `python3 scripts/site-data.py`, as `site/README.md` describes.
- An optional argument is the path to `logo.txt`. The default is `~/.local/share/omarchy/logo.txt`.

## Driving it with bash

Preconditions:

- Launch and Doctor passed.
- This drive does not overlap a `scripts/verify.sh` run.

- **Logo missing.** Run `test -f "$HOME/.local/share/omarchy/logo.txt"`. When that exits 1, run `mkdir -p "$run/artifacts/verify/site-data"` and write `python3 scripts/site-data.py` and `logo missing` into `$run/artifacts/verify/site-data/unreachable.txt`. Do not run the script.
- **Logo present.** Run `drive_gate site-data python3 scripts/site-data.py`. stdout is one line. It starts with `bay.js 192x` and contains `wordmark.svg`. The work tree is dirty after the run. Do not commit that output from this skill.

## Gotchas

- The script writes `site/bay.js` before it reads the logo. A missing logo still changes `bay.js` if you run the script. Check the logo first.
- The output is not byte-stable. Do not run the script from CI or from a check.
- The script fetches the coastline from Overpass. A verification drive does not need that network call.
