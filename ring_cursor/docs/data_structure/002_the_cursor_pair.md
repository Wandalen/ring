# Data Structure: The Cursor Pair

### Scope

- **Purpose**: Give `CursorPair` as 192 bytes, show that the separation guarantee survives the compiler's freedom to order the fields, and record that its test asserts the line predicate in both of its non-equivalent forms.
- **Responsibility**: State the layout, derive the size, and explain why `on_distinct_lines` is checked on real addresses rather than inferred from the type.
- **In Scope**: `CursorPair`'s three fields, its size, and `on_distinct_lines` at `src/lib.rs:438-441`.
- **Out of Scope**: Why `capacity` is a field, which is [`decisions/002`](../decisions/002_the_capacity_is_held_by_the_pair.md); the readings over it, which are [`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md).

### The Layout

```rust
pub struct CursorPair
{
  producer : PaddedCursor,   // 64 bytes, align 64
  consumer : PaddedCursor,   // 64 bytes, align 64
  capacity : Capacity,       //  8 bytes, align 8
}
```

```
CursorPair — 192 bytes, align 64

  ┌─── line 0 ────────┬─── line 1 ────────┬─── line 2 ────────┐
  │ a PaddedCursor    │ a PaddedCursor    │ Capacity + pad    │
  └───────────────────┴───────────────────┴───────────────────┘
  0                   64                  128                192
```

"A `PaddedCursor`" rather than "the producer" is deliberate — see below.

### Deriving the 192

| Step | Value |
|------|------:|
| Two `PaddedCursor` fields | 128 bytes |
| `Capacity` — `pub struct Capacity( usize )` | 8 bytes, non-zero-sized |
| Minimum occupied | 136 bytes |
| `align_of< CursorPair >` — inherited from `PaddedCursor` | 64 |
| Size rounded up to the alignment | **192** |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub struct Capacity' ring_types/src/capacity.rs
out=$( cargo test -p ring_cursor --test cursor_test a_pair_is_two_lines_plus_its_capacity 2>&1 )
printf '%s\n' "$out" | command grep -E '^test .+\.\.\. ' | LC_ALL=C sort
printf '%s\n' "$out" | command grep -E '^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
pub struct Capacity(usize);
test a_pair_is_two_lines_plus_its_capacity ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished
```

**A third line for a `usize`.** That is the unavoidable consequence of a
64-aligned structure needing one more byte than 128, and its cost is argued in
[`decisions/002`](../decisions/002_the_capacity_is_held_by_the_pair.md) — which
also records that the test bounding it says the opposite of what it enforces.

### The Separation Survives Field Reordering

`CursorPair` carries no `repr` attribute, so Rust may place the three fields in
any order. The guarantee holds anyway, and the reason is worth stating:

- each cursor is 64 bytes and 64-aligned, so each must start at a multiple of 64;
- the structure is 192 bytes, so there are exactly three such slots — 0, 64, 128;
- two distinct cursors cannot occupy the same slot.

Whichever slots the compiler picks, the two cursors are on two different lines.
**The layout guarantee comes from the alignment arithmetic, not from the
declaration order** — which is why the diagram above names "a `PaddedCursor`"
rather than a specific field, and why no test asserts an offset.

What a reordering *would* change is which line `capacity` shares, and with
nothing: it sits alone in whichever slot is left, with 56 bytes of padding after
it.

### Checked on Addresses, Not on the Type

```rust
pub fn on_distinct_lines( &self ) -> bool
{
  on_distinct_lines( self.producer.addr(), self.consumer.addr() )
}
```

`size_of` is a promise about a type; two fields being 64 bytes apart is the fact
the promise was made about. `on_distinct_lines` is the only one of the three
reached-test clauses taken from real addresses, and it is the one that would
notice a future layout where the two cursors stopped being separate fields while
each still measured 64 bytes on its own.

`the_gap_survives_the_pair_being_moved` boxes the pair and re-checks. Its own
comment is honest that this cannot fail — field offsets are fixed at compile
time — and says why it is there: to catch a future layout where the two cursors
stop being separate fields.

### The Test Asserts Both Forms of the Predicate

```rust
let gap = pair.producer().addr().abs_diff( pair.consumer().addr() );

assert!( gap >= CACHE_LINE, "clause 3: producer and consumer are {gap} bytes apart, need >= {CACHE_LINE}" );
assert!( pair.on_distinct_lines(), "and the gap actually puts them on different lines, not merely far apart" );
```

Two different predicates, and the relationship between them is one-way:

| Form | Expression | Answers |
|------|-----------|---------|
| subtraction | `abs_diff( a, b ) >= 64` | are they **far apart**? |
| division | `a / 64 != b / 64` | are they **on different lines**? |

**Subtraction implies division, not the reverse.** Two addresses on the same line
both lie in `[ 64k, 64k + 64 )`, so they differ by at most 63 — therefore
`abs_diff >= 64` can only hold for addresses on different lines. The converse
fails: 60 and 68 are 8 apart and still on different lines, which a
subtraction-only check would reject.

So the second assertion cannot fail if the first passed. Its message —
*"and the gap actually puts them on different lines, not merely far apart"* —
reads as though it adds a check, and the implication runs the other way: being
64-or-more apart *already* guarantees different lines.

That does not make it worthless. It exercises `CursorPair::on_distinct_lines`,
which is the method every other consumer calls and which no other test in this
file invokes on its own. It is a smoke test of the predicate wearing the label of
a stronger claim.

**The same distinction is handled worse once elsewhere.**
`ring_mpsc/src/lib.rs:860-866` computes the subtraction form under a method name
shared with `ring_align`'s division-form function, in an assertion that is
near-vacuous for its stated purpose: with padding present, two distinct
64-aligned addresses always differ by at least 64, so it can never fail; without
padding, allocator placement usually keeps it passing anyway. That finding is
recorded in
[`ring_align`'s pitfall/001](../../../ring_align/docs/pitfall/001_a_constant_too_small_buys_nothing.md).
The two are worth reading together: the same arithmetic, mislabelled here and
misused there.

### What the Pair Does Not Guarantee

| # | Not guaranteed | Consequence |
|---|----------------|-------------|
| R1 | Which field is on which line | Nothing depends on it |
| R2 | That `capacity` shares no line with a cursor | It cannot, at 192 bytes — but that is arithmetic, not a declaration |
| R3 | That two *different pairs* do not share lines | Two allocations can land anywhere. Only within one pair is separation structural |
| R4 | That the readings are atomic together | Each performs its own two loads — see [`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md) § How Far That Test Reaches |

### CU11 — The Size Assertion Passes in the State Its Message Forbids

```rust
let size = core::mem::size_of::< CursorPair >();

assert!( size >= 2 * CACHE_LINE, "two cursors at minimum, got {size}" );
assert!( size <= 3 * CACHE_LINE, "capacity must not cost a whole extra line, got {size}" );
```

`3 * CACHE_LINE` is 192. The measured size is 192. The bound is `<=`, so it
holds — at exactly the value the failure message describes as the thing that must
not happen.

**Finding.** A reader auditing the suite for cost control would conclude the
third line was ruled out. It was ruled in. Either repair — tightening the bound
to `<` or replacing it with `==` and rewriting the message — changes what the
suite claims, so this is recorded rather than fixed, in the same shape
`ring_align` recorded for `an_oversized_payload_rounds_up_to_whole_lines`.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'capacity may cost' ring_cursor/tests/cursor_test.rs
```

Live output:

```
  assert!( size <= 3 * CACHE_LINE, "capacity may cost up to one additional cache line, got {size}" );
```

**Disposition:** applied — the repair the finding declined to choose between
was a false choice: the message can be corrected without touching the bound.
`tests/cursor_test.rs:420` now reads "capacity may cost up to one additional
cache line", which is what the `<=` bound has always actually enforced, so the
suite's claim is unchanged and the message no longer contradicts it. Verified
with `cargo test --release -p ring_cursor` —
`a_pair_is_two_lines_plus_its_capacity` still passes.
Now prints: `capacity may cost up to one additional cache line`

---

### CU12 — The Third Line Is Fifty-Six Bytes of Padding for Eight of Payload

| Bytes | Content |
|------:|---------|
| 0–63 | `producer` |
| 64–127 | `consumer` |
| 128–135 | `capacity` |
| 136–191 | padding |

`capacity` is declared after two 64-aligned fields, so it starts a third line and
the type rounds up to 192.

**Finding.** That is the same 56-in-64 ratio
[`001`](001_the_padded_cursor.md) documents for a cursor, paid a second time for
a value written once at construction and never again. The padding around a cursor
buys isolation from a writer on another core; the padding around `capacity` buys
nothing, because nothing writes to it.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The three readings over these fields |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_padded_cursor.md](001_the_padded_cursor.md) | The 64-byte unit this is two of |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_capacity_is_held_by_the_pair.md](../decisions/002_the_capacity_is_held_by_the_pair.md) | Why the third field exists, and what it costs |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | The reached-test whose third clause this structure satisfies |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_cursor_pair_and_its_readings.md](../item/002_cursor_pair_and_its_readings.md) | Every associated function, with signatures |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_a_pair_across_a_full_lap.md](../lifecycle/002_a_pair_across_a_full_lap.md) | This structure's states, from empty to full and back |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:223-252` | The declaration and its argument |
| `ring_cursor/src/lib.rs:438-441` | `on_distinct_lines` |
| `ring_types/src/capacity.rs:23` | `pub struct Capacity( usize )` |
| `ring_align/src/lib.rs` | `on_distinct_lines` — the division form |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:80-87` | Both forms of the line predicate, asserted separately |
| `tests/cursor_test.rs:90-101` | The gap survives a move onto the heap |
| `tests/cursor_test.rs:417-421` | The size bounds — and the assertion whose message once contradicted its bound |
| `tests/cursor_test.rs:319-333` | Writing one cursor leaves the other alone — separate *values*, not just separate lines |
