#!/usr/bin/env bash
# G18 — every crate under module/ is claimed by exactly one gate family, or
# exempted with a written reason.
#
# The gate the other seventeen cannot be: each of those grades the crates its
# family declares, and is silent about a crate no family declares at all. A
# family whose crates.txt names four of its ten crates reports the same
# "REACHED" as one that names all ten, and nothing in a per-family run can tell
# the two apart. Coverage is the only property that has to be read across
# families rather than within one, which is why this gate is family-independent
# and does not appear in any family's gates.txt.
#
# Exactly one, not at least one. A crate named by two families is graded twice
# under two different gate lists, and a fix that satisfies one can break the
# other with no run reporting the contradiction — so a double claim fails here
# rather than being tolerated as redundant coverage.
#
# Exemptions are counted out loud on every run, including a reached one. An
# exemption is the one way to reach this gate without gating anything, so the
# count has to stay visible in the passing output; a gate that goes quiet once
# it is green cannot report that it went green by emptying itself.
set -uo pipefail
GATE=G18
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"

EXEMPT="$DECL_ROOT/exempt.txt"

# Crates on disk: a directory under any crate root carrying a manifest. The
# manifest is what separates a crate from a bare directory, and is the same
# test cargo itself applies when resolving a workspace member.
mapfile -t disk < <( all_crate_names )

[ ${#disk[@]} -gt 0 ] || fail "no crate manifests found under ${CRATE_ROOTS[*]#"$REPO"/} — the gate would report clean having read nothing"

mapfile -t families < <( find "$DECL_ROOT" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort )
[ ${#families[@]} -gt 0 ] || fail "no families declared under ${DECL_ROOT#"$REPO"/} — nothing to check coverage against"

# Claim table: crate -> space-separated family names that name it. Built by
# reading every family's declaration rather than the one this run defaults to.
declare -A claim=()
for f in "${families[@]}"; do
  while read -r c; do
    [ -n "$c" ] || continue
    claim["$c"]="${claim[$c]:-} $f"
  done < <( decl_lines "$DECL_ROOT/$f/crates.txt" )
done

# Exemptions: `crate_name  reason`. A bare name with no reason is not an
# exemption, it is an omission wearing one, so it is collected separately.
declare -A exempt=()
reasonless=()
if [ -f "$EXEMPT" ]; then
  while read -r line; do
    name="${line%%[[:space:]]*}"
    reason="${line#"$name"}"
    reason="${reason#"${reason%%[![:space:]]*}"}"
    [ -n "$name" ] || continue
    if [ -z "$reason" ]; then
      reasonless+=( "$name" )
    else
      exempt["$name"]=1
    fi
  done < <( decl_lines "$EXEMPT" )
fi

unclaimed=() doubled=()
for c in "${disk[@]}"; do
  set -- ${claim[$c]:-}
  case $# in
    0 ) [ -n "${exempt[$c]:-}" ] || unclaimed+=( "$c" ) ;;
    1 ) [ -z "${exempt[$c]:-}" ] || doubled+=( "$c: claimed by $1 and also exempted" ) ;;
    * ) doubled+=( "$c: claimed by $*" ) ;;
  esac
done

# A name in a declaration with no crate behind it. The mirror of an unclaimed
# crate and the more dangerous direction: it inflates a family's apparent reach
# while grading nothing, and every per-family gate skips it in silence.
stale=()
for c in "${!claim[@]}"; do
  crate_dir "$c" >/dev/null || stale+=( "$c (claimed by${claim[$c]})" )
done
for c in "${!exempt[@]}"; do
  crate_dir "$c" >/dev/null || stale+=( "$c (exempted)" )
done

problems=0
if [ ${#unclaimed[@]} -gt 0 ]; then
  problems=$(( problems + ${#unclaimed[@]} ))
  echo "  ${#unclaimed[@]} crate(s) in no family and not exempted:"
  printf '%s\n' "${unclaimed[@]}" | sed 's/_.*//' | sort | uniq -c | sort -rn \
    | awk '{ printf "    %3d  %s_*\n", $1, $2 }'
  printf '    %s\n' "${unclaimed[@]}"
fi
for m in "${doubled[@]}"     ; do echo "  double claim: $m"; problems=$(( problems + 1 )); done
for m in "${stale[@]}"       ; do echo "  declared but absent from disk: $m"; problems=$(( problems + 1 )); done
for m in "${reasonless[@]}"  ; do echo "  exemption without a reason: $m"; problems=$(( problems + 1 )); done

n_ex=${#exempt[@]}
n_fam=${#families[@]}
if [ "$problems" -gt 0 ]; then
  echo "  fix: add the crate to a family's declared/<family>/crates.txt, or to"
  echo "       declared/exempt.txt with the reason it is not gated"
  fail "${#disk[@]} crate(s) across $n_fam famil(ies), $problems unresolved, $n_ex exempted"
fi
pass "${#disk[@]} crate(s), each claimed exactly once across $n_fam famil(ies), $n_ex exempted"
