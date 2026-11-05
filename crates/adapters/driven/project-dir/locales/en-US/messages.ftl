# Messages for the crate's errors. Each id is an error's code without the
# crate's name, which the build prefixes (angzarr_diagnostic::build); the
# variables are its args.

id-empty = a project id cannot be empty
id-nul = the project id { $id } has a NUL character in it
id-absolute = the project id { $id } is an absolute path: an id is one name, not a place
id-separator = the project id { $id } has a path separator in it: an id is one name, not a path
id-relative = the project id { $id } is a relative path component: an id is a name, not . or ..
