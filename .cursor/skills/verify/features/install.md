# Install guide

The install page at `/install/`: the `./omahoy` and `./omahoy panel` commands,
the questions a first run asks, real terminal stills of help, doctor, the
questions and the plan, and a clearly marked placeholder where the setup
panel could not be captured.

## Sub-features

- `install-render` the page paints with its stills and the panel placeholder
- `install-nav` the nav lists every app page, then install, and marks install
- `install-home` the home page Install section links here as `install/`

## How to get to it (user POV)

- Open `https://omahoy.org/install/`. Locally, `$base/install/`.
- The Install section on the home page.
- The last link in the nav on any app page.
- The pager on the Dash AI shed page.

## Driving it with headless Chromium

Preconditions:

- The baseline in `README.md`: Launch and Doctor pass, with a fresh `$run/profile`.

- **Render.** Open the install page. Run `drive install-render "$base/install/"`. `page.png` shows the hero and the first still. `dom.html` contains `src="../media/omahoy-help.webp"`, `src="../media/omahoy-setup-questions.webp"`, `src="../media/omahoy-setup-plan.webp"`, `src="../media/omahoy-doctor.webp"`, and the text `Screenshot not captured`.
- **Nav.** Same run. `grep -o 'href="[^"]*" aria-current="page"' "$run/artifacts/verify/install-render/dom.html"` prints `href="./" aria-current="page"`.
- **Home.** From the home-page `home-render` artifacts, `grep -c 'href="install/"' "$run/artifacts/verify/home-render/dom.html"` prints 2: the hero Install omahoy button and the Install section.

## Gotchas

- The stills are WebP screenshots, not `<video>` recordings, so the reduced-motion player-controls check on app pages does not apply here.
- The panel placeholder is deliberate: the stills were captured on a machine without Quickshell or an Omarchy shell.
- The first-run stills use the documented example boat Wavelength, and they stop before Go. They are not a recording from Dash.
