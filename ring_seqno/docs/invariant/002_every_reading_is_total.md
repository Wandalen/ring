# Invariant: Every Reading Is Total

### Scope

- **Purpose**: Establish that no function in this crate can fail on any input, and identify which parts of that guarantee are local and which are borrowed.
- **Responsibility**: Enumerate the ways arithmetic could fail, show each is excluded, and mark the one exclusion that depends on a claim made in another crate that does not hold.
- **In Scope**: Totality of `ring_seqno`'s five functions.
- **Out of Scope**: Whether the *answers* are meaningful — that is [`001`](001_the_sequence_is_never_folded_here.md) and [`decisions/002`](../decisions/002_saturating_rather_than_signed.md).

### The Invariant

> Every function in `ring_seqno` returns a value for every input. None returns
> `Result`, none takes a fallible path, and none can panic.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_seqno
# the pattern matches this very file — the control for the empty result below
grep -cE 'Result|panic!|unwrap|expect|assert|\?' src/lib.rs
# … and every one of those matches is inside a /// doc example, so filtering
# the doc lines out leaves nothing at all in a function body
grep -nE 'Result|panic!|unwrap|expect|assert|\?' src/lib.rs | grep -vE ':\s*///' \
  || echo '(no matches — nothing outside the doc lines)' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
16
(no matches — nothing outside the doc lines)
```

Sixteen matches in the file, and nothing left once the doc lines are removed.
All sixteen are `Capacity::new( 8 ).unwrap()` and `assert_eq!` inside doctests.

### The Four Ways It Could Have Failed

| # | Hazard | Where it would arise | Excluded by | Local? |
|---|--------|----------------------|-------------|:------:|
| T1 | Subtraction underflow | `later - earlier` when the pair is backwards | `Seq::distance_to` uses `saturating_sub` | ❌ `ring_types` |
| T2 | Subtraction underflow | `capacity - in_flight` when the ring is over-full | `free_slots` uses `saturating_sub` | ✅ |
| T3 | **Division by zero** | `distance / capacity` in `laps_between` | `Capacity::new` rejects `0` | ❌ `ring_types` |
| T4 | Empty-input panic | `min()` over an empty slice | `Iterator::min` returns `Option` | ✅ *(std)* |

Two of the four are borrowed. T2 is the only exclusion written in this crate,
and T4 is std's.

### T3 Is the One Worth Reading Twice

```rust
// ring_seqno/src/lib.rs:52
earlier.distance_to( later ) / capacity.get() as u64
```

An unguarded integer division. It is total only because a `Capacity` can never
hold zero, and that is established two crates away:

```rust
// ring_types/src/capacity.rs:40-51
pub const fn new( slots : usize ) -> Result< Self, RingError >
{
  if slots == 0 { return Err( RingError::CapacityZero ); }
  if !slots.is_power_of_two() { return Err( RingError::CapacityNotPowerOfTwo( slots ) ); }
  Ok( Self( slots ) )
}
```

The guarantee is genuinely airtight, and the reason is a detail easy to miss:

```rust
pub struct Capacity( usize );   // ← the field is NOT pub
```

`Capacity`'s field is private and the type has no `Default`, so `new` is the only
way to obtain one, so every `Capacity` in existence passed the zero check.
Contrast the sibling type in the same file:

```rust
pub struct Seq( pub u64 );      // ← the field IS pub
```

`Seq`'s field is public because any `u64` is a valid sequence — there is no
invariant to protect. The two visibility choices are correct and opposite, and
together they are what makes T3 safe and T1 necessary.

**Nothing in `ring_seqno` restates T3.** No comment at line 52 says "safe because
`Capacity` is non-zero", and no test in this crate constructs a capacity to prove
it. A reader auditing this file for panics has to leave it to establish that the
one division is sound. That is a documentation gap rather than a defect — the
guarantee holds — and this section is where it is now recorded.

### T1's Guarantee Is Correctly Documented; a Neighbouring One Is Not

`distance_to` saturates, and says so:

```rust
/// Saturating rather than signed: the caller that needs the direction has
/// already compared the two, and every caller that does not wants a count.
pub const fn distance_to( self, later : Self ) -> u64
{
  later.0.saturating_sub( self.0 )
}
```

Accurate. But two methods above it, on the same type, the documentation of
`Seq::next` was not. When SQ44 was filed it read:

```text
/// Panics on overflow in a debug build and saturates in a release build, which
/// is the standard `u64` addition behaviour — deliberately not wrapping, since
/// a wrapped `Seq` would silently violate the monotonicity every gate in the
/// family relies on.
```

over a body of `Self( self.0 + 1 )`.

**Finding SQ44 — since corrected upstream.** Plain `u64 + 1` does not saturate in
release. With `overflow-checks` off — the default, and unset anywhere in this
workspace — it **wraps**. So at `u64::MAX` the method produces `Seq( 0 )`, which
is precisely the "wrapped `Seq`" the sentence said was deliberately excluded.
`advanced_by` has the same body shape (`self.0 + n`) and, at filing time, carried
no overflow note at all.

Both have since been repaired in `ring_types`, and the recipe below prints the
live text of each alongside the manifest evidence the finding rested on:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- no manifest overrides overflow-checks, so release keeps the default and wraps --'
command grep -r 'overflow-checks' Cargo.toml */Cargo.toml \
  || echo '(no matches — no manifest overrides it)'
echo '-- control: the identical expression over a manifest that does set it --'
printf '[profile.release]\noverflow-checks = true\n' > /tmp/-ovf_control.toml
command grep -r 'overflow-checks' /tmp/-ovf_control.toml
rm -f /tmp/-ovf_control.toml
echo '-- next, as it reads now --'
command grep -m1 -A4 -F '  /// Panics on overflow in a debug build and wraps to zero in a release' ring_types/src/id.rs
echo '-- advanced_by, which had nothing and now has this --'
command grep -m1 -A5 -F '  /// Overflow behaves exactly as [`Seq::next`] documents' ring_types/src/id.rs
echo '-- and the word SQ44 was about --'
command grep -c 'saturates in a release build' ring_types/src/id.rs \
  | sed 's/^/   occurrences of the original claim: /'
```

Live output:

```
-- no manifest overrides overflow-checks, so release keeps the default and wraps --
(no matches — no manifest overrides it)
-- control: the identical expression over a manifest that does set it --
overflow-checks = true
-- next, as it reads now --
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
-- advanced_by, which had nothing and now has this --
  /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
  /// release wraps to zero. The reachability argument does not carry over
  /// unchanged: `next` needs 2⁶⁴ increments to reach the wrap, while this
  /// takes `n` from the caller and reaches it in a single call from any
  /// position. A caller deriving `n` from a batch length or a configured
  /// count owns that bound; nothing here checks it.
-- and the word SQ44 was about --
   occurrences of the original claim: 0
```

The correction does not change what this invariant asserts. `ring_seqno`'s readings
are total because `distance_to` saturates, and that method's documentation was
accurate all along — SQ44 was about a *neighbouring* method whose doc contradicted
the arithmetic this crate is built on, which is why an invariant about totality is
the right place to have caught it.

Two things must be said plainly about it:

- **The consequence is unreachable.** Overflow needs 2⁶⁴ publications; at a
  billion per second that is roughly 584 years of continuous running. No ring in
  this family will reach it.
- **The claim is still wrong**, and it is wrong in the direction that matters —
  the doc reassures a reader about exactly the failure mode that is not
  prevented. Someone auditing the family's monotonicity assumption will read
  that paragraph and stop looking.

It belongs to `ring_types` and is recorded here because `ring_seqno`'s entire
arithmetic rests on the monotonicity it claims to guarantee. The accurate
sentence would be "panics in debug and wraps in release; overflow is unreachable
at any realistic publication rate, so neither branch is defended against."

### Totality Is Not the Same as Correctness

Every function returning a value for every input is a weaker property than it
sounds, and the crate is careful not to oversell it. The saturating cases return
*a* value, and that value is deliberately indistinguishable from a legitimate one:

| Input | Reading | Also produced by |
|-------|---------|------------------|
| consumer ahead of producer | `pending` → `0` | a ring that is fully caught up |
| consumer ahead of producer | `free_slots` → `capacity` | a ring that has published nothing |
| consumer ahead of producer | `may_claim` → `true` | a ring with room |

So totality here buys *no panic*, not *no confusion*. The confusing state is
unreachable through correct use and is detected from outside the crate —
`ring_debug::Violation::ConsumerAheadOfProducer` exists for it. See
[`decisions/002`](../decisions/002_saturating_rather_than_signed.md) and
[`lifecycle/001`](../lifecycle/001_one_pair_across_one_lap.md).

### What Totality Buys Downstream

It is the reason none of the five call sites in the family handle an error:

| Caller | Call | Error handling |
|--------|------|----------------|
| `ring_cursor:370` | `free_slots(…)` | none — returns `usize` directly |
| `ring_cursor:389` | `pending(…)` | none |
| `ring_cursor:419` | `may_claim(…)` | none |
| `ring_gating:226` | `free_slots(…)` | none — inside a `map_or` |
| `ring_batch:323` | `free_slots(…)` | none — the *caller* decides `RingError::Full` |
| `ring_consume:342` | `pending(…)` | none — inside a `map_or` |

`ring_batch` is the shape to notice: the arithmetic is total, and the *policy*
that a full ring is an error lives in the caller, not here. A `free_slots`
returning `Result` would have forced every one of these six sites to make a
decision it does not have the context to make.

### SQ24 — Totality Borrowed From One Tier Down

The only division in the crate is total, and the reason is not in the crate:

```
ring_seqno/src/lib.rs:52   earlier.distance_to( later ) / capacity.get() as u64
                                                         ^ never zero, because
ring_types/src/capacity.rs   Capacity::new rejects zero and non-powers-of-two
```

**Finding.** The division in `laps_between` is total only because `Capacity::new` rejects zero — a guarantee from another crate, with nothing local restating it.

---

### SQ25 — Totality Stated in the Signatures

Five signatures, no `Result`, and the one `Option` is not a failure channel:

```
laps_between -> u64            total
may_claim    -> bool           total
free_slots   -> usize          total
pending      -> u64            total
slowest      -> Option< Seq >  total; None is an empty input, not an error
```

**Finding.** No function returns `Result` and the crate's single `Option` encodes an absent input rather than a failure, so every reading is total in the strict sense and the type signatures say so without a word of prose.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_five_functions_and_no_types.md](../api/001_five_functions_and_no_types.md) | The surface, with its zero `Result`s |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_saturating_rather_than_signed.md](../decisions/002_saturating_rather_than_signed.md) | Why saturation rather than a signed distance |

### Invariants

| File | Relationship |
|------|--------------|
| [001_the_sequence_is_never_folded_here.md](001_the_sequence_is_never_folded_here.md) | The same claim from the information-loss side |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_one_pair_across_one_lap.md](../lifecycle/001_one_pair_across_one_lap.md) | Where the saturating readings go blind |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) | A way the readings can disagree that totality does not prevent |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:52` | The unguarded division — T3 |
| `ring_seqno/src/lib.rs:97-98` | The one local saturation — T2 |
| `ring_types/src/capacity.rs:22-51` | Private field plus checked constructor — what makes T3 safe |
| `ring_types/src/id.rs:25` | `Seq`'s public field, and why it is right |
| `ring_types/src/id.rs:32-47` | SQ44 — the release-mode claim that does not hold |
| `ring_types/src/id.rs:70-85` | T1, correctly documented |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:92-102` | `free_slots` saturates rather than wrapping — T2 |
| `tests/seq_test.rs:104-111` | `pending` of a backwards pair — T1 |
| `tests/seq_test.rs:125-132` | The empty slice — T4 |
| `tests/seq_test.rs:18-21` | `cap()`, the helper that hides T3's constructor behind an `expect` |
