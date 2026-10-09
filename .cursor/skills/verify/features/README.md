# omahoy verification map

This directory is the maintained source for verifying what a visitor to
omahoy.org sees. Read this index before driving the site, then use the
matching feature file as the recipe. Keep it current: a change that adds or
alters a visitor path updates its feature file in the same PR.
`scripts/verify.sh` fails when a feature file here is not linked below or
its headings drift from the contract.

## Baseline preconditions

- This checkout served per `../SKILL.md`, from its own temp `$run`
  directory (server log, browser profile, artifacts).
- Doctor passes and names this run's server and a browser binary.
- A fresh `$run/profile`, so no theme remembered by an earlier run leaks in.
- Never drive omahoy.org or a server this run did not start.

## Driving conventions

- Start every recipe from the baseline unless its preconditions say otherwise.
- Commands are literal; keep quoted names and flags unchanged.
- Prefer stable handles: URLs, element ids (`#ais`, `#load-state`), the
  `data-theme` on `<html>`, `aria-current`. Not screen coordinates.
- Nothing here changes the checkout. Don't delete proof during cleanup.

## Proof and skip reporting

- Page proof: `page.png`, `dom.html` and `command.txt` (the URL, flags and
  virtual-time budget) under `$run/artifacts/verify/<feature-id>/`.
- State proof: a read-only second look at `dom.html`, such as
  `grep -o '<html[^>]*>' "$run/artifacts/verify/theme-query/dom.html" | grep -o 'data-theme="[^"]*"'`.
  Read `data-theme` off `<html>`: every picker button carries one too.
- Record the feature ID and entry point with every artifact.
- A missing `page.png` or an empty `dom.html` means the run did not reach
  the page.
- An unreachable path is reported with the command tried and the unmet
  precondition, never as verified through a different path.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph describing the
user-visible behavior, then exactly these four H2s, in order:

1. `Sub-features`: short IDs, one line each.
2. `How to get to it (user POV)`: every user entry point.
3. `Driving it with <harness>`: starts with `Preconditions:`, then labeled
   bullets pairing each user action with an exact command and the
   observable result.
4. `Gotchas`: traps that waste or invalidate a run.

Keep implementation details out of the map. Name user paths, stable
handles, required state, commands and observable proof.

## Features

- [Home page](./home.md) covers the wordmark, the Bay chart and its sample traffic, and the app list.
- [App pages](./app-pages.md) covers each app page's recordings, its nav, and reduced motion.
- [Theme](./theme.md) covers the `?theme=` link, the default, the picker and the T key.
- [Omatiller](./omatiller.md) covers the 3D viewer over HTTP and its static fallbacks.
