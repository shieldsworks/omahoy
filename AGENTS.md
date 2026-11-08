# Working on omahoy

omahoy is the umbrella repo of the Omahoy suite, small Omarchy apps for
sailors. It holds the source of omahoy.org and `omahoy`, the setup program
that installs and configures the suite (`docs/onboarding.md`). Vercel serves `site/` as static
files with no build step, and every push to `main` deploys, so anything
under `site/` is public as soon as it lands. The apps live in their own
repos under github.com/shieldsworks.

The `omahoy` crate at the root is the setup program. It keeps the apps' Rust
rules: `mise` pins the toolchain, every cargo command is `--locked`, and the
Cargo `[lints]` table and `clippy.toml` are gates. There is no QML here.

## Toolchain

`git`, `bash`, `tmux`, and `python3` 3.10 or newer, standard library only.
Chromium or Google Chrome for the verify skill. For the crate, run `mise
install` first: `mise.toml` pins Rust 1.98 with rustfmt and clippy.
`Cargo.toml`'s `rust-version` is 1.89, and `clippy.toml`'s `msrv` must
equal it.

## Done means verified

You are not done until this passes from the repo root:

```sh
scripts/verify.sh
```

`scripts/verify.sh --list` names each check and the rule it guards. The
`crate` check runs rustfmt, clippy on all targets with warnings denied and
`cargo test`, through `mise exec`, whenever `Cargo.toml` is at the root.
`scripts/verify.sh <check>...` runs only the checks you name. Every run
ends by checking that verification created or changed no file. CI runs the
same command, then its own `git status` check. Fix what it reports; never
weaken the check that reported it.

To prove a change from a visitor's side (the pages rendering and behaving
in a browser), use the verify skill: `.cursor/skills/verify/SKILL.md` and
its feature map in `.cursor/skills/verify/features/`.

To prove the suite gate the way a reviewer does, use
`.cursor/skills/verify-omahoy/SKILL.md` and its feature map in
`.cursor/skills/verify-omahoy/features/`.

## The gates are not yours to move

These files set the rules and are changed only in a PR whose whole purpose
is changing them:

- `.github/workflows/`
- `scripts/verify.sh`, `scripts/verify.py`, `scripts/check-comments.sh`
- `[lints]` in `Cargo.toml`, and `clippy.toml`

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

## The setup crate

- std only. No crates; the answer to adding one is no.
- No `unwrap()` outside tests, and `expect` only with the invariant that
  makes it safe (clippy, `unwrap_used` and `expect_used`). No `unsafe`
  (`unsafe_code = "forbid"`). A silenced lint says why.
- Every behavior change lands with a test. The tested shapes are: `plan`
  tables over a hand-built `Machine`, `conf::edit` goldens, `Boat`
  parse/render round trips with line-numbered problems, coverage and
  `Area::around` cases, depth math, probe-line parsing, and the questions
  driven through `sys::fake::Script` with `sys::fake::Commands`.
- Tests never run a real command. Commands go through the `Runner` trait,
  the terminal through `Terminal`; the fakes fail any command they weren't
  scripted with. Tests write only to a temp dir.
- `Machine::survey` only reads. `plan` and `findings` are pure. `Step::run`
  in `src/run.rs` is the only code that changes the machine.
- Never replace what someone else made: a file in `~/.local/bin` that isn't
  a link, a link to anything but a build of the same binary, a unit file
  that isn't omahoy's link, or a dev symlink in
  `~/.config/omarchy/plugins`, which is built where it points.
- `docs/onboarding.md` is the contract for what setup asks, writes and
  runs. Change it in the same commit as the behavior.
- To prove a change, run `target/release/omahoy doctor` and a scripted
  setup with `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and
  `XDG_STATE_HOME` pointed at a temp dir and stub `omarchy`, `systemctl`,
  `pacman`, `sudo` and `sg` first on `PATH`. Never run `apply` against a
  real home to test it.

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

The script's first command is `trap - EXIT ERR DEBUG RETURN`. It then
unsets `GIT_LITERAL_PATHSPECS`, `GIT_GLOB_PATHSPECS`,
`GIT_NOGLOB_PATHSPECS`, `GIT_ICASE_PATHSPECS`, `GIT_DIR`,
`GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, `GIT_NAMESPACE`,
`GIT_OBJECT_DIRECTORY`, and `GIT_ALTERNATE_OBJECT_DIRECTORIES` before
it lists files. `git`, `grep`, `realpath`, and `mktemp` are run with
`command -p`, which uses the default `PATH` and does not call a function
named `git`, `grep`, `realpath`, or `mktemp`. Names come from
`git ls-files -z --recurse-submodules`. The script's own path is
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
  reviews it and runs `scripts/verify.sh`. For the features the change
  touches, that agent drives the pages with
  `.cursor/skills/verify/SKILL.md` and the suite gate with
  `.cursor/skills/verify-omahoy/SKILL.md`.
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

- `omahoy`: the bootstrap. It gets mise, the pinned Rust, builds the crate
  and execs `target/release/omahoy`.
- `Cargo.toml`, `Cargo.lock`, `mise.toml`, `clippy.toml`, `src/`: the
  setup crate. `docs/onboarding.md` has its code map.
- `docs/onboarding.md`: what setup asks, writes and runs.

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
- `.cursor/skills/verify/`: the visitor drive of the pages, and its feature map.
- `.cursor/skills/verify-omahoy/`: the reviewer drive of the suite gate, the comment pin, and `scripts/check-comments.sh`.
- `README.md`: the suite README on GitHub. Not served.
- `LICENSE`: MIT.
