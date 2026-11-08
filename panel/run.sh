#!/usr/bin/env bash
# Opens the setup panel in a window of its own, from this checkout, in its
# own Quickshell process. It needs `omahoy` on PATH, or OMAHOY_BIN naming one.
set -euo pipefail
cd "$(dirname "$0")"
exec quickshell -p ui/shell.qml "$@"
