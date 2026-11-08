# Omahoy setup

Set up [Omahoy](https://omahoy.org), the sailing suite for Omarchy, from a
panel instead of the terminal.

It asks what `omahoy` asks: the boat, its GPS and AIS hardware, home and
sailing area, apps and logbook. Then it shows the plan and applies it.
Wherever setup asks about specific hardware, each device can be Used or
Skipped for now. With no hardware chosen, the hub plays the demo sail with
invented ships until you add some.

The panel draws; the `omahoy` binary decides. Every check, probe, plan and
apply goes through `omahoy gui ...`.

## Install

Omarchy 4, with `omahoy` on PATH.

```sh
omarchy plugin add https://github.com/shieldsworks/omahoy-setup --enable
```

Open it:

```sh
omarchy shell shell toggle org.omahoy.setup "{}"
```

Alt+Left and Alt+Right move between steps, Ctrl+Return goes on, Tab moves
between fields and buttons, and Escape closes it.

## Develop

`scripts/fake-omahoy` answers each `omahoy gui` command with sample JSON, so
the panel can run before, or without, the real binary:

```sh
OMAHOY_BIN=$PWD/scripts/fake-omahoy ./run.sh
```

`scripts/verify.sh` runs qmllint, the comment check and the script checks.
`scripts/link-plugin.sh` points the Omarchy plugin at this checkout.

## License

MIT
