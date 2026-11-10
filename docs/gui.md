# omahoy gui: the interface for graphical setup

`omahoy gui ...` is how a graphical front end runs setup. The front end is
the Omarchy shell panel `org.omahoy.setup` in this repo's `panel/`, opened
by `omahoy panel` (onboarding.md). It holds the form; this crate keeps
every rule. The front end never decides
what is valid, what an area is, which apps can work where, or what apply
does. It asks.

Wire format. Input is boat.toml text on stdin, in the same `key = value`
dialect `boat.toml` uses (see onboarding.md). Output is one JSON document
on stdout, except `gui apply`, which prints progress lines. Human progress
(building omakeel, stopping the hub) goes to stderr. Every gui command runs
its child commands with their output captured, so nothing a child prints
(a build, `pkexec`) can reach stdout and corrupt the JSON. Exit codes: 0 when the
JSON was written, 2 when the command itself failed (its message on stderr).
A boat with problems is still exit 0: the problems are in the JSON.

All numbers are JSON numbers. A missing or unparsed value is `null`.

## Commands

### `omahoy gui draft`

Where the form starts. Reads `~/.config/omahoy/boat.toml` when it exists,
else the defaults from app configs already on the machine
(`Partial::from_existing`). Answers still unset get the defaults the
terminal questions offer: `units` feet, `clearance` the units' default,
`vault` `~/Logbook`, `git` true, and `apps` all five, less the apps refused
where the boat sails once there is a home. Answers already there win.
`name`, `home`, `area`, `draft` and `source` stay unset, so they are
problems until the sailor answers.

```json
{"exists": true, "report": REPORT}
```

`exists` is true when boat.toml was there. A boat.toml that fails to parse
is exit 2 with its problems on stderr: the panel tells the sailor to run
`omahoy doctor`.

### `omahoy gui check` (stdin: boat text)

Checks answers so far. Keys may be missing; nothing is saved.

```json
REPORT
```

### `omahoy gui probe [--uucp] [SPEC...]`

Listens for hardware, as the terminal's hardware question does: builds
omakeel first when it isn't fresh, stops `omakeel.service` while it
listens and starts it again after (a hub that was not running is left alone;
when it was and won't stop, no probe runs and that is exit 2; when it won't start again, that is exit 2, with the probe's own
failure named too if it also failed). With no SPEC it scans every serial
device. A SPEC is `serial:PATH` or `tcp:HOST:PORT` (`HOST:PORT` alone is
read as tcp). `--uucp` listens through `sg uucp`, for a sailor who was just
added to the group; when `/etc/group` doesn't list the user in uucp it is
exit 2 with a message, because `sg` would ask for a group password on a
terminal that isn't there.

```json
{
  "devices": [
    {
      "spec": "serial:/dev/serial/by-id/usb-...-if00",
      "name": "usb-...-if00",
      "kind": "serial",
      "verdict": "ok",
      "describe": "GPS, 4800 baud, 41 good sentences, fix 37°52.0′N 122°18.8′W",
      "source": "serial:/dev/serial/by-id/usb-...-if00:4800",
      "fix": {"lat": 37.8667, "lon": -122.3134}
    }
  ],
  "uucp": {"member": true, "session": true}
}
```

- `kind`: `serial` or `tcp`.
- `verdict`: `ok`, `denied`, `busy`, `silent`, `missing`, `unreachable`
  or `error`. `describe` is the sentence the terminal prints after the
  device name.
- `source`: the boat.toml `source` value to use, on `ok`. On `silent`
  serial devices it is the AIS default at 38400 the terminal offers
  (`serial:PATH:38400`). On every tcp device, whatever the verdict, it is
  the canonical `tcp:HOST:PORT` (the terminal offers "keep it anyway" for an
  unreachable one). Null otherwise.
- `fix`: the position heard, or null.
- `uucp.member`: the user is in the uucp group in /etc/group.
  `uucp.session`: this login session has it.
- A SPEC that doesn't parse, an unsupported serial baud included, is exit 0 with one device whose verdict is
  `error` and whose `describe` says why. Spec errors come first in `devices`.

### `omahoy gui join-uucp`

Adds the user to the uucp group with `pkexec gpasswd -a USER uucp` (polkit
asks for the password). Prints `{"member": true, "session": false}`. Exit 2
when pkexec fails or is dismissed.

### `omahoy gui plan` (stdin: boat text)

What apply would do, for these answers, without saving them.

```json
{"problems": [...], "steps": ["install packages: ...", "..."], "findings": ["..."]}
```

`problems` has the shape of REPORT's and is non-empty when the boat doesn't
parse; `steps` and `findings` are then empty. Surveying only reads the machine.

### `omahoy gui apply` (stdin: boat text)

Saves boat.toml (strict parse; a bad boat is exit 2 with its problems on
stderr, and nothing is saved), then applies it with no prompt. Prints the
same progress lines `omahoy apply --yes` prints, one per line, as they
happen. Commands that need root run through `pkexec` instead of `sudo`:
`pkexec gpasswd -a USER uucp`, and `pkexec pacman -S --noconfirm --needed
-- PKG...` in place of `omarchy pkg add`. Child command output is captured,
never mixed into stdout. Exit 0 when the machine matches and nothing needs
the sailor, 1 when something is left (the lines say what), 2 on failure.

## REPORT

```json
{
  "answers": {
    "name": "Dash",
    "home": {"lat": 37.8667, "lon": -122.3134, "show": "37°52.0′N 122°18.8′W"},
    "area": {"south": 36.867, "west": -123.413, "north": 38.867, "east": -121.213},
    "units": "feet",
    "draft": 3.5,
    "clearance": 2,
    "sources": ["demo"],
    "apps": ["helm", "logbook", "tide", "wind"],
    "vault": "~/Logbook",
    "git": true
  },
  "problems": [{"key": "draft", "line": 7, "message": "draft 0 is not a depth"}],
  "complete": false,
  "suggestedArea": {"south": 36.867, "west": -123.413, "north": 38.867, "east": -121.213},
  "coverage": {"noaa": true, "hrrr": true},
  "depths": {"safety": 6, "shallow": 3, "safetyContour": 6, "deep": 30},
  "draftIn": {"feet": 3.5, "meters": 1.07, "fathoms": 0.58},
  "clearanceIn": {"feet": 2, "meters": 0.61, "fathoms": 0.33},
  "defaultClearance": {"feet": 2, "meters": 0.6, "fathoms": 0.5},
  "apps": [
    {"short": "helm", "name": "omahelm", "what": "chartplotter", "refusal": null, "caveat": null}
  ],
  "appsAllowed": ["helm", "logbook", "tide", "wind"]
}
```

- `answers`: each key as parsed, null when missing or bad (`apps` with any
  unrecognized name is null, and `vault` must name a folder below `/` or `~/`). `sources` is
  the list of `source` values, `[]` when none.
- `problems`: `key` is the boat.toml key the problem is on, or null when it
  is on no key (an unknown key, a line without `=`). A missing key carries
  that key (`{"key": "vault", "line": 0, "message": "missing vault"}`), and
  demo beside another source is on `source`. `line` is the line in the text
  sent, 0 when the problem has none. Missing keys are problems, so
  `complete` is true exactly when `problems` is empty.
- `suggestedArea`: the area setup would use for this home: the current
  area when it contains home, else about a degree around home. Null
  without a home. The panel sets the area to it whenever home changes.
- `coverage`, `apps`: for `suggestedArea`; null and every app without a
  refusal when there is no home.
- `depths`: omahelm's four depths, null until units, draft and clearance
  all parse.
- `draftIn`, `clearanceIn`: the value converted into each unit, null when
  the value or the units are missing. The panel uses them when the sailor changes units.
- `appsAllowed`: `answers.apps` without the apps refused in
  `suggestedArea`; `[]` when `answers.apps` is null. The panel uses it whenever the area changes.

## Hardware choices

The panel keeps one entry per source, keyed by what the source is: a
serial device by its path (the baud is not part of it), a tcp source by
`tcp:HOST:PORT` with the host lowercased. Each entry holds the boat's own
value, the value the last probe offered (`source` in a probe device), and
the sailor's choice: Keep (the boat's value), Use (the offered value),
Skip for now, or none yet. The boat's sources are read from the choices;
`demo` is never an entry, and with no source chosen the boat gets `source
= demo`.

- The entries start from the boat's sources, each on Keep.
- A listen, a reprobe after joining uucp, and a network test only add
  entries and update what was offered and whether it was heard. They never
  change a choice, so a receiver chosen and then unplugged, or skipped and
  then rescanned, stays as the sailor left it.
- A device heard at a different baud than the boat's is one entry: Keep
  keeps the boat's baud, Use replaces it with the probe's.
- A silent device offers Use as AIS at 38400; an unreachable network source
  offers Use anyway. Skip for now leaves it out of this boat; `omahoy
  hardware` or the panel can add it later.
- The step's own Skip for now puts every entry back to the boat's sources
  (the demo on a first run).

## Unit changes

Choosing other depth units converts draft and clearance with the report's
`draftIn` and `clearanceIn`. The change waits until a report for the text
as it stands, with no field refused, has a conversion for every depth field
that isn't empty (`2m` has none, so it waits until it reads as a number);
until then units, draft and clearance stay as typed, and Go waits too.
A clearance at the old units' default becomes the new units' default.

## Apply outlives the panel

The shell reloads every panel when a plugin is added, and apply adds
plugins (`omarchy plugin add`), so the panel does not own the apply process.
`panel/scripts/apply.sh start` runs `omahoy gui apply` in a transient user
unit (`systemd-run --user --collect`, with the panel's environment), or under
`setsid` where there is no user manager, with the boat text read from a file.
Its files live in `$XDG_RUNTIME_DIR/omahoy-setup/`: `in` (the boat text),
`out` and `err` (apply's stdout and stderr), `pid`, and `code` (the exit
status, written last), and `id`, the run's, which start prints. `lock` is
flocked by a start while it checks and writes, by the worker from its start
until `code` is written, by a clear, and by every probe the panel runs
(`apply.sh probe`) for the whole probe, so of every open panel only one
starts an apply, no probe stops the hub during an apply, and no apply starts
during a probe. A refused start or probe (exit 3) does nothing and says
setup is busy in another window. A panel follows, reports and clears only
the run whose id it started or picked up; a run with another id is never its
result. The panel follows
`out` while it runs; a panel that loads with an apply there, running or
finished, shows that apply's answers on the review step and follows it to
the end, then forgets the files. A worker that died without writing `code`
is reported as stopped, with `omahoy apply` to finish. A probe is never
started while an apply runs, in this panel or another.

## Which binary the panel runs

The panel runs every command through one path. `omahoy panel` passes its
own binary in the shell's open payload, `{"bin": "/…/target/release/omahoy"}`,
because on a first run `omahoy` is not on PATH yet. Opened any other way (a
keybinding with `{}`), the panel uses `omahoy` on PATH, which setup links to
the bootstrap. Run standalone (`panel/run.sh`), it reads `OMAHOY_BIN`.
