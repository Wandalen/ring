# Decision: The Capacity Is Held by the Pair

### Scope

- **Purpose**: Record why `CursorPair` carries a `Capacity` field rather than taking one per reading, what that costs in bytes, and that the test bounding the cost states the opposite of what it enforces.
- **Responsibility**: Give the decision, the four alternatives, the derived size, and the assertion whose message contradicts its own bound.
- **In Scope**: `CursorPair`'s third field at `src/lib.rs:251`; the readings that use it; `a_pair_is_two_lines_plus_its_capacity`.
- **Out of Scope**: What the readings compute, which is [`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md); the layout the two cursors impose, which is [`data_structure/002`](../data_structure/002_the_cursor_pair.md).

### The Decision

```rust
pub struct CursorPair
{
  producer : PaddedCursor,
  consumer : PaddedCursor,
  capacity : Capacity,
}
```

The argument, from `src/lib.rs:226-230`:

> The capacity is held here rather than passed to each reading because a pair is
> always a pair *for* a ring of some size: every question worth asking of two
> cursors … is unanswerable without it, and a caller supplying it per call could
> supply a different one each time.

**The second clause is the load-bearing one.** A per-call parameter is not
merely inconvenient — it is a way for one caller to ask `free_slots( 8 )` and
another to ask `free_slots( 16 )` about the same ring, and for both to get an
internally consistent answer to the wrong question. Holding it makes the ring
size a property of the pair rather than of the question.

### Alternatives

| # | Alternative | Why it lost |
|---|-------------|-------------|
| G1 | `fn free_slots( &self, capacity : Capacity )` | Two callers can disagree about the ring they share, and nothing detects it |
| G2 | A type parameter — `CursorPair< const N : usize >` | Makes capacity a compile-time constant. `Capacity::new` is fallible and checked at runtime precisely because ring sizes come from configuration |
| G3 | Store it once in the ring, hand the pair a reference | Adds a lifetime to a type whose whole job is to be a field of the ring. The ring would own a pair that borrows the ring |
| G4 | No capacity — expose only `pending` | `pending` is the one reading that does not need it (see [`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md)). Dropping the other two makes the pair a container rather than a gate |

**G4 is not hypothetical — `ring_barrier` took it.** `Barrier::over( &cursors )`
holds a cursor slice and no capacity, because its question is *distance* rather
than *room*. So the family has both answers, and each is right for its question.

### The Family Is Split, by Layer

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the layers that TAKE a capacity, as a parameter --'
command grep -rho 'pub fn [a-z_]*.*capacity : Capacity' --include=*.rs ring_seqno/src/ ring_batch/src/ | sort
echo '  -- the layers that HOLD one, as a field, and hand it back --'
for c in ring_cursor ring_gating ring_barrier; do
  field=$( command grep -c '^  capacity : Capacity,$' "$c/src/lib.rs" || true )
  getter=$( command grep -c 'pub const fn capacity( &self ) -> Capacity' "$c/src/lib.rs" || true )
  printf '    %-12s field: %s  accessor: %s\n' "$c" "$field" "$getter"
done
```

Live output:

```
  -- the layers that TAKE a capacity, as a parameter --
pub fn drain_order( claim : &BatchClaim, capacity : Capacity
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity
  -- the layers that HOLD one, as a field, and hand it back --
    ring_cursor  field: 1  accessor: 1
    ring_gating  field: 1  accessor: 1
    ring_barrier field: 0  accessor: 0
```

| Layer | Holds or takes | Example |
|-------|----------------|---------|
| `ring_seqno` | **takes** — every function has a `capacity` parameter | `free_slots( producer, consumer, capacity )` |
| `ring_batch` | **takes** — `claim_gated( …, capacity, order )` | generic over `SeqCell`, so it has nothing to hold it on |
| `ring_cursor` | **holds** | `CursorPair.capacity` |
| `ring_gating` | **holds** | `GatingSet` — same decision, same reason |
| `ring_barrier` | **neither** | its question does not need one |

The split is not inconsistency. The lower two layers are *functions* over
sequences and cannot hold state; the upper two are *structures* that own the
cursors and can. The boundary between "takes" and "holds" is exactly the
boundary between a free function and a type with fields.

### What It Costs

Two `PaddedCursor` fields occupy bytes 0–63 and 64–127. `Capacity` is
non-zero-sized, so it lands at or beyond byte 128; `CursorPair` inherits
alignment 64 from its cursors, so the size rounds up to the next multiple of 64:

| Quantity | Value |
|----------|-------|
| Two cursors | 128 bytes |
| `Capacity` payload | 8 bytes |
| `align_of::< CursorPair >()` | 64 — inherited from `PaddedCursor` |
| `size_of::< CursorPair >()` | **192** — 128 rounded up past 136 |
| Bytes spent to carry 8 bytes of capacity | **64** |

The doctests run merged and in parallel, so their names arrive in an order that
changes between runs. Three parts of each line are not the claim: the file
prefix, which is constant; the timing, which is never twice the same; and the
`(line NNN)` rustdoc appends, which is an absolute source address that moves
whenever anything above it is edited. All three are stripped and the names
sorted, leaving only what the count below is actually about:

```sh
cd "$(git rev-parse --show-toplevel)"
out=$( cargo test -p ring_cursor --doc 2>&1 )
printf '%s\n' "$out" | command grep -E '^test .+\.\.\. ' \
  | sed -E 's:^test ring_cursor/src/lib.rs - ::; s: \(line [0-9]+\)::' | LC_ALL=C sort
printf '%s\n' "$out" | command grep -E '^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
CursorPair ... ok
CursorPair::capacity ... ok
CursorPair::consumer ... ok
CursorPair::free_slots ... ok
CursorPair::may_claim ... ok
CursorPair::new ... ok
CursorPair::on_distinct_lines ... ok
CursorPair::pending ... ok
CursorPair::producer ... ok
GATING ... ok
PaddedCursor ... ok
PaddedCursor::addr ... ok
PaddedCursor::new ... ok
slowest ... ok
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

**Fourteen doctests, one per public item plus the two type-level examples** —
and none of them is the size assertion. `assert_eq!( core::mem::size_of::< CursorPair >(), 192 )`
is not written anywhere; the closest thing to it is
`tests/cursor_test.rs`'s `a_pair_is_two_lines_plus_its_capacity`, which brackets
the size between two bounds rather than pinning it:

```rust
let size = core::mem::size_of::< CursorPair >();

assert!( size >= 2 * CACHE_LINE, "two cursors at minimum, got {size}" );
assert!( size <= 3 * CACHE_LINE, "capacity may cost up to one additional cache line, got {size}" );
```

`3 * CACHE_LINE` is 192, and the measured size is 192 — the upper bound is not
merely satisfied, it is met exactly. A test that admits one additional cache
line is spending the whole allowance, so any future field added to `CursorPair`
fails it immediately. That is a tighter guard than the `<=` suggests, and it is
tight by accident rather than by assertion: nothing here would notice if the
compiler placed `Capacity` such that 128 bytes sufficed.

**A whole cache line for a `usize`.** That is the honest cost, and it is
accepted: a `CursorPair` is one per ring, not one per element, so 64 bytes is
paid once for the lifetime of the ring. The alternative that saves it —
threading the capacity through every call — trades 64 bytes of static overhead
for the correctness hazard G1 describes.

### The Test Said the Opposite of What It Checked

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A12 -F '  // 64 bytes of which 8 carry a sequence. If the type ever grows a field, the' ring_cursor/tests/cursor_test.rs
```

Live output:

```
  // 64 bytes of which 8 carry a sequence. If the type ever grows a field, the
  // size stops being CACHE_LINE and the first test catches it — but this one
  // says why that would be wrong: the padding is meant to be empty, not to be
  // budget for state that belongs elsewhere.
  assert_eq!( core::mem::size_of::< PaddedCursor >(), CACHE_LINE );
  assert_eq!( core::mem::size_of::< core::sync::atomic::AtomicU64 >(), 8 );
}

#[ test ]
fn a_pair_is_two_lines_plus_its_capacity()
{
  // Not pinned to an exact number: the capacity's own placement within the
  // struct's trailing padding is the compiler's business. What must hold is
```

```rust
// … and that adding the capacity did not cost a third line of padding.
let size = core::mem::size_of::< CursorPair >();

assert!( size >= 2 * CACHE_LINE, "two cursors at minimum, got {size}" );
assert!( size <= 3 * CACHE_LINE, "capacity may cost up to one additional cache line, got {size}" );
```

`3 * CACHE_LINE` is 192. The upper bound **permits** exactly the outcome the
message now names as the accepted price, and 192 is the value the type
actually has — so the assertion passes in exactly the state its message
describes.

| | Message said | Bound enforces | Actual |
|---|---|---|---|
| Third line | must not be spent | may be spent | **is** spent |

The bound that matched the old message was `size < 3 * CACHE_LINE`, and it
would have failed at 192. The message has since been corrected to match the
bound that was always actually enforced — see
[`data_structure/002`](../data_structure/002_the_cursor_pair.md) CU11.

**Disposition: applied.** `tests/cursor_test.rs:420`'s message now reads
"capacity may cost up to one additional cache line", matching the `<=` bound
it has always enforced. The repair changed the message, not the bound —
192 is the correct and unavoidable size, and the suite's claim is unchanged.
Verified with `cargo test --release -p ring_cursor` —
`a_pair_is_two_lines_plus_its_capacity` still passes.

**This is the same defect shape `ring_align` recorded** in
`an_oversized_payload_rounds_up_to_whole_lines`: an assertion whose bound is
wide enough to accept the case its comment said it excluded. Worth noting
that `ring_align`'s instance is recorded as declined, not fixed — this crate's
own instance shows the repair was cheap once the false choice (rewrite the
message, or tighten the bound and fail the suite) was seen for what it was:
only the message needed to change.

### What Would Reopen It

| # | Condition | Effect |
|---|-----------|--------|
| H1 | `CACHE_LINE` rises to 128 | The pair becomes 384 bytes and the field costs 128. The ratio worsens; the decision does not change |
| H2 | A ring needing more than one capacity-like field | The third line is already paid — a fourth and fifth field are free until byte 192 |
| H3 | Millions of pairs | 64 bytes each stops being static overhead. Nothing in the family creates pairs at that rate |

**H2 is worth noticing.** Having spent the third line, the pair has 56 unused
bytes. Any future per-ring scalar is free to add — which is an argument for
putting the next one here rather than making it a parameter, and an argument
against `tests/manual/readme.md` M2's reasoning applied one level up: M2 warns
that `PaddedCursor`'s padding is not budget for state, and the same warning
applies to the pair's.

### CU15 — The Decision Is Argued From Correctness and Costs a Cache Line

The doc comment gives one reason: "a caller supplying it per call could supply a
different one each time." That is the load-bearing argument and it is right.

The price is 64 bytes — a whole line to carry eight, per
[`data_structure/002`](../data_structure/002_the_cursor_pair.md) CU12.

**Finding.** Both halves are true and only the first is written down. A reader of
`src/lib.rs` sees why the field is held and not what it costs, so the tradeoff
reads as free. It is not free; it is cheap, because a `CursorPair` is one per ring
rather than one per element — which is the sentence the doc comment is missing.

---

### CU16 — The Number the Decision Turns On Is Asserted Nowhere

```
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

Fourteen doctests assert behaviour. `tests/cursor_test.rs:419-420` asserts a
bound. No assertion anywhere states `size_of::< CursorPair >() == 192`.

**Finding.** The 192 that this decision's cost table is built on is measured by
reading a table in this document. If the size changed to 256 — a fourth field, a
wider `Capacity` — the bound at CU11 would fail and the reader would learn it;
if it changed to 128, everything would pass and every cost figure here would be
silently wrong.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The two readings that use the field, and the one that does not |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | The layout the 192 comes from |

### Decisions

| File | Relationship |
|------|--------------|
| [001_gating_is_fixed_not_a_parameter.md](001_gating_is_fixed_not_a_parameter.md) | The other choice about where a fact lives rather than what it is |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_cursor_pair_and_its_readings.md](../item/002_cursor_pair_and_its_readings.md) | `capacity()`, the accessor the field exists behind |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_the_layout_claim_is_testable.md](../non_functional_requirement/001_the_layout_claim_is_testable.md) | The suite this assertion belongs to, and what it does and does not establish |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:223-252` | The type and its argument |
| `ring_types/src/capacity.rs` | `Capacity` — non-zero-sized, runtime-checked |
| `ring_gating/src/lib.rs` | `GatingSet`, which took the same answer |
| `ring_barrier/src/lib.rs:191-194` | `Barrier`, which took G4 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:410-422` | `a_pair_is_two_lines_plus_its_capacity` — the assertion whose message once disagreed with its bound |
| `src/lib.rs:338-345` | `capacity()`'s doctest |
| `tests/cursor_test.rs:177-187` | A fresh pair reports its capacity as free slots |
