#!/usr/bin/env bash
# The vocabulary rule (CLAUDE.md): this repository is engine- and
# tool-agnostic, so its crates know neither the consumers' domains nor their
# names. Fail on `task`, `session`, `bundle`, `plan`, `harp`, `taskloom` or
# `ctxloom` (any case, as a word or the start of one: `tasks`, `task_id`,
# `Session`) in any .rs, .ftl, .toml or .md file under crates/ or templates/.
# A consumer's names reach a crate only as arguments, from the consumer.
#
# The exceptions are scripts/vocabulary-allow.txt, one per line:
#   <file> | <text on the line> | <why>
# so each carries its reason and a new one is seen in review; an entry that
# no longer matches anything fails too, so the list stays true.
#
# Usage: vocabulary-check.sh [root]   (root defaults to the repository)
set -euo pipefail
top=${1:-$(git rev-parse --show-toplevel)}
cd "$top"
allow_file=scripts/vocabulary-allow.txt
pattern='(^|[^[:alpha:]])(task|session|bundle|plan|harp|taskloom|ctxloom)'

files=() fragments=()
if [ -f "$allow_file" ]; then
  while IFS= read -r entry; do
    [[ -z ${entry//[[:space:]]/} || $entry == \#* ]] && continue
    IFS='|' read -r file fragment _ <<< "$entry"
    file=$(echo "$file" | xargs)
    fragment=${fragment#"${fragment%%[![:space:]]*}"}
    fragment=${fragment%"${fragment##*[![:space:]]}"}
    if [ -z "$file" ] || [ -z "$fragment" ]; then
      echo "FAIL lint-vocab: $allow_file: an entry needs a file and the text on its line: $entry" >&2
      exit 1
    fi
    files+=("$file") fragments+=("$fragment")
  done < "$allow_file"
fi
used=()
for _ in "${files[@]+"${files[@]}"}"; do used+=(0); done

found=0
if [ -d crates ] || [ -d templates ]; then
  dirs=()
  [ -d crates ] && dirs+=(crates)
  [ -d templates ] && dirs+=(templates)
  while IFS= read -r hit; do
    file=${hit%%:*}
    rest=${hit#*:}
    line=${rest#*:}
    allowed=0
    for i in "${!files[@]}"; do
      if [[ $file == "${files[$i]}" && $line == *"${fragments[$i]}"* ]]; then
        allowed=1 used[i]=1
      fi
    done
    [ "$allowed" = 1 ] && continue
    echo "$hit"
    found=$((found + 1))
  done < <(grep -rniE --include='*.rs' --include='*.ftl' --include='*.toml' --include='*.md' "$pattern" "${dirs[@]}" || true)
fi

stale=0
for i in "${!files[@]}"; do
  if [ "${used[$i]}" = 0 ]; then
    echo "$allow_file: matches nothing now: ${files[$i]} | ${fragments[$i]}" >&2
    stale=$((stale + 1))
  fi
done

if [ "$found" -gt 0 ]; then
  echo "FAIL lint-vocab: $found lines use a consumer's vocabulary: take it as an argument from the consumer, or list an allowed one in $allow_file with why" >&2
  exit 1
fi
if [ "$stale" -gt 0 ]; then
  echo "FAIL lint-vocab: $stale entries of $allow_file match nothing: remove them" >&2
  exit 1
fi
echo "ok   lint-vocab (crates, templates)"
