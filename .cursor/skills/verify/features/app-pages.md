# App pages

Each app that runs today has a page that tells its story with recordings of
the real app on omakeel's sample sail, and a nav across the top that links
every app page with the current one marked.

## Sub-features

- `app-render` the page paints with its recordings and their posters
- `app-nav` the nav lists every app page and marks this one
- `app-reduced-motion` with reduced motion on, the recordings wait, with player controls

## How to get to it (user POV)

- The nav at the top of any app page.
- The app list on the home page.
- A direct link such as `https://omahoy.org/omahelm/`.

## Driving it with headless Chromium

Preconditions:

- The baseline in `README.md`.
- Drive `omahelm/` at least, plus every app page the change touched, by swapping the directory in each command.

- **Render.** Open the page. Run `drive app-render "$base/omahelm/"`. `page.png` shows the hero and the first recording. `dom.html` contains `poster="../media/omahelm-wind.webp"` and `poster="../media/omahelm-themes.webp"`.
- **Nav.** Same run. `grep -o 'href="[^"]*" aria-current="page"' "$run/artifacts/verify/app-render/dom.html"` prints this page's own link, `href="../omahelm/" aria-current="page"`. On the omatiller page that link is `href="./"`.
- **Reduced motion.** Turn on reduced motion. Run `drive app-reduced-motion "$base/omahelm/" --force-prefers-reduced-motion`. `grep -o '<video[^>]*>' "$run/artifacts/verify/app-reduced-motion/dom.html" | grep -c 'controls=""'` prints `2`, one per recording, where the same count on `app-render` prints `0`. `page.png` shows the player bar on the first recording.

## Gotchas

- `--force-prefers-reduced-motion` works in Chrome 154 (measured). When a browser ignores it, the count stays `0`; report `app-reduced-motion` unreachable with that browser, not failed and not passed.
- Google Chrome plays the H.264 recordings (Chrome 154 loaded the clip and its player reads `0:00 / 0:16`, measured). A Chromium build without proprietary codecs may show only the poster, so playback is unreachable there.
- The pager at the bottom is a second nav, `aria-label="More apps"`, curated by hand on each page. It is not the app nav.
