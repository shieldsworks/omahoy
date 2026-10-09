# Theme

Every page paints in an Omarchy theme from the first frame. A link can pick
one with `?theme=`, the picker and the T key change it, and a pick (not a
link) is remembered for the next visit.

## Sub-features

- `theme-query` `?theme=night-watch` paints the page red on black, on the home page and on an app page
- `theme-query-unknown` an unknown name falls back to the default
- `theme-default` a link's theme is applied without being remembered
- `theme-picker` a picker button applies a theme and remembers it
- `theme-key` T cycles to the next theme, Shift+T to the previous one

## How to get to it (user POV)

- `https://omahoy.org/?theme=night-watch`, or the same query on any page.
- The "See it here" link under Night watch on the home page.
- The Theme picker on every page.
- The T key anywhere outside a text field.

## Driving it with headless Chromium

Preconditions:

- The baseline in `README.md`, with a fresh `$run/profile`. Run these in order, in the same `$run`.

- **Query.** Open a shared link. Run `drive theme-query "$base/?theme=night-watch"` and `drive theme-query-app "$base/omatide/?theme=night-watch"`. For each, `grep -o '<html[^>]*>' "$run/artifacts/verify/<id>/dom.html" | grep -o 'data-theme="[^"]*"\|--bg: #[0-9a-f]*'` prints `data-theme="night-watch"` and `--bg: #0c0404`, and `page.png` is red on black.
- **Unknown.** Run `drive theme-query-unknown "$base/?theme=nope"`. The same read of `<html>` prints `rose-pine` or `tokyo-night`, never `nope`.
- **Not remembered.** Right after Query, in the same profile, run `drive theme-default "$base/"`. The same read of `<html>` does not print `night-watch`.
- **Picker and T.** These need a click and a key press, which the headless command line cannot send. Report `theme-picker` and `theme-key` unreachable with this harness, or drive them by hand in a desktop browser with a screenshot per step.

## Gotchas

- The default follows `prefers-color-scheme`, and headless Chrome 154 reports light, so the default is `rose-pine` (measured). Read `data-theme`; don't assume `tokyo-night`.
- A reused profile carries `localStorage` (`omahoy-theme`) from earlier runs, which changes the default. Start from a fresh `$run/profile`.
- Every picker button has its own `data-theme`, so a bare `grep data-theme` over `dom.html` prints eight lines. Read it off `<html>`.
