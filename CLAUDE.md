# Introduction

ctxloom-lib is the shared substrate of ctxloom's Rust applications (taskloom,
ctxloom-rs): the pieces both need and angzarr must not know. It is the Rust
successor to the archived Go `ctxloom/shared`. It is engine- and
tool-agnostic and public; nothing here is private to one application.

It is extracted slice by slice from the applications, each slice a code move
with its tests (characterization tests are committed in the consumer BEFORE
the move). A crate here that is still a stub holds no code yet.

# The vocabulary rule

This repository knows neither `task`, `session`, `bundle`, `plan`, `harp`,
`taskloom` nor `ctxloom`, nor any consumer's name, in any `.rs`, `.ftl`,
`.toml` or `.md` under `crates/` or `templates/`. A consumer's names reach a
crate only as arguments: the program name, directory and file names, the
environment prefix, the root rule's marker, the store's file name. A default
that names a consumer is a leak. Tests use made-up names (`.demo`).

`just lint-vocab` (part of `just lint`) enforces it
(`scripts/vocabulary-check.sh`, itself tested by
`scripts/vocabulary-check.selftest.sh`). A word used for another meaning is an
exception in `scripts/vocabulary-allow.txt`, with its reason; a stale
exception fails. Diagnostic codes are prefixed with the crate's name.

Scope rulings (owner, 2026-11-04): configuration here is intake mechanics
only (each consumer keeps its schema); the root rule is mechanics only, the
stop predicate injected.

# Working rules

## Tests go in `*.test.rs` files beside the code they test

A source file `foo.rs` keeps its tests in `foo.test.rs` in the same
directory, included at the end of `foo.rs`:

```rust
#[cfg(test)]
#[path = "foo.test.rs"]
mod tests;
```

Not inline `mod tests { ... }`, and not a `tests/` directory.

## Layout: a crate's directory is its role

Every crate lives at `crates/<ring>/<role>/<crate>`. The ring and role are
the architecture; there is no other table of crates. This repository is
adapters, root helpers and a little pure core, so far:

- `adapters/driven`: port implementations over the filesystem, the store.
- `adapters/driving`: the CLI edge's shims: translation only.
- `root/lib`: helpers for a binary's composition root.
- `root/bin`: binaries: `arch-lint`, the architecture lint (`templates/README.md`),
  which holds this layout to `arch-rules.toml` (`just lint-arch`, part of `just
  lint`).

- `core/domain`: pure computation (no I/O, clock or environment; "now"
  arrives as an argument or a factory): `duration-text`, the grammar of a
  span or a date as text. The core's purity rules (`crates/core/clippy.toml`)
  repeat the root's, since clippy reads the nearest `clippy.toml`; `just lint`
  checks the nesting threshold and every OS-rule entry are in it. A further
  core role (`core/ports`, ...) appears only if a crate needs it. A crate takes plain
`&str` and `Path` arguments, never a consumer's type, so no consumer type
can leak in. `templates/` is not a crate: files consumers copy.

## OS and filesystem go through the platform seams

All file I/O goes through angzarr-platform's `file-system` (`Fs`), and what
differs between operating systems lives in its `os` crate. `clippy.toml`
disallows the `std::fs`, `std::path::Path` and `tempfile` methods that
bypass it, and the `std::env` reads and writes: the environment is read once
into `process-env`'s `EnvSnapshot`, the only allowed site. The same list is `templates/clippy-disallowed.toml`, the canonical copy
consumers take; `just lint` refuses to run if the two disagree, and a consumer
checks its own with `just lint-clippy-sync`. `just lint-os` (`scripts/os-cfg-check.sh`)
keeps OS-naming cfgs out of `crates/`; the gate-kit (`templates/README.md`) says
what consumers copy. Their self-tests are `scripts/*.selftest.sh`.

## Errors are localizable: a code, args, and an en-US message

Every error implements `angzarr_diagnostic::Diagnostic` with an English
`Display` fallback, a stable code prefixed with the crate's name, an en-US
message in `locales/en-US/<crate>.ftl` using exactly its args, and a sample in
the crate's `diagnostic_samples()`; the crate exports its catalog as `EN_US`
for each consuming binary to assemble. Generated code is never committed.

## Pins: by full commit, one of each

Every dependency outside this repository is a git dependency pinned by full
commit (never a branch) in the workspace `Cargo.toml`. The angzarr crates are
pinned to the SAME commits as ctxloom-rs and taskloom pin them, so a
consumer's lock holds one of each. An angzarr re-pin lands here first, then
in taskloom, then in ctxloom-rs, each green.

The consumers pin this repository by full commit too
(`git = "https://github.com/ctxloom/ctxloom-lib", rev = "<sha>"`), all its
crates at one rev. A commit a consumer has pinned is immutable: no rebase or
amend of it, or the pin dangles. Where a consumer reaches this repository
twice (directly and through another consumer's crate), both shas are equal.

## Co-developing with a consumer

Until this repository is pushed, and while changing it with a consumer, the
consumer builds against this checkout through a gitignored `[patch]` in its
`.cargo/config.toml`, copied from its `.cargo/local-deps.toml`:

```toml
[patch."https://github.com/ctxloom/ctxloom-lib"]
cli-support = { path = "../ctxloom-lib/crates/adapters/driving/cli-support" }
# one line per crate the consumer uses
```

CI, a fresh clone and the pre-commit gates never read it. A consumer
restores its `Cargo.lock` before committing, and pins the sha of a commit
here. This repository's own `.cargo/local-deps.toml` does the same for the
angzarr crates (`cp .cargo/local-deps.toml .cargo/config.toml`, `rm` to turn
it off); restore `Cargo.lock` before committing.

## Use `just`

`just build`, `just test` (cargo-nextest, required), `just lint` (clippy,
warnings denied, plus the vocabulary and template checks), `just lint-deps`
(cargo-deny), `just fmt` and `just fmt-check`; `just test-pkg <crate>` for the
quick loop. Every gate is terse: a pass prints one `ok <gate> (<scope>)
<duration>` line; a failure prints `FAIL <gate>: ...` and the extract, with
the full log in `target/logs/<gate>.log` (`VERBOSE=1` shows everything).

## The pre-commit gates are in gates/; the justfile gives their parameters

`gates/` is a copy of ctxloom-rs's gates (`gates/gates.just`,
`gates/lefthook.yml`, `gates/MIGRATING.md`; it came from ctxloom-rs's
`main`, itself from the shared gates repository, with ctxloom-rs's history
there). The copies are independent: a change here does not reach the others.
The justfile gives them this repository's parameters. `just hooks-install`
installs the hooks, once per clone.

Pre-commit runs, one at a time, stopping at the first failure: fmt (formats
the staged `*.rs` in the index), secrets (gitleaks, `.gitleaks.toml`), clippy
(`just lint`), deps (`just lint-deps`, `deny.toml`) and tests (`just
test-commit`). A failing gate is a finding: fix the cause and commit again;
never `--no-verify`, and never weaken a gate to get through.

## Mutation is a by-hand, required close-out step

No git hook runs mutation testing. Before a batch of work reaches `main`,
`just mutants-staged` runs over the whole batch's diff (staged against the
batch's base) and passes: every hand-written tier (`crates/core`, `crates/adapters`,
`crates/root/lib`, `crates/root/bin`) at 90% or more (the justfile's `mutation_thresholds`).
Each mutant runs only its own crate's tests (`.cargo/mutants.toml`), so a
crate kills its own mutants. A mutant judged equivalent or untestable gets
`#[cfg_attr(test, mutants::skip)]` on its function, with `mutants` a
dev-dependency (`mutants.workspace = true`) and a one-line comment above it
saying why; never an exclude in `.cargo/mutants.toml` or a lowered
threshold. Nothing enforces this but the author: merging without it is how
survivors reach `main`. Stubs have no logic and so no mutants; the first
commit of real code in a crate runs it.
