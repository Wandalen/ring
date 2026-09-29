# Lifecycle: Config State Through a Build

### Scope

- **Purpose**: Track a configuration's states from the caller's first call to the moment it is consumed, and identify the one state that exists only because validation and clamping happen in a different crate from construction.
- **Responsibility**: State the states, transitions, and behavioral invariants.
- **In Scope**: The record's states; which are reachable from this crate; where the requested value stops existing.
- **Out of Scope**: The name's states (→ [`lifecycle/004`](004_name_state_through_a_registration.md)); the ring's own states after construction.

### States

| # | State | Meaning | Reachable from this crate |
|---|-------|---------|---------------------------|
| S0 | **Requested** | A set of numbers the caller intends. Not a value — it exists in the caller's source, a manifest, or a sweep row | **No.** Never materialises as data |
| S1 | **Rejected** | `RingConfig::new` returned `Err` | No. The arc ends before this crate |
| S2 | **Legal** | A `RingConfig` exists; every field is within range | Yes — this is what `build` receives |
| S3 | **Corrected** | Legal, and at least one field differs from S0 | **Yes, and indistinguishable from S2** |
| S4 | **Consumed** | The record has been read and a ring exists | Yes — the terminal state |

**S3 is the state this machine exists to name.** It is not a distinct value: a
config in S3 is a perfectly ordinary `RingConfig`, structurally identical to one
that was never corrected. The distinction lives entirely in the relationship
between S0 and the record, and S0 is not data.

```rust
let a = RingConfig::new( 16 ).unwrap().with_batch( 999 );  // S3 — asked 999, holds 16
let b = RingConfig::new( 16 ).unwrap().with_batch( 16 );   // S2 — asked 16, holds 16
assert_eq!( a, b );                                        // and they are equal
```

**`a == b` is the whole problem in one line.** Two configs in different states
by this machine's reckoning are equal by every mechanism the language provides,
which is why [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)
holds — equal configs build equal rings — while
[`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)'s
acceptance gap is real. The invariant and the pitfall are both true because
they are about different halves of the same equality.

**S1 exists for exactly one field.** `Capacity` is the only input that can reach
S1; the other four go to S2 or S3 and never to S1. A machine with one rejecting
input and two clamping ones is asymmetric by design, and the asymmetry is
`ring_config`'s deliberate choice, not an oversight
(→ [`ring_config`](../../../ring_config/readme.md)).

### Transitions

| # | From → To | Trigger | Owner |
|---|-----------|---------|-------|
| X1 | S0 → S1 | `new( slots )` with zero or a non-power-of-two | `ring_config` |
| X2 | S0 → S2 | `new( slots )` with a legal capacity | `ring_config` |
| X3 | S2 → S2 | A setter whose argument is already in range | `ring_config` |
| X4 | S2 → S3 | A setter whose argument is out of range — **silently corrected** | `ring_config` |
| X5 | S3 → S3 | A further setter, in range or not | `ring_config` |
| X6 | S2 → S4 | `build( cfg )` reads the record | **This crate** |
| X7 | S3 → S4 | `build( cfg )` reads the record | **This crate, and it cannot tell X6 from X7** |
| X8 | S4 → ∅ | The record is dropped after construction; `Copy` means the caller still has theirs | This crate |

**X4 is the transition with no signal.** No return value changes, no flag is
set, no log line is written. The setter returns `Self` in both X3 and X4, and
the two are distinguishable only by comparing the argument to the result —
which the caller can do and does not, and which this crate cannot do at all
because the argument is gone by X6.

**X6 and X7 are the same code path.** That is the machine's central fact for
this crate: there is no branch to write, no check to add, and no information
present at build time that would let one be told from the other. Any mitigation
must act before X4 or after X8 (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)'s
mitigations 1 and 2), never between them.

**There is no transition out of S4 back to S2.** A consumed config is not
returned, not stored on the handles, and not recoverable from the ring
(→ [`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md)).
X8 is terminal for this crate's copy. The caller's copy survives — `RingConfig`
is `Copy` and was passed by value — which is what makes mitigation 2 possible at
all: the *caller* can still read the effective config after the build, even
though the factory cannot hand it back.

### Behavioral Invariants

| # | Invariant | Holds because |
|---|-----------|---------------|
| B1 | Every config reaching X6 or X7 is legal | X1 removed the only illegal input; X4 corrected the rest |
| B2 | X6 and X7 produce identical rings for equal records | [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md); the states differ, the values do not |
| B3 | No transition after X2 can fail **inside `ring_config`** | X3–X8 are total within this machine. The build that consumes S4 can still refuse it — see below |
| B4 | S3 is unobservable from S4 | No field records that a correction occurred, and nothing downstream stores the request |
| B5 | The caller's copy outlives X8 | `RingConfig: Copy`, passed by value at X6/X7 |

**B3 was stated too widely and the correction is instructive.** It read "No
transition after X2 can fail — X3–X8 are all total. This is why `build` returns
no `Result` on the unnamed path." The first clause is about *this* state
machine, which models a config's journey through `ring_config`'s setters and out
to a build; it is still true. The second clause reached past the machine's own
boundary to make a claim about a crate the machine does not model, and that
claim is now false: `ring_core::Ring::new` refuses `OverflowPolicy::DropOldest`
(→ [`type/002`](../type/002_build_error.md)).

**S4 is legal and buildable are not the same predicate, and this machine only
decides the first.** Every state and transition here is `ring_config`'s; a
config in S4 has passed every check `ring_config` performs, which is what B1
asserts. Whether a backend will accept it is a separate question with a separate
owner, and the machine has no state for "legal here, refused there" because the
refusal happens after X8:

| Predicate | Decided by | Modelled here |
|-----------|-----------|---------------|
| The config is well-formed | `ring_config` (X1, X4) | Yes — S1 vs S2/S3 |
| The config is buildable | `ring_core` | **No.** After X8, outside this machine |

**B3 is still the invariant that shapes the API, in its narrowed form**, and B4
is still the one that costs. Both are consequences of the same decision —
clamping instead of rejecting — which buys ergonomics (no `?` mid-chain) at the
price of an unobservable state. **What changed is that the ergonomics no longer
extend to `build`**: the caller writes no `?` while building a config and writes
one when they use it.

**B4 is falsifiable and worth testing at the `ring_config` boundary rather than
here.** A test that constructs `a` and `b` above and asserts `a == b` pins the
state collapse deliberately, so that a future change making them unequal — a
`requested_batch` field, say — is caught as a behaviour change rather than
landing quietly. That test belongs to `ring_config`; this instance is where the
*reason* to want it is recorded.

**B5 is the invariant a mitigation can be built on.** The caller retains a
readable, corrected config across the build, so a sweep harness that labels its
runs from `cfg.batch()` *after* calling `build` gets the truth with no change to
this crate at all.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | X6/X7's first read, and the branch a corrected `producers` can flip |
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | X6/X7 in full, field by field |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | B3's boundary — the error channel that exists on the far side of X8 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | The record whose states these are; its Validated/Clamped columns are X1 and X4 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | B2 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) | L1 and L2 are X1–X5; L3 and L4 are X6/X7 |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | S3, and why every mitigation acts outside X4–X8 |

### State Machines

| File | Relationship |
|------|--------------|
| [004_name_state_through_a_registration.md](004_name_state_through_a_registration.md) | The other machine a named build runs, concurrently with this one |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | X1–X5, and the doctests that assert X4's corrections |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ✅ B2 and B5 asserted; B4 correctly left to `ring_config/tests/config_test.rs`, since S3 is unreachable here as a distinguishable input. B5's zero-cost route — the caller reads `cfg.batch()` back out after construction, `RingConfig` being `Copy` — is why [`decisions/`](../decisions/readme.md) records "should `build` return the effective config" as a question **deliberately not opened**: a decision is not needed for a problem with a free solution elsewhere |

### FC31 — The Requested Value Has No State: It Is Overwritten in Place at the Setter

The state list has the requested value stop existing somewhere. The exact place
is an assignment into `self`, and because `RingConfig` is `Copy` and the setters
consume and return it, the caller's own copy is the clamped one too:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- where the requested value is overwritten --'
command grep 'self.producers = \|self.batch = ' ring_config/src/lib.rs
echo '  -- the setter shape that makes the caller copy the clamped one --'
command grep 'pub const fn with_producers\|pub const fn with_batch' ring_config/src/lib.rs
echo '  -- and the field that has no setter at all --'
command grep -E '^  pub (const )?fn (new|with_capacity)' ring_config/src/lib.rs | sed 's/(.*//'
```

Live output:

```
  -- where the requested value is overwritten --
    self.producers = if producers == 0 { 1 } else { producers };
    self.batch = if capped == 0 { 1 } else { capped };
  -- the setter shape that makes the caller copy the clamped one --
  pub const fn with_producers( mut self, producers : usize ) -> Self
  pub const fn with_batch( mut self, batch : usize ) -> Self
  -- and the field that has no setter at all --
  pub fn new
```

`with_producers( 0 )` returns a record holding `1`. There is no state in which
the record remembers `0`, no accessor that reports one, and no second record
holding the request — the chain `RingConfig::new( 8 ).with_producers( 0 )`
evaluates to a value in which the request is unrecoverable, in the same
expression the caller wrote it.

Two consequences follow that the state list does not carry. First, the "requested"
state is not a state of the record at all — it is a state of the *argument
expression*, which exists only during the call, so nothing downstream can ever
observe or report it (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).
Second, `capacity` has no setter — it is fixed at `new` and refused there rather
than clamped — so `with_batch`'s cap against capacity cannot be invalidated by a
later call, and the state list is order-independent for a reason that is a
property of the API's shape rather than of the clamping rules.
