# Exit codes: a convention for command-line programs

This document is a convention any command-line program can follow. It is
written to be copied into another project unchanged; nothing in it names a
particular program.

## The blocks

An exit code says which of four things happened, by the block it falls in:

| codes   | class    | meaning                                                        |
|---------|----------|----------------------------------------------------------------|
| 0-9     | success  | the command did what was asked                                 |
| 10-19   | failure  | the command tried, and something broke                         |
| 20-29   | decline  | the command did not try: the request or the state forbids it   |
| 128+n   | signal   | the command was stopped by signal `n`                          |

Codes 30-127 belong to no block and are not used.

Within the success block, 0 is a plain success and 1-9 are *distinguishing*
successes: the command succeeded, and it has something more to say about how.
They are returned **only** when the caller asked for them with
`--detailed-exitcode`. Without that flag every success exits 0.

### Why only the success block collapses

By long Unix convention a non-zero exit status means an error. `set -e`,
`&&`, `||`, make and every CI system treat any non-zero status as a failure.
A success must therefore exit 0 unless the caller has said it understands
otherwise, which is what `--detailed-exitcode` is: the caller opting in to
reading 1-9 as successes.

The failure (10-19) and decline (20-29) blocks are always returned as they
are, with or without the flag. They are already non-zero, so under that same
convention they already read correctly as errors; collapsing them would only
lose information.

## The defined codes

| code | name             | class   | when                                                                                   |
|------|------------------|---------|----------------------------------------------------------------------------------------|
| 0    | ok               | success | the command succeeded; also a dry run or preview that succeeded                       |
| 1    | would change     | success | a dry run or preview succeeded and applying it would change something (flag only)     |
| 10   | failed           | failure | the command tried and something broke: I/O, a store, a child program, the network     |
| 20   | refused          | decline | the request or the state forbids it: a bad argument, a usage error, a taken name, unreadable configuration |
| 21   | fatal findings   | decline | the command examined its input and found it cannot produce a working result            |
| 128+n| signal n         | signal  | the command was asked to stop by signal `n` and did                                    |

What belongs where:

- **Success (0-9).** Anything that completed as asked. A dry run, a preview
  or a check that ran to its end is a success even when what it reports is
  that something would change; the distinction is a success subcode, not an
  error.
- **Failure (10-19).** The command made an honest attempt and the attempt
  broke for a reason outside the request: a file could not be written, a
  database was locked, a child process crashed, a remote could not be
  reached. Trying again later, unchanged, may succeed.
- **Decline (20-29).** The command decided not to act, before or instead of
  acting: the request is malformed (a usage error from the argument parser
  is a decline, 20, not the parser's own exit status), the state forbids it
  (the thing is in use, the name is taken), or the input was examined and
  found unworkable (fatal findings, 21). Trying again unchanged will be
  declined again; something in the request or the state has to change.
- **Signal (128+n).** The shell's own convention for a process ended by a
  signal, kept so that a program that catches a signal, cleans up and exits
  reads to its caller exactly as if the signal had ended it.

An argument parser's request for help or for the version is a success (0),
not a usage error.

A child program's own exit status is not one of these codes, and is never
passed through as it is: a child's 1 would read as a success subcode. It is
said in the classes: 0 is ok, a child ended by signal `n` (or a shell's
`128+n`) is signal `n`, and any other status is a failure (10).

## When two outcomes apply

A command sometimes has more than one thing to report: a launch both refused
and found fatal problems in its input, or a batch in which one item failed
and another was declined. It exits with the **most severe**, and severity is
the order of the codes themselves:

```text
signal (128+n)  >  decline (20-29)  >  failure (10-19)  >  success (0-9)
```

and within a block the higher code wins (21 fatal findings over 20 refused;
1 would change over 0 ok; signal 15 over signal 2). One rule, "the higher
detailed code wins", is the whole order.

Why a decline outranks a failure: a decline is a definitive verdict on the
request, while a failure may be transient. A caller that sees a failure is
invited to retry unchanged; if part of the same request was also declined,
that retry is declined again. Reporting the decline tells the caller the one
thing that must happen whatever else does: the request or the state has to
change. A failure is still said in the command's messages; only the code
carries the decline.

Why a signal outranks everything: the command was stopped from outside, so
nothing it would otherwise have reported is known to be complete.

Why any error outranks a success: an outcome that is partly an error is an
error to a caller reading the status under the Unix convention.

All the messages are still printed; only the exit code is chosen.

## Precedent

A dry run that succeeds exits 0 in the tools people already script against:
`rsync --dry-run`, `apt-get -s` and `terraform plan` all exit 0 when the
dry run itself worked, whatever it found. `terraform plan
-detailed-exitcode` is the model for the opt-in: it exits 0 for no changes,
2 for changes present and 1 for an error. This convention keeps the opt-in
and moves the distinguishing successes into a block of their own (1-9), so a
success subcode can never be confused with an error code.

The blocks are ten wide so that each class has room for subcodes. A caller
that only cares about the class tests the block (`status >= 10 && status <
20` is a failure); a caller that cares about the detail reads the code. A
new subcode never changes the class an existing caller sees.

## Adding a subcode

1. Decide its class first: did the command succeed, try and break, or
   decline? The class is the block; it is what every existing caller reads.
2. Take the next free code in that block: success 2-9, failure 11-19,
   decline 22-29. Never reuse a code that was published with another
   meaning.
3. A new success subcode is returned only under `--detailed-exitcode`, like
   1. It must be a success the caller might want to branch on (as "would
   change" is); a warning is not a subcode.
4. Place it in the severity order: within its block, a higher code is more
   severe. If the new subcode should not outrank an existing one, it is the
   wrong block or the wrong number.
5. Document it in this table and in the program's own list of codes.

## The type that encodes it

In the `exit-code` crate (pure: no I/O and no parser, so a program's core
may name its errors' classes with it, and re-exported by `cli-support`, the
command-line edge) the convention is the type `Exit` (and its `Class`):
constructors for each block that refuse a code outside it
(`Exit::success`, `Exit::failed`, `Exit::declined`, `Exit::signal`), the
defined codes as constants (`Exit::OK`, `Exit::WOULD_CHANGE`,
`Exit::FAILED`, `Exit::REFUSED`, `Exit::FATAL_FINDINGS`), `Exit::code`, the
single function that turns an exit and the `--detailed-exitcode` flag into
the process's status and holds the collapse rule, `Exit::worst` (and the
type's ordering), which is the severity order, and `Exit::from_child_status`,
which says a child program's status in the classes, and `Exit::usage`, which maps
an argument parser's verdict to 20 for a usage error (or to 0 for help and
version). An error type says its own class by implementing `ExitClass`. In
`cli-support`, a command's `Failure` carries its `Exit` (`Failure::of` takes
it from an `ExitClass` error), `usage_exit` reads a clap error, and
`detailed_exitcode_arg` and `detailed_exitcode` declare and read the global
flag.
