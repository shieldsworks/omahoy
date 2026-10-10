# Working on omahoy

omahoy is the umbrella repo of the Omahoy suite, small Omarchy apps for
sailors, and the source of omahoy.org. Vercel serves `site/` as static
files with no build step, and every push to `main` deploys, so anything
under `site/` is public as soon as it lands. The apps live in their own
repos under github.com/shieldsworks.

There is no Rust in this repo. The suite's Rust rules (the Cargo `[lints]`
table, clippy pedantic, `--locked`, `mise`, QML) are not part of this repo,
and adding them here is not a planned follow-up.

## Toolchain

`git`, `bash` and `python3` 3.10 or newer, standard library only. Chromium
or Google Chrome for the verify skill. Nothing to install.

## Done means verified

You are not done until this passes from the repo root:

```sh
scripts/verify.sh
```

`scripts/verify.sh --list` names each check and the rule it guards.
`scripts/verify.sh <check>...` runs only the checks you name. Every run
ends by checking that verification created or changed no file. CI runs the
same command, then its own `git status` check. Fix what it reports; never
weaken the check that reported it.

To prove a change from a visitor's side (the pages rendering and behaving
in a browser), use the verify skill: `.cursor/skills/verify/SKILL.md` and
its feature map in `.cursor/skills/verify/features/`.

## The gates are not yours to move

These files set the rules and are changed only in a PR whose whole purpose
is changing them:

- `.github/workflows/`
- `scripts/verify.sh`, `scripts/verify.py`, `scripts/check-comments.sh`

Such a PR merges only after review by someone other than its author. The
reviewer is either Casey or Casey's delegated reviewer Dev.

Adding a check is such a PR: one `@check` function in `scripts/verify.py`
and its test class in `scripts/test_verify.py`. Relaxing a check also
needs Casey.

Dev's review means all three. An independent agent verifies the PR head on
a clean checkout. That agent runs the repo's checks and drives the changed
behavior. An adversarial review challenges the change. CI is green on the
exact head SHA merged. The author agent never approves or merges its own
PR.

## Every behavior change has a test or a golden

- A change to `scripts/verify.py` lands with a case in
  `scripts/test_verify.py` that writes a literal mini-site into a temp git
  repo and asserts literal findings. A fix to a check starts with the
  failing case.
- A change a visitor can see is proven by driving its feature-map entry,
  with the evidence in the PR. A new or changed visitor path updates its
  feature file in the same PR.
- `site/bay.js` and `site/wordmark.svg` are `scripts/site-data.py`'s
  goldens. Never edit them by hand. Regenerating them is its own PR that
  says what changed visibly.
- Checks read the checkout and write nothing to it. Tests write only to a
  temp dir.

## No apologetic comments

Comments state facts about the code and the world it handles ("NOAA ships
cells as ISO 8211, so ..."). They do not apologize, defer or hedge. These
fail CI (`scripts/check-comments.sh`): TODO, FIXME, XXX, HACK, workaround,
temporary fix, quick fix, for the time being, not ideal, should be fixed,
sorry, kludge, band-aid. If something is wrong, fix it in this change or
open an issue and leave the code honest. A workaround for an outside bug is
written as the fact: what the outside thing does and what this code does
about it.

The script reads tracked `.rs`, `.qml`, `.js`, `.sh`, `.py`, `.toml` and
`.yml` files, including files in active submodules. It skips
`tests/fixtures/` and `**/vendor/**`. The vendor pathspec does not match
a top-level `vendor/` directory, so those files are scanned. It does not
read HTML, CSS or Markdown, and the rule holds there too, including the
scripts inline in the pages.

"for now" and "temporary file" are not in the pattern. The apps use "for
now" for the current time.

`scripts/check-comments.sh` is the canonical copy. Downstream repos carry
that file byte for byte. The pin is this sha256. A copy with any other
hash is not this rule.

```
f3ec8a2d07e8f1e7c50770d0bb93579fc7bb0d60fc8e359a22607ba55cebc6c6  scripts/check-comments.sh
```

From the repo root, pipe that line to `sha256sum -c`. Run the script
with no arguments to scan the default set.

```sh
scripts/check-comments.sh
```

Pass git pathspecs to scan a different set. Those arguments replace the
default set. The script still excludes its own path.

The script clears shell traps, then unsets `GIT_LITERAL_PATHSPECS`,
`GIT_GLOB_PATHSPECS`, `GIT_NOGLOB_PATHSPECS`, `GIT_ICASE_PATHSPECS`,
`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`,
`GIT_NAMESPACE`, `GIT_OBJECT_DIRECTORY`, and
`GIT_ALTERNATE_OBJECT_DIRECTORIES` before it lists files. `git`, `grep`,
`realpath`, and `mktemp` are the commands on the default `PATH`, so a
shell function or an earlier `PATH` entry cannot replace them. Names come
from `git ls-files -z --recurse-submodules`. The script's own path is
excluded as a literal pathspec. A listed path that is exactly `-` is read
as `./-`, because grep treats `-` as standard input. A search error fails
the run. An empty selection passes only when that listing has no file for
this scan to read.

## Copy the right pattern

You will copy what you see. Before copying, check that what you copy passes
`scripts/verify.sh` today. In particular:

- Internal links are relative and keep the trailing slash: `../omahelm/`,
  never `../omahelm/index.html` or `/omahelm/`.
- `theme.js` loads in `<head>` as a plain `<script src>`, with no `async`,
  `defer` or `type="module"`, so a page paints in its theme from the first
  frame.
- A new app page copies an existing one and adds itself to the nav on every
  app page, in the same order, with `aria-current="page"` on its own link.
- A recording is an H.264 MP4 in `site/media/` with a WebP poster of the
  same name.
- OG and Twitter URLs are absolute `https://omahoy.org/...` URLs.
- Take the shortcut only if it is also the right path. If the right path is
  hard, say so in the PR instead of shipping the shortcut.

## Review: the builder never approves its own work

- The agent that wrote a change does not approve, merge, or mark it
  verified. A different agent (fresh context, clean checkout) or Casey
  reviews it and runs `scripts/verify.sh` plus the verify skill for the
  features the change touches.
- The PR description lists: what changed, the tests or goldens that prove
  it, which feature-map entries were driven, and what was not checked.
- The reviewer reports what it ran and saw, not what it assumes.

## Rules specific to omahoy

- `scripts/site-data.py` needs the Overpass API and
  `~/.local/share/omarchy/logo.txt`. It writes `bay.js` before it reads the
  logo, so check the logo exists before running it. Its output is not
  byte-stable. Never run it in CI or from a check.
- `site/vendor/three/` is Three.js r180, unmodified, with its MIT license.
  Upgrade it whole or not at all.
- `site/omatiller/model/` comes from `cad/assembly.py` in the sibling
  `omatiller` repo. Copy the GLB and the JSON together, and change `MODEL`
  in `site/omatiller/viewer.js` when the revision's name changes.
- The inline wordmark in `site/index.html` and `site/wordmark.svg` are one
  fact stored twice. Paste the new `viewBox` and `d` from `wordmark.svg`;
  never make a third copy.
- Copy states what is true: the recordings replay a sample sail, the
  omatiller pilot is a concept, and Omahoy is not for navigation.
- Minimal dependencies. Ask before adding any tool, package or action to
  verification or CI.
- Agent files (`AGENTS.md`, `.cursor/`, `scripts/`, `.github/`) stay
  outside `site/`, so they are never served.

## Layout

- `site/index.html`: the home page, with the inline wordmark, the Bay chart
  drawn from `bay.js`, the app list and the theme picker.
- `site/{omakeel,omahelm,omalookout,omalogbook,omatide,omawind}/index.html`:
  a page per app, each with the copied nav and recordings from `media/`.
- `site/omatiller/`: the hardware concept page. `viewer.js` is an ES module
  that needs HTTP, `tiller.css` its styles, `model/` the GLB and its
  metadata.
- `site/style.css`, `site/theme.js`: shared by every page.
- `site/media/`: MP4 recordings with same-name WebP posters, plus stills.
- `site/bay.js`, `site/wordmark.svg`: generated by `scripts/site-data.py`.
- `site/vendor/three/`: vendored Three.js r180.
- `site/favicon.svg`, `site/og.png`, `site/README.md`: the icon, the
  link-preview image, and the notes on every page and asset (served too).
- `scripts/verify.sh`, `scripts/verify.py`, `scripts/test_verify.py`: the
  done command, its checks, and their tests.
- `scripts/check-comments.sh`: the canonical comment rule. Downstream
  repos carry a byte-identical copy and check it against the sha256 pin
  in No apologetic comments.
- `scripts/site-data.py`: regenerates `bay.js` and `wordmark.svg`, run by
  hand.
- `.github/workflows/site.yml`: CI.
- `.cursor/skills/verify/`: the reviewer's drive recipe and feature map.
- `README.md`: the suite README on GitHub. Not served.
- `LICENSE`: MIT.
