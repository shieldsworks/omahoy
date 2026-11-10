# Omahoy setup panel

The Omarchy shell panel `org.omahoy.setup`: Omahoy's setup in a window
instead of the terminal. It asks what `./omahoy` asks, shows the plan and
applies it. The panel draws; the `omahoy` binary decides, through `omahoy gui
...` ([docs/gui.md](../docs/gui.md)).

Open it from the repo root:

```sh
./omahoy panel
```

That links this folder as the plugin, enables it and opens it
([docs/onboarding.md](../docs/onboarding.md)). Alt+Left and Alt+Right move
between steps, Ctrl+Return goes on, Tab moves between fields and buttons,
and Escape closes it.

## Develop

`scripts/fake-omahoy` answers each `omahoy gui` command with sample JSON, so
the panel runs in a window of its own without the real binary:

```sh
OMAHOY_BIN=$PWD/scripts/fake-omahoy ./run.sh
```

From the repo root, `/usr/lib/qt6/bin/qmllint panel/ui/*.qml` lints it and
`panel/scripts/check-panel.sh` drives it headless. `scripts/verify.sh` does
not run them yet.
