# Algorithm: Deciding Line Membership by Division

### Scope

- **Purpose**: State how [`on_distinct_lines`](../type/001_cache_line.md) decides its question, and why the arithmetic is division rather than the subtraction a reader expects.
- **Responsibility**: Give the inputs, the two steps, the alternative that looks equivalent and is not, and the counterexample that separates them.
- **In Scope**: The predicate's computation; why it takes integers rather than references.
- **Out of Scope**: Whether the addresses passed in are the ones a caller cares about — the caller supplies them; how the pair came to be separated, which is [`algorithm/002`](002_rounding_a_payload_up_to_whole_lines.md).

### Inputs

| Input | Type | Source |
|-------|------|--------|
| `a` | `usize` | An address the caller already holds, as a plain integer |
| `b` | `usize` | The other address |
| [`CACHE_LINE`](../type/001_cache_line.md) | `usize` | This crate's own constant, not a parameter |

**The addresses are integers, not references, and that is a deliberate
narrowing** (→ [`decisions/002`](../decisions/002_the_predicate_takes_integers.md)).
The question is about where two *fields* sit. A caller asking it has the
addresses already — [`ring_cursor`](../../../ring_cursor/readme.md) passes
`self.producer.addr()` and `self.consumer.addr()` — and a signature taking
`&T` would invite a doc example over two stack locals, which asserts something
false about the machine while looking reasonable.

### Steps

1. Compute `a / CACHE_LINE` — the index of the line containing `a`.
2. Compute `b / CACHE_LINE` — the same for `b`.
3. Return whether the two indices differ.

That is the whole computation, and it is a `const fn`: no allocation, no
branch the optimiser cannot fold, nothing that can fail
(→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md)).

**Integer division is exactly the line index** because lines are aligned to
their own size: line `n` spans `[ n * 64, n * 64 + 63 ]`, so any address in it
divides to `n`. This holds only because `CACHE_LINE` is a power of two and
lines are naturally aligned — both true on every platform this family targets,
and both silently assumed here rather than asserted.

### The Alternative That Looks Equivalent

The obvious form is subtraction:

```rust
// wrong
a.abs_diff( b ) >= CACHE_LINE
```

It reads as the same question — *are these far enough apart?* — and it agrees
with the division form on most inputs. It is wrong, because **distance is not
the question. Membership is.**

| `a` | `b` | Distance | Same line? | Subtraction says | Division says |
|----:|----:|---------:|-----------|------------------|---------------|
| 0 | 63 | 63 | yes (both line 0) | not separated ✓ | not separated ✓ |
| 63 | 64 | **1** | **no** (lines 0 and 1) | **not separated ✗** | separated ✓ |
| 0 | 64 | 64 | no (lines 0 and 1) | separated ✓ | separated ✓ |
| 100 | 130 | 30 | no (lines 1 and 2) | **not separated ✗** | separated ✓ |

**Two addresses 1 byte apart can be on different lines, and two 62 apart can
share one.** Rows 2 and 4 are where the forms diverge, and both are cases the
subtraction form gets wrong in the *conservative* direction — it reports "not
separated" when the addresses are in fact on different lines. For this crate's
purpose that error is harmless-looking and therefore worse than the alternative
error: a caller checking its own layout would see a false failure and pad
something that did not need it.

The test asserts exactly this distinction rather than only the easy cases:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align
cargo test --test align_test distinct_lines_follows_boundaries_not_distance 2>&1 \
  | command grep -E '^test .+ \.\.\.|^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
test distinct_lines_follows_boundaries_not_distance ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished
```

### Why Both Forms Appear in the Test Suite

`two_wrapped_fields_land_on_different_lines` asserts the subtraction form
(`a.abs_diff( b ) >= CACHE_LINE`) **and** the division form
(`on_distinct_lines( a, b )`), which looks redundant and is not. The
subtraction assertion is the stronger claim for that particular case: it says
the wrapper produced a full line of separation, not merely that the two fields
happened to straddle a boundary. Two fields 1 byte apart across a boundary
would satisfy `on_distinct_lines` and would mean the padding had failed.

**So the predicate is the right general question and the wrong specific one.**
Where the caller is checking *its own padding worked*, distance is what it
wants; where the caller is asking *do these two things contend*, membership is.
The crate exports the second and the test uses both.

### AL1 — The Division Rests on a Precondition This Crate Never Asserts

Integer division equals the line index only because lines are aligned to their
own size, which holds only because `CACHE_LINE` is a power of two. The crate
states this in prose above and asserts it nowhere: no `is_power_of_two`, no
`const` assertion, no test. One tier down, `ring_types` does exactly that for
its own analogous number:

```
  -- the precondition the division rests on, asserted nowhere in this crate --
0
  -- control: the family does assert it, one tier down --
46:    if !slots.is_power_of_two()
```

**Finding.** A future port that sets `CACHE_LINE` to any non-power-of-two —
96 is the plausible mistake, being a real line size on some hardware — leaves
`a / CACHE_LINE` computing something that is not a line index, and every test in
the suite still passes, because they all compare the layout against
`CACHE_LINE` rather than against a line. The one-line guard the family already
uses elsewhere would turn this into a compile-time refusal.

`src/lib.rs` now carries exactly that guard, immediately after the constant's
own declaration:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'is_power_of_two' ring_align/src/lib.rs
```

Live output:

```
    CACHE_LINE.is_power_of_two(),
```

**Disposition:** applied — added `const _ : () = assert!(
CACHE_LINE.is_power_of_two(), "..." );` immediately after `CACHE_LINE`'s
declaration in `src/lib.rs` — the compile-time form of the precondition
`ring_types::Capacity::new` already checks at runtime for its own number
(quoted above); a `pub const` needs no `Result`, so the guard can be
unconditional and load-bearing at every build rather than only when a caller
happens to construct one. The crate's 9 unit tests plus 7 doctests
re-verified passing (`cargo test --all-features`, 2026-09-04). Now prints:
`CACHE_LINE.is_power_of_two(),`

---

### AL2 — The Two Forms Disagree on Half the Interesting Cases

The divergence table above is not decorative: of its four rows, two are cases
where subtraction and division give opposite answers, and both are cases the
subtraction form gets *wrong*. `63` and `64` are one byte apart and on different
lines; `100` and `130` are thirty apart and on different lines. Subtraction
reports "not separated" for both.

**Finding.** The disagreement rate on the cases anyone bothers to write down is
50%, which is the reason the predicate exists as a named function rather than as
an inline comparison a caller writes for itself. It also means a reviewer's
intuition — *distance is separation* — is wrong exactly half the time on the
inputs that matter, which is worse than being wrong always.

---

### AL3 — The Suite Asserts Both Forms, Deliberately, and Says So

`two_wrapped_fields_land_on_different_lines` asserts `abs_diff >= CACHE_LINE`
**and** `on_distinct_lines`, which reads as redundancy and is not: for that
specific case the subtraction form is the *stronger* claim, because the test is
checking that the wrapper produced a whole line of separation rather than that
two fields happened to straddle a boundary.

**Finding.** The crate uses the form it documents as wrong, in its own test
suite, correctly — and the reason is written down in the test rather than left
to be rediscovered. That is the healthy version of this situation, and it is
worth recording precisely because the next reader to notice `abs_diff` in a file
whose docs call `abs_diff` wrong will otherwise file a bug.

---

### AL4 — The Form This Definition Calls Wrong Is in Live Code One Crate Over

```
  -- the subtraction form this definition calls wrong, in live code --
809:    claim.abs_diff( consume ) >= 64
```

`ring_mpsc/src/lib.rs:865` is not a test and not a doctest. It is the
subtraction form, over a literal `64` rather than the constant, in a crate that
does not declare `ring_align` at all.

**Finding.** Two independent defects stack in one expression: the wrong
arithmetic (this definition's subject) and a duplicated magic number
([`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md)). On
a 128-byte-line host it keeps compiling and keeps passing while asserting half
the separation it claims — the failure this definition and that invariant were
each written to prevent, occurring together, unwatched by either.

**Disposition:** declined — closing this cleanly means `ring_mpsc` newly
depending on `ring_align` (it declares no such dependency today, per the
`Cargo.toml` excerpt in [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md)),
which is an architectural decision for that crate's own change, not something
a documentation disposition pass in `ring_align` should decide on `ring_mpsc`'s
behalf. Swapping only the arithmetic form in place, without the dependency,
would not even close the substantive gap: this crate's own
[`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) § The
Duplicate Already Exists already measured that `on_distinct_lines`'s division
form is no better than the subtraction form for `ring_mpsc`'s specific two
cursors, because they are separate allocations rather than fields of one
`PaddedCursor` — "the problem is the different allocations, not the
arithmetic" — and that same section already concludes the real fix is a
different design (dropping the method for an assertion on `PaddedCursor`
itself) filed as `ring_mpsc`'s "own change, with its own test and its own bug
record," not a drop-in arithmetic substitution here.

### Algorithms

| File | Relationship |
|------|--------------|
| [002_rounding_a_payload_up_to_whole_lines.md](002_rounding_a_payload_up_to_whole_lines.md) | Produces the separation this predicate observes — the two halves of the crate's single claim |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_predicate_takes_integers.md](../decisions/002_the_predicate_takes_integers.md) | Why the signature is `( usize, usize )` and what that costs the caller |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The property this predicate is the observable form of |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_on_distinct_lines.md](../item/002_on_distinct_lines.md) | The declaration, and the four crates that reach this name — only one of which calls this function |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | Why a predicate over real addresses exists at all, rather than trusting `size_of` |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The invariant this crate delivers the padding half of — "the fix is layout, not algorithm", and this is the one piece of arithmetic the fix needs |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `distinct_lines_follows_boundaries_not_distance` asserts all four rows of the divergence table above, including both cases the subtraction form gets wrong |
