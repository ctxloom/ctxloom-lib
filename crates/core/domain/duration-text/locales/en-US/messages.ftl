# Messages for the crate's errors. Each id is an error's code without the
# crate's name, which the build prefixes (angzarr_diagnostic::build); the
# variables are its args.

empty = no span given: write a number and a unit, such as 90m, 1h30m or 1.5d, or a date such as 2026-01-01
whitespace = { $text } has a space inside it: write the span as one word, such as 1h30m
negative = { $text } is negative: a span is written without a sign
non-finite = { $text } is not a finite number
malformed = { $text } is not a span: write a number and a unit, such as 90m, 1h30m or 1.5d
no-unit = { $text } has a number with no unit after it: write 30d or 1h30m, not 30
unknown-unit = { $unit } in { $text } is not a unit: the units are w, d, h, m and s
repeated-unit = { $unit } is written twice in { $text }: each unit may appear once
out-of-order = { $unit } comes after { $after } in { $text }: write the longest unit first, as in 1h30m
overflow = { $text } is longer than a span or a date can hold
invalid-date = { $text } is not a date: write it as YYYY-MM-DD, such as 2026-01-01
