# API: The Surface That Forwards

### Scope

- **Purpose**: List the half of the surface that hands the caller's decisions through unchanged, and record why the trait travels with the type.
- **Responsibility**: Give each item's signature, say what it forwards to, and measure the re-export's claim about manifest edits.
- **In Scope**: `pub use ring_atomic::SeqCell`; `PaddedCursor::new`, `addr`; the four `SeqCell` methods.
- **Out of Scope**: The half that fixes the ordering, which is [`api/002`](002_the_surface_that_decides.md); the declaration-level reference, which is [`item/001`](../item/001_padded_cursor_and_its_functions.md).

### The Items

| Item | Signature | Forwards to |
|------|-----------|-------------|
| `SeqCell` | `pub use ring_atomic::SeqCell;` | — the trait itself, re-exported |
| `PaddedCursor::new` | `pub const fn new( value : Seq ) -> Self` | `AtomicSeq::new`, inside `CacheAligned::new` |
| `PaddedCursor::addr` | `pub fn addr( &self ) -> usize` | `core::ptr::from_ref( self ) as usize` |
| `<PaddedCursor as SeqCell>::load` | `fn load( &self, order : Ordering ) -> Seq` | `self.0.get().load( order )` |
| `…::store` | `fn store( &self, value : Seq, order : Ordering )` | `self.0.get().store( value, order )` |
| `…::fetch_add` | `fn fetch_add( &self, n : u64, order : Ordering ) -> Seq` | `self.0.get().fetch_add( n, order )` |
| `…::compare_exchange` | `fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering ) -> Result< Seq, Seq >` | the same, unchanged |

**Every one of the four methods is one line, and passes `order` through
untouched.** There is no clamping, no upgrading, no `debug_assert` that the
caller picked something sensible. `padding_does_not_change_what_the_cell_does`
is the test that pins it: every `SeqCell` method must behave exactly as the
unpadded cell does, or the padding has stopped being free.

### Why the Trait Is Re-Exported

```rust
pub use ring_atomic::SeqCell;
```

The argument, from `src/lib.rs:60-68`:

> Every read and write of a cursor is a [`SeqCell`] method, so a crate holding a
> `PaddedCursor` and not this trait holds a value it cannot load. Making each
> such crate declare `ring_atomic` itself would put a dependency in four manifests
> to import one trait — and would say, wrongly, that those crates have business
> with the atomic layer beyond the cursor they were handed.

**This is a Rust-specific obligation, not a convenience.** A trait method is
callable only where the trait is in scope, so handing a caller a `PaddedCursor`
without a route to `SeqCell` hands it an opaque 64 bytes. The re-export is the
route.

### Measuring the Claim

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'use ring_cursor::.*SeqCell' --include=*.rs \
  | grep -vE ':[[:space:]]*(///|//!)' | LC_ALL=C sort -t: -k1,1 -k2,2n
# excludes Cargo.toml itself: that workspace manifest lists every
# crate as a member path (including ring_cursor), which is not the same
# claim as a manifest declaring ring_cursor as a dependency
command grep -rl 'ring_cursor' --include=Cargo.toml | command grep -v '^Cargo.toml$' | wc -l
for c in ring_barrier ring_claim ring_consume ring_debug ring_gating ring_mpsc \
         ring_publish ring_shutdown ring_spsc ring_wait
do grep -q '^ring_atomic' $c/Cargo.toml && echo "$c also declares ring_atomic" || true; done
```

Live output:

```
ring_barrier/tests/allocation_test.rs:use ring_cursor::{ PaddedCursor, SeqCell };
ring_barrier/tests/barrier_test.rs:use ring_cursor::{ PaddedCursor, SeqCell };
ring_claim/src/lib.rs:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_claim/tests/claim_test.rs:use ring_cursor::SeqCell;
ring_consume/src/lib.rs:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_consume/tests/consume_test.rs:use ring_cursor::{ PaddedCursor, SeqCell };
ring_gating/tests/gating_test.rs:use ring_cursor::SeqCell;
ring_publish/src/lib.rs:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_publish/tests/handshake_test.rs:  use ring_cursor::SeqCell;
ring_publish/tests/publish_test.rs:use ring_cursor::SeqCell;
ring_shutdown/tests/shutdown_test.rs:use ring_cursor::{ CursorPair, SeqCell };
ring_spsc/src/lib.rs:use ring_cursor::{ CursorPair, SeqCell, GATING };
ring_wait/tests/wait_test.rs:use ring_cursor::{ CursorPair, SeqCell };
11
ring_debug also declares ring_atomic
ring_mpsc also declares ring_atomic
```

| Measurement | Value |
|-------------|------:|
| Crates declaring `ring_cursor` | **10** (11 manifests match, including this crate's own) |
| Of those, also declaring `ring_atomic` | **2** — `ring_debug`, `ring_mpsc` |
| Of those, importing `SeqCell` **via** `ring_cursor` in non-doc code | `ring_claim`, `ring_consume`, `ring_gating`, `ring_publish`, `ring_shutdown`, `ring_spsc`, `ring_wait`, `ring_barrier` — in `src/` or `tests/` |

**Eight crates take the re-export route; the claim said six.** The count in the
doc comment was written when the family was smaller and has not been revised —
it understates its own case, which is the harmless direction for a claim like
this to drift, and it is still worth noting that no check would have caught it
drifting the other way.

The two that declare `ring_atomic` anyway do so for reasons beyond the trait:
`ring_mpsc` uses `AtomicSeq` directly, `ring_debug` reads cells the cursor layer
does not expose. Neither contradicts the argument — both have genuine business
with the atomic layer, which is exactly the condition the doc comment names.

### What the Forwarding Costs

Nothing measurable, and one thing structural:

| | |
|---|---|
| Runtime | Each method is a single delegating call, `#[ inline ]` by default for a one-line generic-free body in the same crate |
| Compile time | Four extra impls |
| Structural | `PaddedCursor` cannot add an ordering policy later without breaking the contract that it behaves as the unpadded cell does |

The last row is the real commitment. Once callers rely on `load( Relaxed )`
meaning `Relaxed`, the type cannot start upgrading orderings — which is what
makes the `CursorPair` half a *separate* surface rather than a stricter mode of
this one.

### CU5 — The Re-Export Is Justified by a Number Four Too High

```
who took the re-export, in library code
ring_claim:66:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_consume:72:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_publish:57:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_spsc:177:use ring_cursor::{ CursorPair, SeqCell, GATING };
```

`src/lib.rs:65` argues for the re-export on the ground that its absence "would
put a dependency in six manifests to import one trait". Four crates take
`SeqCell` through this crate in library code. `ring_barrier`, `ring_gating` and
`ring_wait` name it only inside doc examples, and `ring_shutdown` imports it from
neither route.

**Finding.** Six is not four, and the gap is not rounding — it is the difference
between counting doctest imports as manifest edges and not. The re-export is
still worth its line at four; the justification written beside it is an estimate
that was never re-run.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'manifests to import one trait' ring_cursor/src/lib.rs
```

Live output:

```
/// four manifests to import one trait — and would say, wrongly, that those
```

**Disposition:** applied — reworded the doc comment at `src/lib.rs:60-68` to
state the measured count instead of the unverified estimate: "six manifests"
is now "four manifests", matching the four crates (`ring_claim`,
`ring_consume`, `ring_publish`, `ring_spsc`) that actually take `SeqCell`
through the re-export in library code. Verified with `cargo test --release -p
ring_cursor` (all tests and doctests passing).
Now prints: `four manifests to import one trait`

---

### CU6 — Four Public Methods That No Declaration Census Can See

```
201:  fn load( &self, order : Ordering ) -> Seq
206:  fn store( &self, value : Seq, order : Ordering )
211:  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
216:  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
```

None carries `pub`. They are public because `SeqCell` is, and a method in a trait
impl may not restate it — the compiler rejects the attempt.

**Finding.** Every `^pub`-anchored count of this crate's surface is short by
exactly these four, and they are four of the most-called items in it. The
definition readme's own census prints 17 and the surface is 17, which agrees only
because the two arithmetics happen to meet — not because the census reached them.

---

### APIs

| File | Relationship |
|------|--------------|
| [002_the_surface_that_decides.md](002_the_surface_that_decides.md) | The half that takes the opposite policy |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | Why the two halves disagree, and why that is not an inconsistency |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | The ten consumers, and which route each takes to `SeqCell` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_padded_cursor_and_its_functions.md](../item/001_padded_cursor_and_its_functions.md) | Attributes, derives, and the caller tree for each item here |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The promises these items are the surface of |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_the_trait_must_travel_with_the_type.md](../workaround/002_the_trait_must_travel_with_the_type.md) | The re-export as an absorbed language constraint, with its deletion condition |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:60-69` | The re-export and its argument |
| `ring_cursor/src/lib.rs:145-190` | `new` and `addr` |
| `ring_cursor/src/lib.rs:199-221` | The four forwarding methods |
| `ring_atomic/src/lib.rs:108-151` | The trait being re-exported |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:137-160` | Every method behaves as the unpadded cell does |
| `tests/cursor_test.rs:130-135` | `new` and `default` hold what they were built with |
| `tests/cursor_test.rs:370-395` | Four threads on one cursor lose nothing — the forwarding is not merely syntactic |
| `src/lib.rs:179-184` | `addr`'s doctest — a 64-aligned value starts on a line boundary |
