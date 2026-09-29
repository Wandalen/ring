#!/usr/bin/env bash
# G22 — every exemption in declared/exempt.txt still earns it.
#
# The gate G18 structurally cannot be. G18 keeps the exemption count visible in
# its passing output on purpose — its own header says "an exemption is the one
# way to reach this gate without gating anything" — but a visible count is not a
# checked reason. G18 reads `exempt.txt` as a list of names to subtract and
# never asks whether any single entry's justification survived the commit that
# invalidated it. A visible count and an *earned* count print identically.
#
# So this gate reads the one exemption reason the tree can adjudicate for
# itself. An exemption entry expires by the same condition, written into
# `exempt.txt` in those words: an entry is "gated when it carries an
# implementation". That is not a matter of taste — the corpus already defines
# the Substance Threshold (`docs/workstream/readme.md`, Module States): a crate
# is implemented when its `src/` exports a `pub` item or carries `src/main.rs`,
# and a skeleton is a directory with neither. This gate applies that same
# threshold to the exemption list and fails on any entry that has crossed it.
#
# Why a new gate rather than a wider G18. The two ask opposite questions about
# the same file and want opposite answers when the tree changes. G18 asks "is
# every crate accounted for", and an exemption is a valid account. G22 asks "is
# this account still true", and an exemption is the thing under suspicion. Fold
# them together and the merged gate has to treat `exempt.txt` as both evidence
# and defendant on the same run.
#
# What this gate deliberately does not do: judge whether a *skeleton's*
# exemption is wise, or read the reason text at all beyond the expiry condition
# above. An exemption for a crate that genuinely has no implementation is this
# gate's definition of fine, however long it has sat there. Staleness of intent
# is a human reading; crossing the Substance Threshold is a machine one, and
# only the second is gradeable without inventing a policy the declarations never
# stated.
#
# Family-independent, like G18, and for the same reason: it reads a file that
# belongs to no family and must not appear in any family's gates.txt.
set -uo pipefail
GATE=G22
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"

# The list under test. Overridable so the gate's *passing* path can be executed
# against a control fixture, which is the one path a live run cannot reach while
# any exemption is expired. `common.sh` states the hazard for the corpus gates
# in general terms — "an empty result means 'the property holds' and 'the command
# is broken' equally well" — and a green branch that has never once run is that
# hazard in its purest form. The override is read-only and names a file; it
# cannot narrow what a default run grades.
EXEMPT="${GATE_EXEMPT:-$DECL_ROOT/exempt.txt}"

# The Substance Threshold, applied to one crate directory. Kept as a function
# with one caller-visible meaning so the non-vacuity probe below and the expiry
# check itself cannot drift apart into two different definitions of
# "implemented" — the failure mode `crate_rs_listing` exists to prevent for the
# survey gates.
crate_is_implemented() {
  local d
  d="$( crate_dir "$1" )" || return 1
  [ -f "$d/src/main.rs" ] && return 0
  # Fix(g22_modifier_qualified_pub_items_invisible): the item keyword had to
  # immediately follow `pub[[:space:]]+`, so `pub unsafe fn`, `pub async fn`,
  # `pub unsafe trait` and `pub extern "C" fn` — ordinary Rust, not an edge
  # case — matched nothing. Root cause: the pattern named the item-keyword set
  # but never accounted for the qualifier keywords Rust allows in front of
  # `fn`/`trait` (`unsafe`, `async`, `extern "abi"`, alone or combined), so a
  # crate whose entire public surface used a qualified form read as a bare
  # skeleton. Pitfall: `exempt.txt` is empty today, so no live exemption
  # currently depends on this, but the failure direction is the dangerous
  # one — a false "still unimplemented" that would hold a stale exemption
  # open indefinitely, exactly where this repository's unsafe-heavy crates
  # (`ring_spsc`, `ring_mpsc`, `mem_page`) are the likeliest future
  # candidates. Measured, not assumed: `pub unsafe fn`, `pub async fn`,
  # `pub unsafe trait` and `pub extern "C" fn` all failed to match the old
  # pattern in a direct test; only bare `pub fn` did.
  grep -rqE '^[[:space:]]*pub[[:space:]]+((unsafe|async|extern([[:space:]]+"[^"]*")?)[[:space:]]+)*(fn|struct|enum|trait|type|const|static|mod|use)\b' \
    "$d/src" 2>/dev/null
}

[ -f "$EXEMPT" ] || fail "no exemption list at ${EXEMPT#"$REPO"/} — this gate would report clean having read nothing"

# Non-vacuity, and the specific blindness worth guarding against. Every check
# below is a negative: an exemption passes by *not* looking implemented. If the
# threshold probe were broken — a renamed src/, a grep that matches nothing, a
# path that resolves wrong — every entry would pass and the gate would report
# REACHED precisely when it had stopped working. So the probe must first be
# shown to fire somewhere on this tree.
probe_hits=0
while read -r c; do
  crate_is_implemented "$c" && probe_hits=$(( probe_hits + 1 ))
done < <( all_crate_names )
[ "$probe_hits" -gt 0 ] \
  || fail "the Substance Threshold probe matched no crate anywhere under ${CRATE_ROOTS[*]#"$REPO"/} — the probe is broken, not the tree"

expired=() absent=() held=()
while read -r line; do
  name="${line%%[[:space:]]*}"
  [ -n "$name" ] || continue
  if ! crate_dir "$name" >/dev/null; then
    # G18 already reports a declared-but-absent name, and reporting it here too
    # would have one defect fail two gates and get counted twice in any sweep.
    absent+=( "$name" )
  elif crate_is_implemented "$name"; then
    d="$( crate_dir "$name" )"
    # Fix(a_shallow_glob_undercounts_nested_modules): was `cat "$d"/src/*.rs`,
    # a one-level glob — a crate with any `src/<mod>/*.rs` file (nested rather
    # than flat modules) had those lines missing from this count while the
    # `pubs` count two lines below already walked them via `grep -r`, the same
    # recursive-vs-shallow mismatch `g19_measured_columns.sh`'s `module_reading()`
    # carried before its own fix. Pitfall: diagnostic-only — this line runs
    # only inside the `expired` branch, which only executes once some
    # exemption has already crossed the Substance Threshold, and `exempt.txt`
    # currently declares zero entries, so today's gate never reaches it either
    # way. Generalised to the same recursive `find` used there.
    loc=$( find "$d/src" -name '*.rs' -exec cat {} + 2>/dev/null | grep -cvE '^[[:space:]]*$' )
    pubs=$( grep -rhoE '^[[:space:]]*pub[[:space:]]+((unsafe|async|extern([[:space:]]+"[^"]*")?)[[:space:]]+)*(fn|struct|enum|trait|type|const|static|mod|use)\b' \
      "$d/src" 2>/dev/null | wc -l )
    expired+=( "$name ($pubs public item(s), $loc src line(s))" )
  else
    held+=( "$name" )
  fi
done < <( decl_lines "$EXEMPT" )

n_total=$(( ${#expired[@]} + ${#absent[@]} + ${#held[@]} ))
if [ "$n_total" -eq 0 ]; then
  # Two different situations both read as zero here: every past exemption has
  # graduated to a family or been deleted — exempt.txt's own header records
  # exactly that — or $EXEMPT resolved to the wrong, accidentally-empty file
  # and decl_lines correctly found nothing in it. decl_lines cannot tell them
  # apart; it strips comments, so a comment-only retirement record and a truly
  # empty file both parse to zero lines. Raw byte count does distinguish them:
  # the curated file carries its history in comments and is never small; a
  # wrong-path read would be. This is the same non-vacuity philosophy as the
  # probe above, applied to "did we read the real file" instead of "does the
  # threshold logic work".
  bytes=$( wc -c < "$EXEMPT" )
  [ "$bytes" -ge 200 ] \
    || fail "${EXEMPT#"$REPO"/} read as $bytes byte(s) with 0 declared entries — too small to be the curated file; check \$GATE_EXEMPT and the path"
  pass "0 exemption(s) remain — every past entry graduated to a family or was deleted; probe verified on $probe_hits implemented crate(s)"
fi

if [ ${#expired[@]} -gt 0 ]; then
  echo "  ${#expired[@]} of $n_total exemption(s) expired — the crate carries an implementation:"
  printf '    %s\n' "${expired[@]}"
  echo "  fix: declare the crate in a family's declared/<family>/crates.txt and delete"
  echo "       its line from declared/exempt.txt, in the same change"
  fail "${#expired[@]}/$n_total exemption(s) expired, ${#held[@]} still earned, ${#absent[@]} absent from disk"
fi
pass "$n_total exemption(s), each still earned — ${#held[@]} skeleton(s), ${#absent[@]} absent from disk; probe verified on $probe_hits implemented crate(s)"
