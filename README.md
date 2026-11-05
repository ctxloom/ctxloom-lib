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
| `sqlite-store-opener` | `crates/adapters/driven/sqlite-store-opener` | opens a project's SQLite event store at a caller-named path (to write or read-only), refuses one from before the embedded store, and checks one for damage |
| `process-env` | `crates/root/lib/process-env` | the once-read snapshot of the process environment |
| `project-dir` | `crates/adapters/driven/project-dir` | the project-root rule over the filesystem seam |
| `run-within` | `crates/adapters/driven/run-within` | run a program under a deadline: its process tree killed, its output bounded, the wait for it bounded |
| `duration-text` | `crates/core/domain/duration-text` | the grammar of a span (`1h30m`, `1.5d`) or a date as text: parse, cutoff, age and exact comparison, over std's `Duration` and `SystemTime` |

`duration-text` holds code too (pure; see below). Today `cli-support` holds code (the call with an application-owned `globals`, the failure and its exit codes, the notices, the failure reporter, the argument readers); the others are stubs, and their code arrives slice by slice.

`templates/` holds files consumers copy (the OS and filesystem clippy rule);
it is not a crate.

## duration-text

`1h30m`, `2w1d`, `1.5d`, `0d`: units `s m h d w` (fixed lengths), longest to
shortest, each once; fractions; no bare number, sign or inner space. A
cutoff is a span before now or a `YYYY-MM-DD` date at 00:00 UTC. The crate
reads no clock: build a `DurationText` over a closure on your own clock (or a
fixed `SystemTime` in tests), and `frozen()` it to judge many items against
one instant.

```rust
use std::time::{Duration, SystemTime};
use duration_text::{Comparison, DurationText};

// `clock.now()` is the application's own clock seam, converted to std.
let spans = DurationText::new(|| SystemTime::from(clock.now())).frozen();
let cutoff = spans.cutoff("30d")?;           // or "2026-01-01"
let old = spans.age_is(created, Comparison::AtLeast, spans.parse("1.5h")?);
```

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
