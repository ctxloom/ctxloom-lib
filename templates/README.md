# Templates: the gate-kit consumers copy

Gates and clippy settings have no include mechanism, so a consumer copies
these files; the copies are independent. Nothing here is linked.

| File | Copy to | What it is |
|---|---|---|
| `clippy-disallowed.toml` | the `disallowed-methods` array into `clippy.toml` | the canonical OS and filesystem rule, each group commented with why |
| `os-gates.just` | `gates/os-gates.just`, then `import` it | `lint-os`, `lint-clippy-sync`, `check-windows` |
| `fixture-gates.just` | `gates/fixture-gates.just`, then `import` it | `lint-fixtures`: no test gives a file an executable mode to run it (a fake program is `fixture_program::install_program`) |
| `../scripts/exec-mode-check.sh` | `scripts/` | the check behind `lint-fixtures`: an executable octal mode written in a `*.test.rs` or `tests/` file fails, with the allowlist `../scripts/exec-mode-allow.txt` (`<file> \| <text on the line> \| <why>`; empty by default; a stale entry fails) |
| `ltk-rules.yaml` | `.ltk/config.yaml` | ltk rules sending raw cargo, nextest, rustc, lefthook, gitleaks and mutation-testing runs to the just recipes, and blocking every way to skip the hooks (`--no-verify`, `-n`, `core.hooksPath`, `env LEFTHOOK=`/`SKIP=`); tested by `../scripts/ltk-rules.selftest.sh` |
| `arch-gates.just` | `gates/arch-gates.just`, then `import` it | `lint-arch`: the architecture lint (see below) |
| `arch-rules.toml` | the repository's top level | the rules table to start from: which role may depend on which |
| `ci.yml` | `.github/workflows/ci.yml` | Linux gates plus a native Windows build and test |
| `../scripts/os-cfg-check.sh` | `scripts/` | no OS-naming cfg, `cfg!(`, `std::os::`, `split_paths` or target dependency under `crates/` |
| `../scripts/os-cfg-allow.txt` | `scripts/` | the seam: one `<file> \| <text on the line> \| <why>` per exception (empty by default; a stale entry fails) |
| `../scripts/clippy-sync-check.sh` | `scripts/` | fails when a `clippy.toml` lacks an entry of the template |

## ltk rules

`ltk-rules.yaml` goes to `.ltk/config.yaml` (merge with existing rules; ids
must be unique). Its suggestions name the gate-kit's recipes plus `run`,
`clean-target` and `mutants-batch`, which a consumer adds or renames. A rule's
`match.command` is an ordered subsequence of the command's non-option
arguments, so the specific subcommands (`cargo deny check`, `cargo mutants`)
come before the generic ones (`cargo build|check`). ltk unwraps `bash -c`,
`env`, `setsid nohup` and absolute paths, so those spellings are caught; it
cannot see a bare `VAR=x git commit` prefix, `export`, or `sudo`'s command.
`scripts/ltk-rules.selftest.sh [config]` runs 56 sample commands through
`ltk check` (blocked and allowed) and fails on any wrong verdict. Mutation
testing is meant to run only through the recipes; the rules block every raw
spelling of `cargo mutants`.

The gates' failure output is capped for an agent's context: a pass is one
`ok` line, a failure its `FAIL` line and at most 40 lines of extract (failing
tests only, the first 20 diagnostics, 15 survivors, each with a `+K more`
tail); `scripts/output-budget.selftest.sh` checks that on a fixture workspace.

## The architecture lint

`arch-lint` (`crates/root/bin/arch-lint`) reads a workspace with
`cargo metadata --no-deps --offline` and holds every normal and build edge
between workspace members to a table. A crate's directory,
`crates/<ring>/<role>/<crate>`, is its role (`<ring>/<role>`); the table
(`arch-rules.toml`) says, per role, which roles (or globs of them, or
`@class` of crate-name patterns) it may depend on, so the rules are data and
a repository supplies its own. A crate off the layout, a role with no rule and
an exception that matches no finding each fail too. Dev-dependency edges are
checked for the roles named in `dev_checked` only, and a finding there is a
warning (a line after the `ok`; `--deny-warnings` fails on it). Exceptions are
`[[exception]]` entries with a mandatory reason. Self-tested by the crate's
tests over fixture workspaces (`crates/root/bin/arch-lint/fixtures`: a clean
one, one with a warning, one with a finding of every kind) and
`../scripts/arch-gates.selftest.sh` (the recipe on them); no network.

Adopting: copy `arch-gates.just` and `arch-rules.toml`, define `arch_lint` and `_arch-lint-ready`
in the justfile, and install the binary from a pinned commit of this repository
(`cargo install --git <this repository's URL> --rev <sha>
arch-lint --locked --root <dir>`), the same sha the consumer pins its other
crates of this repository at.

## Adopting

1. Append `disallowed-methods` from `clippy-disallowed.toml` to the root
   `clippy.toml`; repeat the array in any nearer `clippy.toml` (clippy reads
   only the nearest). Fix the calls clippy then flags onto `Fs`.
2. Copy the three scripts and `os-cfg-allow.txt` into `scripts/`.
3. Copy `os-gates.just` to `gates/` and `import 'gates/os-gates.just'` in the
   justfile, which defines `TOP`, `windows_packages` (cargo arguments, e.g.
   `-p adapter-a`) and `clippy_template` (the path to this file in a
   checkout of this repository). Make `lint` depend on `lint-os`.
4. Copy `ci.yml`; add protoc to both jobs if the workspace generates code.
5. After this repository's list changes, run `just lint-clippy-sync`; a
   missing entry is the failure. The consumer may disallow more, never less.

This repository's `just lint` runs the self-tests against fixture trees
(`scripts/*.selftest.sh`): a forbidden `std::fs` call is flagged by clippy
under the template, a clean fixture passes, a `cfg!(windows)` outside the
allowlist is flagged.

## The canonical list, and its reconciliation

The list is the 43 entries of the first consumer's root `clippy.toml`
(identical paths and reasons); the second consumer had none. `std::fs`
free functions, `File` and `OpenOptions`, the OS-specific symlink calls (the
Windows ones `allow-invalid`, since they do not resolve elsewhere), the
`Path` methods that touch the disk, and `tempfile`'s constructors are
disallowed; `Path` arithmetic (`join`, `parent`, `file_name`) does no I/O and
stays allowed. A test that needs a real file uses `Rooted::temp`.

The process environment is the same kind of seam: `std::env::var`, `var_os`,
`vars`, `vars_os`, `current_dir`, `temp_dir`, `home_dir` and the writers
(`set_var`, `remove_var`, `set_current_dir`) are disallowed. A program reads
the environment once, at its composition root, with `process-env`'s
`EnvSnapshot::from_process` (the one `#[allow(clippy::disallowed_methods)]`
site) and hands the snapshot down; a test builds one with `EnvSnapshot::of`.
