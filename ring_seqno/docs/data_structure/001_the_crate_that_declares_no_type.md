# Data Structure: The Crate That Declares No Type

### Scope

- **Purpose**: Record that `ring_seqno` declares no data structure at all, identify what it operates on instead, and assess what the absence buys and costs.
- **Responsibility**: Establish the absence mechanically, give the two borrowed types with the guarantees each carries, and name the newtypes that were not introduced.
- **In Scope**: Structure `ring_seqno` owns and does not own.
- **Out of Scope**: The slice parameter of `slowest` — see [`002`](002_the_slice_that_slowest_reads.md).

### The Absence

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE '^pub (struct|enum|union|trait|type)' ring_seqno/src/lib.rs   # 0
grep -E '^(use|pub use)' ring_seqno/src/lib.rs                        # one line
```

Live output:

```
0
use ring_types::{ Capacity, Seq };
```

```rust
// ring_seqno/src/lib.rs:27
use ring_types::{ Capacity, Seq };
```

One import, two names, no declarations. The crate is 136 lines of which five are
function bodies.

### What It Operates On

| Type | Owner | Representation | Guarantee it carries |
|------|-------|----------------|----------------------|
| `Seq` | `ring_types::id` | `pub struct Seq( pub u64 )` | None — any `u64` is valid. Field is public for exactly that reason |
| `Capacity` | `ring_types::capacity` | `pub struct Capacity( usize )` | **Non-zero and a power of two.** Field is private; `new` is the only constructor |

The visibility split is the load-bearing detail, and it is opposite in the two
cases for good reason:

- `Seq`'s field is `pub` because there is no invariant to protect. `Seq( 0 )`, `Seq( u64::MAX )` and everything between are legitimate positions.
- `Capacity`'s field is private because there is one. Every `Capacity` in existence passed `new`'s two checks.

That second guarantee is what makes `laps_between`'s unguarded division total —
see [`invariant/002`](../invariant/002_every_reading_is_total.md) § T3. This crate
gets it for free and restates it nowhere.

### What the Absence Buys

| Benefit | Detail |
|---------|--------|
| **Nothing to fork** | The family's two sharing failures were a free function (`ring_align::on_distinct_lines`, reimplemented by `ring_mpsc`) and a constant (`ring_cursor::GATING`, restated four times). Both are things a consumer can *copy*. A crate declaring no type and no constant offers nothing to copy — the functions can only be called or reimplemented, and reimplementing arithmetic is harder to do accidentally than restating `Ordering::Acquire` |
| No construction cost | Callers already hold `Seq` and `Capacity`; nothing is built to call these functions |
| No lifetime, no `Drop`, no `Send`/`Sync` question | The whole surface is `Copy` scalars and one borrowed slice |
| Nothing to keep consistent | There is no state, so there is no invariant this crate can violate |

The first row is the substantive one, and it is a genuine structural advantage
rather than a stylistic preference. `ring_cursor`'s `GATING` is restated in three
other crates because a `const` is four characters to retype and the compiler
never notices. Nothing in `ring_seqno` has that shape.

### What the Absence Costs

**No newtype means the compiler cannot separate quantities that share a
representation.** Three are visible:

| Quantity | Type as written | Would-be newtype | What the collision permits |
|----------|-----------------|------------------|----------------------------|
| A free-slot count | `usize` | `FreeSlots( usize )` | Indexing a buffer with it — same type, overlapping range as `SlotIndex` |
| A lap count | `u64` | `Laps( u64 )` | Adding it to a `Seq`, which is meaningless without multiplying by capacity |
| A producer vs a consumer position | both `Seq` | `Producer( Seq )`, `Consumer( Seq )` | Swapping arguments — silent, and the wrong answer is a plausible one |

The third is the one with a documented consequence. All four binary readings take
two `Seq` and the compiler cannot tell which is which, so `may_claim( consumer,
producer, cap )` compiles and returns `true` — a permissive answer for a swapped
call. See [`api/002`](../api/002_the_argument_order_split.md) and
[`decisions/002`](../decisions/002_saturating_rather_than_signed.md).

**None of these is a defect in this crate.** Introducing `Producer`/`Consumer`
newtypes here would not help — `ring_cursor` would have to construct them at
every call site from values it loaded, and a wrapper applied at the call site
protects nothing, since the swap happens there. The newtypes would have to live
in `ring_types` and be threaded through `PaddedCursor`, which is a family-wide
change to the vocabulary.

Recorded because a reader asking "why is there no type here" deserves both halves
of the answer: the absence is right for this crate, and the safety it forgoes is
real and is not recovered anywhere else.

### `ring_types` Already Made This Choice Once

The precedent is visible in `ring_types` itself:

| Item | Returns | Newtype? |
|------|---------|:--------:|
| `SlotIndex::get` | `usize` | The *input* is a newtype; the output is bare |
| `Capacity::get` | `usize` | Bare |
| `Seq::distance_to` | `u64` | Bare |

`ring_types` wraps *positions* (`Seq`, `SlotIndex`) and *configuration*
(`Capacity`) but never wraps a **span** — a count of things. `ring_seqno` returns
only spans, so following that convention means returning nothing wrapped. The
crate is consistent with the vocabulary it sits on; the question is whether the
vocabulary should draw the line there, and that is `ring_types`'s to answer.

### SQ9 — Nothing to Fork

The crate exports five functions and no nouns at all:

```
pub struct  in ring_seqno/src/lib.rs:   0
pub enum    in ring_seqno/src/lib.rs:   0
pub const   in ring_seqno/src/lib.rs:   0
```

**Finding.** Declaring no type is what keeps the crate free of the fork risk that hit `ring_align`'s constant — there is nothing to copy.

---

### SQ10 — Five Functions, Five Operations

Every function body in the crate fits on one line, and four of them call the same method:

```
laps_between   earlier.distance_to( later ) / capacity.get() as u64
may_claim      consumer.distance_to( producer ) < capacity.get() as u64
free_slots     consumer.distance_to( producer ) -> saturating_sub -> as usize
pending        consumer.distance_to( producer )
slowest        cursors.iter().copied().min()
```

**Finding.** Five functions reduce to four calls of `Seq::distance_to` and one `.min()`, so the crate borrows all of its arithmetic and contributes only the framing.

---

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_slice_that_slowest_reads.md](002_the_slice_that_slowest_reads.md) | The one aggregate the crate touches |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_five_functions_and_no_types.md](../api/001_five_functions_and_no_types.md) | The surface, and what a type-free crate can promise |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_how_the_fold_crossed_four_tiers.md](../integration/002_how_the_fold_crossed_four_tiers.md) | Why "nothing to copy" held in practice |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_every_reading_is_total.md](../invariant/002_every_reading_is_total.md) | The `Capacity` guarantee this crate inherits |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_what_a_span_is_measured_in.md](../type/001_what_a_span_is_measured_in.md) | The count/index collision in detail |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:27` | The one import — the crate's whole structural vocabulary |
| `ring_types/src/id.rs:24-25` | `Seq`, public field |
| `ring_types/src/capacity.rs:22-51` | `Capacity`, private field and checked constructor |
| `ring_types/src/id.rs:90-106` | `SlotIndex` — the precedent for wrapping a position and not a span |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:18-21` | `cap()` — the only construction any test performs |
| `tests/seq_test.rs:15-16` | Two `use` lines cover the whole crate and its vocabulary |
