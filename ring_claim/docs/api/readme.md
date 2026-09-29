# api

Two types, fifteen methods, seventeen doctests, and — unusually for this family
— nothing on the surface whose result can be discarded without the compiler
saying so. Four items carry no `#[ must_use ]` and all four are covered by the
type they return, which is the kind of property that is true today and has
nothing checking it tomorrow.

The two files split by question. The first asks *what is on the surface and how
is it protected*, and ends up correcting the source's own account of how
`#[ must_use ]` behaves. The second asks *what does a caller get back*, and
finds the same two-variant retry contract written three times across two crates,
one of which cannot call the other.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Seventeen Items and Nothing That Drops Silently](001_seventeen_items_and_nothing_that_drops_silently.md) | CL11, CL12 — the whole surface with annotations and doctests, the four items covered by their return type, the `rustc` behaviour the source comment gets one word wrong, and the public constructor whose named beneficiary never calls it |
| 002 | [The Two Constructors of a Range](002_the_two_constructors_of_a_range.md) | CL13, CL14 — the two fallible signatures against the family's 39, the `BatchTooLarge`/`Full` retry contract stated three times, and the thirteen of seventeen items that touch no atomic |

### The Whole Surface

| Item | Line | `const` | `must_use` | Atomic |
|------|-----:|:-------:|:----------:|:------:|
| `Claim` | `:96` | — | ✔ *with a message* | — |
| `Claim::new` | `:118` | ✔ | via the type | — |
| `Claim::start` | `:131` | ✔ | ✔ | — |
| `Claim::end` | `:144` | ✔ | ✔ | — |
| `Claim::len` | `:157` | ✔ | ✔ | — |
| `Claim::is_empty` | `:170` | ✔ | ✔ | — |
| `Claim::contains` | `:188` | ✔ | ✔ | — |
| `Claim::sequences` | `:205` | — | via `Iterator` | — |
| `Claim::overlaps` | `:226` | ✔ | ✔ | — |
| `Claimer` | `:257` | — | — | — |
| `Claimer::new` | `:276` | — | ✔ | — |
| `Claimer::cursor` | `:315` | ✔ | ✔ | — |
| `Claimer::consumers` | `:331` | ✔ | ✔ | — |
| `Claimer::claimed` | `:354` | — | ✔ | load |
| `Claimer::headroom` | `:382` | — | ✔ | load |
| `Claimer::claim` | `:421` | — | via `Result` | compare-exchange |
| `Claimer::claim_up_to` | `:481` | — | via `Result` | compare-exchange |

Nine items belong to a pure value type, eight to the concurrent one, and only
four of the seventeen ever issue an atomic instruction.

### The Contract, in Two Sentences

| Return | Means | Caller should |
|--------|-------|---------------|
| `Ok( Claim )` | these sequences are yours; write them and publish | write, then publish — never drop |
| `Err( BatchTooLarge { .. } )` | no ring this size can ever satisfy this | stop; fix the configuration |
| `Err( Full )` | no room *right now* | wait however this caller waits, then retry |

The second and third are what `ring_types::RingError::is_configuration` and
`is_transient` exist to distinguish, and getting them backwards produces either
an infinite loop or a spurious fatal error.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the whole surface, with its annotations
grep -E '^\s*(pub (const )?(fn|struct)|#\[ must_use)' ring_claim/src/lib.rs

# doctest fences — expect one pair per public item
grep -c '/// ```' ring_claim/src/lib.rs

# every fallible public signature in the family, grouped by error type
grep -rhE '^\s*pub (const )?fn .*-> *Result<' ring_*/src/*.rs \
  | sed 's/.*-> *//' | sed 's/.*, *//;s/ *>.*//' | sort | uniq -c | sort -rn

# every messaged must_use in the family
command grep -r '#\[ must_use = ' ring_*/src/*.rs

# every Claim::new call site, family-wide
command grep -r 'Claim::new(' */src/*.rs */tests/*.rs | command grep -v BatchClaim

# what the named beneficiary actually imports
command grep -r 'ring_claim' ring_publish/tests/*.rs
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
34
     21 RingError
      4 WorkloadError
      3 Violation
      3 T
      3 Anomaly
      2 BuildError
      1 Seq
      1 RunError
      1 Refusal< T
ring_atomic/src/lib.rs:  #[ must_use = "the returned sequence is the claim — dropping it claims a range nobody will use" ]
ring_claim/src/lib.rs:#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_flush/src/lib.rs:#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
ring_shutdown/src/lib.rs:  #[ must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else" ]
ring_shutdown/src/lib.rs:  #[ must_use = "this is the record itself, not a copy — dropping it loses it" ]
ring_shutdown/src/lib.rs:#[ must_use = "a Wake::Closed means stop, not publish" ]
ring_slot/src/lib.rs:  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
ring_spsc/src/lib.rs:#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again" ]
ring_testkit/src/lib.rs:  #[ must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed" ]
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

| | Value |
|--|------:|
| Public items | 17 — two types, fifteen methods |
| Modules | 0 — everything at the crate root |
| Public fields | 0 |
| Doctests | 17 — one per item |
| `const fn` | 9 |
| Items carrying an explicit `#[ must_use ]` | 12 |
| …covered instead by their return type | 4 |
| …whose result can drop silently | **0** |
| Messaged `must_use` in the family | 12 |
| …in this crate | 1 |
| Items that issue an atomic instruction | **4** of 17 |
| `unsafe` blocks | 0 |
| `std::` paths | 0 |
| Fallible signatures, family-wide | 39 |
| …returning `RingError` | 21 — including both of this crate's |
| `RingError` variants | 9 |
| …this crate can produce | 2 |
| `Claim::new` call sites, family-wide | 27 |
| …outside this crate | **0** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL11 | `ring_claim` | n/a — unenforced | No item's result can be discarded silently; the four without an explicit `#[ must_use ]` are each covered by their return type, and nothing checks that a future method keeps the property |
| CL12 | `ring_claim` | n/a — drift | `Claim::new` is public "because `ring_publish` … need[s] to construct one directly", and all 27 of its call sites are inside this crate; `ring_publish`'s reached-test imports `Claimer` and calls `Claimer::new` ten times |
| CL13 | `ring_claim` | n/a — duplication | Two variants and the distinction is a retry instruction; the same contract is stated three times, one of them in `GatingSet::check`'s `# Errors` section in almost identical words, for a function this crate cannot call |
| CL14 | `ring_claim` | n/a — observation | Thirteen of seventeen public items perform no atomic operation, and the type that names the crate performs none at all; that split is why six single-threaded tests cover the range arithmetic and five `thread::scope` blocks cover two methods |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| The source's stated reason for omitting the attribute on `Claim::new` — that it would *shadow* the type's message — is one word off: `rustc` emits both diagnostics, so the cost is a duplicate message-less warning printed alongside the good one, not a replacement | [001](001_seventeen_items_and_nothing_that_drops_silently.md) |
| The concession still has a live consumer — this crate's own value tests — but the cost it imposes is on the *other* crate, whose `pitfall/001` names the public constructor as what leaves `publish`'s precondition unenforceable | [001](001_seventeen_items_and_nothing_that_drops_silently.md) |
| The surface offers no `release`, no `Drop`, no blocking `claim`, no `WaitKind`, and no published-cursor accessor; the first two are refused for the same reason and the refusal is the crate's thesis | [001](001_seventeen_items_and_nothing_that_drops_silently.md) |
| Both signatures return `Result< Claim, RingError >` and differ only in a parameter *name* — `count` against `max` — which is the sole in-signature signal of which of two opposite contracts applies | [002](002_the_two_constructors_of_a_range.md) |
| It is also why `Claim` is `Copy` and `Claimer` cannot be: copying a range costs nothing, copying a claimer would fork the cursor and grant every sequence twice | [002](002_the_two_constructors_of_a_range.md) |

