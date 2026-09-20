# Omahoy

An onboard computer system for sailors who run Omarchy.

Omahoy is a set of small apps that each stand alone and work better together:
a chartplotter full screen on one workspace, and AIS, tides, the log, and the
boat's systems tiled on the next. Everything is written from scratch in Rust,
drawn in your Omarchy theme, and driven from the keyboard.

It's built in public aboard **Dash**, a Pacific Seacraft 25 berthed in the
Berkeley Marina. She sails San Francisco Bay today, and plans to sail around the
world someday.

**Status: early.** [omakeel](https://github.com/shieldsworks/omakeel) reads GPS
and AIS, and [omalookout](https://github.com/shieldsworks/omalookout) puts the
nearest vessel in your Omarchy bar. Both are tested on a replayed sail, not yet
on a real receiver.

## Apps

Install only the apps you want. Each one is its own repo. omakeel is the hub
the others read from; the rest are Omarchy plugins.

| App | What it does |
|---|---|
| [omakeel](https://github.com/shieldsworks/omakeel) | The data hub: GPS, AIS and sensors, served to every app |
| [omahelm](https://github.com/shieldsworks/omahelm) | Chartplotter |
| [omalookout](https://github.com/shieldsworks/omalookout) | AIS targets and collision alarms |
| [omanchor](https://github.com/shieldsworks/omanchor) | Anchor watch |
| [omalogbook](https://github.com/shieldsworks/omalogbook) | Ship's log: markdown and GPX, kept in git |
| [omatide](https://github.com/shieldsworks/omatide) | Tides and currents, offline |
| [omawind](https://github.com/shieldsworks/omawind) | Wind forecasts from GRIB, kept for when you're offline |
| [omabosun](https://github.com/shieldsworks/omabosun) | Batteries, bilge, engine hours, maintenance |
| [omagauge](https://github.com/shieldsworks/omagauge) | Instrument dials |
| [omatiller](https://github.com/shieldsworks/omatiller) | Tiller pilot control |
| [omarig](https://github.com/shieldsworks/omarig) | Open sensor hardware and firmware |

## Principles

- **Rust from day one.** Engines, the hub, and the firmware.
- **From scratch.** Omahoy uses no existing marine software, and relies on public data like NOAA charts and tide constants.
- **Safe without the internet.** Starlink carries the data. Anchor watch, AIS and position all work without it.
- **Native to Omarchy.** Colors come from the active Omarchy theme, every action has a key, and each app is a standard Omarchy plugin.
- **Honest.** Stale data says it's stale.
- **Any Omarchy machine.** Builds for both x86_64 and aarch64.

## Window chrome

Every Omahoy window names itself, so a workspace of them is read at a glance.
The name goes top left, in the theme's accent color, bold, uppercase, one pixel
under the shell's base font size, and it is always on show. Windows laid out
like a document (omalookout, omawind) set it bare in the top margin; windows
whose content runs to the edge (omahelm) put it on the same chip the rest of
their chrome uses, so it holds up over the chart.

That is the whole of it: no mark beside the name, no letter-spacing, no rule
under it, no light or badge. Liveness belongs in the status bar, where each app
already says what its data is doing.

## Not for navigation

Omahoy is not a primary means of navigation. Carry a backup.

## License

MIT
