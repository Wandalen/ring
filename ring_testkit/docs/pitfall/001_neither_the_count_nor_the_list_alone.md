# Pitfall: Neither The Count Nor The List Alone

### Scope

- **Purpose**: Record the measurement the crate is built on — that the two obvious readings of a ring run each fail to see a dropped record, in opposite directions.
- **Responsibility**: The failure, the numbers behind it, and what reading actually catches it.
- **In Scope**: `OverflowPolicy::{ Fail, DropNewest }` as seen through `Outcome`.
- **Out of Scope**: `OverflowPolicy::DropOldest`, which `ring_core::Ring::new` refuses outright.

### The failure

`ring_core::Producer::try_push` **reports `Ok` on a full ring** under
`OverflowPolicy::DropNewest`. The record is discarded inside the call and the
caller is told it succeeded. Its own doc comment says so plainly:

> On a full ring the configured `OverflowPolicy` decides: `Fail` returns the
> record, `DropNewest` discards it and reports success.

So a fixture that drives a ring has to notice records that were accepted and
destroyed. The two obvious ways to notice both fail.

### The measurement

One script — push eight records into a four-slot ring, then drain everything —
run against two rings differing only in overflow policy:

```text
Fail:       accepted=4  refused_full=4  received=[0,1,2,3]  vanished=0
DropNewest: accepted=8  refused_full=0  received=[0,1,2,3]  vanished=4
```

| Reading | What it says | Why it is wrong |
|---|---|---|
| `received` — the records delivered | **The two rings are identical.** Both delivered `[0,1,2,3]` | It is right about delivery and blind to destruction. Four records entered the `DropNewest` ring and left no trace in this list |
| `accepted` — the count of `Ok`s | The dropping ring accepted **twice as many** | Also true, and reads as *more work done*. Nothing in `8 > 4` says "and half of them were destroyed" |
| `vanished` — accepted, less delivered, less still held | Four records are nowhere | — |

**The prediction that produced this crate was wrong, and wrong in both halves.**
It was that the counts would match and the delivered records would differ. The
opposite holds. Recorded in `tests/manual/readme.md` M1 rather than corrected
into agreement, because the wrong prediction is the reason `vanished` exists as
a method instead of as a comment.

### Consequences

| # | Consequence | Where it lands |
|---|---|---|
| P1 | `Outcome` must carry both the counts and the delivered list | Neither field can be dropped as redundant — they disagree |
| P2 | `Outcome::audit` deliberately does **not** fail on a vanished record | Vanishing is legal under `DropNewest`; a fixture that rejected it would refuse to model a policy the family supports |
| P3 | `vanished` is a method, not a field | It is derived from three fields, and a fourth stored number could drift from them |

**P2 is the one to argue with.** `audit` passes on the `DropNewest` outcome
above — four records destroyed and no anomaly reported. That is correct: the
records were *accounted for*, which is what `audit` checks. Whether destruction
is acceptable is the caller's policy question, and `vanished` is how they ask
it. Folding the two together would make a legal configuration unusable.

### Evidence

| # | Claim | Test |
|---|---|---|
| F1 | Both policies deliver identical records | `neither_the_count_nor_the_delivered_records_sees_a_drop_alone` |
| F2 | The counts differ, in the direction that reads as more work | same test |
| F3 | `vanished` separates them | same test |
| F4 | A ring that never fills vanishes nothing under either policy | `a_ring_with_room_vanishes_nothing_under_either_policy` |
| F5 | `audit` passes on both | `neither_the_count_nor_the_delivered_records_sees_a_drop_alone` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'what the audit sums:             %s\n' "$( command grep -m1 -oE 'let placed = [^;]*' ring_testkit/src/lib.rs | sed 's/self\.//g' )"
printf 'delivered records in that sum:   %s\n' "$( command grep -m1 'let placed = ' ring_testkit/src/lib.rs | command grep -cE 'received|in_ring_at_end' || true )"
printf 'vanished() called in src:        %s\n' "$( command grep -vE '^ *//' ring_testkit/src/lib.rs | command grep -c 'vanished()' || true )"
printf 'vanished() named in src prose:   %s\n' "$( command grep -cE '^ *//[/!].*vanished\(\)' ring_testkit/src/lib.rs || true )"
printf 'vanished() called in tests:      %s\n' "$( cat ring_testkit/tests/*.rs | command grep -c 'vanished()' || true )"
printf 'Outcome public fields:           %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' ring_testkit/src/lib.rs | command grep -cE '^  pub ' || true )"
printf 'tests driving both policies:     %s\n' "$( awk '/^fn [a-z_]+\(/{ n=$2 ; b="" } /^fn /,/^}$/ { b = b $0 } /^}$/ { if ( b ~ /dropping_ring\(/ && b ~ /failing_ring\(/ ) print n }' ring_testkit/tests/testkit_test.rs | wc -l )"
printf 'Ring::new refuses DropOldest by: %s\n' "$( command grep -m1 -oE 'config.overflow\(\) == OverflowPolicy::DropOldest' ring_core/src/lib.rs )"
printf 'the constructor that accepts it: %s\n' "$( command grep -m1 -oE 'pub fn new_crossbeam\( config : &RingConfig \)' ring_core/src/lib.rs )"
printf 'backend checks in Script::run:   %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /^  }$/{exit} f' ring_testkit/src/lib.rs | command grep -ciE 'backend|crossbeam' || true )"
printf 'crossbeam in ring_testkit.toml:  %s\n' "$( command grep -ci crossbeam ring_testkit/Cargo.toml || true )"
printf 'run doc sections on the backend: %s\n' "$( awk '/pub fn run\( &self/{exit} /# Backend-blind/{n++} END{print n+0}' ring_testkit/src/lib.rs )"
```

Live output:

```
what the audit sums:             let placed = accepted + refused_full + refused_closed + refused_staging + staged_at_end
delivered records in that sum:   0
vanished() called in src:        0
vanished() named in src prose:   2
vanished() called in tests:      9
Outcome public fields:           10
tests driving both policies:     2
Ring::new refuses DropOldest by: config.overflow() == OverflowPolicy::DropOldest
the constructor that accepts it: pub fn new_crossbeam( config : &RingConfig )
backend checks in Script::run:   0
crossbeam in ring_testkit.toml:  0
run doc sections on the backend: 1
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `Outcome`'s fields, and why `vanished` is derived |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The accounting law that a vanished record does *not* violate |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `Producer::try_push` — the `Ok`-on-discard contract |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | F1–F5 |
| `tests/manual/readme.md` | M1 — the run, and the wrong prediction |

### TK41 — the audit is blind to this pitfall by construction, and its remedy is opt-in

`Outcome::audit` is the packaged judgement — the one call a consumer makes when
they do not want to read ten fields. What it sums is:

    let placed = accepted + refused_full + refused_closed + refused_staging + staged_at_end

Zero of `received` and `in_ring_at_end` appear in it. A record the ring silently
destroyed was still `accepted`, so it is still `placed`, so `placed == minted`
and the audit returns `Ok( () )`. The crate's own test asserts exactly this,
twice, and says so in its message: *"vanishing is not an accounting failure"*,
*"which is exactly why it needs its own reading"*.

That is the correct design — an audit that failed on a `DropNewest` run would be
calling a configured policy a bug. The cost is where the remedy sits.
`Outcome::vanished()` is the only thing that reports the loss, it has **zero**
call sites in `src/` and nine in the tests, and all ten of `Outcome`'s fields
are public — so a consumer can construct a script, run it, read every field on
the returned value, take the packaged verdict, get `Ok( () )`, and never
encounter the one accessor that would have told them four records were destroyed.

Nothing prompted the call. `vanished` carries `#[ must_use ]`, which fires only
once it has been written. The pitfall this document names was discoverable only
by a reader who already knew to look for it — which is the shape of the failure
the document opens by describing.

**Disposition:** applied — to the two doc comments a reader actually passes
through, and the audit itself is **declined**. Making `audit` fail on a
`DropNewest` run would call a configured policy a bug, which is the design this
entry agrees with; so `audit`'s own doc gained a `# What it does not check`
section naming `vanished` as the reading it deliberately omits, and `vanished`
gained a `# `0` Is Two Different Answers` section stating that a caller who takes
`audit()` and never calls `vanished()` is told nothing about destroyed records.
Both sit on the items a consumer is already reading rather than in a document
they have to know exists. The census needed repairing to keep saying `0`: it
counted `vanished()` anywhere in the file, so the two new doc-comment mentions
read as call sites and turned a true zero into a false two — the exact
measurement error this corpus keeps finding, committed by a fix for it. What this
does not buy: a consumer who reads neither doc page still gets `Ok( () )` and
still never learns; nothing in the type or the build makes the call happen.
Now prints: `vanished() called in src:        0`

### TK42 — the excluded policy is excluded by one constructor, not by the type

The Out of Scope line rules out `OverflowPolicy::DropOldest` on the grounds that
`ring_core::Ring::new` refuses it. That refusal is real — it is the
`config.overflow() == OverflowPolicy::DropOldest` guard quoted in the census
above. It is not a property of the type.

`Ring::new_crossbeam( config : &RingConfig )` is a second constructor on the
same `Ring`, and `ring_core`'s own doc states its purpose plainly: it *"accepts
`OverflowPolicy::DropOldest` where `new` refuses it, because
`ArrayQueue::force_push` does exactly that — the one capability the in-house
rings deliberately lack."*

`Script::run` takes `&mut Ring< u32 >` and contains **zero** references to
`backend` or `crossbeam`. It cannot tell which constructor built the ring it was
handed, and it does not try. So a consumer holding a crossbeam-backed
`DropOldest` ring can drive it with a `Script`, and the fixture will run,
account, and hand back an `Outcome` whose `vanished()` is the single most
important number on it — under the exact policy this document says is out of
scope.

The exclusion is sound for a narrow reason: `crossbeam` appears **zero** times in
`ring_testkit`'s manifest, so the crate cannot enable the feature that makes
`new_crossbeam` exist and its own tests can never reach the case. That is a fact
about this crate's dependencies, not about the fixture, and it changes the day
any consumer turns the feature on. What was written down was a refusal by
`Ring::new`; what is true is a refusal by one of two constructors, in a crate
that cannot see the other.

**Disposition:** applied — as a `# Backend-blind` section on `Script::run`, and a
runtime check is **declined**. `run` could interrogate the ring's backend and
refuse an evicting one, but it has no way to: `Ring::new` and
`Ring::new_crossbeam` produce the same type, `ring_core` exposes no accessor for
which built it, and adding one to serve a fixture would put a test-shaped hole in
a production type. So `run`'s doc now says what is actually true — that it never
asks which implementation is under the ring, that the second constructor accepts
`DropOldest` where the first refuses it, that the fixture's exclusion is therefore
not a property of the fixture, and that it holds only because `new_crossbeam` sits
behind a `ring_core` feature this crate neither enables nor names. A caller who
enables it gets an `Outcome` shaped like any other. What this does not buy: the
day a consumer turns that feature on, nothing changes except that the sentence is
now there to be read — the exclusion is still enforced by a manifest that does not
mention `crossbeam`, which is the weakest possible place to enforce it.
Now prints: `run doc sections on the backend: 1`
