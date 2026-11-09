# Messages for the crate's errors. Each id is an error's code without the
# crate's name, which the build prefixes (angzarr_diagnostic::build); the
# variables are its args.

not-found = { $program } was not found, or the directory to run it in was not: { $reason }
unrunnable = { $program } could not be started: { $reason }
wait-failed = { $program } could not be waited on, and was killed: { $reason }
kill-failed = { $program } outlived its deadline or was to be stopped, and could not be killed: { $reason }
