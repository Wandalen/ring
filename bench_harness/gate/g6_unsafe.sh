#!/usr/bin/env bash
# G6. Only declared crates opt out of the workspace `unsafe-code = "deny"`, and
# each such crate justifies it in its own docs/workaround/readme.md.
#
# The workspace denies unsafe outright. A crate that needs it must stay
# enumerated and justified instead of spreading crate by crate.
#
# Non-vacuity is the half that matters for a family whose allowlist is empty.
# The deny has to be in force. A crate inherits it only by carrying
# `[lints] workspace = true` in its own manifest. A crate that omits that line
# opts out of the whole workspace lint table, unsafe and missing docs included,
# without ever writing the `#![allow(unsafe_code)]` line this gate scans for.
# Checking the allowlist alone would then be a conjunction over an empty set:
# nothing declared, nothing found, REACHED, and the lint that makes the finding
# meaningful never ran.
set -uo pipefail
GATE=G6
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

decl="$DECL/unsafe_allowlist.txt"
[ -f "$decl" ] || fail "no declared unsafe allowlist at ${decl#"$REPO"/} — stage M1 has not ruled"

mapfile -t allowed < <( decl_lines "$decl" | sort -u )

# Fix(g6_stale_allowlist_entry_never_checked): a crate declared into `allowed`
# but carrying no `#[allow(unsafe_code)]` anywhere in its src/ hit the
# `continue` below on every iteration. That happened unconditionally, before the
# `allowed` membership check ran, so the loop never looked it up in `allowed` at
# all, in either direction. A crate that stopped needing the exemption (the
# unsafe code was refactored away, or never used the form this scan recognizes)
# stayed declared forever with nothing to notice. It was not `undeclared` (that
# arm needs the grep on line ~56 to succeed first) and not `unjustified` (same
# precondition). It was invisible. `mutant_survey.sh` already guards against the
# same shape for its own accepted/ list ("An acceptance matching nothing is
# reported as stale rather than ignored"), and the guard was missing here.
# Root cause: the loop only ever asked "is this crate's opt-out declared," never
# "is every declaration's crate still opting out." Pitfall: live, not latent.
# `ring_core` is declared in `declared/ring/unsafe_allowlist.txt` today and
# carries zero `unsafe` usage anywhere in its src/ (checked directly). So G6
# reported REACHED for ring having verified 2 of its 3 declared entries, the
# third never examined. `seen` tracks every crate the loop matched an
# `#[allow(unsafe_code)]` for. Anything declared but never seen is stale.
declare -A seen=()
uninherited=() undeclared=() unjustified=() stale=() n=0
while read -r c; do
  n=$(( n + 1 ))

  # The lint table must reach this crate, or the deny below is not in force.
  grep -Pzoq '\[lints\][^\[]*workspace[[:space:]]*=[[:space:]]*true' "$( crate_dir "$c" )/Cargo.toml" 2>/dev/null \
    || uninherited+=( "$c" )

  # Spaces inside the brackets and parens are the house codestyle. The real
  # spelling is `#![ allow( unsafe_code ) ]`. A pattern written without them
  # matched no crate in this family, so the `continue` below fired for every
  # crate and the justification half never ran. The gate reported
  # "opt-outs confined to the declared and justified" having checked none of
  # them. That is the same vacuity the header warns about, arriving through the
  # scan rather than through an empty allowlist.
  #
  # Fix(g6_unsafe_outer_form_allow_blind_spot): the pattern required the `!`
  # inner-attribute bang unconditionally. It saw a crate- or module-scoped
  # `#![allow(unsafe_code)]` but not the outer, item/fn-scoped
  # `#[allow(unsafe_code)]`. The outer form is legal at any nesting depth
  # because the workspace lint is `deny`, not `forbid` (only `forbid` blocks a
  # narrower override; Cargo.toml:613 confirms `deny`). A crate using only the
  # outer form hit the `continue` below unconditionally and skipped the
  # declared/justified check entirely. That is the same
  # "vacuity arriving through the scan" shape the comment above already names
  # for the missing-spaces gap. No crate in the live corpus uses the outer form
  # today (checked across every crate root), so this was latent, not exploited.
  # Residual, not fixed here: a combined multi-lint attribute such as
  # `#[allow(dead_code, unsafe_code)]` still would not match. No live crate uses
  # that shape, and the house codestyle above is single-lint-per-attribute.
  grep -rqE '^\s*#!?\[ *allow\( *unsafe_code *\) *\]' "$( crate_dir "$c" )/src" 2>/dev/null || continue
  seen[ "$c" ]=1
  if [ ${#allowed[@]} -gt 0 ] && printf '%s\n' "${allowed[@]}" | grep -qx "$c"; then
    grep -qiE 'unsafe' "$( crate_dir "$c" )/docs/workaround/readme.md" 2>/dev/null || unjustified+=( "$c" )
  else
    undeclared+=( "$c" )
  fi
done < <( family_crates )

for a in "${allowed[@]}"; do
  [ -n "${seen[ "$a" ]:-}" ] || stale+=( "$a" )
done

if [ ${#uninherited[@]} -gt 0 ]; then
  fail "${#uninherited[@]}/${n} crate(s) do not inherit the workspace lint table, so unsafe-code=deny is not in force for them: ${uninherited[*]}"
fi

if [ ${#undeclared[@]} -gt 0 ] || [ ${#unjustified[@]} -gt 0 ] || [ ${#stale[@]} -gt 0 ]; then
  fail "${#undeclared[@]} undeclared unsafe opt-out (${undeclared[*]}), ${#unjustified[@]} unjustified (${unjustified[*]}), ${#stale[@]} stale allowlist entry(s) no longer using it (${stale[*]})"
fi
pass "${n} crate(s) inherit unsafe-code=deny; opt-outs confined to the ${#allowed[@]} declared and justified"
