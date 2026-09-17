# omahoy.org

Static, no build step: `index.html`, `bay.js` (the chart data), `favicon.svg`,
`wordmark.svg`, and `og.png` for link previews. Every page shares `style.css`
and `theme.js` (the palettes, the picker and the T key).

Each app that runs today has a page: `omakeel/`, `omahelm/`, `omalookout/`
and `omawind/`, each an `index.html`, so Vercel serves them as `/omahelm/`
with no config. Their recordings are in `media/`: an H.264 MP4 each, with a
WebP poster of the same name.

`omatiller/` is a hardware concept page with an interactive CAD assembly, exploded
view, transparent housing view, and ram travel control. Concept 02's 83 named parts
come from `../omatiller/cad/assembly.py` in the sibling repository. The generated
GLB and component metadata (`omatiller-02.glb`, `omatiller-02.json`) live in
`omatiller/model/`; copy them together after regenerating. The viewer reads the
assembly root, stroke, screw axis, nameplate, cable path and camera target from
that metadata, including the text drawn on the nameplate and heading display. A
new revision that keeps the part names (`screw_shaft`, `screw_journal`,
`screw_helix`, `power_gland`) and the group names changes `MODEL` in
`viewer.js` and the page copy; one with a new size may also need the camera
positions and the ground height (y −170) adjusted. `media/omatiller.webp` is a
static render of the same model for fallback and link previews, captured from
headless Chromium at 1398 × 801. This page makes no claim that the pilot is built.

Three.js r180 and its required modules are vendored with their MIT license under
`vendor/three/`. The model and renderer are served locally, without a CDN. Serve
the site over HTTP for module and model loading:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory site
```

Open `http://127.0.0.1:8765/omatiller/`. Rendering stops while idle or offscreen;
rotation and ram animation start only when requested. With the canvas focused,
arrow keys orbit, `+`/`-` zoom, and Home resets the view. Reduced-motion settings
disable transition animation. A static image remains if WebGL or JavaScript is
unavailable.

The recordings are of the real apps, on omakeel's sample sail out of Berkeley
Marina (a replay, not a real track, and its vessels are invented), never on
the boat's real GPS. The wind is live HRRR and NDBC over the Bay. They were
captured with `grim` at 15 fps from a headless Hyprland output, so nothing
had to be on screen, then encoded with ffmpeg at CRF 26. The theme clip is
each theme once its tiles were drawn, crossfaded, so the redraw isn't shown.

`bay.js` and `wordmark.svg` are generated. Regenerate them with:

```sh
python3 scripts/site-data.py
```

The script fetches the OpenStreetMap coastline for the Bay from Overpass once,
caches it in `.cache/`, and fails if any sample AIS route crosses land. The
wordmark is OMAHOY cut from the letters of Omarchy's `logo.txt`. `index.html`
inlines the same path, so paste the new `d` attribute there if it changes.

Press T on the page to cycle through Omarchy's theme palettes. They're copied
from each theme's `colors.toml`. The last one, `night-watch`, is Omahoy's own
red theme for the helm after dark. A link can pick the theme:
`https://omahoy.org/?theme=night-watch`. Every theme's text colors are chosen
to reach at least 4.5:1 contrast.

## Hosting

Vercel, project `omahoy`, imported from this repo with the root directory set
to `site`. Every push to `main` deploys. omahoy.org is registered at GoDaddy
and points at Vercel.

Review a change without deploying:

```sh
chromium --headless=new --disable-gpu --hide-scrollbars --window-size=1280,1600 --screenshot=/tmp/omahoy-site.png file://$PWD/site/index.html
```
