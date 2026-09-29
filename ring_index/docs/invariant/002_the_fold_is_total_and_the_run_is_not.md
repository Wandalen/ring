# Invariant: The Fold Is Total and the Run Is Not

### Scope

**Purpose:** State totality precisely — which of the three functions is total
over its whole input domain and which is not — and record that the crate asserts
the property once, about one function, in a place a reader will generalize from.

**Responsibility:** The domain each function is defined over, and where the
boundary between them is drawn.

**In Scope:** `ring_index/src/lib.rs:24-26, 48-52, 73-77, 118-122`.

**Out of Scope:** what happens past the boundary is
[`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md).
The mask identity itself is [`invariant/001`](001_the_mask_equals_the_modulo.md).

---

## Three Functions, Two Domains

| Function | Total over | Not defined for |
|----------|-----------|-----------------|
| `of` | every `( Seq, Capacity )` — all `2^64` sequences, every constructible capacity | nothing |
| `aliases` | every `( Seq, Seq, Capacity )` — it is two `of` calls and a comparison | nothing |
| `run` | `( start, count, capacity )` where `start.0 + ( count - 1 ) ≤ u64::MAX` (`count > 0`; `count = 0` adds nothing and is always in-domain) | anything above that: panics in debug, wraps in release |

`of` and `aliases` are total because masking discards bits and never carries.
`run` is not, because it adds `count` to `start` before it folds anything.

---

### IX31 — Totality Is Claimed Once, for One Function, in the Module Comment

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! The validation lives upstream in [`ring_types::Capacity`], not here. Because' ring_index/src/lib.rs
echo '  -- and every Panics section in the crate --'
echo "  $( command grep -c '# Panics' ring_index/src/lib.rs ) occurrences"
```

Live output:

```
//! The validation lives upstream in [`ring_types::Capacity`], not here. Because
//! a `Capacity` cannot exist unless it is a power of two, [`of`] needs no check
//! and no error path: it is total.
  -- and every Panics section in the crate --
  1 occurrences
```

**Finding.** The claim is scrupulously scoped: "`[of]` needs no check and no
error path: it is total." It names the function, gives the reason, and stops.
Read strictly, it is correct and complete.

Read the way module comments are actually read — as the crate's summary of
itself — it says this crate is total. The comment sits at the top of the file,
it is the only sentence in the crate on the subject, and the two functions it
does not cover are one and four lines further down with no counterpart note.
`aliases` inherits totality legitimately. `run` does not inherit it and is not
told so.

Zero `# Panics` sections across the crate. `run` can panic. The two facts were
each individually defensible — `run`'s panic comes from `ring_types`'
`advanced_by`, not from anything written here — and their combination left a
reader with no signal anywhere that any function in this crate has a domain.

**Disposition:** applied — [`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md)
added a `# Panics` section to `run`'s own doc comment, closing the same gap
this finding names from the other side: the module comment's totality claim
is still scoped correctly, and now `run` carries its own local signal instead
of relying on a reader to notice the module comment doesn't mention it. The
crate has exactly one `# Panics` section, and it is on the one function that
needs it.

---

### IX32 — The Boundary Is Not Where a Reader Would Guess

The natural guess is that `run` fails when the run is longer than the ring —
asking for 4096 slots from a 1024-slot capacity. It does not:

```
--- (3) what one `run` costs, by count ---
  count 0      0 alloc       0 bytes  len 0  capacity 0
  count 1      1 alloc       8 bytes  len 1  capacity 1
  count 8      1 alloc      64 bytes  len 8  capacity 8
  count 1024   1 alloc    8192 bytes  len 1024  capacity 1024
  count 4096   1 alloc   32768 bytes  len 4096  capacity 4096
```

**Finding.** `run( start, 4096, cap 1024 )` returns 4096 slot indices, cheerfully,
with slots repeating four times over. Over-long runs are in the domain — the
crate's own test `an_oversized_run_repeats_slots` asserts exactly this and calls
it correct.

The actual boundary is `start.0 + ( count - 1 )` against `u64::MAX` — the last
step `run`'s `0..count` loop actually takes, one short of the coarser `start.0
+ count` a quick reading of the range suggests. It depends on `start` rather
than on `capacity` and is therefore invisible from any of the arguments a
caller is thinking about. A run of length 2 is fine at `start = 0` and panics
at `start = u64::MAX`; a run of length 2^40 is fine almost everywhere.

So the crate has two properties that sound similar and are not:

- **Not a constraint:** `count` versus `capacity`. Any relationship is legal,
  and repeats are the documented answer.
- **A constraint:** `start + ( count - 1 )` versus `u64::MAX`. Unstated,
  unasserted, and build-profile-dependent past the edge.

`run`'s doc comment discusses the first and not the second, in a sentence about
batch claims wrapping at most once — which is
[`algorithm/002`](../algorithm/002_a_run_is_the_fold_applied_count_times.md)
IX3's finding, and is the closest the crate comes to describing a domain for
`run` while describing the wrong one.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_the_mask_equals_the_modulo.md) | The identity `of`'s totality rests on |
| [`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) | What happens past `run`'s boundary, measured in both profiles |
| [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) | Why `of` is total: validation bought upstream |
| [`algorithm/002`](../algorithm/002_a_run_is_the_fold_applied_count_times.md) | `run`'s doc comment, describing the constraint that is not one |

### Sources

| Fact | Where |
|------|-------|
| The totality claim | `ring_index/src/lib.rs:24-26` |
| One `# Panics` section, on `run` | Census above |
| Over-long runs returning repeats | Release probe, section 3, quoted above |
| The over-long run ruled correct | `ring_index/tests/index_test.rs` — `an_oversized_run_repeats_slots` |

### Tests

| Test | Covers |
|------|--------|
| `an_oversized_run_repeats_slots` | `count > capacity`, asserted as in-domain |
| `an_empty_run_is_empty` | `count = 0`, the other end of the same axis |
| `capacity_one_maps_everything_to_slot_zero` | The degenerate capacity, still total |
| `run_overflows_at_the_top_of_the_sequence_space` | `run` at `start = u64::MAX`, the boundary that actually exists |
| `run_does_not_overflow_one_below_the_top_of_the_sequence_space` | The boundary is `start + ( count - 1 )`, not `start + count` — one below still succeeds |
