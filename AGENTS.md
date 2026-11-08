# Working on omahoy-setup

omahoy-setup is the Omahoy suite's graphical setup: an Omarchy shell panel,
`org.omahoy.setup`, that fills the same form as the terminal `omahoy`
setup. The panel holds the form; the `omahoy` binary keeps every rule. The
panel never decides validity, areas, app refusals or what apply does. It
calls `omahoy gui draft|check|probe|join-uucp|plan|apply` and renders the
JSON. The contract lives in `omahoy/docs/gui.md` in the omahoy repo.

## Layout

- `ui/Setup.qml`: the form's state and every call to `omahoy gui`.
- `ui/Runner.qml`: one command at a time, stdin through `printf |`.
- `ui/SetupWindow.qml`: the window, keys and the `omahoysetup` IpcHandler.
- `ui/*Step.qml`: one file per step. `Btn`, `Field`, `Tick`, `Words`: widgets.
- `ui/Theme.qml`: the Omarchy theme, read from colors.toml.
- `scripts/fake-omahoy`: contract-shaped answers, for driving the panel.
- `scripts/verify.sh`: the gate.

## Done means verified

```sh
scripts/verify.sh
```

It runs qmllint on `ui/*.qml`, the comment check, `bash -n` on the scripts,
and a check that verification changed no file. Fix what it reports; never
weaken the check that reported it.

To drive the panel without the real binary:

```sh
OMAHOY_BIN=$PWD/scripts/fake-omahoy quickshell -p ui/shell.qml &
quickshell ipc -p ui/shell.qml call omahoysetup status
```

`goTo`, `next`, `back`, `set KEY VALUE`, `probe`, `go` and `skipHardware`
drive it. Never run `scripts/link-plugin.sh` or restart the shell from a
check: that is the live desktop.

## Rules

- `scripts/check-comments.sh` is the suite's canonical copy; its sha256 is
  pinned in `verify.sh`. Comments say why, and never mark deferred work or
  apologize.
- Boat text is composed by the panel and sent on stdin. A value with a
  newline, control character, `"` or `#` is refused in the form, never sent.
- Never stop a running `gui probe` or `gui apply`: they stop and restart
  the hub's service, and apply changes the machine.
- Commit messages: `<scope>: <imperative summary>`, a body that says why.
