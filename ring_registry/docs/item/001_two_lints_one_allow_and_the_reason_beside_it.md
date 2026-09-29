# Item: Two Lints, One Allow, and the Reason Beside It

### Scope

**Purpose:** Record the crate's declaration-level lint surface — one crate-wide
`deny`, one scoped `allow` carrying a written reason — and place both against the
seven `allow` attributes the family holds in total.

**Responsibility:** `#![ deny( missing_docs ) ]`; the `result_large_err` allow and
its `reason` string; the family-wide `allow` census and which of the seven explain
themselves; and what `#[ expect ]` would have caught that `#[ allow ]` cannot.

**In Scope:** `ring_registry/src/lib.rs:41`, `:123-156`; every
`#[ allow` in `ring_*/src/` and `ring_*/tests/`.

**Out of Scope:** The `#[ must_use ]` census is
[`item/002`](002_eight_declarations_and_four_must_use.md). Why the error is large
in the first place is
[`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md).
The decision the allow records is
[`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md).

---

## Six Suppressions, Two Explanations

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every lint attribute in the crate --'
command grep '^#!\[\|^  #\[ allow\|^#\[ allow' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- and every allow in the thirty-three crates --'
command grep -r '#\[ allow' --include=*.rs ring_*/src/ ring_*/tests/ 2>/dev/null | sed 's|ring/||' | sed 's/^/    /'
echo '  -- how much of this crate is prose --'
for c in ring_registry ring_handle ring_trace ring_core; do
  d=$( command grep -c '^ *///\|^//!' ring/$c/src/lib.rs || true )
  t=$( wc -l < ring/$c/src/lib.rs )
  printf '    %-14s %3s doc lines of %3s   %s%%\n' "$c" "$d" "$t" "$(( d * 100 / t ))"
done
```

Live output:

```
  -- every lint attribute in the crate --
    #![ deny( missing_docs ) ]
      #[ allow( clippy::result_large_err, reason = "the large payload is the caller's ring, handed back rather than destroyed" ) ]
  -- and every allow in the thirty-three crates --
    ring_mpsc/src/lib.rs:  #[ allow( clippy::mut_from_ref ) ]
    ring_registry/src/lib.rs:  #[ allow( clippy::result_large_err, reason = "the large payload is the caller's ring, handed back rather than destroyed" ) ]
    ring_spsc/src/lib.rs:  #[ allow( clippy::mut_from_ref ) ]
    ring_testkit/src/lib.rs:  #[ allow( clippy::too_many_lines, reason = "seven arms cover ten variants, three collapsed into their Many form; the length comes from the three Many loops and the Flush body, not a one-arm-per-variant shape" ) ]
    ring_core/tests/core_test.rs:#[ allow( clippy::len_zero ) ]
    ring_mpsc/tests/mpsc_test.rs:  #[ allow( clippy::clone_on_copy ) ]
    ring_slot/tests/slot_test.rs:  struct NotDefault( #[ allow( dead_code ) ] u32 );
    ring_spsc/tests/spsc_test.rs:  #[ allow( unsafe_code ) ]
  -- how much of this crate is prose --
    ring_registry  124 doc lines of 236   52%
    ring_handle    138 doc lines of 276   50%
    ring_trace     222 doc lines of 376   59%
    ring_core      276 doc lines of 653   42%
```

## What the Other Form Would Have Reported

`#[ expect ]` is `#[ allow ]` that also warns when the lint it names stops
firing. Both attributes, on two functions neither of which triggers the lint:

```rust
// compile/-expect_vs_allow.rs
// `allow` survives the lint no longer firing; `expect` reports it.
#[ allow( unused_variables ) ]
pub fn allow_form() { let used = 1; let _ = used; }

#[ expect( unused_variables ) ]
pub fn expect_form() { let used = 1; let _ = used; }
```

```
warning: this lint expectation is unfulfilled
 --> compile/-expect_vs_allow.rs:5:12
  |
5 | #[ expect( unused_variables ) ]
  |            ^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unfulfilled_lint_expectations)]` on by default

warning: 1 warning emitted
```

---

### RG25 — The Family Explains Its Suppressions in Inverse Proportion to What They Cost

Thirty-three crates hold seven `allow` attributes between them, and exactly two
carry a `reason` string. This crate wrote one of them, twelve words long, on
`clippy::result_large_err`; `ring_testkit` wrote the other, on
`clippy::too_many_lines`. Both are ergonomics lints — a wide `Result` and a long
function, neither capable of producing a wrong answer.

The five unexplained ones run the other way. `clippy::mut_from_ref` is suppressed
in `ring_mpsc` and again in `ring_spsc`, both times bare: it is the lint that
fires when a function hands out a `&mut` derived from a `&`, which is the closest
thing clippy has to a soundness warning, and in a lock-free ring crate it is
firing on exactly the code where it would matter. The three test-file suppressions
are genuinely trivial.

So the family's explanations are spent where the suppression is harmless and
withheld where it is not — not by policy, since there is no policy, but as the
accumulated result of each author deciding alone.

**Finding.** Recorded as a family-level inconsistency this crate is on the right
side of. The cheap repair is to require a `reason` on every `allow`, which the
compiler already supports and which would have forced the two `mut_from_ref`
sites to say what makes them sound. The narrower one, if a rule is too much, is
to write the two missing reasons: whatever argument justified those suppressions
was made once, by someone, and is currently recorded nowhere.

---

### RG26 — The Reason Is Written Twice and Checked Zero Times

`register` carries its justification in two places. Twenty lines of doc comment
at `:123-142` set out the lint, both remedies clippy suggests, why each is
refused, and what the choice costs. Eight lines later the attribute repeats the
conclusion in twelve words: "the large payload is the caller's ring, handed back
rather than destroyed."

Neither is checked, and the attribute form is the one that could be. `#[ allow ]`
is silent forever: if the payload were shrunk — if `Split< T >` stopped carrying
a whole ring, or the error dropped it — `result_large_err` would stop firing and
the suppression, the twelve-word reason and the twenty-line argument would all
remain in place, describing a lint that no longer exists. `#[ expect ]` is the
same attribute with the opposite default, and the probe shows it: a lint
expectation that goes unfulfilled produces `warning: this lint expectation is
unfulfilled`, on by default, naming the line.

The family uses `#[ expect ]` zero times across every source and test file, so
this is not a local omission; it is an available mechanism nobody has picked up.

**Finding.** Recorded as an unadopted guard rather than a defect — the current
attribute is correct today and the duplication between it and the doc section is
mild, since the short form is a summary of the long one and they agree. What
`#[ expect ]` adds is a link between the argument and the condition it was made
under: swap the one word and the compiler starts telling whoever shrinks the
payload that twenty lines of reasoning above them have expired. That is worth
more here than in most places, because the argument is unusually long and
unusually specific, and therefore unusually expensive to leave standing after it
stops being true.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_eight_declarations_and_four_must_use.md) | The other attribute census, and the one method that needs it |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | The 448 bytes the allow is about |
| [`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md) | Closed 2, where the same argument is recorded a third time |
| [`pattern/001`](../pattern/001_an_error_that_hands_the_payload_back.md) | The shape the lint objects to |
| [`api/001`](../api/001_the_registry_surface.md) | Where the allow is counted as a cost of the surface |

### Sources

| Fact | Where |
|------|-------|
| The crate-wide deny | `ring_registry/src/lib.rs:41` |
| The twenty-line argument | `ring_registry/src/lib.rs:123-142` |
| The allow and its reason | `ring_registry/src/lib.rs:156` |
| Six allows in the family, two with reasons | Census above |
| `#[ expect ]` reporting an unfulfilled expectation | Compile probe above |
| Zero `#[ expect ]` uses across the family | [`item/002`](002_eight_declarations_and_four_must_use.md) census |

### Tests

| Test | Covers |
|------|--------|
| `a_refused_registration_hands_the_ring_back` | The payload the allow exists to preserve |
| `the_error_names_the_taken_name` | The documented `Err` shape the lint fires on |
| `a_refused_registration_does_not_drop_the_ring_already_there` | The data loss both remedies would have caused |
