#!/usr/bin/env bash
# G8 — every reached-test asserts against a closed form, an identity, a
# conservation law or a diff, never against a recorded literal answer, AND the
# count of assertions scanned is greater than zero.
#
# Why a gate rather than a review habit. Correctness here is cheap to check
# because mathematics supplies the oracle. The failure mode is a change six
# weeks in whose author reaches for a recorded expected value because deriving
# the oracle is momentarily harder than capturing the output. That test still
# passes, still goes green, and tells its reader only that something changed —
# never whether the new answer is right. This is the Opaque Pass anti-pattern;
# this gate makes it a failure instead of a judgement call.
#
# The discriminator, stated precisely enough to be decidable:
#
#   A FLOATING-POINT literal standing ALONE as a whole operand on the expected
#   side of an equality or near-equality assertion is the violation.
#
# What that deliberately does NOT catch, and why each is exempt:
#
#   * Tolerances — `< 1e-9` on the right of a comparison. Not the answer, the
#     allowance around it.
#   * Sweep inputs — `let e = 0.999;`. An input, not an expectation, and never
#     inside the assertion.
#   * Coefficients inside a derived expression — `2.0 * PI * r`. The literal is
#     part of the oracle rather than a substitute for one, which is exactly the
#     distinction; `( x - 6.28318 ).abs()` IS caught, `( x - 2.0 * PI ).abs()`
#     is not.
#   * Integer literals — `len() == 4`, `count == 22`. Structural counts, not
#     physical quantities.
#   * Zero — `assert_eq!( residual, 0.0 )` and `( a - b ).abs() < eps` against
#     zero. Zero is what a conservation law and an identity both assert; it is
#     never a captured measurement.
#   * One — `( got / want - 1.0 ).abs() < eps`, `rel_err( x, 1.0 )`. Added
#     alongside zero early in this gate's own history, and for the same reason
#     the sentence above gives: one is the multiplicative identity exactly as
#     zero is the additive one. `sin² + cos²`, `cosh² − sinh²` and every
#     relative error in the family are residuals against one, and none of them
#     is a captured measurement. Restricted to a literal one with no exponent,
#     so `1e-9` stays catchable as an expectation.

# Yardstick note. Four changes were made to this file after its baseline
# reading, each a deliberate, recorded decision rather than a silent one:
#
#   1. The trailing-operand pattern below is anchored to the end of a statement
#      and to lines carrying an assert macro. Unanchored it matched `, LIT )` in
#      any two-argument call — `powf( base, 0.4 )`, `Mu( 3.986e14 )` — which is
#      not an assertion at all, let alone one against a recorded literal. That
#      is a false positive against the gate's OWN stated discriminator, and a
#      gate that fires on correct code teaches people to route around it.
#   2. The exemption of one, above.
#   3. A third operand shape: a literal between two commas, immediately
#      followed by a quote. `assert_eq!( actual, 1000.0, "message" )` matched
#      neither existing shape — the first argument is `actual`, not a literal,
#      and the statement ends at the message's own `)`, not the literal's. A
#      custom failure message is the ordinary, encouraged way to write an
#      assertion, which made this the gate's largest blind spot in practice:
#      `cluster_macro/tests/macro_test.rs:177,226`,
#      `cluster_physics/tests/physics_test.rs:341` and
#      `visibility_cull/tests/cull_test.rs:424-425` each carried a
#      recorded-literal assertion in exactly this shape, invisible to the gate
#      before this change. Bounded the same way Change 1 bounded the trailing
#      shape: the quote is required, not optional, so an ordinary two-argument
#      `helper( a, 0.4, b )` still does not match. The gap this leaves —
#      `helper( a, 0.4, "x" )` as a nested call, its own string argument
#      mistaken for assert's message — is accepted rather than chased: closing
#      it needs paren-depth tracking this file does not otherwise carry, for a
#      shape no crate here actually writes.
#   4. The one-exemption's boundary character class excluded digits and dots
#      but not a minus sign, so `-1.0` read as "a boundary, then the digit 1" —
#      the sign was discarded instead of treated as part of the number. Two
#      separate test files, one of them independent of Change 3, both named
#      negative one — a specific recorded answer with nothing multiplicatively
#      identical about it — and both were passing this exemption unexempted in
#      name only. Fixed by excluding `-` from the one-branch's own boundary,
#      split from the zero-branch so `-0.0` stays exempted: negating zero does
#      not change which value is being asserted, negating one does.
#
# Change 1 narrows nothing the discriminator ever claimed. Changes 2 and 4
# widen or reshape the exemption set, and are the ones to be suspicious of; the
# test that keeps all four honest is that `( x - 6.28318 ).abs()` and
# `assert_eq!( y, 3.5 )` are both still caught, which `tests/manual/readme.md`
# in bench_harness records as a run.
set -uo pipefail
GATE=G8
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

files=()
while read -r c; do
  d="$( crate_dir "$c" )" || continue
  [ -d "$d/tests" ] || continue
  while read -r f; do files+=( "$f" ); done < <( find "$d/tests" -name '*.rs' )
done < <( family_crates )

[ ${#files[@]} -gt 0 ] || fail "no test files in the crates under test — nothing to scan"

# A float literal: a digit run with a decimal point, an exponent, or both.
# `0.0` and `-0.0` are subtracted from the class below rather than excluded
# here, so the pattern stays readable.
FLOAT='-?[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][-+]?[0-9]+)?'

scanned=0
violations=()
for f in "${files[@]}"; do
  # Normalise: join the file into one line-per-statement stream so a wrapped
  # assert! is scanned as the single expression it is.
  norm="$( tr '\n' ' ' <"$f" | sed -E 's/[[:space:]]+/ /g; s/; /;\n/g' )"

  n="$( grep -cE 'assert(_eq|_ne)?!' <<<"$norm" || true )"
  scanned=$(( scanned + n ))

  # Only lines carrying an assertion are scanned. Both patterns below describe
  # the shape of an EXPECTATION, and an ordinary call with a float argument has
  # the same shape without being one.
  asserts="$( grep -E 'assert(_eq|_ne)?!' <<<"$norm" || true )"

  # (a) A bare float literal as a whole operand of assert_eq!/assert_ne!.
  # Leading operand anchored to the macro; trailing operand anchored to the end
  # of the statement, so `assert_eq!( f( a, 0.4 ), b )` is not a match. A third
  # shape (Change 3) covers a literal in any other position, bounded to where a
  # quote immediately follows the next comma — the shape a custom failure
  # message produces — so the same nested-call text still does not match.
  while read -r hit; do
    [ -n "$hit" ] || continue
    violations+=( "${f#"$REPO"/}: $hit" )
  done < <( grep -oE "assert_(eq|ne)! *\( *($FLOAT) *,|, *($FLOAT) *\) *;?$|, *($FLOAT) *, *\"" <<<"$asserts" \
            | grep -E '\.|[eE][-+]?[0-9]' \
            | grep -vE '[^0-9.]-?0(\.0+)? *[,)]|[^0-9.-]1(\.0+)? *[,)]' || true )

  # (b) A bare float literal subtracted inside a near-equality: (x - LIT).abs()
  while read -r hit; do
    [ -n "$hit" ] || continue
    violations+=( "${f#"$REPO"/}: $hit" )
  # `-e` on both, because these patterns START with a minus and grep otherwise
  # reads them as options. Without it the pipeline emitted `invalid option`
  # to stderr and nothing to stdout, so pattern (b) silently matched nothing —
  # it had never actually run before an early probe planted
  # `( x - 6.28318 ).abs()` and got a pass.
  done < <( grep -oE -e "- *($FLOAT) *\) *\. *abs *\(" <<<"$asserts" \
            | grep -E '\.|[eE][-+]?[0-9]' \
            | grep -vE -e '- *[01](\.0+)? *\)' || true )
done

[ "$scanned" -gt 0 ] || fail "${#files[@]} test file(s) scanned but no assertions found — a test corpus that asserts nothing"

if [ ${#violations[@]} -gt 0 ]; then
  fail "${#violations[@]} assertion(s) against a recorded literal rather than a derived oracle: ${violations[*]}"
fi

pass "${scanned} assertion(s) across ${#files[@]} test file(s), none against a recorded literal"
