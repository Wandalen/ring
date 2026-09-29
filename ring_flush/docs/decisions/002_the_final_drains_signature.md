# Decision: `drain_final`'s Signature

**Status:** open, and more expensive to change than when it was raised. Rules
Pending 4. Pending 5 was closed without it, against Pending 5's own instruction
that the two be ruled together, and the closing put two tests behind the
behaviour a consuming signature would remove.

### Scope

- **Purpose**: Record what a consuming `drain_final` would buy, what it now costs, and why the pair it was supposed to be ruled with was ruled without it.
- **Responsibility**: The three candidate signatures, the retry path that complicates all of them, and the change in cost since the question was raised.
- **In Scope**: `Flusher::drain_final`'s receiver and return type; its relationship to `drive` and `drive_at_barrier`; the tests that pin the current behaviour.
- **Out of Scope**: Whether the enums are `#[non_exhaustive]` (→ [`001`](001_whether_the_policy_enum_is_non_exhaustive.md)); the teardown phase as a lifecycle stage (→ [`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md)); the catalogue view of the same signature (→ [`item/002`](../item/002_seventeen_verbs_that_never_touch_the_producer.md)).

### The Question, Restated

`drain_final` is documented as a named teardown phase, called once, by the
owner. The signature is `&mut self`, which permits any number of calls. Taking
`self` by value would make the once-only property structural rather than
documentary.

**The retry path is what stops that being a one-line change.** A final drain can
be [`FlushOutcome::Rejected`](../type/002_flush_outcome.md) — the ring had no
room — and a rejected drain *must* be retried or the staged records are lost.
A consuming signature therefore has to hand the flusher back on rejection:

| # | Signature | Once-only | Retry |
|---|-----------|-----------|-------|
| A | `fn drain_final( &mut self ) -> FlushOutcome` (today) | By convention, name and documentation | Call it again |
| B | `fn drain_final( self ) -> FlushOutcome` | Structural | **Impossible.** A rejection loses the records it was reporting on |
| C | `fn drain_final( self ) -> Result< Drained, ( Self, FlushOutcome ) >` | Structural | Return the flusher in the error arm and call again |

B is unusable, which leaves A and C. C is more honest and considerably less
pleasant to call: every teardown site grows a match, and the happy path — by far
the common one — pays for the rare one.

### What Changed Since the Question Was Raised

Pending 5 asked what happens to appends *after* a final drain, marked the answer
"unspecified", and said that was the one answer definitely wrong because callers
would find the behaviour empirically and depend on it. It closed by observing
that **Pendings 4 and 5 should be ruled together**, since C resolves 5 as a side
effect: with a consuming signature there is no flusher left to append to.

Pending 5 was then closed on its own, by measurement, and two tests were written
to pin the result. That was the right call for Pending 5 — an empirically
discovered behaviour is worse than a documented one — and it is why C now costs
more than it did.

### Why It Is Filed Rather Than Decided

Ruling for C means deleting a documented, tested behaviour that a consumer may
already rely on, in exchange for a compile-time guarantee about a call nobody
has yet made twice by accident. Ruling for A means writing down that the
once-only property is a convention and will stay one.

Neither is obviously right, and A is reversible while C is not — a consuming
signature can be relaxed back to `&mut self` without breaking a caller, but the
reverse breaks every teardown site. **The reversible option is the one currently
in force**, which is the correct direction for an unresolved question to default.

### Related

- [`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md) —
  the teardown phase, and the table recording what enforces the cardinality
- [`api/002`](../api/002_the_driver_surface.md) — the three driver methods as a
  caller meets them
- [`type/002`](../type/002_flush_outcome.md) — `Rejected`, the variant that makes
  B unusable and C ugly

### FL15 — Three Methods, One Signature, and Only One of Them Has a Cardinality

The driver surface is three names over one shape:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three driver methods --'
command grep -E '^  pub fn (drive|drive_at_barrier|drain_final)' ring_flush/src/lib.rs
echo '  -- and every by-value receiver in the crate --'
command grep -E '^  pub (const )?fn [a-z_]+\( *(mut )?self' ring_flush/src/lib.rs
```

Live output:

```
  -- the three driver methods --
  pub fn drive( &mut self ) -> FlushOutcome
  pub fn drive_at_barrier( &mut self ) -> FlushOutcome
  pub fn drain_final( &mut self ) -> FlushOutcome
  -- and every by-value receiver in the crate --
  pub fn with_log( mut self ) -> Self
```

`&mut self` in, `FlushOutcome` out, three times. The type system cannot tell the
three apart, and a caller who reaches for the wrong one gets no error — just a
publication at a moment nobody chose, which is the failure the crate exists to
prevent.

**The crate's answer to that is greppability**, stated as `integration/002`'s Z3:
`drive` and `drive_at_barrier` stay separate names because "a reader auditing
whether barriers are announced correctly can grep for `drive_at_barrier` and
cannot grep for `true`." The argument is sound and it covers two of the three.

It does not cover the third, and the third is the one carrying an obligation the
others do not have. `drive` and `drive_at_barrier` are idempotent in the sense
that matters — calling either twice is ordinary use. `drain_final` is documented
as once-only, and it is spelled identically to two methods for which repetition
is the normal case.

**One by-value receiver already exists**, and it is `with_log` — the builder.
So the shape option C wants is not foreign to the crate; what is foreign is
using it on a method that can fail in a way the caller must recover from.

### FL16 — A Pair That Was to Be Ruled Together Was Ruled Half, and the Half Raised the Other's Price

The instruction and what happened to it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the instruction --'
command grep 'ruled together' ring_flush/docs/decisions/readme.md
echo '  -- the two status rows as they stand --'
command grep -E '^\| P[45] \|' ring_flush/docs/decisions/readme.md
echo '  -- what now pins the behaviour a consuming signature would delete --'
awk '/^fn [a-z_]+/{ f = $0; n = 0 } /\.drain_final\(/{ n += 1; if ( n == 2 ) print "    " f }' \
  ring_flush/tests/flush_test.rs
```

Live output:

```
  -- the instruction --
side effect). **Pendings 4 and 5 should be ruled together.**
| FL16 | the split pair | n/a — drift | Pending 5 said the pair should be ruled together, was closed alone by measurement, and the two tests written to pin it are what a consuming signature would delete |
  -- the two status rows as they stand --
| P4 | `drain_final`'s signature | **Open**, unchanged — `&mut self`, once-only by convention |
| P5 | Appends after `drain_final` | **Answered by measurement** — the flusher is reusable, and it is now a test rather than a discovery |
  -- what now pins the behaviour a consuming signature would delete --
    fn a_refused_final_drain_keeps_the_records()
    fn a_second_final_drain_is_an_empty_trigger()
```

Pending 5 said the two should be ruled together. The status table records
Pending 5 as answered and Pending 4 as open and unchanged — the pair was split,
and split in the direction that makes the remaining half harder.

**Closing Pending 5 by measurement was right and it is not the defect.** An
unspecified behaviour that callers discover by experiment is worse than a
documented one, and the two tests are the documentation. The defect is that
nothing recorded the consequence for its partner: those same two tests are
precisely what a consuming signature would have to delete, so Pending 4's cost
rose at the moment Pending 5's fell, and neither entry says so.

That is the general shape worth keeping. **When a decision is deferred *as a
pair* and one half is later closed alone, the other half's entry is stale from
that moment**, and nothing checks it — the closing edit touches the half being
closed, and the pairing lives in a sentence at the bottom of the half that is
not.
