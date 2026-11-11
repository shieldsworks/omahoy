# Install guide

The install page at `/install/`: clone the repo, then pick a path — the
Setup panel (the default, for Omarchy) or the Terminal — to run
`./omahoy panel` or `./omahoy`. Shared prerequisites sit above the tabs.
The page has real terminal stills of help, doctor, the questions and the
plan, and a six-page walkthrough of the Omarchy setup panel.

## Sub-features

- `install-render` the page paints with its terminal stills and the six panel stills
- `install-tabs` the Setup panel and Terminal tabs, with `#panel` and `#terminal` opening the matching path
- `install-nav` the nav lists every app page, then install, and marks install
- `install-home` the home page Install section links here as `install/`

## How to get to it (user POV)

- Open `https://omahoy.org/install/`. Locally, `$base/install/`.
- `https://omahoy.org/install/#panel` or `https://omahoy.org/install/#terminal`.
- The Install section on the home page, and the Install omahoy button in the hero.
- The last link in the nav on any app page.
- The pager on the Dash AI shed page.

## Driving it with headless Chromium

Preconditions:

- The baseline in `README.md`: Launch and Doctor pass, with a fresh `$run/profile`.

- **Render.** Open the install page. Run `drive install-render "$base/install/"`. `page.png` shows the hero, the clone step, and the Setup panel tab. `dom.html` contains `src="../media/omahoy-help.webp"`, `src="../media/omahoy-setup-questions.webp"`, `src="../media/omahoy-setup-plan.webp"`, `src="../media/omahoy-doctor.webp"`, `src="../media/omahoy-panel-boat.webp"`, `src="../media/omahoy-panel-hardware.webp"`, `src="../media/omahoy-panel-home.webp"`, `src="../media/omahoy-panel-apps.webp"`, `src="../media/omahoy-panel-logbook.webp"`, and `src="../media/omahoy-panel-review.webp"`. It does not contain `Screenshot not captured`. It contains `role="tablist"`, `href="#panel"`, and `href="#terminal"`. The Setup panel tab is `aria-selected="true"`.
- **Tabs.** Run `drive install-terminal "$base/install/#terminal"`. In `dom.html`, the Terminal tab is `aria-selected="true"` and the Setup panel tab is `aria-selected="false"`. Run `drive install-panel "$base/install/#panel"`. The Setup panel tab is `aria-selected="true"`.
- **Nav.** From the install-render artifacts, `grep -o 'href="[^"]*" aria-current="page"' "$run/artifacts/verify/install-render/dom.html"` prints `href="./" aria-current="page"`.
- **Home.** From the home-page `home-render` artifacts, `grep -c 'href="install/"' "$run/artifacts/verify/home-render/dom.html"` prints 2: the hero Install omahoy button and the Install section.

## Gotchas

- The stills are WebP screenshots, not `<video>` recordings, so the reduced-motion player-controls check on app pages does not apply here.
- The six panel stills are a real run of `./omahoy panel` on an Omarchy machine, for Dash, stopped before Go. The home-port coordinates, the line that reads them back, the sailing-area bounds, and a local network source are blurred because the site is public.
- The terminal first-run stills use the documented example boat Wavelength, and they stop before Go. They are not a recording from Dash.
- Without JavaScript both path sections stay stacked, and the tab links are ordinary `#panel` and `#terminal` anchors. With JavaScript the default (no hash) is the Setup panel; the Terminal stills stay in the DOM, hidden.
