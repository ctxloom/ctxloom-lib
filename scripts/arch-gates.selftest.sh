#!/usr/bin/env bash
# Self-test of templates/arch-gates.just against the arch-lint fixture
# workspaces: a clean one passes in one `ok` line, one with a warning shows it
# after the ok line, one with every kind of finding fails with the `FAIL` line
# and its findings, within the output budget. $ARCH_LINT names the binary;
# without it the self-test builds this repository's.
set -euo pipefail
# Under a git hook, git exports variables naming the real repository; the
# fixtures' own git commands must not inherit them.
while IFS= read -r v; do unset "$v"; done < <(git rev-parse --local-env-vars)
here=$(cd "$(dirname "$0")" && pwd)
top=$(cd "$here/.." && pwd)
fixtures=$top/crates/root/bin/arch-lint/fixtures
if [ -z "${ARCH_LINT:-}" ]; then
  cargo build -q --manifest-path "$top/Cargo.toml" -p arch-lint
  ARCH_LINT=$(cd "$top" && cargo metadata --format-version 1 --no-deps --offline \
    | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')/debug/arch-lint
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail() { echo "FAIL arch-gates.selftest: $*" >&2; exit 1; }

fx() { # workspace, rules file: a fixture repository with the gate wired
  local d=$tmp/$1-$2
  mkdir -p "$d/gates"
  cp -R "$fixtures/$1/." "$d/"
  cp "$fixtures/$2" "$d/arch-rules.toml"
  cp "$top/gates/gates.just" "$d/gates/gates.just"
  cp "$top/gates/drop-worktree.sh" "$d/gates/drop-worktree.sh"
  cp "$top/templates/arch-gates.just" "$d/gates/arch-gates.just"
  cat > "$d/justfile" <<J
TOP := \`git rev-parse --show-toplevel\`
import 'gates/gates.just'
import 'gates/arch-gates.just'
arch_lint := "$ARCH_LINT"
_arch-lint-ready:
    @true
staged_tree := "../.{name}.staged"
feature_rules := ""
features_prepare := ""
mutation_thresholds := "crates=90"
workspace_wide := "Cargo.toml"
J
  git -C "$d" init -q
  echo "$d"
}

out=$tmp/out
d=$(fx clean clean-rules.toml)
(cd "$d" && just lint-arch > "$out" 2>&1) || fail "the clean workspace failed: $(cat "$out")"
[ "$(grep -c '' "$out")" = 1 ] && grep -qE '^ok   lint-arch \(workspace\) [0-9]+\.[0-9]s$' "$out" \
  || fail "a pass is one ok line: $(cat "$out")"

d=$(fx warned clean-rules.toml)
(cd "$d" && just lint-arch > "$out" 2>&1) || fail "warnings alone failed: $(cat "$out")"
[ "$(grep -c '' "$out")" = 2 ] && grep -q '^ok   lint-arch' "$out" && sed -n 2p "$out" | grep -q '^warning: fx-store ' \
  || fail "a pass with a warning is the ok line and the warning: $(cat "$out")"

d=$(fx violations violations-rules.toml)
rc=0
(cd "$d" && just lint-arch > "$out" 2>&1) || rc=$?
[ "$rc" = 1 ] || fail "the violations workspace exited $rc"
[ "$(head -n 1 "$out")" = 'FAIL lint-arch: 11 violations, 2 warnings (full log: target/logs/lint-arch.log)' ] \
  || fail "the FAIL line: $(head -n 1 "$out")"
[ "$(grep -c '' "$out")" -le 41 ] || fail "a failure is at most 41 lines: $(grep -c '' "$out")"
[ "$(grep -c '^violation: ' "$out")" = 11 ] && [ "$(grep -c '^warning: ' "$out")" = 2 ] \
  || fail "the findings: $(cat "$out")"

# A tool failure (no rules file) is a FAIL line saying why.
rm "$d/arch-rules.toml"
rc=0
(cd "$d" && just lint-arch > "$out" 2>&1) || rc=$?
[ "$rc" = 2 ] && head -n 1 "$out" | grep -q '^FAIL lint-arch: cannot read the rules ' \
  || fail "a missing rules file: rc $rc: $(cat "$out")"
echo "ok   arch-gates selftest"
