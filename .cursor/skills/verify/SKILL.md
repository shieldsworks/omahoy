---
name: verify-omahoy
description: "Drive omahoy.org the way a visitor does (the static site over local HTTP, in headless Chromium) and capture proof. Use before calling a visible site change done, when reviewing someone else's site change, or when asked to verify omahoy."
---

# Verify omahoy

`scripts/verify.sh` proves the tree is sound: every local URL resolves,
every recording has its poster, the inline wordmark matches its file,
`theme.js` loads first, and the app navs agree. `scripts/verify.sh --list`
names each check. This skill proves the pages render and behave in a
browser, served from this checkout. It does not re-check what verify
checks. Run both. The agent that built a change does not sign off on this
run.

## Launch

From the repo root:

```sh
run=$(mktemp -d /tmp/omahoy-verify.XXXXXX)
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
python3 -m http.server "$port" --bind 127.0.0.1 --directory site >"$run/server.log" 2>&1 & echo $! >"$run/pid"
base=http://127.0.0.1:$port
for i in $(seq 50); do curl -fsS "$base/" -o /dev/null 2>/dev/null && break; sleep 0.2; done
```

Ready when `curl -fsS "$base/"` succeeds. The server writes only to
`$run/server.log`, never into the checkout.

## Doctor

One read-only check before driving, and whenever anything looks off:

```sh
kill -0 "$(cat "$run/pid")"
curl -fsS "$base/theme.js" | cmp - site/theme.js && echo "serving this checkout at $base"
browser=$(command -v chromium || command -v chromium-browser || command -v google-chrome-stable || command -v google-chrome) && "$browser" --version
```

It must show a live server this run started, serving this checkout's
`site/`, and a browser binary. Never drive omahoy.org itself or a server
this run did not start.

`google-chrome-stable` comes before `google-chrome` because the cloud VM's
`google-chrome` is a wrapper that adds `--remote-debugging-port`, and with
it headless Chrome never exits after `--screenshot`. `drive` bounds each
browser call with `timeout 60` so a hang ends as a missing artifact.

## Drive

One shell function, used by every feature file. The first argument is the feature id, the second is the URL, and the rest are Chromium flags. Set `BUDGET` to override the 5000 ms default.

```sh
drive() {
  local out="$run/artifacts/verify/$1" url=$2 budget=${BUDGET:-5000}; shift 2; mkdir -p "$out"
  timeout 60 "$browser" --headless=new --hide-scrollbars --user-data-dir="$run/profile" --window-size=1280,1600 \
    --virtual-time-budget="$budget" "$@" --screenshot="$out/page.png" "$url" 2>"$out/stderr.txt"
  timeout 60 "$browser" --headless=new --user-data-dir="$run/profile" --virtual-time-budget="$budget" "$@" \
    --dump-dom "$url" >"$out/dom.html" 2>>"$out/stderr.txt"
  printf '%s\n' "BUDGET=$budget $url $*" >"$out/command.txt"
}
```

Follow the feature map in `features/`, starting with its `README.md`. A
proof that drives one convenient entry point is incomplete when the map
lists others.

## Evidence

- `$run/artifacts/verify/<feature-id>/` holds `command.txt` (the URL, flags
  and budget), `page.png`, `dom.html` and `stderr.txt`.
- Check state with a second, read-only look at `dom.html` (the
  `data-theme` on `<html>`, the `#load-state` text), not only the
  screenshot.
- Exercise the real visitor path: URLs a visitor can type or click, with
  the browser flags the feature file names. No test-only pages, no
  injected scripts.
- A missing `page.png` or an empty `dom.html` is an unreachable run, never
  a pass. `stderr.txt` is mostly D-Bus noise on a box without a session
  bus; read it when an artifact is missing.
- Report anything you could not reach, with the command tried and the
  missing precondition. A skipped entry point is never reported as passed.

## Cleanup

Kill only the pid this run wrote. Do not use `pkill` by name.

```sh
kill "$(cat "$run/pid")" 2>/dev/null || true
rm -rf "$run/profile"
```

Evidence stays in `$run/artifacts/`. Check it still exists after cleanup
(`ls "$run/artifacts/verify"`), and attach the screenshots to the PR.
