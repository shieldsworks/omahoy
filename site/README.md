# omahoy.org

Static, no build step: `index.html`, `bay.js` (the chart data), `favicon.svg`,
`wordmark.svg`, and `og.png` for link previews.

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
