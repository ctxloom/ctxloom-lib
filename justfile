# The pre-commit gates and their recipes (fmt, fmt-check, fmt-staged, test,
# test-staged, mutants-staged, lint-deps, secrets-scan-staged and the `_run`
# machinery and extractors) are gates/gates.just, imported below; this file
# gives them ctxloom-lib's parameters and keeps its own recipes (`build`,
# `lint`, `lint-vocab`, `test-pkg`); the OS gates (`lint-os`, `lint-clippy-sync`,
# `check-windows`) are templates/os-gates.just, which consumers copy.
# cargo-deny's duplicate-version warnings stay in target/logs/lint-deps.log,
# not silenced in deny.toml.

TOP := `git rev-parse --show-toplevel`

import 'gates/gates.just'
import 'templates/os-gates.just'
import 'templates/arch-gates.just'

# What templates/os-gates.just takes from this justfile (sqlite-store-opener is
# left out of check-windows: its SQLite build needs a mingw C compiler).
windows_packages := "-p project-dir -p run-within -p process-env -p config-layers"
clippy_template := TOP / "templates/clippy-disallowed.toml"
# What templates/arch-gates.just takes from this justfile.
arch_lint := "cargo run -q -p arch-lint --"

# `cargo run` builds it on demand.
_arch-lint-ready:
    @true

# The staged tree is a sibling of the checkout, not under target/: cargo
# reads .cargo/config.toml from every parent directory, so under the checkout
# it would pick up a local-deps override (.cargo/local-deps.toml) and gate the
# sibling checkouts instead of the commit's pinned dependencies.
staged_tree := "../.{name}.staged"

# No feature is turned on by what a commit changes.
feature_rules := ""
features_prepare := ""

# Mutation kill-rate thresholds, percent, by crate path: the one place they
# are stated. A mutant is judged by the longest prefix its file matches; one
# matching none fails the gate, so a new tier gets a threshold before it gets
# code. This repository has no domain, so its hand-written tiers are the
# core (pure computation), the adapters and the root's helper libraries, all
# at 90.
mutation_thresholds := "crates/core=90 crates/adapters=90 crates/root/lib=90 crates/root/bin=90"

# Staged paths outside any crate that can change what every crate builds or
# tests (bash globs, `*` crossing `/`): one of these staged tests the whole
# workspace.
workspace_wide := "Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml .cargo/config.toml .cargo/config"

# Build every crate.
[no-exit-message]
build:
    #!/usr/bin/env bash
    set -euo pipefail
    terse=(-q --message-format=short)
    [ "${VERBOSE:-}" = 1 ] && terse=()
    exec just --justfile {{TOP}}/justfile _run build workspace cargo \
        cargo build "${terse[@]}" --manifest-path {{TOP}}/Cargo.toml --workspace

# Clippy, warnings denied: lefthook's `clippy` step. First the vocabulary rule
# (`lint-vocab`), and a check that the OS and filesystem rule consumers copy
# (templates/clippy-disallowed.toml) is the one this repository's own
# clippy.toml applies to itself.
[no-exit-message]
lint: lint-vocab lint-os selftest-gate-kit lint-arch
    #!/usr/bin/env bash
    set -euo pipefail
    {{TOP}}/scripts/clippy-sync-check.sh --exact {{TOP}}/clippy.toml {{TOP}}/templates/clippy-disallowed.toml
    {{TOP}}/scripts/output-budget.selftest.sh
    {{TOP}}/scripts/arch-gates.selftest.sh
    # Clippy reads only the nearest clippy.toml, so the core's repeats the
    # root's nesting threshold and every entry of the OS and filesystem rule.
    {{TOP}}/scripts/clippy-sync-check.sh {{TOP}}/crates/core/clippy.toml {{TOP}}/templates/clippy-disallowed.toml
    root=$(grep '^excessive-nesting-threshold' {{TOP}}/clippy.toml)
    core=$(grep '^excessive-nesting-threshold' {{TOP}}/crates/core/clippy.toml)
    if [ "$root" != "$core" ]; then
        echo "FAIL lint: clippy.toml ($root) and crates/core/clippy.toml ($core) disagree on nesting" >&2
        exit 1
    fi
    terse=(-q --message-format=short)
    [ "${VERBOSE:-}" = 1 ] && terse=()
    exec just --justfile {{TOP}}/justfile _run lint workspace cargo \
        cargo clippy "${terse[@]}" --manifest-path {{TOP}}/Cargo.toml --workspace --all-targets -- -D warnings

# The vocabulary rule: no consumer's word (task, session, bundle, plan, harp,
# taskloom, ctxloom) in the crates' sources, catalogs, manifests or docs; its
# own self-test first.
[no-exit-message]
lint-vocab:
    @{{TOP}}/scripts/vocabulary-check.selftest.sh
    @{{TOP}}/scripts/vocabulary-check.sh

# Wire lefthook's git hooks into the repository's .git/hooks. Worktrees share
# .git/hooks, so this is once per clone, and again whenever lefthook.yml or
# gates/lefthook.yml names a hook not installed yet. Needs lefthook (1.13.6)
# and gitleaks (v8), both by `go install`; cargo-nextest, cargo-deny and
# cargo-mutants by `cargo install <name> --locked`; jq, flock, setsid, ps
# and pkill.
hooks-install:
    lefthook install

# The tests of the named crates only, by cargo-nextest (the gates' `_tests`):
# the quick loop on one crate. `just test` runs every crate's.
[no-exit-message]
test-pkg +packages: (_require-nextest "test-pkg")
    @just --justfile '{{TOP}}/justfile' _run test-pkg "{{packages}}" tests just --justfile '{{TOP}}/justfile' _tests {{packages}}

# The ltk rules template (templates/ltk-rules.yaml) against sample commands:
# needs ltk on PATH (`go install`), so it is not part of `lint`, which CI runs.
[no-exit-message]
selftest-ltk:
    @{{TOP}}/scripts/ltk-rules.selftest.sh
