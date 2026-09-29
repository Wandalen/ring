# data_structure

`Publisher` is one private field. Everything else in this directory is about what
that field is *not* — not the claimed cursor, not the consumer's position, not the
capacity — and what it costs to keep them apart.

The two files are a zoom: 001 goes down, four crates deep, from `Publisher` to
the `AtomicU64` that holds the eight bytes of state; 002 goes out, to the four
cursors the whole handshake runs on and the wiring that connects two of them
across a crate boundary.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [One Padded Cursor and Nothing Else](001_one_padded_cursor_and_nothing_else.md) | PB13, PB14 — the four-layer type stack, 56 bytes of padding and what it is padding against, the one `const fn`, and why the constructor is not it |
| 002 | [The Four Cursors of the Handshake](002_the_four_cursors_of_the_handshake.md) | PB15, PB16 — the topology, the two shared cursors, the wiring bug that passes every other test, and `ring_claim`'s two documented seams into this crate that do not exist |

### Down: The Type Stack

| Layer | Crate | Adds | Bytes |
|-------|-------|------|------:|
| `Publisher` | here | six methods | 64 |
| `PaddedCursor` | `ring_cursor:143` | the `SeqCell` impl, and `addr()` | 64 |
| `CacheAligned< T >`, `#[ repr( align( 64 ) ) ]` | `ring_align:68` | a whole cache line to itself | 64 |
| `AtomicSeq` | `ring_atomic:166` | named orderings, and the `loom` seam | 8 |
| `AtomicU64` | `core` **or** `loom` | the state | 8 |

Four crates for eight bytes, and each layer is load-bearing. The bottom one is
also where the crate's one visible compromise comes from: `Publisher::new` cannot
be `const`, because the `loom` variant of the constructor three layers down is
not.

### Out: The Four Cursors

| Cursor | Owner | Written by | Read by |
|--------|-------|-----------|---------|
| claimed | `ring_claim::Claimer` | `claim` / `claim_up_to` | **nobody** |
| **published** | **`Publisher`** | **`try_publish`** | **the consumer's `Barrier`** |
| consumer position | `ring_gating::GatingSet` | `Consumer::commit` | `Claimer`, via `headroom` |
| capacity | the same `GatingSet` | never — set at construction | `Claimer`, to bound the claim |

Two are shared, in opposite directions, and the consumer touches both — it
*reports into* a cursor borrowed from the producer's gating set and *waits on* a
barrier built over the publisher's. That is two lines of wiring
(`handshake_test.rs:390-391`) and the source of the failure 002 is built around:
substitute a fresh cursor for either share and everything still compiles, still
commits, still delivers in order, and silently stops gating.

The ring has two boundaries and each needs two cursors — one for *taken*, one for
*done* — which is why there are four and not two.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the size and alignment Publisher inherits, asserted in ring_cursor's own doctest
command grep -m1 -A17 -F '/// at different addresses.' ring_cursor/src/lib.rs

# which methods are const, and which is not
grep -nE '^\s*pub (const )?fn ' ring_publish/src/lib.rs

# the const/non-const constructor split three layers down
command grep -m1 -A15 -F '  /// let _ = PaddedCursor::new( Seq( 1 ) );' ring_cursor/src/lib.rs

# the two documented seams into this crate, and every real call site
grep -rn 'Claim::new\|claimer\.cursor()' */src/*.rs */tests/*.rs | grep -v '///'
grep -rn 'Claim::new\|claimer\.cursor()' ring_publish/

# what this crate actually uses from ring_claim
grep -rhoE 'claim(er)?\.[a-z_]+\(' ring_publish/tests/handshake_test.rs | sort | uniq -c

# the two address-level assertions in the suite
grep -rn 'core::ptr::eq' ring_publish/tests/*.rs
```

| | Value |
|--|------:|
| Fields on `Publisher` | 1, private |
| Generics, lifetimes, `PhantomData` | 0 |
| `size_of::< Publisher >()` | 64 |
| …that is state | 8 |
| …that is padding | **56 (87.5%)** |
| Crates in the type stack | 4 |
| Public methods | 6 |
| …that are `const` | **1** — and it is an accessor |
| Cursors in the handshake | 4 |
| …shared across a crate boundary | 2 |
| …this crate owns | 1 |
| …read by nobody | 1 — the claimed cursor |
| `core::ptr::eq` assertions in the suite | 2 |
| Documented `ring_claim` → `ring_publish` seams | 2 |
| …with a real call site in this crate | **0** |
| Items this crate uses from `ring_claim` | 3 — `claim()` ×17, `start()` ×10, `len()` ×7 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB13 | `ring_publish` | n/a — observation | 64 bytes of struct for 8 bytes of state, and the padding is against what the *caller* puts next to it rather than anything inside `Publisher`; one publisher per ring makes the trade obviously right, which is why [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) rejected the identical type for a per-slot stamp array |
| PB14 | `ring_publish` | n/a — observation | The crate's one `const fn` is an accessor, not the constructor; `new` could be `const` in an ordinary build but would have to carry `ring_cursor`'s `cfg(loom)` duplication down into this crate to stay so |
| PB15 | `ring_publish` | n/a — coverage | A consumer holding a cursor the producer does not read passes every other test in the file and gates nothing; the guard is an address comparison because two cursors are value-equal until the first commit, which is exactly when a value assertion would run |
| PB16 | `ring_claim` | n/a — drift | `ring_claim` justifies the visibility of two public items by naming `ring_publish` as their consumer; this crate calls neither, and given the dependency direction never can |
