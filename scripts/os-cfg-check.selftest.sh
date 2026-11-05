#!/usr/bin/env bash
# Self-test of os-cfg-check.sh against fixture trees: each OS-naming form is
# flagged, a clean tree and a listed exception pass, a stale exception fails.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
check="$here/os-cfg-check.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fixture() { # name, content, [allow]
  local d="$work/$1"
  mkdir -p "$d/crates/a/b/x/src" "$d/scripts"
  printf '%s\n' "$2" > "$d/crates/a/b/x/src/lib.rs"
  [ -n "${3:-}" ] && printf '%s\n' "$3" > "$d/scripts/os-cfg-allow.txt"
  echo "$d"
}
expect() { # code, dir, label
  local got=0
  "$check" "$2" >/dev/null 2>&1 || got=$?
  if [ "$got" != "$1" ]; then echo "selftest FAIL: $3 (exit $got, wanted $1)" >&2; exit 1; fi
}

expect 0 "$(fixture clean 'pub fn open() {}')" clean
expect 1 "$(fixture cfg-macro 'pub fn f() -> bool { cfg!(windows) }')" "cfg!(windows)"
expect 1 "$(fixture cfg-attr '#[cfg(unix)]
pub fn f() {}')" "#[cfg(unix)]"
expect 1 "$(fixture cfg-nested '#[cfg(all(test, not(target_os = "linux")))]
pub fn f() {}')" "nested target_os"
expect 1 "$(fixture cfg-attr-attr '#[cfg_attr(windows, allow(dead_code))]
pub fn f() {}')" "cfg_attr(windows)"
expect 1 "$(fixture std-os 'use std::os::unix::fs::PermissionsExt;')" "std::os::"
expect 1 "$(fixture split 'let p = std::env::split_paths(&v);')" split_paths
expect 0 "$(fixture feature '#[cfg(feature = "x")]
pub fn f() {}')" "a non-OS cfg"
expect 0 "$(fixture allowed 'pub fn f() -> bool { cfg!(windows) }' 'crates/a/b/x/src/lib.rs | cfg!(windows) | the seam itself')" allowed
expect 1 "$(fixture stale 'pub fn f() {}' 'crates/a/b/x/src/lib.rs | cfg!(windows) | gone')" stale

d=$(fixture target-dep 'pub fn f() {}')
printf '[package]\nname = "x"\n\n[target.'"'"'cfg(windows)'"'"'.dependencies]\nw = "1"\n' > "$d/crates/a/b/x/Cargo.toml"
expect 1 "$d" "target-specific dependency"
d=$(fixture deep-target-dep 'pub fn f() {}')
mkdir -p "$d/crates/a/b/x/y"
printf '[target.x.dependencies]\n' > "$d/crates/a/b/x/y/Cargo.toml"
expect 1 "$d" "target-specific dependency, deeper manifest"
echo "ok   lint-os selftest"
