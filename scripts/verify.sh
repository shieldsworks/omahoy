#!/usr/bin/env bash
# Everything that must pass before a change lands. Run from anywhere.
set -euo pipefail
cd "$(dirname "$0")/.."

pin=f3ec8a2d07e8f1e7c50770d0bb93579fc7bb0d60fc8e359a22607ba55cebc6c6
qmllint=${QMLLINT:-/usr/lib/qt6/bin/qmllint}
before=$(git status --porcelain)

echo "qmllint"
"$qmllint" ui/*.qml

echo "check-comments"
got=$(sha256sum scripts/check-comments.sh | cut -d' ' -f1)
[[ $got == "$pin" ]] || { echo "scripts/check-comments.sh differs from the pinned copy" >&2; exit 1; }
scripts/check-comments.sh

echo "bash -n"
for f in run.sh scripts/*.sh scripts/fake-omahoy; do bash -n "$f"; done

echo "json"
jq -e . manifest.json >/dev/null

echo "clean"
after=$(git status --porcelain)
[[ $before == "$after" ]] || { echo "verification changed files:" >&2; diff <(echo "$before") <(echo "$after") >&2 || true; exit 1; }
echo "ok"
