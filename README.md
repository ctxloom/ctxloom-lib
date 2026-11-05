# ctxloom-lib

The shared, engine- and tool-agnostic substrate that ctxloom's Rust
applications build on. It is the Rust successor to the archived Go
`ctxloom/shared`, and it is not a port of it: it holds what more than one
application needs and angzarr must not know.

It knows no application's domain. Each application passes its own names
(program name, directory and file names, environment prefix, root rule) as
arguments.

## Crates

| Crate | Path | Role |
|---|---|---|
| `cli-support` | `crates/adapters/driving/cli-support` | the driving edge's shared shims |
| `config-layers` | `crates/adapters/driven/config-layers` | layered configuration intake (env scan, override split, stamp check) |
| `sqlite-store-opener` | `crates/adapters/driven/sqlite-store-opener` | places and opens a project's SQLite event store |
| `process-env` | `crates/root/lib/process-env` | the once-read snapshot of the process environment |
| `project-dir` | `crates/adapters/driven/project-dir` | the project-root rule over the filesystem seam |

Today `cli-support` holds code (the call with an application-owned `globals`, the failure and its exit codes, the notices, the failure reporter, the argument readers); the others are stubs, and their code arrives slice by slice.

`templates/` holds files consumers copy (the OS and filesystem clippy rule);
it is not a crate.

## Using it

Pin the repository by full commit, as the angzarr crates are pinned:

```toml
cli-support = { git = "https://github.com/ctxloom/ctxloom-lib", rev = "<full sha>" }
```

All crates move together: a consumer bumps the whole repository, never one
crate. To build a consumer against a local checkout, see `CLAUDE.md`.

## Working on it

`just build`, `just test`, `just lint`, `just lint-deps`; see `CLAUDE.md`.

## Licence

AGPL-3.0-only: see [LICENSE](LICENSE).

Commercial licensing: this project is also available under a commercial
licence. The author is open to it; contact Benjamin Abbitt.
