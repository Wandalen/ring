#!/usr/bin/env bash
# G5 — only the declared crates are reachable from outside the family.
#
# Two halves. The assertion: no manifest outside the family names an
# undeclared family crate as a dependency. The non-vacuity pairing: the
# declared set is non-empty and every crate the family declares actually
# exists, which `assert_declared_crates_exist` establishes before either half
# runs. Without that pairing this gate is a conjunction over an empty set and
# reports REACHED against a family that has not been written — measured doing
# exactly that during an early review.
#
# "Outside the family" is decided by the declared membership list, not by a
# path glob. A family's own demo binary is typically named after the family
# and lives beside the crates it grades, so a bare `<prefix>_*` path test
# would class the family's own smoke binary as an external consumer and
# demand its whole dependency list be exported.
set -uo pipefail
GATE=G5
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

decl="$DECL/export_surface.txt"
[ -f "$decl" ] || fail "no declared export surface at ${decl#"$REPO"/} — stage M1 has not ruled"

mapfile -t allowed < <( decl_lines "$decl" | sort -u )
[ ${#allowed[@]} -gt 0 ] || fail "declared export surface is empty"

# The second half of the non-vacuity pairing, and the one an early skeleton
# reading forced: a declared surface has to actually BE a surface. With every
# crate empty, "no external consumer names an undeclared crate" is true
# because nothing is nameable at all — the same vacuous REACHED caught one
# level down, recurring one level up.
#
# Under a stage run this half grades only the export crates that stage owns —
# the same scoping G7 applies to smoke binaries, and for the same reason. A
# declared surface can name a crate that a later stage owns, so an unscoped
# hollow check failed every stage before that crate was reached, which is a
# gate reporting the plan's own ordering as a defect. Nothing escapes grading:
# the surface is declared once for the family, and the unscoped run — the one
# that decides the Final Goal — still checks every name on it.
graded=( "${allowed[@]}" )
if [ -n "${GATE_CRATES:-}" ]; then
  in_scope=()
  while read -r c; do
    printf '%s\n' "${allowed[@]}" | grep -qx -- "$c" && in_scope+=( "$c" )
  done < <( family_crates )
  graded=( "${in_scope[@]}" )
fi

hollow=()
for c in "${graded[@]}"; do
  n="$( grep -rhcE '^[[:space:]]*pub[[:space:]]' "$( crate_dir "$c" )/src" 2>/dev/null | paste -sd+ | bc )"
  [ "${n:-0}" -gt 0 ] || hollow+=( "$c" )
done
if [ ${#hollow[@]} -gt 0 ]; then
  fail "${#hollow[@]}/${#graded[@]} declared export crate(s) export nothing, so the surface is empty in substance: ${hollow[*]}"
fi

mapfile -t members < <( family_members )
[ ${#members[@]} -gt 0 ] || fail "family '$GATE_FAMILY' declares no members"

# The dependency names to look for: every family member, as an alternation.
#
# Disclosed, not fixed: this alternation matches against the dependency KEY —
# the identifier to the left of `=` in the manifest — never against a
# `package = "..."` rename inside the value. `alias = { path =
# "../real_crate_name", package = "real_crate_name" }` names real_crate_name as
# a real Cargo dependency while presenting the key `alias`, which matches no
# member name and so violations+= at line 74 never fires — an undeclared
# cross-family dependency taking this one specific shape would pass silently.
# Zero live trigger: no manifest anywhere in this repository uses
# `package = "` at all (confirmed by a repo-wide grep), so nothing is
# currently hidden by this gap. Closing it would mean parsing each dependency
# table's `package` value, not just its key, which this line's plain
# alternation-over-keys approach does not otherwise carry.
#
# A second, distinct shape the same key-based matching cannot see: Cargo's
# long-form `[dependencies.NAME]` dotted-table header, where the crate name
# never sits to the left of a bare `=` at all — it is the table's own name,
# and every key inside it (`path = `, `version = `, ...) is generic. A
# dependency declared that way would be invisible to this gate exactly as a
# `package =` rename is, for a syntactically unrelated reason. Zero live
# trigger, for the same reason: no manifest in this repository opens a
# `[dependencies.` (or `[dev-dependencies.`/`[build-dependencies.`) table
# anywhere — the only hit repo-wide is a trybuild fixture manifest under this
# gate's own gitignored build output, neither a family member nor reachable
# from outside one.
# Closing both gaps for good would mean an actual TOML parser rather than
# text matching, which is a larger change than a drive-by here.
member_re="$( printf '%s\n' "${members[@]}" | paste -sd'|' )"

violations=()
while read -r manifest; do
  # A family member's own manifest may name any sibling — the surface bounds
  # what leaves the family, not what moves inside it.
  owner="$( basename "$( dirname "$manifest" )" )"
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
  # Fix(g5_workspace_dependencies_read_as_edges): a `[workspace.dependencies]`
  # table names every family crate a member may inherit, and the scan read each
  # row as the root manifest depending on it — 24 violations the moment the
  # workspace centralized its dependencies. Root cause: a key-based match
  # cannot tell a declaration of availability from a dependency edge. The edge
  # is still seen where it is made: a consumer inheriting one writes
  # `ring_x = { workspace = true }` under its own dependency table.
  while read -r dep; do
    [ -n "$dep" ] || continue
    printf '%s\n' "${allowed[@]}" | grep -qx -- "$dep" || violations+=( "${manifest#"$REPO"/}: $dep" )
  done < <( awk '/^\[/ { skip = ( $0 == "[workspace.dependencies]" ) } !skip' "$manifest" \
              | grep -oE "^[[:space:]]*(${member_re})[[:space:]]*=" | tr -d ' =' | sort -u )
  # Hyphen-prefixed directories are temporary by convention and gitignored, so a
  # manifest inside one is scratch work rather than project content. Scanning
  # them makes the gate report a violation for a throwaway crate — observed at
  # S12 with a measurement harness written to produce numbers for a manual
  # check, which legitimately depended on a family member and was deleted the
  # same day.
  #
  # .claude/ worktrees are stale agent checkouts left behind by earlier
  # isolation:"worktree" Agent dispatches — old commits, pre-relocation paths,
  # nothing to do with the live tree being graded. Without this exclusion they
  # are the one thing this gate's repo-root find picks up that every other
  # gate script's $CRATE_ROOTS-scoped find never sees.
done < <( find "$REPO" -name Cargo.toml -not -path '*/target/*' -not -path '*/-*' -not -path '*/.claude/*' )

if [ ${#violations[@]} -gt 0 ]; then
  fail "${#violations[@]} undeclared cross-family dependency: ${violations[*]}"
fi

# Both halves are reported, and a skipped hollow half says so. A stage owning no
# export crate still has its dependency half graded — that half is a whole-repo
# scan and is stage-independent — but reporting only "confined to N crates"
# would read as though the substance check had run and passed.
if [ ${#graded[@]} -eq 0 ]; then
  pass "external dependencies confined to the ${#allowed[@]} declared crates; no export crate in this stage, so substance is graded by the stage that owns one"
fi
pass "external dependencies confined to the ${#allowed[@]} declared crates, ${#graded[@]} of which export a non-empty surface"
