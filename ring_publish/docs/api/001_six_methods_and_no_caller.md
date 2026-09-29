# API: Six Methods and No Caller

### Scope

- **Purpose**: Enumerate the crate's entire public surface, state what each item guarantees, and record that every one of them is called only from this crate's own tests.
- **Responsibility**: List the seven public items with their signatures and annotations, show where each is exercised, and account for the two methods that carry no `#[ must_use ]`.
- **In Scope**: Everything `pub` in `src/lib.rs`.
- **Out of Scope**: The `Result< Seq, Seq >` return shape — see [`api/002`](002_a_result_whose_error_is_not_an_error.md). Why no crate depends on this one — see [`integration/002`](../integration/002_the_two_crates_that_declined.md).

### The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*(pub (const )?(fn|struct)|#\[ must_use \])' ring_publish/src/lib.rs
```

Live output:

```
pub struct Publisher
  #[ must_use ]
  pub fn new() -> Self
  #[ must_use ]
  pub const fn cursor( &self ) -> &PaddedCursor
  #[ must_use ]
  pub fn published( &self ) -> Seq
  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  #[ must_use ]
  pub fn is_published( &self, seq : Seq ) -> bool
```

Seven items — one type and six methods on it, all at the crate root, no modules:

| Item | Line | Signature | `must_use` | Doctest |
|------|------|-----------|:----------:|:-------:|
| `Publisher` | `:85` | `struct Publisher { cursor : PaddedCursor }` | — | `:74-83` |
| `new` | `:100` | `fn new() -> Self` | ✔ `:99` | `:94-98` |
| `cursor` | `:117` | `const fn cursor( &self ) -> &PaddedCursor` | ✔ `:116` | `:107-115` |
| `published` | `:133` | `fn published( &self ) -> Seq` | ✔ `:132` | `:124-131` |
| `try_publish` | `:161` | `fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >` | — | `:148-160` |
| `publish` | `:200` | `fn publish( &self, start : Seq, len : usize ) -> Seq` | — | `:174-181` |
| `is_published` | `:229` | `fn is_published( &self, seq : Seq ) -> bool` | ✔ `:228` | `:217-227` |

Seven items, seven doctests, `#![ deny( missing_docs ) ]` at `:55`. The struct
holds exactly one field and it is private; `cursor()` is the only way out to it,
and it hands out a `&PaddedCursor` rather than a copy for a reason
`tests/publish_test.rs:48-50` states as a failure mode:

> The barrier a consumer builds is over *this* cursor, so a `cursor()` that
> returned a copy would give every consumer a frontier frozen at zero — a ring
> that compiles, runs, and never delivers anything.

`:57` asserts the identity directly with `core::ptr::eq`, which is the only
assertion in the suite that checks an address rather than a value.

### PB10 — Every Call Site of Every Method Is a Test in This Crate

No crate in the family depends on `ring_publish`
([`integration/001`](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) § PB1),
so the surface's entire exercised call graph is internal:

| Item | Callers in `src/` | Callers in `tests/` |
|------|-------------------|---------------------|
| `Publisher` / `new` / `default` | — | both suites, every test |
| `cursor` | — | `publish_test.rs:32,52,57`; the barrier wiring in `handshake_test.rs` |
| `published` | `is_published:231` | throughout both suites |
| `try_publish` | `publish:204` | `publish_test.rs` ×6, `handshake_test.rs` ×3 |
| `publish` | — | 21 sites across both suites |
| `is_published` | — | `publish_test.rs:33,196-200,210,230` |

Only two items have a caller inside the library at all — `published`, from
`is_published`, and `try_publish`, from `publish` — and both of those are one
method calling its own simpler sibling. Nothing here is dead: every item is
covered, every doctest runs, and the four-crate handshake exercises the whole
surface end to end. What is absent is a *consumer*, and that absence is a ruled
outcome rather than an oversight
([`integration/002`](../integration/002_the_two_crates_that_declined.md) § PB4).

The practical consequence is that this crate's contract has never been tested by
the thing that actually tests contracts: somebody else trying to use it and
finding a corner it does not cover. `handshake_test.rs` comes closest — it wires
four crates together and did force `ring_barrier` to change a signature
(`ring_barrier/src/lib.rs:40-44`) — but it was written by the same hand,
in this crate, against this crate's own understanding of the handshake.

### PB11 — The One Return Value That Can Be Dropped Is Derivable From Its Own Arguments

Four of the six methods carry `#[ must_use ]`. The two that do not are
`try_publish` and `publish`, and they differ:

- **`try_publish` needs none.** It returns `Result`, which is `#[ must_use ]` on
  the type. Dropping it warns already.
- **`publish` returns a bare `Seq`**, which drops silently. It is the only item
  on this surface whose result can vanish without a diagnostic.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r '\.publish(' ring_publish/src/lib.rs ring_publish/tests/*.rs \
  | grep -v '///'
```

Live output:

```
ring_publish/tests/handshake_test.rs:          publisher.publish( claim.start(), claim.len() );
ring_publish/tests/handshake_test.rs:          publisher.publish( lap.start(), lap.len() );
ring_publish/tests/handshake_test.rs:          publisher.publish( seq, claim.len() );
ring_publish/tests/handshake_test.rs:    publisher.publish( first.start(), first.len() );
ring_publish/tests/handshake_test.rs:    publisher.publish( second.start(), second.len() );
ring_publish/tests/handshake_test.rs:    publisher.publish( claim.start(), claim.len() );
ring_publish/tests/handshake_test.rs:      publisher.publish( claim.start(), claim.len() );
ring_publish/tests/handshake_test.rs:          publisher.publish( seq, claim.len() );
ring_publish/tests/handshake_test.rs:            publisher.publish( seq, claim.len() );
ring_publish/tests/handshake_test.rs:    publisher.publish( claim.start(), claim.len() );
ring_publish/tests/publish_test.rs:  publisher.publish( Seq::ZERO, 3 );
ring_publish/tests/publish_test.rs:  publisher.publish( Seq::ZERO, 8 );
ring_publish/tests/publish_test.rs:  publisher.publish( Seq::ZERO, 5 );
ring_publish/tests/publish_test.rs:  assert_eq!( publisher.publish( Seq::ZERO, 3 ), Seq( 3 ) );
ring_publish/tests/publish_test.rs:  assert_eq!( publisher.publish( Seq( 3 ), 1 ), Seq( 4 ) );
ring_publish/tests/publish_test.rs:  assert_eq!( publisher.publish( Seq( 4 ), 6 ), Seq( 10 ) );
ring_publish/tests/publish_test.rs:      publisher.publish( Seq( 4 ), 4 )
ring_publish/tests/publish_test.rs:    assert_eq!( publisher.publish( Seq::ZERO, 4 ), Seq( 4 ), "A goes first" );
ring_publish/tests/publish_test.rs:      scope.spawn( move || publisher.publish( start, WIDTH as usize ) );
ring_publish/tests/publish_test.rs:  publisher.publish( Seq::ZERO, 3 );
ring_publish/tests/publish_test.rs:    publisher.publish( Seq( frontier ), 1 );
```

Twenty-one call sites. **Fifteen** are statements ending in `;` that discard the
value outright; five bind or assert it (`publish_test.rs:121-123,139,150`); one
(`:176`) is a closure tail whose `JoinHandle` is dropped unjoined.

That looks like a missing annotation and is not one. `publish( start, len )`
returns `start.advanced_by( len as u64 )` — a value the caller computed the
inputs to and can recompute for free. Discarding it is not losing information;
it is declining to be handed back an argument. A `#[ must_use ]` here would fire
on fifteen correct call sites and teach callers to write `let _ =`.

The comparison that makes this concrete is the sibling with the same shape:

| | `publish -> Seq` | `try_publish -> Result< Seq, Seq >` |
|--|------------------|-------------------------------------|
| `Ok`/success payload | `start + len` — derivable | `start + len` — derivable |
| Failure payload | none — cannot fail | `published` — **not** derivable |
| Silently droppable | yes | no (`Result` is `must_use`) |
| Should it be `must_use` | no | already is, by its type |

Both success payloads are derivable; the annotation difference tracks the *error*
payload, which is the only genuinely new information either method returns. The
crate ends up correctly annotated by a route that looks accidental, and it is
worth writing down which route, because a future `publish` that grew a failure
mode would need the annotation and nothing would notice its absence.

For contrast, the family's `#[ must_use ]` density puts this crate at 4/6 = 0.66,
mid-range against `ring_cursor` and `ring_seqno` at 1.00 and `ring_wait` at 1/6.
Density is not the story here — *which* two lack it is.

### What the Surface Deliberately Does Not Offer

| Absent | Why |
|--------|-----|
| A `Publisher::with_start( Seq )` | The frontier starts at zero and advances; a constructor that starts elsewhere would make `is_published` answer `false` for published sequences below it |
| A way to *retreat* the frontier | `try_publish` refuses a start behind it ([`invariant/001`](../invariant/001_the_frontier_moves_only_by_compare_exchange.md)); a reset would un-publish slots a consumer may be reading |
| A `publish_up_to` / highest-contiguous variant | [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) — deferred to `ring_mpsc`, and checked absent by `tests/manual/readme.md § P4` |
| A `WaitKind` parameter on `publish` | [`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) — the wait is bounded by a committed predecessor, so a strategy would offer a `Park` that can only hurt |
| Any `RingError` | [`api/002`](002_a_result_whose_error_is_not_an_error.md) — the crate names no error variant and imports no error type |
| A claimed-cursor accessor | This crate does not know about claims; that cursor is `ring_claim`'s, and keeping them apart is the point ([`pitfall/002`](../pitfall/002_conflating_the_two_cursors.md)) |

The last row is the one a reader is most likely to want and least likely to get.
A `Publisher` that could see the claimed cursor could detect
[`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md)'s hang —
and would be a `Publisher` that no longer works for a caller whose claims come
from somewhere else.

### PB48 — Five Receivers, All `&self`, and No `&mut self` in the Crate

```sh
cd "$(git rev-parse --show-toplevel)"
# every receiver in this crate's public surface
grep -E 'pub (const )?fn ' ring_publish/src/lib.rs
# not one takes &mut self …
grep -c '&mut self' ring_publish/src/lib.rs || true
# … control: the identical expression, counting the crates that do
grep -lE 'pub (const )?fn [a-z_]*\( *&mut self' ring_*/src/lib.rs | wc -l
```

Live output:

```
  pub fn new() -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub fn published( &self ) -> Seq
  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  pub fn is_published( &self, seq : Seq ) -> bool
0
14
```

Six public functions: `new` takes no receiver and the other five take `&self`.
Not one `&mut self` anywhere in the crate, against twelve of the thirty-three
`ring_*` crates that carry at least one.

That is the property that lets a `Publisher` be shared rather than owned. Every
producer needs the same publisher, and a single `&mut self` method on the type
would force callers into an `Arc< Mutex< Publisher > >` — re-introducing, in the
wrapper, precisely the serialisation the compare-exchange exists to avoid. The
whole mutation surface is behind `&self`, through the atomic, which is what
[`data_structure/001`](../data_structure/001_one_padded_cursor_and_nothing_else.md)'s
single `PaddedCursor` field buys.

The load-bearing half of that argument is `Sync`, and
[`type/002`](../type/002_two_derives_and_the_ones_that_are_absent.md) records
that nothing asserts it. So the receiver shape is checked by the compiler at
every call site, and the property the shape exists to support is checked
nowhere. A future field that is `Send` but not `Sync` would leave all six
signatures intact, break sharing, and be caught by whichever downstream crate
first tried to put a publisher in an `Arc` — which, per this instance's own
finding about callers, does not exist yet.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | What `try_publish` does |
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | What `publish` adds to it |

### APIs

| File | Relationship |
|------|--------------|
| [002_a_result_whose_error_is_not_an_error.md](002_a_result_whose_error_is_not_an_error.md) | The return type this surface is built around |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The single private field `cursor()` hands out |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | The dependent count that makes "no caller" a graph fact |
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | Why that count is zero |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_readings_of_the_cursor.md](../item/001_the_three_readings_of_the_cursor.md) | `cursor`, `published`, `is_published` — the read half |
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | `try_publish` and `publish` — the write half |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The caller error this surface cannot detect |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_two_derives_and_the_ones_that_are_absent.md](../type/002_two_derives_and_the_ones_that_are_absent.md) | What `Publisher` derives, and what it deliberately does not |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:55,84-232` | `deny( missing_docs )`, the struct, and all six methods with their doctests |
| `ring_barrier/src/lib.rs:40-44` | The signature this crate's handshake test forced open |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:1-17` | What the doc examples do not cover, stated by the suite that covers it |
| `tests/publish_test.rs:26-58` | Construction, and that `cursor()` hands out the live cell |
| `tests/publish_test.rs:116-124` | `publish` returns the end of what it published |
| `tests/handshake_test.rs:1-50` | The whole surface exercised as one of four participants |
