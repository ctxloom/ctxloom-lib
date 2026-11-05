#!/usr/bin/env bash
# Self-test of the canonical list itself against fixture cargo projects: a
# call to std::fs (or Path::exists) fails clippy under the template's rules, a
# clean project passes. Offline: the fixtures have no dependencies.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
template="$here/../templates/clippy-disallowed.toml"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="$work/target"

fixture() { # name, source
  local d="$work/$1"
  mkdir -p "$d/src"
  printf '[package]\nname = "fixture"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n' > "$d/Cargo.toml"
  { echo 'excessive-nesting-threshold = 6'; echo; sed -n '/^disallowed-methods/,$p' "$template"; } > "$d/clippy.toml"
  printf '%s\n' "$2" > "$d/src/lib.rs"
  echo "$d"
}
expect() { # 0 (passes) or 1 (any failure), dir, label
  local got=0
  cargo clippy -q --offline --manifest-path "$2/Cargo.toml" -- -D warnings >"$work/out" 2>&1 || got=$?
  if { [ "$1" = 0 ] && [ "$got" != 0 ]; } || { [ "$1" != 0 ] && [ "$got" = 0 ]; }; then cat "$work/out" >&2; echo "selftest FAIL: $3 (exit $got, wanted $1)" >&2; exit 1; fi
}

expect 0 "$(fixture clean 'pub fn name(p: &std::path::Path) -> Option<&std::ffi::OsStr> { p.file_name() }')" "clean fixture"
expect 1 "$(fixture fs-read 'pub fn f() { let _ = std::fs::read_to_string("x"); }')" "std::fs::read_to_string"
expect 1 "$(fixture fs-file 'pub fn f() { let _ = std::fs::File::open("x"); }')" "std::fs::File::open"
expect 1 "$(fixture path-exists 'pub fn f(p: &std::path::Path) -> bool { p.exists() }')" "Path::exists"
echo "ok   clippy-disallowed selftest"
