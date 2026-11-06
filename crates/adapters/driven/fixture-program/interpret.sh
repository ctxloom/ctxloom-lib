#!/bin/sh
# A fixture program's shim, committed and never written by any test process.
#
# `fixture_program::install_program` links a program (`git`, `docker`, ...) to
# this file and writes the script beside it as `<program>.body`. Running the
# program runs this file, which runs the body with `sh`; `sh` only reads the
# body, and reading a file does not conflict with a writer.
#
# Exec'ing a file a test has just written is racy in a multithreaded test
# process: a thread that forks while the file is open for writing gives its
# child a copy of the descriptor until that child execs, and until then the
# exec fails with "text file busy" (ETXTBSY) though the file is closed. This
# file is only ever read by the process (it is in the repository, not made by
# a test), so no process holds it for writing. `$0` in the body is
# `<program>.body`.
exec /bin/sh "$0.body" "$@"
