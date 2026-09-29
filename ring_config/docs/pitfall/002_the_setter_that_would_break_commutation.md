# Pitfall: The Setter That Would Break Commutation

### Scope

**Purpose:** Record what actually happens if the missing `with_capacity` is
added — which of the two failure shapes it produces, and why the suite would stay
green through it.

**Responsibility:** The measured difference between the two orderings, the fact
that both results are legal, and the shape of a commutation test that enumerates
calls rather than orderings.

**In Scope:** `ring_config/src/lib.rs:44`, `:71`, `:142`, `:154-156`,
`:197`; `ring_config/tests/config_test.rs:85`, `:89-101`.

**Out of Scope:** That commutation holds today and why is
[`invariant/002`](../invariant/002_the_setters_commute_and_one_absence_is_why.md).
Why `capacity` is a constructor argument is
[`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md).

---

## What the Suite Checks the Property With

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the property the suite claims for the setters --'
command grep -m1 -F '/// Setters commute: the same five values in any order produce the same record.' ring_config/tests/config_test.rs
echo '  -- every setter the commutation test names --'
command grep -m1 -A12 -F '  let forward = RingConfig::new( 32 ).unwrap()' ring_config/tests/config_test.rs | command grep -o 'with_[a-z_]*' | sort -u
echo '  -- machinery in the suite that enumerates orderings rather than naming them --'
command grep -c 'permut\|Step::' ring_config/tests/config_test.rs || true
echo '  -- the field a fifth setter would write, and the read that would then be mutable --'
command grep 'capacity' ring_config/src/lib.rs | command grep -v '///\|//!'
echo '  -- and the range both orderings would still satisfy --'
command grep -m1 -F '  /// The batch size, always between one and the capacity inclusive.' ring_config/src/lib.rs
```

Live output:

```
  -- the property the suite claims for the setters --
/// Setters commute: the same five values in any order produce the same record.
  -- every setter the commutation test names --
with_batch
with_overflow
with_producers
with_wait
  -- machinery in the suite that enumerates orderings rather than naming them --
6
  -- the field a fifth setter would write, and the read that would then be mutable --
  capacity : Capacity,
        capacity : Capacity::new( slots )?,
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
  pub const fn capacity( &self ) -> Capacity
    self.capacity
  -- and the range both orderings would still satisfy --
  /// The batch size, always between one and the capacity inclusive.
```

---

## The Same Record With the Fifth Setter Added

A copy of `RingConfig` plus the one setter the real type does not have. The
counterexample cannot be built from the shipped type — that is the whole point
of the finding — so it is built beside it, compiled, and run:

```sh
cd "$(git rev-parse --show-toplevel)"
# `with_batch`'s body is not retyped here. It is spliced out of the real source
# by the two lines below and pasted into the probe, so the copy cannot drift
# from the original the way a transcribed one would.
body=$( sed -n '/pub const fn with_batch/,/^  }/p' ring_config/src/lib.rs \
  | command grep '^    let capped\|^    self\.batch' )
echo '  -- the body spliced in, straight from ring_config/src/lib.rs --'
printf '%s\n' "$body"

{
cat <<'HEAD'
// `Capacity` is stubbed to the one operation `with_batch` uses: `get`. The real
// constructor is fallible; that is irrelevant to whether two setters commute.
#[ derive( Clone, Copy ) ]
struct Capacity( usize );
impl Capacity
{
  const fn new( slots : usize ) -> Self { Self( slots ) }
  const fn get( self ) -> usize { self.0 }
}

#[ derive( Clone, Copy ) ]
struct Cfg { capacity : Capacity, batch : usize }

impl Cfg
{
  const fn new( slots : usize ) -> Self { Cfg { capacity : Capacity::new( slots ), batch : 1 } }

  /// The setter `ring_config` does not have.
  const fn with_capacity( mut self, capacity : Capacity ) -> Self
  {
    self.capacity = capacity;
    self
  }

  /// Body spliced verbatim from `ring_config::RingConfig::with_batch`.
  const fn with_batch( mut self, batch : usize ) -> Self
  {
HEAD
printf '%s\n' "$body"
cat <<'TAIL'
    self
  }
}

fn main()
{
  let base = Cfg::new( 16 );
  let wide = Capacity::new( 64 );
  let grow_then_batch = base.with_capacity( wide ).with_batch( 32 );
  let batch_then_grow = base.with_batch( 32 ).with_capacity( wide );
  println!( "base                                   capacity {:2}  batch {}", base.capacity.get(), base.batch );
  println!( "with_capacity( 64 ).with_batch( 32 )   capacity {:2}  batch {}", grow_then_batch.capacity.get(), grow_then_batch.batch );
  println!( "with_batch( 32 ).with_capacity( 64 )   capacity {:2}  batch {}", batch_then_grow.capacity.get(), batch_then_grow.batch );
  println!( "the two orders agree                   {}", grow_then_batch.batch == batch_then_grow.batch );
  println!( "both records inside 1..=capacity       {}",
    grow_then_batch.batch <= grow_then_batch.capacity.get() && batch_then_grow.batch <= batch_then_grow.capacity.get() );
}
TAIL
} > /tmp/-rc43.rs

echo '  -- what the fifth setter does to the other four --'
# bare `rustc` is edition 2015 and will not accept a hyphen-prefixed output
# path as a crate name, hence the explicit `--crate-name`
rustc -O -A dead_code --crate-name rc43 -o /tmp/-rc43 /tmp/-rc43.rs 2>&1 | command grep '^error' || /tmp/-rc43
```

Live output:

```
  -- the body spliced in, straight from ring_config/src/lib.rs --
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
    self.batch = if capped == 0 { 1 } else { capped };
  -- what the fifth setter does to the other four --
base                                   capacity 16  batch 1
with_capacity( 64 ).with_batch( 32 )   capacity 64  batch 32
with_batch( 32 ).with_capacity( 64 )   capacity 64  batch 16
the two orders agree                   false
both records inside 1..=capacity       true
```

**This section used to quote a scratch crate, and the crate is gone.** It cited
`-cfg_probe/src/bin/added_capacity_setter.rs` — a hyphen-prefixed working
directory that was swept, taking the only executable form of this document's
central claim with it and leaving a quotation nothing could re-run. The probe
above is the same experiment written to be re-runnable from the document: it
compiles and runs in one gated block, and the one part that had to match the
shipped code — `with_batch`'s body — is now cut from `src/lib.rs` at run time
rather than transcribed, so "copied verbatim" is a thing the recipe does
instead of a thing the prose asserts.

---

### RC43 — The Dangerous Break Produces Two Legal Records, Not an Illegal One

[`invariant/002`](../invariant/002_the_setters_commute_and_one_absence_is_why.md)
names the failure as `with_batch( 8 ).with_capacity( 2 )` holding `batch = 8`
against `capacity = 2` — a record outside the range the getter at `:197` declares.
That break is loud in principle: it contradicts a stated invariant, so any
assertion, debug check or downstream consumer that trusted the range could catch
it.

The probe shows the other shape, and it is the one worth worrying about. Growing
the capacity rather than shrinking it, with the same two calls in the two orders,
gives `batch = 32` one way and `batch = 16` the other. Both records satisfy
`1..=capacity` — 32 ≤ 64 and 16 ≤ 64 — so both are entirely legal, and the two
rings that come out of them differ only in how much a publisher batches.

**Finding.** No invariant is violated, which means no invariant check could ever
find it. The only thing that distinguishes the two records is that they are not
equal to each other, and equality between orderings is not something any
production caller computes — it exists in this crate solely so `setters_commute`
can state its property, and
[`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md)
records that no two configurations are compared anywhere outside it.

So the hazard has an inverted severity profile. The obvious example breaks a
documented range and is detectable. The unobvious one silently changes throughput
on a ring that passes every check the family can make, and the caller's only
symptom is a batch size half what they meant, in a field
[`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) records as
having no production reader — so today it would not even reach the ring.

---

### RC44 — The Test Named for the Property Cannot Notice the Property Being Lost

`setters_commute` names four setters, twice each, in two orders. The suite
contains no permutation machinery at all: the count of anything enumerating
orderings rather than spelling them out is zero.

That is a reasonable test for a four-setter surface. It is not a test of the
property its own comment states — "the same five values in any order produce the
same record" — because the set of setters is written into the test body. Add a
fifth and the test does not call it, does not fail, and does not mention it. The
suite goes green, and the sentence at `:85` becomes false the moment the new
function compiles.

**Finding.** The failure mode is the specific one that makes commutation
regressions hard: the test degrades in coverage exactly as the surface grows, and
degrades silently, because nothing connects "every setter" in the prose to the
four names in the code.

The mechanism that would have caught it exists and is not in the suite. RC24's
probe walks all twenty-four orderings of the four setters with both clamps firing,
built from a `Step` enum rather than from named chains; extending that to five
setters is adding one variant, and it would go red on the ordering the probe above
measured. It lives in this corpus as a one-off binary because nothing asked for
it in `tests/`.

Worth recording as a pitfall rather than a coverage note because the trap is not
that the test is thin. It is that the test's name and comment promise a general
property, so a developer adding `with_capacity` has every reason to read a green
`setters_commute` as confirmation that they did not break it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](../invariant/002_the_setters_commute_and_one_absence_is_why.md) | The property today, and the absence that holds it up |
| [`pitfall/001`](001_a_clamp_with_no_way_to_detect_it.md) | The other hazard in the same cross-field read |
| [`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md) | Why the fifth setter does not exist |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | The field whose divergence nothing would consume |
| [`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md) | Why comparing two configurations is a within-crate facility |

### Sources

| Fact | Where |
|------|-------|
| The commutation test's stated property | `ring_config/tests/config_test.rs:85` |
| The four setters it names, and its two orderings | `ring_config/tests/config_test.rs:89-101` |
| No permutation machinery in the suite | Census above, zero occurrences |
| `capacity` written once and read once | `ring_config/src/lib.rs:44`, `:71`, `:142` |
| The declared range both orderings satisfy | `ring_config/src/lib.rs:197` |
| Batch 32 one way and 16 the other, both legal | Probe above, `with_batch` body copied verbatim |

### Tests

| Test | Covers |
|------|--------|
| `setters_commute` | Two orderings of four named setters, on unclamped values |
| `batch_clamps_into_one_through_capacity` | That the clamp follows the capacity, without ordering around it |
| `each_setter_is_independent` | That each setter writes only its own field |
