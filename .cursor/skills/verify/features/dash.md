# Dash AI shed

The build-plan page at `/dash.html`: the facts about Dash as she is today,
the phased house-power plan, three inline SVG diagrams (wiring, data, cost),
a shopping list with manufacturer links, and the checklist for the next
boat visit.

## Sub-features

- `dash-render` the page paints with its three diagrams and the shopping list
- `dash-from-home` the Aboard Dash section on the home page links here
- `dash-from-install` the install guide links here

## How to get to it (user POV)

- Open `https://omahoy.org/dash.html`. Locally, `$base/dash.html`.
- The Aboard Dash section on the home page.
- The pager on the install guide.

## Driving it with headless Chromium

Preconditions:

- The baseline in `README.md`: Launch and Doctor pass, with a fresh `$run/profile`.

- **Render.** Open the build-plan page. Run `drive dash-render "$base/dash.html"`. `page.png` shows the hero and the start of the schematic. `dom.html` contains `Single-line schematic`, `Illustrative, not an installation drawing`, `Data and compute`, `Shopping list`, and `Next time at the boat`.
- **From home.** From the home-page `home-render` artifacts, `grep -o 'href="dash.html"' "$run/artifacts/verify/home-render/dom.html"` prints `href="dash.html"`.
- **From install.** From the install `install-render` artifacts, `grep -o 'href="../dash.html"' "$run/artifacts/verify/install-render/dom.html"` prints at least one `href="../dash.html"`.

## Gotchas

- The page is `site/dash.html`, not `site/dash/index.html`. The app-nav check does not treat it as an app page, and a directory link `dash/` would fail the links check.
- The diagrams are inline SVG that use theme CSS variables, so they follow the picker and the T key. They are not images in `media/`.
- Prices with a date were read on a product page on 11 Nov 2026; the other figures stay labeled as estimates.
