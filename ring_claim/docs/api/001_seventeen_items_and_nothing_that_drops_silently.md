# API: Seventeen Items and Nothing That Drops Silently

### Scope

- **Purpose**: Enumerate the complete public surface — two types, fifteen methods — with its annotations, doctests and call sites, and establish that no item on it can have its result discarded without a compiler diagnostic.
- **Responsibility**: List every item, account for each of the four that carry no `#[ must_use ]`, and check the crate's stated reason for exposing `Claim::new` against who actually calls it.
- **In Scope**: Every `pub` item, its annotation, its doctest, and its callers.
- **Out of Scope**: The two fallible signatures' error contract — see [`api/002`](002_the_two_constructors_of_a_range.md).

### The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*(pub (const )?(fn|struct)|#\[ must_use)' ring_claim/src/lib.rs
```

Live output:

```
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
pub struct Claim
  pub const fn new( start : Seq, len : usize ) -> Self
  #[ must_use ]
  pub const fn start( self ) -> Seq
  #[ must_use ]
  pub const fn end( self ) -> Seq
  #[ must_use ]
  pub const fn len( self ) -> usize
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  #[ must_use ]
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  #[ must_use ]
  pub const fn overlaps( self, other : Self ) -> bool
pub struct Claimer< 'a >
  #[ must_use ]
  pub fn new( consumers : &'a GatingSet ) -> Self
  #[ must_use ]
  pub const fn cursor( &self ) -> &PaddedCursor
  #[ must_use ]
  pub const fn consumers( &self ) -> &'a GatingSet
  #[ must_use ]
  pub fn claimed( &self ) -> Seq
  #[ must_use ]
  pub fn headroom( &self ) -> usize
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
```

| Item | Line | `const` | `must_use` | Doctest | Touches an atomic |
|------|-----:|:-------:|:----------:|:-------:|:-----------------:|
| `Claim` | `:96` | — | ✔ **with a message** | ✔ | — |
| `Claim::new` | `:118` | ✔ | — (see below) | ✔ | — |
| `Claim::start` | `:131` | ✔ | ✔ | ✔ | — |
| `Claim::end` | `:144` | ✔ | ✔ | ✔ | — |
| `Claim::len` | `:157` | ✔ | ✔ | ✔ | — |
| `Claim::is_empty` | `:170` | ✔ | ✔ | ✔ | — |
| `Claim::contains` | `:188` | ✔ | ✔ | ✔ | — |
| `Claim::sequences` | `:205` | — | — (see below) | ✔ | — |
| `Claim::overlaps` | `:226` | ✔ | ✔ | ✔ | — |
| `Claimer` | `:257` | — | — | ✔ | — |
| `Claimer::new` | `:276` | — | ✔ | ✔ | — |
| `Claimer::cursor` | `:315` | ✔ | ✔ | ✔ | — |
| `Claimer::consumers` | `:331` | ✔ | ✔ | ✔ | — |
| `Claimer::claimed` | `:354` | — | ✔ | ✔ | **✔** load |
| `Claimer::headroom` | `:382` | — | ✔ | ✔ | **✔** load, via the gate |
| `Claimer::claim` | `:421` | — | — (see below) | ✔ | **✔** compare-exchange |
| `Claimer::claim_up_to` | `:481` | — | — (see below) | ✔ | **✔** compare-exchange |

Seventeen items, seventeen doctests — one per item, no item without one and no
item with two — `#![ deny( missing_docs ) ]` at `:64`, nine
`const fn`, no `unsafe`, and no `std::` path anywhere in the file.

### CL11 — Four Items Carry No `#[ must_use ]`, and All Four Are Already Covered by Their Return Type

`ring_publish`'s equivalent census found exactly one item whose result can be
discarded without a diagnostic. This crate has none, and the reason is that each
of the four unannotated items returns a type that carries the annotation itself:

| Item | Returns | Where the annotation is |
|------|---------|-------------------------|
| `Claim::new` | `Claim` | the **type**, `:95` — with a message |
| `Claim::sequences` | `impl Iterator< Item = Seq >` | `Iterator` is `#[ must_use ]` in core |
| `Claimer::claim` | `Result< Claim, RingError >` | `Result` is `#[ must_use ]` in core |
| `Claimer::claim_up_to` | `Result< Claim, RingError >` | same |

`Claim::new`'s omission is deliberate and the source says so (`:115-117`):

> No `#[ must_use ]` here: `Claim` itself already carries one *with a message*,
> and a bare attribute on the constructor would only shadow it with a less
> informative warning.

The conclusion is right and the stated mechanism is not. `rustc` does not
shadow — it emits **both** diagnostics. Reproducible in a dozen lines:

```rust
#[ must_use = "TYPE LEVEL MESSAGE" ]
pub struct Claim;

impl Claim
{
  pub fn bare() -> Self { Self }
  #[ must_use ]
  pub fn annotated() -> Self { Self }
}

pub fn exercise()
{
  Claim::bare();       // 1 warning, carrying TYPE LEVEL MESSAGE
  Claim::annotated();  // 2 warnings — the same one, plus a message-less second
}
```

`cargo build` on that reports three warnings, not two: the type's message fires
on both call sites, and `annotated` additionally produces *"unused return value
of `Claim::annotated` that must be used"* with no note attached. So the cost of
the attribute is a duplicate, less informative diagnostic printed *alongside*
the good one — noisier than shadowing, and easier to start ignoring.

The omission is therefore correct for a reason one word away from the recorded
one, and the recorded one is checkable in under a minute by anyone who doubts
it. Worth stating precisely, because "the attribute would hide the message" and
"the attribute would double the output" lead to different conclusions if the
type's annotation is ever removed: under shadowing, removing it would silently
weaken the constructor; under duplication, removing it leaves the constructor
exactly as loud as it was.

So the crate's `must_use` coverage is complete by two different mechanisms, and
the twelve explicit annotations plus five type-level ones account for all
seventeen items. The property worth stating is the one that survives a refactor:
**a future method returning a plain value would break it, and nothing checks.**
`tests/manual/readme.md § C4` checks that the type's message exists and that no
`Drop` impl was added; it does not check that every item is covered.

### CL12 — `Claim::new` Is Public for a Reason That No Longer Holds

`:106-108` states the justification:

> Public because `ring_publish` and the test suites of both crates need to
> construct one directly; a producer obtains real claims from
> [`Claimer::claim`], which is the only path that establishes exclusivity.

`ring_publish` does not construct one. Every call site, family-wide:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r 'Claim::new(' */src/*.rs */tests/*.rs | command grep -v BatchClaim
command grep -r 'ring_claim' ring_publish/tests/*.rs
```

Live output:

```
ring_claim/src/lib.rs:/// let claim = Claim::new( Seq( 4 ), 3 );
ring_claim/src/lib.rs:  /// assert_eq!( Claim::new( Seq::ZERO, 0 ).len(), 0 );
ring_claim/src/lib.rs:  /// assert_eq!( Claim::new( Seq( 9 ), 2 ).start(), Seq( 9 ) );
ring_claim/src/lib.rs:  /// assert_eq!( Claim::new( Seq( 9 ), 2 ).end(), Seq( 11 ) );
ring_claim/src/lib.rs:  /// assert_eq!( Claim::new( Seq::ZERO, 5 ).len(), 5 );
ring_claim/src/lib.rs:  /// assert!( Claim::new( Seq( 3 ), 0 ).is_empty() );
ring_claim/src/lib.rs:  /// let claim = Claim::new( Seq( 4 ), 2 );
ring_claim/src/lib.rs:  /// let seen : Vec< u64 > = Claim::new( Seq( 2 ), 3 ).sequences().map( | s | s.0 ).collect();
ring_claim/src/lib.rs:  /// let first = Claim::new( Seq( 0 ), 4 );
ring_claim/src/lib.rs:  /// assert!( !first.overlaps( Claim::new( Seq( 4 ), 4 ) ), "adjacent, not overlapping" );
ring_claim/src/lib.rs:  /// assert!( first.overlaps( Claim::new( Seq( 3 ), 4 ) ) );
ring_claim/src/lib.rs:  /// assert!( !first.overlaps( Claim::new( Seq( 0 ), 0 ) ), "an empty claim covers nothing" );
ring_claim/src/lib.rs:        Ok( _ ) => return Ok( Claim::new( current, count ) ),
ring_claim/src/lib.rs:        Ok( _ ) => return Ok( Claim::new( current, granted ) ),
ring_claim/tests/claim_test.rs:  let claim = Claim::new( Seq( 4 ), 3 );
ring_claim/tests/claim_test.rs:  let empty = Claim::new( Seq( 5 ), 0 );
ring_claim/tests/claim_test.rs:  assert!( !empty.overlaps( Claim::new( Seq( 0 ), 100 ) ), "a zero-width range covers no slot" );
ring_claim/tests/claim_test.rs:  assert!( !Claim::new( Seq( 0 ), 100 ).overlaps( empty ), "and the check is symmetric" );
ring_claim/tests/claim_test.rs:      let claim = Claim::new( Seq( start ), len );
ring_claim/tests/claim_test.rs:  let first = Claim::new( Seq( 0 ), 4 );
ring_claim/tests/claim_test.rs:  let second = Claim::new( Seq( 4 ), 4 );
ring_claim/tests/claim_test.rs:          let a = Claim::new( Seq( a_start ), a_len );
ring_claim/tests/claim_test.rs:          let b = Claim::new( Seq( b_start ), b_len );
ring_claim/tests/claim_test.rs:  const A : Claim = Claim::new( Seq( 4 ), 4 );
ring_claim/tests/claim_test.rs:  const B : Claim = Claim::new( Seq( 6 ), 4 );
ring_claim/tests/claim_test.rs:  const C : Claim = Claim::new( Seq( 8 ), 4 );
ring_claim/tests/claim_test.rs:  let a = Claim::new( start, 4 );
ring_publish/tests/handshake_test.rs://! - the **claimed** cursor is private to [`ring_claim::Claimer`]; nobody reads it
ring_publish/tests/handshake_test.rs:  use ring_claim::Claimer;
ring_publish/tests/handshake_test.rs:  use ring_claim::Claimer;
ring_publish/tests/handshake_test.rs:  /// CL44 in `ring_claim/docs/pitfall/001_dropping_a_claim.md` describes this
ring_publish/tests/handshake_test.rs:  /// and could not host a reproduction: `ring_claim` has no dependency, normal
ring_publish/tests/handshake_test.rs:  /// that fails to arrive. This file can — `ring_claim` is one of its four
```

| Where | Sites | What they are |
|-------|------:|---------------|
| `ring_claim/src/lib.rs` | 2 | `:442` and `:493` — the two success arms of the loops |
| `ring_claim/src/lib.rs` doctests | 12 | one per `Claim` method, plus the type's own |
| `ring_claim/tests/claim_test.rs` | 13 | the pure-value tests, which need ranges without a ring, plus the three `const` bindings CL52's pin evaluates |
| `ring_publish/tests/handshake_test.rs` | **0** | it imports `Claimer` and calls `Claimer::new` ×11 |
| `ring_mpsc` | **0** | it never names `Claim` at all outside one doc sentence |

The named beneficiary uses the *other* constructor. `ring_publish`'s reached-test
builds a real `Claimer` over a real `GatingSet` and takes real claims, which is
what a handshake test should do — so the justification names a need the family
does not have.

The concession still has a live consumer: this crate's own value tests, which
assert `contains`, `overlaps` and `sequences` on ranges that never existed in a
ring. Those are genuinely easier to write against a constructor. But the reason
recorded in the source is the wrong one, and it matters because
`ring_publish/docs/pitfall/001` argues that the public constructor is precisely
what leaves `publish( start, len )`'s precondition unenforceable — a cost paid,
today, for two test modules rather than for the crate that was named.

### The Two Intra-Library Edges

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'self\.\(claimed\|headroom\|end\)(' ring_claim/src/lib.rs | grep -v '///'
```

Live output:

```
    seq.0 >= self.start.0 && seq.0 < self.end().0
    ( self.start.0..self.end().0 ).map( Seq )
      && other.start.0 < self.end().0
    self.consumers.headroom( self.claimed() )
    let mut current = self.claimed();
    let mut current = self.claimed();
```

| Caller | Callee | Why |
|--------|--------|-----|
| `Claim::contains` | `Claim::end` | containment is `>= start && < end` |
| `Claim::sequences` | `Claim::end` | the range's exclusive upper bound |
| `Claim::overlaps` | `Claim::end` ×2 | both ranges' ends |
| `Claimer::headroom` | `Claimer::claimed` | the gate needs a producer position |
| `Claimer::claim` | `Claimer::claimed` | the loop's initial `current` |
| `Claimer::claim_up_to` | `Claimer::claimed` | same |

Two hubs, `end` and `claimed`, and nothing else calls anything. `end` is the
`Claim` half's only derived value; `claimed` is the `Claimer` half's only read.
The surface is a fan-out from two methods.

### What the Surface Deliberately Omits

| Absent | Why |
|--------|-----|
| a `release` / `cancel` on `Claim` | rewinding the cursor would hand out sequences twice (`:44-49`) |
| a `Drop` impl | same reason ([`decisions/002`](../decisions/002_must_use_without_drop.md)) |
| a blocking `claim` | the wait strategy must stay above this crate (`:25-35`) |
| a `WaitKind` parameter | same |
| a published-cursor accessor | that is `ring_publish`'s cursor, and this crate does not depend on it |
| a `merge` / `split` on `Claim` | no caller has needed one; `claim_up_to` covers partial grants |

The first two are the ones a reader most expects and the crate most firmly
refuses. The refusal is the crate's thesis, not an omission.

### APIs

| File | Relationship |
|------|--------------|
| [002_the_two_constructors_of_a_range.md](002_the_two_constructors_of_a_range.md) | The two fallible signatures and their error contract |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_must_use_without_drop.md](../decisions/002_must_use_without_drop.md) | The messaged annotation the constructor's omission protects |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_eight_readings_of_a_range.md](../item/001_the_eight_readings_of_a_range.md) | `Claim`'s eight items, one at a time |
| [../item/002_the_seven_of_the_claimer.md](../item/002_the_seven_of_the_claimer.md) | `Claimer`'s seven, and which four touch an atomic |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_dropping_a_claim.md](../pitfall/001_dropping_a_claim.md) | What the `must_use` message warns about, and what it cannot prevent |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:94-100` | The type, its derives, and the messaged annotation |
| `ring_claim/src/lib.rs:104-120` | `Claim::new`'s justification and its deliberate lack of an attribute |
| `ring_claim/src/lib.rs:196-205` | `sequences`, covered by `Iterator`'s own annotation |
| `ring_claim/src/lib.rs:238-385` | `Claimer` and its five non-claiming items |
| `ring_publish/tests/handshake_test.rs:66,101,201` | The named beneficiary, using `Claimer::new` instead |
| `ring_publish/docs/pitfall/001_publishing_a_range_you_never_claimed.md` | The cost the public constructor imposes on the other half |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:42-123` | The six pure-value tests, all built on `Claim::new` |
| `tests/manual/readme.md § C4` | One messaged `must_use`, no `Drop` |
