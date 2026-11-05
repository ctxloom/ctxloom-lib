#!/usr/bin/env bash
# Self-test of vocabulary-check.sh against fixture trees: each banned word is
# caught, a clean tree and a listed exception pass, a stale exception fails.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
check="$here/vocabulary-check.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fixture() { # name, content, [allow]
  local d="$work/$1"
  mkdir -p "$d/crates/x/src" "$d/scripts"
  printf '%s\n' "$2" > "$d/crates/x/src/lib.rs"
  [ -n "${3:-}" ] && printf '%s\n' "$3" > "$d/scripts/vocabulary-allow.txt"
  echo "$d"
}
expect() { # code, dir, label
  local got=0
  "$check" "$2" >/dev/null 2>&1 || got=$?
  if [ "$got" != "$1" ]; then echo "selftest FAIL: $3 (exit $got, wanted $1)" >&2; exit 1; fi
}

expect 0 "$(fixture clean 'pub fn open() {}')" clean
for w in task Tasks session bundle plan harp taskloom ctxloom task_id; do
  expect 1 "$(fixture "w-$w" "pub const A: &str = \"$w\";")" "word $w"
done
expect 0 "$(fixture allowed 'let planet = 1;' 'crates/x/src/lib.rs | let planet | a word, not the vocabulary')" allowed
expect 1 "$(fixture stale 'pub fn open() {}' 'crates/x/src/lib.rs | let planet | gone')" stale
echo "ok   lint-vocab selftest"
