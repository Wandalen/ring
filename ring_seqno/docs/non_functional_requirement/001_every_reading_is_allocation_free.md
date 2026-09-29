# Non-Functional Requirement: Every Reading Is Allocation-Free

### Scope

- **Purpose**: State the allocation requirement, establish that this crate meets it, and record what happened to the one allocation the family used to pay because of a signature chosen here.
- **Responsibility**: Show the five bodies allocate nothing, show that the whole gating read path is now allocation-free with them, and record the cost the removal charged to this crate's own surface.
- **In Scope**: Allocation behaviour of `ring_seqno` and of the call sites its signatures shape.
- **Out of Scope**: What `GatingSet`'s owned buffer costs at construction — that is [`ring_gating`'s own NFR](../../../ring_gating/docs/non_functional_requirement/001_every_gating_read_allocates_nothing.md), not a read-path cost.

### The Requirement

> No function in `ring_seqno` may allocate. These readings run on the write path of
> a lock-free ring, including inside compare-exchange retry loops, where an
> allocator call is both a latency spike and a source of contention that feeds
> back into the retry rate.

### The Crate Meets It

```sh
cd "$(git rev-parse --show-toplevel)"
# not one allocating name in the whole crate, doc lines included. The empty
# result is the evidence, so it is caught rather than left to end the block
echo '  -- not one allocating name in this crate, doc lines included --'
command grep -E 'Vec|String|Box|to_vec|collect|format!|vec!' ring_seqno/src/lib.rs | command grep -vE '^[[:space:]]*///' \
  || echo '    (no matches — ring_seqno names nothing allocating)'
# nor anywhere else on the path a gating read travels. This used to read 1 for
# ring_cursor and is the subject of § The Allocation That Was One Tier Up
echo '  -- nor anywhere else on the gating read path --'
for c in ring_cursor ring_barrier ring_claim ring_consume; do
  printf '    %-14s %s\n' "$c" "$( command grep -E 'Vec|String|Box|to_vec|collect|format!|vec!' $c/src/lib.rs | command grep -cvE '^[[:space:]]*(///|//!|//)' || true )"
done
# control — the one crate on this path that does still own a heap buffer, so a
# zero above is a measurement and not a broken pattern
echo '  -- control: the one crate on this path that does own a heap buffer --'
command grep -E '^  cursors : Vec|Vec::with_capacity' ring_gating/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- not one allocating name in this crate, doc lines included --
    (no matches — ring_seqno names nothing allocating)
  -- nor anywhere else on the gating read path --
    ring_cursor    0
    ring_barrier   0
    ring_claim     0
    ring_consume   0
  -- control: the one crate on this path that does own a heap buffer --
      cursors : Vec< PaddedCursor >,
        let mut cursors = Vec::with_capacity( consumers );
```

Nothing in this crate, and nothing in the four crates its readings travel
through. The only heap buffer left anywhere on the path is `GatingSet`'s own
field, allocated once when a set is constructed and never on a read. Every body
here is arithmetic on `Copy` scalars or a fold over a borrowed slice:

| Function | Body | Allocates |
|----------|------|:---------:|
| `laps_between` | one subtraction, one division | ❌ |
| `may_claim` | one subtraction, one comparison | ❌ |
| `free_slots` | one subtraction, one saturating subtraction | ❌ |
| `pending` | one subtraction | ❌ |
| `slowest` | `.iter().copied().min()` over a borrowed slice | ❌ |

`slowest` is the only one where it is worth checking rather than assuming.
`Iterator::min` over `Copy` items keeps a single running candidate; nothing is
buffered, and the `&[ Seq ]` is borrowed, never owned.

### The Crate Uses Nothing Outside `core`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_seqno
# nothing outside core is named anywhere in the crate …
grep -E 'std::|Vec|String|Box|HashMap' src/lib.rs | grep -vE ':\s*///' \
  || echo '(no matches — nothing outside core)'
# … and the control: the crate-level attributes are readable, and the only one
# present is the docs lint — there is no no_std declaration to have missed
grep '^#!\[' src/lib.rs
```

Live output:

```
(no matches — nothing outside core)
#![ deny( missing_docs ) ]
```

The first command prints nothing; the second shows the file's only crate-level
attribute. `Option`, `Iterator`, slices and integer arithmetic are all `core`. So `ring_seqno`
is `#![ no_std ]`-compatible as written, and is not declared as such.

Three crates in the family are, and this is not one of them:

```sh
cd "$(git rev-parse --show-toplevel)"
# three of the 33 crates declare it and `ring_seqno` is not among them, so
# allocation-freedom *here* stays a property of the code rather than something
# the toolchain enforces
grep -r 'no_std' ring_*/src/*.rs \
  || echo '(no matches — expected three; the declarations have gone)'
# control — the identical expression over a file that does declare it
printf '#![ no_std ]\n' > /tmp/-nostd_control.rs
grep -r 'no_std' /tmp/-nostd_control.rs
rm -f /tmp/-nostd_control.rs
```

Live output:

```
ring_overflow/src/lib.rs:#![ no_std ]
ring_overflow/src/lib.rs:// `no_std` here is an assertion, not a convenience. This crate, `ring_stats`,
ring_overflow/src/lib.rs:// property is only worth anything transitively: a `no_std` crate depending on a
ring_stats/src/lib.rs:#![ no_std ]
ring_types/src/lib.rs:#![ no_std ]
#![ no_std ]
```

Some crates in the family genuinely need `std` — `ring_gating` owns a `Vec`,
`ring_wait` parks threads — so the family will never declare it uniformly. But
the partial split that argument was raised against already exists:
`ring_overflow`, `ring_stats` and `ring_types` declare it, and `ring_overflow`'s
own comment at `src/lib.rs` says why — the property is worth having
transitively, so a `no_std` crate can depend on them. `ring_seqno` is compatible
as written and declares nothing, which leaves it outside a split it could join
for one line.

**Correction (2026-09-20):** this paragraph read "That is consistent rather than
an oversight … a partial `no_std` split would buy little", and closed on "the
answer to 'why isn't it' is 'because being alone in that would not help anyone'".
The recipe directly above it has printed three declaring crates ever since it was
retargeted at `ring/`, so `ring_seqno` would not be alone and the split it was
argued against is already in place. The narrower claim survives — the crate is
`no_std`-compatible and undeclared — but the reason for the omission is now
unrecorded rather than explained.

### The Allocation That Was One Tier Up, and What Removed It

`slowest` takes `&[ Seq ]`. Its only cross-crate caller held `&[ PaddedCursor ]`
— atomics, not values — and for as long as it delegated there was no way to
bridge those without materialising the loads. It did so with a `Vec`:

```rust
// ring_cursor/src/lib.rs, as it stood before commit b7e075ca
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

A heap allocation per call, in the condition of `ring_claim::claim`'s
compare-exchange retry loop, so under contention the loop spun and each spin
allocated. That is gone. Both bodies now:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the fold as it now stands, one tier up --'
awk '/^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_cursor/src/lib.rs
echo '  -- and the one in this crate, which it no longer calls --'
awk '/^pub fn slowest\( cursors : &\[ Seq \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_seqno/src/lib.rs
# the delegation was the only cross-crate use, so this now prints nothing and
# exits nonzero — the empty result is the finding, not a broken pattern
echo '  -- every cross-crate caller of ring_seqno::slowest, code lines only --'
command grep -r 'ring_seqno::slowest' --include=*.rs */ \
  | command grep -v '^ring_seqno/' | command grep -vE ':[[:space:]]*(///|//!|//)' \
  || echo '    (none — the delegation was the only one)'
```

Live output:

```
  -- the fold as it now stands, one tier up --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
  -- and the one in this crate, which it no longer calls --
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
  -- every cross-crate caller of ring_seqno::slowest, code lines only --
    (none — the delegation was the only one)
```

**The fix taken was not the one this section used to propose.** The proposal
here was a signature change — `slowest< I : IntoIterator< Item = Seq > >` — after
which `ring_cursor` would have passed its map directly and nothing would be
materialised, with `ring_seqno` keeping the single definition of the fold. What was
done instead was to stop delegating: `ring_cursor::slowest` now folds the loads
itself, and `ring_seqno::slowest` is untouched.

Both remove the allocation. They differ in what they leave behind. The signature
change would have kept one fold with two entry points; the duplication leaves two
folds, four lines apart across a crate boundary, with nothing keeping them equal
— `.iter().copied().min()` here and `.iter().map( | c | c.load( GATING ) ).min()`
there, the same reduction over two representations of the same thing. And it left
`ring_seqno::slowest` without a caller.

### SQ35 — The Allocation Is Gone, and the Function It Was Bridging To Went Quiet

The parameter type was the whole cause, and the cost landed one crate away:

```
ring_cursor::slowest( cursors : &[ PaddedCursor ] )
  -> loads each cursor and folds them, in place
  -> calls nothing

and the caller of that is a gating read inside a claim retry, which now
allocates nothing per spin.
```

**Finding.** `slowest`'s `&[ Seq ]` parameter forced `ring_cursor` to
heap-allocate in the condition of a lock-free retry loop. Removing the
allocation by duplicating the fold rather than by widening the parameter closes
the cost and opens a smaller thing in its place: this crate's `slowest` is now
the second of its five functions with no caller outside its own tests, next to
`laps_between` (SQ27, SQ36).

**Disposition:** applied — the allocation is gone from the whole gating read
path, removed in `ring_cursor/src/lib.rs` by folding the loads in place
rather than by the `IntoIterator` signature this section proposed; the retry-loop
cost this finding was about no longer exists, and the census above records both
that and the caller `ring_seqno::slowest` lost to the change.
Now prints: `(none — the delegation was the only one)`

---

### SQ36 — The Expensive One Is the Unused One

A shift would do, if the divisor were known at compile time. It is not, and it does not matter:

```
capacity is always a power of two (Capacity::new enforces it)
but capacity.get() is a runtime value, so `/` is a real division
and laps_between has zero callers (SQ27)
```

**Finding.** `laps_between` divides by a runtime value the compiler cannot prove is a power of two, so the family's only lap count is also its only integer division — and nothing calls it.

### What Is Not Known

One thing, unmeasured:

| # | Question | How to settle it |
|---|----------|------------------|
| U1 | Did removing the allocation change measurable throughput under contention? The cost was never measured while it existed, so its removal has no before-figure to be compared against | `ring_bench`'s harness, producer count swept 1→N, against the pre-`b7e075ca` body restored locally |

The two questions this section used to carry — whether the `Vec` survived
optimisation in the one-consumer case, and whether it cost measurable throughput
— are both moot: there is no `Vec` to optimise away and no cost to attribute.
They are replaced by the one question the removal itself raises, which is
unanswered for the same reason the originals were: nothing in the family
measures the gating read.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold, and the chain that carries it |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `slowest`'s signature and coverage |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_arithmetic_must_survive_a_narrow_usize.md](002_the_arithmetic_must_survive_a_narrow_usize.md) | The requirement this crate does not meet |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_beside_its_quantity.md](../pattern/001_the_predicate_beside_its_quantity.md) | Why `may_claim` exists rather than `free_slots() != 0` at each call site |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno::slowest` in `ring_seqno/src/lib.rs` | The signature that used to force the collection |
| `ring_cursor::slowest` in `ring_cursor/src/lib.rs` | Where the `Vec` was, and the fold that replaced it |
| `ring_claim::Claimer::claim` in `ring_claim/src/lib.rs` | The retry loop it sat in the condition of |
| `ring_bench` (harness crate) | The harness U1 would use |

### Tests

| File | Relationship |
|------|--------------|
| `slowest_is_the_minimum_wherever_it_sits`, `slowest_of_nothing_is_none_not_zero` in `tests/seq_test.rs` | `slowest`'s behaviour — now the only exercise this function gets |
| — | No test asserts allocation-freedom. Nothing would catch a regression |
