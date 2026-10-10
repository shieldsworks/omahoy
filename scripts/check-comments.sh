#!/usr/bin/env bash
set -euo pipefail

unset GIT_LITERAL_PATHSPECS GIT_GLOB_PATHSPECS GIT_NOGLOB_PATHSPECS GIT_ICASE_PATHSPECS

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

listing=$(mktemp)
trap 'rm -f "$listing"' EXIT
git ls-files -z -- "${pathspec[@]}" >"$listing"
files=()
while IFS= read -r -d '' path; do
  files+=("$path")
done <"$listing"

if ((${#files[@]} == 0)); then
  exit 0
fi

set +e
hits=$(grep -HnE "$pattern" -- "${files[@]}")
status=$?
set -e
if ((status > 1)); then
  if [[ -n $hits ]]; then
    printf '%s\n' "$hits"
  fi
  exit "$status"
fi
if ((status == 0)); then
  printf '%s\n' "$hits"
  printf '\nApologetic or deferred-work comments are not allowed (see AGENTS.md).\n' >&2
  exit 1
fi
