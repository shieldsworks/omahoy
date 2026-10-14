# Named check

A reviewer lists the checks, or runs the ones a change can affect.

## Sub-features

- `named-list` prints each check name and the rule it guards.
- `named-comments` runs that check, then `clean`.
- `named-bad` exits 2 and runs nothing.

## How to get to it (user POV)

- Run `scripts/verify.sh --list`.
- Run `scripts/verify.sh comments`.
- Run `scripts/verify.sh nosuch` to see a rejected name.

## Driving it with bash

Preconditions:

- Launch and Doctor passed.
- No other command is writing this work tree.

- **List.** Run `drive_gate named-list scripts/verify.sh --list`. `exit.txt` is `0`. `stdout.txt` contains a `comments` line and a `clean` line. It does not contain `All verification steps passed.`
- **One check.** Run `drive_gate named-comments scripts/verify.sh comments`. `exit.txt` is `0`. `stdout.txt` is `== comments`, then `== clean`, then a blank line, then `All verification steps passed.` `stderr.txt` is empty.
- **Bad name.** Run `drive_gate named-bad scripts/verify.sh nosuch`. `exit.txt` is `2`. `stderr.txt` begins with `unknown check nosuch` and ends with `usage: scripts/verify.sh [--list | CHECK...]`. `stdout.txt` is empty.

## Gotchas

- `--list` is accepted only as the sole argument. `scripts/verify.sh --list comments` is an unknown option and exits 2.
- A named run still executes `clean` last.
- An unknown name runs no check.
