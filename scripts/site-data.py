#!/usr/bin/env python3
"""Build the generated files for omahoy.org.

site/bay.js       San Francisco Bay drawn on the page's dot grid, with labels,
                  Dash's berth and the sample AIS routes. Coastline is
                  OpenStreetMap (ODbL), fetched from Overpass.
site/wordmark.svg OMAHOY, cut letter by letter from Omarchy's own logo.

    python3 scripts/site-data.py [path/to/omarchy/logo.txt]
"""
import base64
import json
import math
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"

# Point Bonita to Oakland, Candlestick to Richmond.
SOUTH, WEST, NORTH, EAST = 37.72, -122.54, 37.93, -122.24
WIDTH = 192  # dots across; the height follows from Mercator

OVERPASS = "https://overpass-api.de/api/interpreter"

WATER, LAND, COAST = 0, 1, 2

# (text, lat, lon, kind). Water names are drawn in italics, as on a paper chart.
PLACES = [
    ("SAN FRANCISCO", 37.772, -122.440, "land"),
    ("OAKLAND", 37.806, -122.268, "land"),
    ("BERKELEY", 37.872, -122.278, "land"),
    ("SAUSALITO", 37.872, -122.515, "land"),
    ("ANGEL I.", 37.862, -122.432, "land"),
    ("ALCATRAZ", 37.8267, -122.4230, "land"),
    ("TREASURE I.", 37.826, -122.371, "land"),
    ("Golden Gate", 37.8130, -122.4990, "water"),
    ("San Francisco Bay", 37.842, -122.378, "water"),
]

BERTH = ("DASH", 37.8663, -122.3148)  # Berkeley Marina

# Sample traffic for the illustration. Each route is checked against the grid
# so no target sails across land.
ROUTES = [
    {"name": "FERRY", "knots": 24, "loop": False, "points": [
        (37.7960, -122.3890), (37.8150, -122.3950), (37.8450, -122.4050),
        (37.8800, -122.4050), (37.9250, -122.4400)]},
    {"name": "CARGO", "knots": 11, "loop": False, "points": [
        (37.8120, -122.5350), (37.8190, -122.4785), (37.8120, -122.4450),
        (37.8150, -122.4100), (37.8000, -122.3780), (37.7980, -122.3500),
        (37.7985, -122.3300)]},
    {"name": "SAILING", "knots": 5, "loop": True, "points": [
        (37.8550, -122.3450), (37.8700, -122.3700), (37.8850, -122.3500),
        (37.8650, -122.3350)]},
]

# Letters of Omarchy's logo.txt as (first, last + 1) columns.
LOGO_LETTERS = {"O": (0, 9), "M": (11, 26), "A": (27, 37), "H": (59, 71), "Y": (72, 81)}
LOGO_ROWS = 9  # the tenth row only carries the R's leg


def merc(lat):
    return math.log(math.tan(math.pi / 4 + math.radians(lat) / 2))


HEIGHT = round(WIDTH * (merc(NORTH) - merc(SOUTH)) / math.radians(EAST - WEST))


def to_dot(lat, lon):
    x = (lon - WEST) / (EAST - WEST) * WIDTH
    y = (merc(NORTH) - merc(lat)) / (merc(NORTH) - merc(SOUTH)) * HEIGHT
    return x, y


def fetch_coastline():
    """Coastline ways, cached in .cache/ because Overpass is often busy."""
    cache = ROOT / ".cache" / "coastline.json"
    if cache.exists():
        try:
            elements = json.loads(cache.read_text())["elements"]
            if elements:
                return elements
        except (ValueError, KeyError):
            pass
        cache.unlink()  # a bad download; fetch it again
    pad = 0.06
    query = (f'[out:json][timeout:120];way["natural"="coastline"]'
             f'({SOUTH - pad},{WEST - pad},{NORTH + pad},{EAST + pad});out geom;')
    req = urllib.request.Request(
        OVERPASS, data=urllib.parse.urlencode({"data": query}).encode(),
        headers={"User-Agent": "omahoy-site (github.com/shieldsworks/omahoy)"})
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                body = r.read()
            break
        except urllib.error.HTTPError as e:
            if e.code not in (429, 504) or attempt == 3:
                raise
            print(f"Overpass busy ({e.code}), retrying in {20 * attempt} s", file=sys.stderr)
            time.sleep(20 * attempt)
    elements = json.loads(body)["elements"]
    if not elements:
        sys.exit("Overpass returned no coastline; nothing cached, try again later")
    cache.parent.mkdir(exist_ok=True)
    cache.write_bytes(body)
    return elements


def draw_line(grid, a, b):
    x0, y0 = int(a[0]), int(a[1])
    x1, y1 = int(b[0]), int(b[1])
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        if 0 <= x0 < WIDTH and 0 <= y0 < HEIGHT:
            grid[y0][x0] = COAST
        if x0 == x1 and y0 == y1:
            return
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


def build_grid(ways):
    UNSEEN = 3
    grid = [[UNSEEN] * WIDTH for _ in range(HEIGHT)]
    for way in ways:
        pts = [to_dot(p["lat"], p["lon"]) for p in way.get("geometry", [])]
        for a, b in zip(pts, pts[1:]):
            draw_line(grid, a, b)
    # Flood the water in from the Pacific, four-connected so it can't slip
    # between the diagonal steps of a coastline.
    sx, sy = map(int, to_dot(37.760, -122.532))
    assert grid[sy][sx] == UNSEEN, "ocean seed is on the coastline"
    stack = [(sx, sy)]
    while stack:
        x, y = stack.pop()
        if not (0 <= x < WIDTH and 0 <= y < HEIGHT) or grid[y][x] != UNSEEN:
            continue
        grid[y][x] = WATER
        stack += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    return [[LAND if c == UNSEEN else c for c in row] for row in grid]


def at(grid, lat, lon):
    x, y = to_dot(lat, lon)
    return grid[int(y)][int(x)]


def check(grid):
    land = [("downtown SF", 37.790, -122.410), ("Angel Island", 37.862, -122.432),
            ("Treasure Island", 37.824, -122.371), ("Berkeley", 37.870, -122.270)]
    water = [("Golden Gate", 37.8190, -122.4785), ("mid-Bay", 37.840, -122.380)]
    bad = [n for n, la, lo in land if at(grid, la, lo) != LAND]
    bad += [n for n, la, lo in water if at(grid, la, lo) != WATER]
    for route in ROUTES:
        pts = route["points"] + (route["points"][:1] if route["loop"] else [])
        for (la0, lo0), (la1, lo1) in zip(pts, pts[1:]):
            for i in range(41):
                t = i / 40
                la, lo = la0 + (la1 - la0) * t, lo0 + (lo1 - lo0) * t
                if at(grid, la, lo) != WATER:
                    bad.append(f"{route['name']} crosses land at {la:.4f},{lo:.4f}")
                    break
    if bad:
        sys.exit("chart check failed:\n  " + "\n  ".join(bad))


def pack(grid):
    flat = [c for row in grid for c in row]
    flat += [0] * (-len(flat) % 4)
    out = bytearray()
    for i in range(0, len(flat), 4):
        out.append(flat[i] | flat[i + 1] << 2 | flat[i + 2] << 4 | flat[i + 3] << 6)
    return base64.b64encode(bytes(out)).decode()


def dot(lat, lon):
    x, y = to_dot(lat, lon)
    return [round(x, 2), round(y, 2)]


def write_bay(grid):
    nm_per_dot = (EAST - WEST) * math.cos(math.radians((NORTH + SOUTH) / 2)) * 60 / WIDTH
    data = {
        "w": WIDTH, "h": HEIGHT, "nmPerDot": round(nm_per_dot, 5), "grid": pack(grid),
        "places": [{"text": t, "at": dot(la, lo), "kind": k} for t, la, lo, k in PLACES],
        "berth": {"text": BERTH[0], "at": dot(BERTH[1], BERTH[2]),
                  "lat": BERTH[1], "lon": BERTH[2]},
        "routes": [{"name": r["name"], "knots": r["knots"], "loop": r["loop"],
                    "points": [dot(la, lo) for la, lo in r["points"]]} for r in ROUTES],
    }
    (SITE / "bay.js").write_text(
        "// Generated by scripts/site-data.py. Do not edit.\n"
        "// Coastline © OpenStreetMap contributors, ODbL.\n"
        f"window.OMAHOY_BAY = {json.dumps(data, separators=(',', ':'))};\n")


def write_wordmark(logo_path):
    rows = logo_path.read_text().split("\n")[:LOGO_ROWS]
    width = max(len(r) for r in rows)
    rows = [r.ljust(width) for r in rows]
    cells = [""] * LOGO_ROWS
    for i, letter in enumerate("OMAHOY"):
        a, b = LOGO_LETTERS[letter]
        for y in range(LOGO_ROWS):
            cells[y] += ("  " if i else "") + rows[y][a:b]
    cols = len(cells[0])
    # Each character cell is 1 wide and 2 tall; half blocks fill one half.
    halves = {"█": (True, True), "▀": (True, False), "▄": (False, True)}
    path = []
    for y, row in enumerate(cells):
        for half in (0, 1):
            x = 0
            while x < cols:
                if halves.get(row[x], (False, False))[half]:
                    start = x
                    while x < cols and halves.get(row[x], (False, False))[half]:
                        x += 1
                    path.append(f"M{start} {y * 2 + half}h{x - start}v1h-{x - start}z")
                else:
                    x += 1
    (SITE / "wordmark.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {cols} {LOGO_ROWS * 2}">'
        f'<path d="{"".join(path)}"/></svg>\n')
    return cols


def main():
    logo = Path(sys.argv[1] if len(sys.argv) > 1
                else Path.home() / ".local/share/omarchy/logo.txt")
    grid = build_grid(fetch_coastline())
    check(grid)
    write_bay(grid)
    cols = write_wordmark(logo)
    water = sum(row.count(WATER) for row in grid) / (WIDTH * HEIGHT)
    print(f"bay.js {WIDTH}x{HEIGHT} dots, {water:.0%} water; wordmark.svg {cols} columns")


if __name__ == "__main__":
    main()
