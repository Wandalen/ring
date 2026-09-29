# Type: The Ring That Can Gate Against Itself

### Scope

**Purpose:** Record that `claim_gated`'s two cursor parameters are unrelated in
the type system, that passing the same cell for both type-checks and grants every
request, and how the rest of the family avoided the same shape.

**Responsibility:** `claim_gated< P : SeqCell, C : SeqCell >`'s signature, the
`free_slots` arithmetic it feeds, and the family's other ways of holding both
ends of a ring.

**In Scope:** `ring_batch/src/lib.rs:306-314`;
`ring_seqno/src/lib.rs:81-83`, `:90`, `:95-99`.

**Out of Scope:** The gate's race against concurrent producers is
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md).
The gated/ungated split itself is
[`pattern/002`](../pattern/002_two_functions_where_one_would_have_hidden_it.md).

---

## Two Parameters, No Relationship

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- two independent parameters, two independent references --'
command grep -m1 -A8 -F 'pub fn claim_gated< P : SeqCell, C : SeqCell >' ring_batch/src/lib.rs
echo '  -- and the arithmetic they feed --'
command grep -m1 -A2 -F '/// Zero when the ring is full; never negative, because a producer that appears' ring_seqno/src/lib.rs
command grep -m1 -A4 -F 'pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize' ring_seqno/src/lib.rs
echo '  -- which the doctest states outright --'
command grep -m1 -F '/// assert_eq!( free_slots( Seq( 0 ), Seq( 0 ), cap ), 4 );' ring_seqno/src/lib.rs
```

Live output:

```
  -- two independent parameters, two independent references --
pub fn claim_gated< P : SeqCell, C : SeqCell >
(
  producer : &P,
  consumer : &C,
  count : usize,
  capacity : Capacity,
  order : Ordering,
)
-> Result< BatchClaim, RingError >
  -- and the arithmetic they feed --
/// Zero when the ring is full; never negative, because a producer that appears
/// to be behind its consumer is a state the family's monotonic sequences
/// exclude.
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
  -- which the doctest states outright --
/// assert_eq!( free_slots( Seq( 0 ), Seq( 0 ), cap ), 4 );
```

`P` and `C` are two type parameters with one shared bound and nothing connecting
them. `&P` and `&C` are two references with nothing connecting them either. The
signature's only statement about its two cursors is that both can hold a `Seq`.

---

### BA46 — A Ring Gated Against Itself Is Never Full

If the same cell arrives as both arguments, `at` and `behind` are the same value,
`consumer.distance_to( producer )` is zero, and `free_slots` returns the whole
capacity — every time, at every cursor position. Measured:

```
--- and the signature that lets one cell be both sides ---
  a ring of 4, gated against itself, 6 claims of 1 : 6 granted, 0 refused
  cursor now at Seq(6), on a ring whose capacity is 4
```

**Finding.** `claim_gated( &cell, &cell, 1, Capacity::new( 4 ).unwrap(), AcqRel )`
compiles, runs, and returns `Ok` on every call forever. Six claims of one on a
ring of four; the fourth, fifth and sixth should have been `Err( RingError::Full )`
and the cursor should have stopped at `Seq( 4 )`.

`free_slots` is not wrong. Its doc comment says the "never negative" property
comes from a producer never appearing to be behind its consumer — which is a true
statement about monotonic sequences and says nothing about the two being the same
sequence. Its own doctest, `free_slots( Seq( 0 ), Seq( 0 ), cap ) == 4`, is the
aliased case written down as correct behaviour, because for two genuinely
distinct cursors at the same position it *is* correct: an empty ring has all its
slots free.

The defect is not in the arithmetic. It is that `claim_gated`'s signature admits
an input the arithmetic was never meant to receive, and neither function checks
because neither can — nothing at this tier can distinguish two cursors that
happen to be equal from one cursor passed twice.

The "one sentence in the doc comment" BA47 below names as the remedy is now
there:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '/// # `producer` and `consumer` must be distinct' ring_batch/src/lib.rs
```

Live output:

```
/// # `producer` and `consumer` must be distinct
///
/// `P` and `C` share one bound and nothing else connects them; nothing in
/// this signature stops the same cell from being passed as both. Doing so
```

**Disposition:** applied — added a `# `producer` and `consumer` must be
distinct` section to `claim_gated`'s doc comment in `src/lib.rs`, stating the
contract in words rather than in the type system, per BA47's own conclusion
that no `S : SeqCell` substitution would close this without the dependency
the crate is deliberately avoiding. Does not add a runtime check — a
same-cell comparison would need `PartialEq` on `C`/`P`, which `SeqCell`
implementors are not required to provide, and would not catch two distinct
`SeqCell`s wrapping the same underlying storage either, so a check would be
partial coverage sold as complete. The crate's 21 unit tests plus 10
doctests re-verified passing (`cargo test --all-features`, 2026-09-04). Now
prints: `this signature stops the same cell from being passed as both`

---

### BA47 — The Only Function in the Family That Can Be Handed One Ring End Twice

How every other crate holds both ends:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- both ends as two separate arguments --'
command grep -rn -A1 'producer : &' --include=lib.rs ring_*/src/ | command grep -B1 'consumer : &' | sed 's|ring/||; s|\.rs[:-][0-9]*[:-]|.rs:  |' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- both ends through one aggregate --'
command grep -rn 'consumers : &.*GatingSet\|ring : &.*Ring< S >' --include=lib.rs ring_*/src/ | command grep -v '///' | sed 's|ring/||; s|\.rs:[0-9]*:|.rs:  |' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- both ends as two separate arguments --
ring_batch/src/lib.rs:    producer : &P,
ring_batch/src/lib.rs:    consumer : &C,
--
ring_debug/src/lib.rs:    producer : &Producer< '_, T >,
ring_debug/src/lib.rs:    consumer : &Consumer< '_, T >,
  -- both ends through one aggregate --
ring_claim/src/lib.rs:    consumers : &'a GatingSet,
ring_claim/src/lib.rs:    pub fn new( consumers : &'a GatingSet ) -> Self
ring_mpsc/src/lib.rs:    ring : &'a Ring< S >,
ring_mpsc/src/lib.rs:    ring : &'a Ring< S >,
ring_mpsc/src/lib.rs:    ring : &'a Ring< S >,
ring_mpsc/src/lib.rs:    ring : &'a Ring< S >,
ring_mpsc/src/lib.rs:    ring : &'a Ring< S >,
ring_spsc/src/lib.rs:    ring : &'a Ring< S >,
ring_spsc/src/lib.rs:    ring : &'a Ring< S >,
ring_spsc/src/lib.rs:    ring : &'a Ring< S >,
ring_spsc/src/lib.rs:    ring : &'a Ring< S >,
```

**Finding.** Exactly two functions in the family take a producer and a consumer as
separate parameters, and only one of them can be handed the same object twice.
`ring_debug`'s takes `&Producer< '_, T >` and `&Consumer< '_, T >` — two distinct
concrete types, so the aliased call does not type-check. Every other crate holds
both ends through a single value: nine `ring : &'a Ring< S >` fields across the
two assemblies, and `Claimer`'s `consumers : &'a GatingSet` beside a cursor it
owns outright.

So the family converged on the structural answer and this crate is the one
exception, for a reason that is sound at its tier: `ring_batch` has no storage
dependency by design, so there is no `Ring< S >` for it to borrow and no
`GatingSet` it is allowed to know about. Two loose cursors is what remains.

That does not make the hazard disappear, and the remedy does not require the
dependency the crate is avoiding. A single `S : SeqCell` for both would not help
— `claim_gated( &cell, &cell, .. )` would still compile. What is missing is one
sentence in the doc comment saying the two cursors must be distinct ends of one
ring, which is the kind of contract a Tier 2 crate states rather than enforces.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | The other way this gate grants room that is not there |
| [`pattern/002`](../pattern/002_two_functions_where_one_would_have_hidden_it.md) | `Claimer` as the structural alternative to this signature |
| [`type/002`](002_the_iterator_nobody_can_name.md) | The crate's other type-level decision |
| [`api/001`](../api/001_twelve_items_seven_must_use.md) | The full public surface these two parameters sit in |

### Sources

| Fact | Where |
|------|-------|
| The signature | `ring_batch/src/lib.rs:306-314` |
| The arithmetic, and its doc | `ring_seqno/src/lib.rs:81-83`, `:95-99` |
| The aliased case as a passing doctest | `ring_seqno/src/lib.rs:90` |
| Six of six granted on a ring of four | Release probe, quoted above |
| How the rest of the family holds both ends | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_full_ring_refuses_with_full_and_advances_nothing` | The gate with two distinct cursors |
| `a_gated_claim_of_zero_always_succeeds_even_on_a_full_ring` | The zero path through the same arithmetic |
| *(to create)* | Nothing passes one cell as both arguments; the call is legal and untested |
