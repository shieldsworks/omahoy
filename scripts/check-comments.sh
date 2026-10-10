#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
rel=$(realpath --relative-to="$root" -- "$0")
cd "$root"

prefixes='(^[[:space:]]*(#|\*|/\*|<!--)|//)'
banned=(
  'TODO'
  'FIXME'
  'XXX'
  'HACK'
  '[Ww]orkaround'
  '[Tt]emporary (fix|hack|solution)'
  '[Qq]uick fix'
  '[Ff]or the time being'
  '[Nn]ot ideal'
  '[Ss]hould be fixed'
  '[Ss]orry'
  '[Kk]ludge'
  '[Bb]and-?aid'
)
joined=$(printf '%s|' "${banned[@]}")
joined=${joined%|}
pattern="${prefixes}.*\\b(${joined})\\b"

if (($#)); then
  pathspec=("$@")
else
  pathspec=(
    '*.rs' '*.qml' '*.js' '*.sh' '*.py' '*.toml' '*.yml'
    ':!:tests/fixtures/**' ':!:**/vendor/**'
  )
fi
# git ls-files rejects a pathspec that leaves the repository.
case $rel in
  ..*) ;;
  *) pathspec+=(":!:$rel") ;;
esac

mapfile -t files < <(git ls-files -- "${pathspec[@]}")
((${#files[@]})) || exit 0

if hits=$(grep -HnE "$pattern" -- "${files[@]}"); then
  printf '%s\n' "$hits"
  printf '\nApologetic or deferred-work comments are not allowed (see AGENTS.md).\n' >&2
  exit 1
fi
