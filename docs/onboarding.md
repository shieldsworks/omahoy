# Onboarding: one boat file, one command

A new sailor clones omahoy and runs `./omahoy`. Setup asks about the boat, its hardware and where it sails, saves the answers to `~/.config/omahoy/boat.toml`, and makes the machine match. The same command, run again, checks and repairs. The apps never read `boat.toml`; each one keeps its own config file and still stands alone.

## Usage

```
$ git clone https://github.com/shieldsworks/omahoy && cd omahoy
$ ./omahoy
```

`./omahoy` is a short bash bootstrap. Its only job is to reach Rust: it gets mise if it is missing (`omarchy pkg add mise`), runs `mise trust` and `mise install` for the toolchain pinned in this repo, builds the `omahoy` crate with `--release --locked`, and execs `target/release/omahoy "$@"`. Everything after that is Rust, std only, in `src/`.

The probe that finds receivers is `omakeel probe`, so omakeel has to be built before the hardware question. When it isn't built, or its build is stale, setup says so, clones it if needed and builds it first (a minute or two), then asks the questions. An omakeel already on PATH as a link into `<checkout>/target/release/omakeel` is that checkout, and is used where it is.

The first run asks the following, in order. Hardware comes before home, so a GPS fix can be the home port.

1. Boat name.
2. Depth units: feet, meters or fathoms.
3. Draft, and the clearance wanted under the keel, both in those units. The clearance offered is 2 ft, 0.6 m or 0.5 fathoms. Both are rounded to two decimals as they are typed, so the first apply works from the numbers boat.toml keeps; a draft that rounds to zero is refused. When a re-run changes the units, the offered draft and clearance are converted (1 fathom is 6 ft, 1 m is 3.28 ft) and rounded the same way.
4. Hardware. Plug in receivers, then press Enter. omakeel probes each serial device and lists what it heard: GPS, AIS, silent, or permission denied. Then an optional network source (`host:port`). If nothing is found, the hub runs the demo sail.
5. Home port. The default is the GPS fix when there is one; otherwise type `lat, lon`. Degrees-and-minutes forms are accepted too.
6. Sailing area. It is derived from home, about ±1° in each direction, and shown with what the US data covers there: NOAA charts, NOAA tides, HRRR wind.
7. Apps, each with a reason when it can't work where the boat sails.
8. Logbook folder (default `~/Logbook`), and whether to keep it in git. Asked only when omalogbook is chosen.

Setup saves `boat.toml`, then shows the plan: packages, clones, builds, links, plugins, config keys, services, downloads. It asks "Go?" and applies it. "No" applies nothing; `omahoy apply` picks it up later. Apart from building omakeel first, nothing on the machine changes before "Go?". Ending the questions early stops setup before anything is saved, and it says so.

On a machine that already has app configs, the first run takes its defaults from them: omalogbook's `boat`, `vault` and `git`, omahelm's `units`, omatide's or omawind's `home` and `region`, omakeel's `source` lines, and the apps whose plugin folders exist. An omalogbook config without a `boat` line offers "Dash" as the name, because that was omalogbook's default and its log entries already carry it. With no omalogbook config there is no default name.

After the first run:

```
omahoy            # boat.toml exists: show it, offer to change a section, then plan and apply
omahoy doctor     # read-only: what's wrong, what apply would do; exit 1 if anything
omahoy apply      # make the machine match boat.toml; a no-op when healthy (--yes skips the prompt)
omahoy hardware   # re-run only the hardware question
```

A source that was in boat.toml but isn't heard on a re-run is offered back ("Keep it from before?", default no), so an unplugged receiver is never dropped by surprise. Re-running the hardware question never moves home or the area: the home question is asked only when boat.toml has no home. To move home, change the home section.

## boat.toml

It uses the same flat `key = value` dialect as every app config, but it is parsed strictly. A bad line stops `apply` and names the line number. A wrong value copied into five app configs does more harm than stopping does.

```
name      = "Wavelength"
home      = 47.687, -122.413
area      = 46.687,-123.513,48.687,-121.313
units     = feet
draft     = 5.5
clearance = 2
source    = serial:/dev/serial/by-id/usb-Prolific_Technology_Inc._USB-Serial_Controller-if00-port0:4800
source    = serial:/dev/serial/by-id/usb-dAISy_AIS_Receiver_0001-if00:38400
apps      = helm, lookout, logbook, tide, wind
vault     = ~/Logbook
git       = true
```

The rules for the file:

- Every key is required once, except `source`, which repeats. An unknown key, a repeated key or a line without `=` is a problem with its line number. `omahoy apply` and `omahoy doctor` stop and list every problem.
- `source = demo` must be the only `source` line.
- A serial source names its baud (`serial:PATH:BAUD`). The probe found it, and omakeel's 4800 default would be a silent guess. `tcp:HOST:PORT` and `replay:FILE` are accepted too.
- `area` must contain `home` and must not cross the antimeridian.
- `apps` lists the chosen apps; omakeel always comes and is not listed. An app that can't work in `area` (omatide outside NOAA, omawind outside HRRR) is a problem on the `apps` line.
- `draft` and `clearance` are read to two decimals: `5.001` is `5`. A nonzero value that rounds to zero is a problem on its line.
- `name` and `vault` can't hold `"` or `#`, because every app's parser reads those as syntax. `vault` starts with `/` or `~/`.
- Saving the same answers twice leaves the file byte for byte the same.

## What setup writes into each app

Setup owns a fixed set of keys in each app's config. It leaves every other line alone.

| App | Config | Keys omahoy owns |
|---|---|---|
| omakeel | `~/.config/omakeel/config.toml` | `source`, repeated |
| omahelm | `~/.config/omahelm/config.toml` | `units`, `safety_depth`, `shallow_contour`, `safety_contour`, `deep_contour`, `home` |
| omalogbook | `~/.config/omalogbook/config.toml` | `boat`, `vault`, `git` |
| omatide | `~/.config/omatide/config.toml` | `region`, `home` |
| omawind | `~/.config/omawind/config.toml` | `region`, `home` |
| omalookout | none | none |

- **Safety depth** is draft plus clearance, rounded up to a whole unit.
- **Safety contour** equals the safety depth.
- **Shallow contour** is half the safety depth, rounded down.
- **Deep contour** is the larger of 30 ft (rounded up in the chosen units: 30 ft, 10 m, 5 fathoms) and twice the safety depth.
- `boat` and `vault` are written quoted. Values are compared the way the apps read them, so `8.0` matches `8` and spacing inside a region doesn't count. Only the depths, `home` and `region` are compared as numbers; `boat`, `vault`, `units` and `git` are text, so boat `007` is not `7`.
- A key that differs is replaced where it stood, keeping its trailing comment; later copies of it are dropped. A missing key is appended under `# Set by omahoy from ~/.config/omahoy/boat.toml.` Every other line, comment and `[section]` stays byte for byte.
- The contours keep omahelm's rule that shallow ≤ safety ≤ deep, because omahelm reverts all three to its defaults if that order breaks.

Drift is computed each time, never stored. A managed key that differs from what boat.toml says is drift. `doctor` shows both values, and `apply` puts boat.toml's value back.

## Hardware

Detection lives in omakeel as `omakeel probe`, because omakeel already owns the serial port code (termios) and the NMEA parser.

```
$ omakeel probe [SPEC...]
serial:/dev/serial/by-id/usb-...-port0:4800  ok  fix=41 ais=0 rejected=2 at=47.6870000,-122.4130000
serial:/dev/serial/by-id/usb-dAISy...-if00:38400  ok  fix=0 ais=6 rejected=0
serial:/dev/serial/by-id/usb-u-blox...-if00  denied
serial:/dev/serial/by-id/usb-FTDI...-if00  busy
serial:/dev/serial/by-id/usb-Other...-if00  silent
tcp:192.168.1.20:10110  unreachable  connection refused
```

How the probe works:

- **Targets.**
  - With no arguments it scans `/dev/serial/by-id/*`, falling back to `/dev/ttyUSB*` and `/dev/ttyACM*` when that directory is missing.
  - A bare path or `serial:PATH` sweeps every baud.
  - `serial:PATH:BAUD` tries that one baud.
  - `tcp:HOST:PORT` listens to the stream.
- **Output.** The first field of an `ok` line is a valid `--source` SPEC, so setup copies it as is.
- **Exclusive access.** The probe opens the port exclusively (`TIOCEXCL`) and restores the port's termios settings when it is done. A port that is already held reports `busy`.
- **Baud sweep.** The order is 4800, 38400, 9600, 115200, 19200, 57600.
  - At each rate it listens for 2.5 s.
  - It stops at the first rate with at least 3 checksum-valid sentences.
  - A rate where any AIS framing appears is extended to 8 s, because AIS can be quiet.
  - GPS and AIS are told apart by the sentences heard (RMC/GGA versus VDM/VDO), never by USB IDs.
- **Exit code.** It exits 0 whether or not anything was found.

How setup handles each case:

- **Permission denied.** Group membership is read from `getent group uucp` and the session's from `id -nG`. Outside the group, setup offers `sudo gpasswd -a $USER uucp`; in the group (now or already) but not in this login, it probes the denied paths again through `sg uucp -c 'omakeel probe PATH...'`.
  - `doctor` keeps saying "log out and back in once" until the session has the group.
- **Busy.** omakeel.service holds the ports. Whenever setup probes (first run, a re-run, `omahoy hardware`) and `systemctl --user is-active omakeel.service` says it runs, setup stops it, probes, then starts it again. A port still busy names the other program's hold.
- **Silent.** A silent device can still be kept. Setup offers "keep it as AIS at 38400, the usual rate" (default no), because a dAISy in a quiet anchorage can stay silent for minutes.
- **Network.** A `host:port` is probed as `tcp:HOST:PORT`. Unreachable, it is offered "keep it anyway" (default yes), because phone apps share only while they're open.
- **Unknown lines.** A probe line setup doesn't understand is shown, not guessed at.
- **Probe won't run.** If `omakeel probe` exits non-zero, or prints nothing that looks like a probe report (an omakeel from before `probe`), setup stops with "couldn't run omakeel probe: ...". It never reads that as "no hardware", because that would set the hub to the demo.
- **No hardware.** `source = demo`. This is a new omakeel source: the sample sail is compiled into omakeel, loops forever, and is named `demo` in `state.sources`.
- **Unplugged later.** The by-id path survives a replug onto a different `ttyACMn`. The hub already retries every 2 s. When omakeel.service is active, `doctor` reads `omakeel watch --once` (its ` | NAME STATUS (n ok, m bad)` segments) and names any source in `error`, or says the hub isn't answering.

## Region and coverage

- Home is the one geographic fact the sailor gives. `area` is derived from it and can be edited in boat.toml, and setup keeps an edited `area` for as long as it still contains home.
- `coverage(area)` is computed and never stored. It reports two things:
  - **NOAA:** the area touches one of ten coarse boxes covering the coasts of the lower 48, the Great Lakes, Alaska and the Aleutians, Hawaii, PR/USVI, and Guam/CNMI (`place::NOAA`).
  - **HRRR:** the area lies inside S 21.2, N 52.6, W -134, E -61 and is at most 8° by 10°, mirrored from omawind's `Region::check`. A unit test pins the numbers.
- `Area::around(home)` is 1° of latitude and 1.1° of longitude each side, rounded to 0.001°, cut short at a pole or the antimeridian rather than wrapped.
- Outside NOAA, omatide is not offered, and omahelm is offered with "import your own ENC: `omahelm import ZIP|DIR`". `doctor` repeats the hint while omahelm has no charts on disk.
- Outside HRRR, omawind is not offered. It cannot be offered, because omawind's lenient parser would drop an out-of-coverage region back to the SF Bay default and show Bay wind as yours.
- Charts come from `omahelm fetch --region S,W,N,E`. omahelm owns the NOAA collection table and picks the collections that intersect the area.
- Tides come from `omatide fetch`, which reads the region setup just wrote.

## Install and lifecycle

- **Packages.** `git` always, `curl` for omahelm, omatide and omawind, `libarchive` (bsdtar) for omahelm. Each is checked with `pacman -Q` and installed with `omarchy pkg add`.
- **Plugins.** Each plugin installs with `omarchy plugin add <repo> --yes` into `~/.config/omarchy/plugins/<id>` and is built in place. It is enabled only after the build, with `omarchy plugin enable <id>`.
  - Plugin state is read from `omarchy plugin list --json` (`id` and `enabled` at each plugin's top level). When the shell doesn't answer, nothing is enabled and `doctor` says the state is unknown.
  - A dev symlink made by `link-plugin.sh` is built where it is and is never replaced.
- **omakeel.** It is not a plugin, so setup clones it to `~/.local/share/omahoy/src/omakeel`.
- **Builds.** `mise trust <dir>`, `mise install` and `mise exec -- cargo build --release --locked`, run in the checkout, use each repo's own pinned Rust. After a successful build setup writes the commit it built from to `target/release/.omahoy-built`. A build is stale when that record differs from `git rev-parse HEAD`, so a docs-only commit costs one rebuild and then settles. A binary with no record is stale once.
- **Binaries.** Each is linked as `~/.local/bin/<bin> -> <dir>/target/release/<bin>`, and `~/.local/bin/omahoy` links to this checkout's `./omahoy` bootstrap. A link into another checkout's `target/release/<bin>`, or a dangling link, is repointed. A file that is not a link, or a link to anything else (a wrapper script, say), is a conflict: `doctor` reports it and setup never touches it.
- **Daemons.** Each unit ships in its own repo (`dist/omakeel.service`, `dist/omalogbook.service`). The units are static, for example `ExecStart=%h/.local/bin/omakeel run`. Setup symlinks them into `~/.config/systemd/user/`, then runs `daemon-reload` and `enable --now`.
  - It restarts a running unit when its binary or its app config file was written after the unit started (`systemctl --user --timestamp=unix show -p ActiveEnterTimestamp --value`), or when this run builds the binary or writes the config. That includes a unit that is running but not enabled, because `enable --now` leaves a running unit as it was. An apply cut off between a config write and the restart restarts on the next run.
  - A unit file that is already there and isn't omahoy's link is left alone and reported. A checkout without `dist/<unit>` is reported too ("update that checkout").
- **Demo logs.** omalogbook refuses to log from a hub whose sources are all `demo`, so demo sails never land in a real log.
- **Fetch state.** `~/.local/state/omahoy/fetched` holds one line per kind, `charts = AREA` and `tides = AREA`: the last area fetched successfully.
  - A different area, or data missing from disk, means fetch again. So switching areas A, then B, then back to A fetches A again.
  - omawind fetches by itself and needs no fetch step.
- **Vault.** Setup runs `git init` in the vault only when `git = true` and the vault is not already inside one. A vault inside a bigger repo (say ~/notes/Logbook in ~/notes) is already in git: omalogbook commits there, and setup asks git rather than looking for a `.git` folder.
  - A missing `user.name` or `user.email` is reported by `doctor`. Logging still works without them.
- **Safe to re-run.** `Machine::survey` reads the machine once; `plan(&Boat, &Machine) -> Vec<Step>` is pure, and each step does nothing when it is already true. Running `apply` twice prints "Everything matches boat.toml." the second time. Writes and links go to a temp name and are then renamed into place. A config that is itself a symlink (a dotfiles manager's) is written through to the file it points at, which keeps its permissions. A failing step stops the run; the steps before it stay done, and the next `apply` plans the rest. When the steps have run, `apply` surveys and plans again. It says "Done." and exits 0 only when no step is left and nothing needs the sailor; otherwise it lists what is left ("Not finished. Still to do:", then "Needs you:") and exits 1, as does an `apply` that finds nothing to do but something for the sailor.
- **Doctor.** `omahoy doctor` prints the plan as "omahoy apply would:" and, under "Needs you:", what apply can't fix: link and unit conflicts, uucp not yet in this login, a missing git identity, no charts outside NOAA, a hub source in `error`. It exits 0 only when both are empty, 1 otherwise, and 2 when boat.toml is missing or has problems.
- **Uninstalling.** Setup never uninstalls. Unticking an app stops managing it.

## Per-repo changes

| Repo | Change |
|---|---|
| omakeel | 1. The `demo` source. 2. `omakeel probe`. 3. A config file with repeatable `source`; `run` with no `--source` reads it, and flags replace it; `watch --once`; `dist/omakeel.service`. |
| omalogbook | Runtime default boat "Boat", not "Dash". Ignore all-demo hubs. `dist/omalogbook.service`. |
| omahelm | A `home = lat, lon` key. The first camera is the last view, then home, then the chart extent, then the built-in default. `fetch --region S,W,N,E`. |
| omatide | Hide the SF Bay panel when the configured region doesn't touch the Bay. |
| omalookout | The no-hub hint points at `omahoy doctor` and no longer names Casey's dAISy. |
| omahoy | The crate, the bootstrap, the `crate` check in `scripts/verify.py` (a gate change, landed on its own), and the README "Get started" section. |

## Why one boat file

The alternative was a shared `omahoy-boat` crate that every app reads. It lost on three counts: a git dependency in all six repos, against their minimal-dependency rule; two configuration sources in every app, with a precedence rule; and about eight large PRs, including a protocol change. Here the apps never learn that omahoy exists.

## Code map

| File | What it holds |
|---|---|
| `src/main.rs`, `src/cli.rs` | The four commands, and the converge loop: survey, plan, show, confirm, run. |
| `src/boat.rs` | `Boat` with its strict parse and render, `Partial` (answers so far) and the defaults read from existing app configs. |
| `src/apps.rs` | `enum App`, the only app list: repo, plugin id, binary, unit, config, coverage refusals, and the managed keys. |
| `src/place.rs` | `Position`, `Area`, `coverage`, the NOAA boxes and the HRRR bounds. |
| `src/depth.rs` | Units, and omahelm's four depths from draft and clearance. |
| `src/hub.rs` | `Hub`, `Source`, `Baud`, and `omakeel probe` line parsing. |
| `src/conf.rs` | `compare` and `edit` of flat `key = value` files. Pure. |
| `src/machine.rs` | `Machine::survey`: checkouts, builds, links, plugins, units, uucp, fetch record, vault. |
| `src/plan.rs` | `Step`, `plan` and `findings`. Pure. |
| `src/run.rs` | `Step::run`, the only code that changes the machine. |
| `src/ask.rs` | The questions, over a `Terminal` trait so tests script the answers. |
| `src/sys.rs` | `Runner` (commands) and `Terminal`, their real and fake forms, `Dirs`, atomic writes. |

## Tradeoffs accepted

- We accept that `apply` overwrites a hand edit to a managed key, after showing both values, in exchange for a single obvious answer to "which file wins".
- We accept HRRR bounds mirrored from omawind, plus a coarse US-waters test, in exchange for knowing coverage before anything is built. The apps remain the authority at runtime.
- We accept a fetch state file owned by omahoy in exchange for no changes to omatide's or omahelm's fetch behavior.
- We accept a baud sweep of up to about 30 s per device in exchange for never asking a sailor what a baud rate is.
- We accept that omakeel is always installed, in exchange for every keel-reading app always having a hub (the demo when there is no hardware).

## Open questions

- Should setup move omakeel's clone forward with `git pull` on `apply`, or pin release tags once releases exist? v1 does neither, and `doctor` reports a checkout that is behind.
- Every window should label demo data. That is UI work in helm and lookout, and it is left as a follow-up.
- `omakeel probe`, `source = demo`, omakeel's config file, `watch --once`, the two `dist/*.service` files and `omahelm fetch --region` land in their own repos. Until they do, setup's probe and fetch steps fail with that command's own error, and `doctor` reports a checkout without its unit file.
