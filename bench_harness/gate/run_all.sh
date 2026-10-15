#!/usr/bin/env bash
# Runs every gate declared for a family and reports one line each. Exits 0 only
# when all of them report REACHED. That conjunction is the Final Goal's own
# reached-test.
#
# Usage:
#   run_all.sh                        every declared gate, whole family (the Final Goal)
#   run_all.sh g2 g3                  named gates, whole family
#   run_all.sh --stage S1             every declared gate, scoped to stage S1
#   run_all.sh --stage S1 g1 g3       named gates, scoped
#   run_all.sh --family ring          another family, whole family
#   run_all.sh --family ring --stage S4
#   run_all.sh --every                every family-independent gate plus every declared family
#
# A scoped run is a stage's own reached-test, per the plan's Stage Table column
# "G1-G9 over the N crates". Unscoped is the Final Goal's.
#
# Which family a bare run grades is declared in `declared/family.txt`, and which
# gates apply to it in `declared/<family>/gates.txt`. Every run echoes the
# loaded gate list, so a gate dropped from a family shows up in that family's
# own output, not only in a diff of its declaration.
set -uo pipefail
here="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
GATE=run_all

# `--every` is the whole-repository reading. It runs each family-independent
# gate once, then every family, in one process with one exit status.
#
# It exists because that reading used to be assembled by hand: one invocation
# per family, the family-independent gates run standalone, then arithmetic. A
# family can drop out of a reading assembled by hand with no message. This
# script gets the family list by listing `declared/` and never writes it down
# here. A hardcoded list would bring back the omission this mode exists to
# prevent, one directory at a time, and a family added tomorrow would be missing
# from the total with nothing saying so.
#
# The script runs the family-independent gates directly, not through a family.
# Each is family-independent by construction and appears in no `gates.txt`.
# Coverage has to be read across families, not within one. The family Overview
# Table spans every family this repository could ever declare. `exempt.txt`
# belongs to no family either.
#
# `declared/repo_gates.txt` declares which gates those are; this file does not
# list them. The family list two paragraphs above is discovered because a member
# can drop out of a hardcoded list with no message, and that reason applies
# unchanged to this set. The set was hardcoded anyway while G18 and G19 were the
# whole of it. Adding a third made the gap visible.
if [ "${1:-}" = "--every" ]; then
  shift
  [ $# -eq 0 ] || {
    echo "--every takes no other arguments — it already runs every gate of every family" >&2
    exit 2
  }
  source "$here/common.sh"

  mapfile -t every < <( find "$DECL_ROOT" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort )
  [ ${#every[@]} -gt 0 ] || {
    echo "no families declared under ${DECL_ROOT#"$REPO"/} — nothing to run" >&2
    exit 2
  }

  mapfile -t repo_gates < <( decl_lines "$DECL_ROOT/repo_gates.txt" )
  [ ${#repo_gates[@]} -gt 0 ] || {
    echo "no family-independent gates declared in ${DECL_ROOT#"$REPO"/}/repo_gates.txt" >&2
    exit 2
  }

  short=0

  # Run each family-independent gate once, before the family blocks. None
  # appears in any gates.txt, so this preamble is the only place any of them
  # runs. That is why the set comes from a declaration and is not listed here.
  for g in "${repo_gates[@]}"; do
    found=( "$here/${g}"_*.sh )
    [ -e "${found[0]}" ] || {
      echo "repo_gates.txt declares '${g}' and no gate script matches ${g}_*.sh" >&2
      exit 2
    }
    echo "══ ${g^^}, family-independent"
    bash "${found[0]}" || short=$(( short + 1 ))
    echo
  done

  for f in "${every[@]}"; do
    echo
    echo "══ family: $f"
    # Each family gets a fresh process with the ambient scope cleared. A
    # GATE_FAMILY or GATE_STAGE inherited from the caller would regrade one
    # family under another's scope with no warning, and the block header would
    # still name the right one.
    #
    # Fix(run_all_every_gate_control_leak_across_families): GATE_CONTROL was
    # missing from this list. An --every run started in a shell that still had
    # `export GATE_CONTROL=1` left over from an earlier `--control` invocation
    # would carry it into every family sub-invocation unnoticed. The
    # "── control: grading the seeded fixture" banner only prints when
    # `--control` is read as a positional argument below, and this re-invocation
    # never passes it. A family's corpus gates (G14-G17, G20-G21) would then
    # grade corpus_control/ fixtures instead of the family's real docs/, report
    # REACHED/NOT REACHED like a normal verdict, and give no sign anywhere in
    # the output that the scope had been swapped. Reproduced directly: a
    # subshell using the old unset list, with GATE_CONTROL exported ambiently
    # beforehand, still saw GATE_CONTROL=1 afterward. Bash does not require
    # re-listing an already-exported var for it to survive a subshell. Only an
    # explicit `unset` removes it. No live trigger: no script in this repository
    # exports GATE_CONTROL outside this file's own `--control` branch below.
    # That branch is scoped to run_all.sh's own process and never escapes to a
    # parent shell that might later invoke `--every` in the same session.
    ( unset GATE_FAMILY GATE_CRATES GATE_FEATURES GATE_STAGE GATE_CONTROL
      bash "$0" --family "$f" ) || short=$(( short + 1 ))
  done

  blocks=$(( ${#every[@]} + ${#repo_gates[@]} ))
  label="${#repo_gates[@]} family-independent gate(s) plus ${#every[@]} famil(ies)"
  echo "==="
  # The status comes from the count of short blocks, not from whatever ran last.
  # Otherwise a trailing green family would report the repository green.
  if [ "$short" -gt 0 ]; then
    echo "$short/$blocks block(s) short — $label"
    exit 1
  fi
  echo "$blocks/$blocks blocks reached — $label"
  exit 0
fi

if [ "${1:-}" = "--family" ]; then
  [ -n "${2:-}" ] || { echo "--family needs a family name, e.g. --family ring" >&2; exit 2; }
  export GATE_FAMILY="$2"
  shift 2
fi

source "$here/common.sh"

[ -d "$DECL" ] || {
  echo "unknown family '${GATE_FAMILY}' — declared: $( find "$DECL_ROOT" -mindepth 1 -maxdepth 1 -type d -printf '%f ' )" >&2
  exit 2
}

if [ "${1:-}" = "--stage" ]; then
  [ -n "${2:-}" ] || { echo "--stage needs a stage id, e.g. --stage S1" >&2; exit 2; }
  load_stage "$2" || exit 2
  shift 2
  echo "── scope: $GATE_STAGE — crates: ${GATE_CRATES//,/ } — features: ${GATE_FEATURES//,/ }"
fi

# `--control` points the corpus gates at a deliberately broken fixture and
# inverts their polarity. REACHED now means the gate found the defect seeded for
# it, and a gate that reports the fixture clean is the failure.
#
# The control run is the other half of the gates. G14-G17 each pass by finding
# nothing, and an empty result means "the property holds" and "the checker is
# broken" equally well. No output can carry that distinction. It is the reason
# *Documentation : Executable Recipes* requires a control beside every
# must-be-empty check in this repository. Only the corpus gates honour it. The
# older gates grade code and have their own non-vacuity pairings.
#
# No family declares the corpus gates any more (see declared/ring/gates.txt), so
# a bare `--control` loads only gates that ignore it and prints their ordinary
# verdicts under the banner below. Name the corpus gates to run the control:
# `run_all.sh --control g14 g15 g16 g17 g20 g21`.
if [ "${1:-}" = "--control" ]; then
  export GATE_CONTROL=1
  shift
  echo "── control: grading the seeded fixture — REACHED means the gate saw its defect"
fi

if [ $# -gt 0 ]; then
  want=( "$@" )
else
  mapfile -t want < <( family_gates )
fi

[ ${#want[@]} -gt 0 ] || { echo "family '$GATE_FAMILY' declares no gates" >&2; exit 2; }

echo "── family: $GATE_FAMILY — gates: ${want[*]}"

gates=()
for g in "${want[@]}"; do
  found=( "$here/${g}"_*.sh )
  [ -e "${found[0]}" ] || { echo "no gate script for '${g}'" >&2; exit 2; }
  gates+=( "${found[@]}" )
done

reached=0 total=0
for g in "${gates[@]}"; do
  total=$(( total + 1 ))
  bash "$g" && reached=$(( reached + 1 ))
done

echo "---"
echo "$reached/$total gates reached"
[ "$reached" -eq "$total" ]
