# Data Structure: The Padded Cursor

### Scope

- **Purpose**: Give `PaddedCursor` as a layout — how many bytes, how many carry state, and why the array stride matters more than the alignment.
- **Responsibility**: State the sizes with the commands that regenerate them, show why alignment alone would not separate two cursors, and say what the layout does not fix.
- **In Scope**: `PaddedCursor`'s size, alignment, stride, and the padding's storage.
- **Out of Scope**: The promises a consumer may rely on, which are [`type/002`](../type/002_padded_cursor.md); the pair built from two of these, which is [`data_structure/002`](002_the_cursor_pair.md).

### The Layout

```
PaddedCursor — 64 bytes, align 64

  ┌────────────────────────────────────────────────────────────┐
  │ AtomicSeq (8 bytes)  ·  56 bytes of padding                │
  └────────────────────────────────────────────────────────────┘
  ↑
  address ≡ 0 (mod 64)
```

The atomic's position inside those 64 bytes is drawn first because that is where
a reader expects it, not because it is fixed —
see § What Is Not Fixed.

| Type | `size_of` | `align_of` |
|------|----------:|-----------:|
| `AtomicU64` | 8 | 8 |
| `AtomicSeq` | 8 | 8 |
| `CacheAligned< AtomicSeq >` | 64 | 64 |
| `PaddedCursor` | 64 | 64 |

Both rows are asserted, by name, in one filtered run. Cargo's own progress lines
carry a build timing and a content hash, so the output is reduced to the test
names and the verdict — everything the claim rests on, and nothing that changes
between runs:

```sh
cd "$(git rev-parse --show-toplevel)"
out=$( cargo test -p ring_cursor --test cursor_test a_padded_cursor_ 2>&1 )
printf '%s\n' "$out" | command grep -E '^test .+\.\.\. ' | LC_ALL=C sort
printf '%s\n' "$out" | command grep -E '^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
test a_padded_cursor_is_its_atomic_and_nothing_else ... ok
test a_padded_cursor_occupies_exactly_one_cache_line ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished
```

**Eight bytes of state in sixty-four.** The 56 bytes are not a field and are not
initialised — they are the difference between the payload's size and the type's,
which the compiler creates by rounding the size up to the alignment. Nothing
writes them, nothing reads them, and they cost address space rather than
instructions.

### Why the Size Matters More Than the Alignment

`align_of == 64` constrains where a value may *start*. It says nothing about how
much room the value takes, so it does not, on its own, separate two cursors:

| Hypothetical | `size_of` | `align_of` | Two in an array land |
|--------------|----------:|-----------:|----------------------|
| aligned but small | 8 | 64 | **8 bytes apart** — same line |
| sized but unaligned | 64 | 8 | possibly at offset 8, **straddling two lines** and sharing both |
| `PaddedCursor` | 64 | 64 | 64 bytes apart, one per line |

The second row is the subtler failure: a 64-byte type starting at offset 8
occupies bytes 8–71, touching line 0 and line 1, so *two* such values contend on
line 1 even though each is "a cache line in size".

`#[ repr( align( 64 ) ) ]` gives both — it rounds the size up as well as
constraining the start — but the reached-test names them separately because a
future layout change could break one while leaving the other intact.

### The Stride Is the Observable Consequence

```rust
let cursors : [ PaddedCursor; 4 ] = Default::default();

for window in cursors.windows( 2 )
{
  let gap = window[ 1 ].addr() - window[ 0 ].addr();
  assert_eq!( gap, CACHE_LINE, "consecutive cursors are one line apart, not {gap}" );
}
```

`an_array_of_cursors_gives_each_its_own_line` is the test that would fail for the
"aligned but small" row above, and the only one that would: `align_of` constrains
only the first element's address, while `size_of` constrains the stride between
all of them.

**This is the case the whole family actually uses.** `ring_gating::GatingSet`
holds a `Vec< PaddedCursor >` of consumer cursors, read on every producer claim.
An array of cursors sharing lines is the exact contention the padding exists to
remove, multiplied by the number of consumers.

### What Is Not Fixed

| # | Unspecified | Why it does not matter |
|---|-------------|------------------------|
| Q1 | The atomic's offset within the 64 bytes | `#[ repr( align ) ]` implies neither `repr( C )` nor `repr( transparent )`. Every access goes through `CacheAligned::get`, so no code names an offset |
| Q2 | Where the padding sits — before, after, or around the atomic | Same reason. It is unnamed space, not a field |
| Q3 | Whether two cursors in *different allocations* share a line | Allocator placement. `CursorPair` is where separation becomes a guarantee, because there the two are fields of one structure |

**Q1 is worth stating because it is easy to assume otherwise.** A 64-aligned
newtype around an 8-byte value looks like it must put the value at offset 0, and
in practice it does. The declaration does not say so, and
[`ring_align`'s own layout document](../../../ring_align/docs/data_structure/001_the_cache_aligned_wrapper.md)
records the same caveat about `CacheAligned` itself.

### Where the Padding Is Not Free

| Context | Cost |
|---------|------|
| One `CursorPair` per ring | 128 bytes of cursors where 16 would carry the state — paid once per ring |
| `GatingSet` with `n` consumers | `64n` bytes where `8n` would do. At 16 consumers, 1 KiB for 128 bytes of sequences |
| A cursor read on a single-threaded path | The full 64 bytes still occupy a line the CPU must fetch, for 8 bytes of use |

The last row is the honest one: on an uncontended path the padding is pure loss —
it costs a cache line's worth of fetch to read a `u64`. The trade only pays under
contention, which is why the measurement belongs to
`ring_bench` rather than to an assertion here.

### CU9 — Every Byte and Every Behaviour Comes From Somewhere Else

```
144:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
203:    self.0.get().load( order )
208:    self.0.get().store( value, order );
213:    self.0.get().fetch_add( n, order )
219:    self.0.get().compare_exchange( current, new, success, failure )
```

One tuple field, no named field, no constant, no method body longer than a line.
The 64 bytes are `ring_align`'s and the four behaviours are `ring_atomic`'s.

**Finding.** It is the smallest structure in the family that still owns a name,
and the name is the whole contribution: it is what lets a cursor be a distinct
type from every other 64-byte-aligned atomic, and what
[`pitfall/001`](../pitfall/001_the_obvious_implementation_forks_the_constant.md)
says would be lost by inlining the wrapper.

---

### CU10 — The Two Structures Differ in Constructibility and Nothing Says So

```rust
#[ derive( Debug, Default ) ]
pub struct PaddedCursor( CacheAligned< AtomicSeq > );

#[ derive( Debug ) ]
pub struct CursorPair { … }
```

`PaddedCursor` derives `Default`; `CursorPair` cannot, because `Capacity` has no
default and should not — a ring size of zero is the one value the type exists to
reject.

**Finding.** The asymmetry is correct and undocumented. No test asserts it, no
doc comment mentions it, and a caller reaching for `CursorPair::default()` learns
it from a compiler error rather than from the page describing the type.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The loads this layout separates |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_cursor_pair.md](002_the_cursor_pair.md) | Two of these in one structure — where separation becomes a guarantee |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | The three-clause statement of this layout |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_the_layout_claim_is_testable.md](../non_functional_requirement/001_the_layout_claim_is_testable.md) | What the suite establishes about this layout, and what it defers |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | The newtype that carries this layout without adding to it |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_obvious_implementation_forks_the_constant.md](../pitfall/001_the_obvious_implementation_forks_the_constant.md) | The reimplementation that produces this same layout and a second constant |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The same structure as a set of promises |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:125-144` | The declaration |
| `ring_cursor/src/lib.rs:23-36` | The module-level argument for asserting size and alignment separately |
| `ring_align/src/lib.rs` | `CacheAligned`, which supplies the attribute |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:54-72` | Size and alignment, with `64` pinned to `CACHE_LINE` |
| `tests/cursor_test.rs:102-111` | Every cursor starts on a line boundary |
| `tests/cursor_test.rs:113-126` | The array stride — the assertion alignment alone would not satisfy |
| `tests/cursor_test.rs:399-408` | 64 bytes of which 8 carry a sequence |
| `tests/cursor_test.rs:9-22` | The suite's own argument for three clauses rather than one |
