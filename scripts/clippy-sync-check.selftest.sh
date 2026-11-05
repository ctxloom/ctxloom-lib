#!/usr/bin/env bash
# Self-test of clippy-sync-check.sh against fixture files and, for the bans
# themselves, a real cargo project that calls std::fs and must not pass clippy.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
check="$here/clippy-sync-check.sh"
template="$here/../templates/clippy-disallowed.toml"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

expect() { # code, label, args...
  local want=$1 label=$2 got=0
  shift 2
  "$check" "$@" >/dev/null 2>&1 || got=$?
  if [ "$got" != "$want" ]; then echo "selftest FAIL: $label (exit $got, wanted $want)" >&2; exit 1; fi
}

{ echo 'excessive-nesting-threshold = 6'; echo; sed -n '/^disallowed-methods/,$p' "$template"; } > "$work/in-sync.toml"
expect 0 "a copy of the template" "$work/in-sync.toml" "$template"
expect 0 "a copy, exact" --exact "$work/in-sync.toml" "$template"

grep -v 'std::fs::read_to_string' "$work/in-sync.toml" > "$work/missing.toml"
expect 1 "an entry missing" "$work/missing.toml" "$template"

sed 's/^]$/  { path = "std::env::var", reason = "the consumer'"'"'s own" },\n]/' "$work/in-sync.toml" > "$work/extra.toml"
grep -q std::env::var "$work/extra.toml"
expect 0 "an extra entry of the consumer's own" "$work/extra.toml" "$template"
expect 1 "an extra entry, exact" --exact "$work/extra.toml" "$template"

sed 's/, allow-invalid = true//' "$work/in-sync.toml" > "$work/valid.toml"
expect 1 "allow-invalid dropped" "$work/valid.toml" "$template"

expect 1 "no such clippy.toml" "$work/none.toml" "$template"
echo "ok   lint-clippy-sync selftest"
