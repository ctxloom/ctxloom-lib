# Templates: the gate-kit consumers copy

Gates and clippy settings have no include mechanism, so a consumer copies
these files; the copies are independent. Nothing here is linked.

| File | Copy to | What it is |
|---|---|---|
| `clippy-disallowed.toml` | the `disallowed-methods` array into `clippy.toml` | the canonical OS and filesystem rule, each group commented with why |
| `os-gates.just` | `gates/os-gates.just`, then `import` it | `lint-os`, `lint-clippy-sync`, `check-windows` |
| `ci.yml` | `.github/workflows/ci.yml` | Linux gates plus a native Windows build and test |
| `../scripts/os-cfg-check.sh` | `scripts/` | no OS-naming cfg, `cfg!(`, `std::os::`, `split_paths` or target dependency under `crates/` |
| `../scripts/os-cfg-allow.txt` | `scripts/` | the seam: one `<file> \| <text on the line> \| <why>` per exception (empty by default; a stale entry fails) |
| `../scripts/clippy-sync-check.sh` | `scripts/` | fails when a `clippy.toml` lacks an entry of the template |

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
