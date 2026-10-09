# Omatiller

The tiller pilot concept as a 3D assembly the visitor can orbit, explode,
open and run, with a static render and a note when 3D can't load or
JavaScript is off.

## Sub-features

- `omatiller-load` the interactive assembly loads and says so
- `omatiller-no-webgl` without WebGL, the static render stays, with a note saying 3D is unavailable
- `omatiller-no-js` without JavaScript, the static render stays, with an "Enable JavaScript" note

## How to get to it (user POV)

- The omatiller link in any app page's nav, or in the home page's app list.
- `https://omahoy.org/omatiller/`.

## Driving it with headless Chromium

Preconditions:

- The baseline in `README.md`. Served over HTTP: from `file://` the module and the model cannot load.

- **Load.** Open the page. Run `BUDGET=15000 drive omatiller-load "$base/omatiller/"`. `grep -o 'id="load-state"[^>]*>[^<]*' "$run/artifacts/verify/omatiller-load/dom.html"` ends in `Interactive assembly ready.`, and `page.png` shows the rendered model on the dark stage.
- **No WebGL.** Run `drive omatiller-no-webgl "$base/omatiller/" --disable-webgl`. `#load-state` reads `Static preview · interactive 3D is unavailable in this browser.`, and `page.png` shows the static render under that note.
- **No JavaScript.** Unreachable with this harness. With `--blink-settings=scriptEnabled=false`, Chrome 154 exits 0 and writes neither `page.png` nor `dom.html` (measured). Report `omatiller-no-js` unreachable, or open the page by hand in a desktop browser with JavaScript off and screenshot the static render and the "Enable JavaScript to orbit the model" note.

## Gotchas

- Headless Chrome 154 on the cloud VM rendered the model with software WebGL (measured). A browser with no WebGL at all reaches only the fallback; then `omatiller-load` is unreachable, not passed.
- The GLB is 2.8 MB. A shorter budget can capture it mid-load, with `#load-state` still `Loading the assembly…`. Raise `BUDGET` before calling that a failure.
- A new model revision changes `MODEL` in `viewer.js` and copies the GLB and the JSON together. `scripts/verify.sh` fails when either file is missing.
