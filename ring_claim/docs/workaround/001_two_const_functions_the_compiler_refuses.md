# Workaround: Two Const Functions the Compiler Refuses

### Scope

- **Purpose**: Record the language constraint that cost `Claim` two `const fn`s until this finding was acted on, the four-character workaround that escapes it, and the receiver choice that is a second, independent workaround pointing the other way.
- **Responsibility**: Establish which of the two crates paid which cost, prove the constraint and both escapes against the compiler rather than asserting them, and show that the combination neither crate had taken was available and free — which is why `ring_claim` then took it.
- **In Scope**: `E0015` on `Seq`'s derived `PartialOrd`, edition-2024 RPIT lifetime capture, and the `.0` and `use< >` escapes.
- **Out of Scope**: Workarounds for the project's own conventions rather than the language's — see [`workaround/002`](002_the_constructor_that_exists_for_the_test_directory.md).

### The Constraint

`Claim::contains` and `Claim::overlaps` were the only two methods on the type
that could be `const` and were not. Both are `const` today — this finding was
acted on and the escape below was applied — so the recipe now prints one
non-`const` method where it once printed three:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^  pub (const )?fn' ring_claim/src/lib.rs | sed -n '1,8p'
```

Live output:

```
  pub const fn new( start : Seq, len : usize ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> usize
  pub const fn is_empty( self ) -> bool
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn overlaps( self, other : Self ) -> bool
```

| Line | Method | `const` |
|-----:|--------|:-------:|
| 118 | `new` | ✔ |
| 131 | `start` | ✔ |
| 144 | `end` | ✔ |
| 157 | `len` | ✔ |
| 170 | `is_empty` | ✔ |
| 188 | `contains` | **✔** — was ✘ |
| 205 | `sequences` | ✘ — returns an iterator; inherently cannot be |
| 226 | `overlaps` | **✔** — was ✘ |

Seven of eight. `sequences` is not a candidate — it builds a `.map( Seq )`
closure, and no version of Rust makes that `const`. So the real score is **seven
of seven achievable**. It was five of seven when this was written, and the two
missing ones were missing for a reason the crate did not state anywhere — which
is why the fix that closed the gap carries that reason as a comment on both
methods.

Adding `const` to `contains` reproduces it exactly. Against a minimal standalone
crate rather than the workspace, so the mechanism is visible without the family's
other types in scope — rustc 1.97.1, its own line numbers:

```
error[E0015]: cannot call non-const operator in constant functions
15 |     seq >= self.start && seq < self.end()
   |     ^^^^^^^^^^^^^^^^^
note: impl defined here, but it is not `const`
 1 | #[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord ) ]
   |                                               ^^^^^^^^^^
   = note: calls in constant functions are limited to constant functions,
           tuple structs and tuple variants
```

Two errors, one per comparison. `overlaps` has two more of its own.

The note is the interesting half: the compiler blames a **derive on another
crate's type**. `Seq`'s `PartialOrd` is derived at `ring_types/src/id.rs:24`,
and a derived operator impl is not `const`, so every `<` or `>=` between two
`Seq`s is a non-const call. `ring_claim` cannot fix this in `ring_claim` — the
constraint is imported along with the type, and the diagnostic a developer sees
points two crates away at a line they did not write.

### CL51 — A Sibling Crate Escapes With Four Characters, In Bodies Otherwise Identical

`ring_batch::BatchClaim` is the same shape as `Claim` — a `Seq` and a count, the
same eight methods, the same purpose one tier over. Its `contains` and
`overlaps` **are** `const`, and `ring_claim`'s were not. Set the two pairs as
they stood then side by side — the `ring_claim` column is the pre-fix body,
kept because it is what the E0015 above was reproduced against, and its
addresses are the pre-fix ones:

```rust
// ring_claim:188 pre-fix               ring_batch:145
pub fn contains( self, seq : Seq )      pub const fn contains( &self, seq : Seq )
{                                       {
  seq >= self.start                       seq.0 >= self.start.0
    && seq < self.end()                     && seq.0 < self.end().0
}                                       }
```

```rust
// ring_claim:223 pre-fix               ring_batch:185
pub fn overlaps( self, other : Self )   pub const fn overlaps( &self, other : &Self )
{                                       {
  !self.is_empty() && !other.is_empty()   !self.is_empty() && !other.is_empty()
    && self.start < other.end()             && self.start.0 < other.end().0
    && other.start < self.end()             && other.start.0 < self.end().0
}                                       }
```

`ring_claim:188` and `:226` now read like the right-hand column — same `.0`
comparisons, same `const`, still a by-value receiver.

Line for line the same predicate. The entire difference is `.0` — reaching
through `Seq`'s public field to compare two `u64`s, which is a primitive
comparison and therefore const-callable, instead of two `Seq`s, which is a
derived trait impl and therefore not.

Verified directly rather than inferred — both bodies in one `const fn`, same
file, same compiler:

| Body | Result |
|------|--------|
| `seq >= self.start && seq < self.end()` | **E0015 ×2** |
| `seq.0 >= self.start.0 && seq.0 < self.end().0` | **compiles** |

So this is a real workaround with a real cost, not a style difference. `.0`
discards the newtype for the length of the expression: inside those two
comparisons `ring_batch` is doing `u64` arithmetic with no type distinguishing a
sequence from a count from a slot index. That is exactly the confusion
[`type/001`](../type/001_a_seq_a_usize_and_three_casts.md) records the family
using `Seq` to prevent, and it is available only because `Seq`'s field is `pub`
— the same choice that lets any crate build a nonsense `Seq`.

Across the family's three range types, the ranking is unambiguous:

| Type | Const | Achievable | Escape used |
|------|------:|-----------:|-------------|
| `ring_batch::BatchClaim` | 7 of 8 | 7 | `.0` |
| `ring_consume::Available` | 5 of 6 | 5 | none needed — no `contains`/`overlaps` to lose |
| `ring_claim::Claim` | 7 of 8 | **7** | `.0` — adopted after this finding |

`ring_claim` was the only one of the three leaving anything on the table, and it
was the crate where `const` mattered most: `contains` and `overlaps` are the
two methods the exhaustive 900-pair test calls, and the only two a caller could
plausibly want in a `const` assertion about a fixed ring layout. That is the
reason this finding was acted on rather than accepted as a cost.

Neither crate's source mentions the other's choice, and that has not changed.
What has: `ring_batch` still does not say why it reaches through the newtype,
while `ring_claim` now does — both methods carry a comment naming `PartialOrd`'s
unavailability in a `const fn` as the reason they compare `.0`.

### CL52 — The Second Escape Points the Other Way, and Nobody Took Both

`ring_batch` pays for its `const` in a second currency, and this one *is*
visible in the signature:

```rust
pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >   // ring_batch:162
pub fn sequences( self ) -> impl Iterator< Item = Seq >             // ring_claim:205
```

`+ use< >` is edition 2024's opt-out from implicit RPIT lifetime capture. In
edition 2024 an `impl Trait` return captures every lifetime in scope by default,
so a method taking `&self` returns an iterator artificially bound to that borrow
— even when the body copies two integers and borrows nothing. `use< >` says
"capture nothing," restoring the truth.

The failure it prevents is the one worth recording, because it is invisible
where it is caused:

```sh
cd "$(git rev-parse --show-toplevel)"
# the annotation as shipped, and the crate carrying it building clean -- which
# is the whole point, because the cost of omitting it is not payable here
command grep -c 'use< >' ring_batch/src/lib.rs
cargo build -p ring_batch 2>&1 | command grep -cE '^warning|^error'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
2
0
```

Omitting `use< >` is **not a local error**, which is why nothing above can
measure its absence: `ring_batch` builds clean either way. The diagnostic has
this shape, and it arrives in a *different crate* — at a call site that tried to
keep the iterator longer than the value it came from, with only a note pointing
back at the signature (illustrative, not captured; no consumer in this workspace
holds one long enough to provoke it):

```
error[E0597]: `c` does not live long enough
  --> tests/t.rs:7:49
  --> src/lib.rs:13:35        note: the signature that caused it
```

A crate can ship this and never see it.

The family's whole population of iterator-returning methods splits with no
exceptions:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -rE 'fn [a-z_]+\( *(&?self|&?mut self)[^)]*\) *-> *impl Iterator' ring_*/src/*.rs
```

Live output:

```
ring_batch/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
ring_claim/src/lib.rs:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
ring_consume/src/lib.rs:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
ring_mpsc/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< S >
ring_mpsc/src/lib.rs:  pub fn iter( &self ) -> impl Iterator< Item = &S >
ring_registry/src/lib.rs:  pub fn names( &self ) -> impl Iterator< Item = &str >
ring_spsc/src/lib.rs:  pub fn iter( &self ) -> impl Iterator< Item = &S >
ring_tls/src/lib.rs:  pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_
```

| Method | Receiver | Yields | `use<…>` | Why |
|--------|----------|--------|:--------:|-----|
| `ring_batch:162` `sequences` | `&self` | owned `Seq` | ✔ `use< >` | needs the opt-out |
| `ring_mpsc:1195` `sequences` | `&self` | owned `Seq` | ✔ `use< S >` | needs it, keeping `S` |
| `ring_claim:205` `sequences` | **`self`** | owned `Seq` | ✘ | nothing to capture |
| `ring_consume:182` `sequences` | **`self`** | owned `Seq` | ✘ | nothing to capture |
| `ring_registry:232` `names` | `&self` | `&str` | ✘ | capture is **wanted** |
| `ring_mpsc:1251` `iter` | `&self` | `&S` | ✘ | capture is **wanted** |
| `ring_spsc:1124` `iter` | `&self` | `&S` | ✘ | capture is **wanted** |

Seven methods, three groups, zero exceptions: the annotation appears exactly
where a `&self` receiver yields owned values, and is correctly absent both where
the receiver is by value and where the items genuinely borrow. One further
`use< >` in the family sits outside this census by construction — `ring_batch`'s
free function `drain_order`, which takes `&BatchClaim` rather than a receiver
and so is not matched by a pattern anchored on `self`.

Each crate escaped one constraint and paid the other, until this finding was
acted on and `ring_claim` stopped paying:

| | `ring_claim` | `ring_batch` |
|--|--------------|--------------|
| Receiver | `self` (16-byte `Copy`) | `&self` |
| Needs `use< >` | **no** | **yes** |
| Comparison | `.0` raw `u64`, in the two predicates | `.0` raw `u64` |
| `const fn` | **7 of 7** | **7 of 7** |
| Newtype held through the predicate | no | no |

**The two choices were independent, and the combination that costs nothing was
the one neither crate had made.** `self`-by-value with `.0` comparisons is
`const`, needs no `use< >`, and takes no reference to a `Copy` type. Proven, not
argued — the probe above that compiled `contains_raw` as `const` took `self` by
value, and `contains` and `overlaps` now take that same route.

`ring_claim` had the better half of the combination and did not know it: its
by-value receiver is why it is the one crate in the table that needs no
edition-2024 escape hatch at all. Two `.0` pairs on two lines bought it the
other half. The price was the newtype discipline inside four comparisons, in
methods that already reached through `Seq`'s public field in `sequences`
(`self.start.0..self.end().0`, `:207`) three lines up — which is why the honest
summary was always that **the crate already used the workaround, once, in the
one method that cannot be `const` anyway.**

**Disposition:** applied — `contains` and `overlaps` are now `const fn`
comparing `.0`, each carrying a comment naming `PartialOrd`'s unavailability in
a `const fn` as the reason it reaches through the newtype. Both are pinned by
`claim_predicates_answer_in_a_const_context` in `tests/claim_test.rs`, which
evaluates them inside `const { assert!( … ) }` blocks so a regression is a
compile error rather than a silent loss, and repeats each assertion against a
`std::hint::black_box` value so the test is not vacuous at runtime. Now prints:
`pub const fn overlaps( self, other : Self ) -> bool`

### Workarounds

| File | Relationship |
|------|--------------|
| [002_the_constructor_that_exists_for_the_test_directory.md](002_the_constructor_that_exists_for_the_test_directory.md) | The other kind of constraint — the project's own conventions, not the language's |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_three_casts.md](../type/001_a_seq_a_usize_and_three_casts.md) | Why `Seq`'s public field exists, and what else it buys and costs |
| [../type/002_the_lifetime_on_the_claimer.md](../type/002_the_lifetime_on_the_claimer.md) | The other place a lifetime's reach is decided in a signature and documented nowhere |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_eight_readings_of_a_range.md](../item/001_the_eight_readings_of_a_range.md) | The eight methods, and `ring_batch`'s two extra `const fn`s as an item-level difference |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_half_open_range_as_a_value.md](../pattern/002_the_half_open_range_as_a_value.md) | The three range types this compares, and what else they disagree about |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | Why a `&self` receiver on a 16-byte `Copy` type buys nothing |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:188,205,226` | The two methods this finding moved to `const`, and the one that inherently cannot be |
| `ring_batch/src/lib.rs:145,162,185` | The same three, `const` where they can be, with both escapes |
| `ring_types/src/id.rs:24` | The derived `PartialOrd` the E0015 note blames |
| `ring_consume/src/lib.rs:182` | The third range type, by-value receiver, no escape needed |
| `ring_mpsc/src/lib.rs:1195, 1251` | `use< S >`, and a case where capture is correctly kept |
| `ring_registry/src/lib.rs:232` | The other case where capture is correctly kept |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:112` — `overlap_is_symmetric_and_detects_every_shared_sequence` | 900 pairs through the two methods this finding moved to `const` |
| `tests/claim_test.rs:55` — `a_claim_is_half_open` | `contains` at a call site that a `const` assertion could replace |
| `tests/claim_test.rs:601` — `claim_predicates_answer_in_a_const_context` | The gap this finding left behind, now closed: four `const { assert!( … ) }` blocks make a regression a compile error |
