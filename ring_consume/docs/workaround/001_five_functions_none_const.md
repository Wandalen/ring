# Workaround: Five Functions, None `const`

### Scope

**Purpose:** Record two things this crate works around rather than fixes, both
owned by dependencies: `ring_seqno`'s missing `const`, and `Seq`'s tuple-field
escape hatch — and, since the second was recorded wrongly, what an audit of
that escape actually finds when it is run again.

**Responsibility:** Constraints imposed on `ring_consume` from below, the shape
each forces, and what would have to change to remove them.

**In Scope:** `ring_seqno`'s five public functions; `Seq`'s `.0`; the sites in this
crate that reach for each; the classification of the 84 `.0` escapes across the
family into `const`-forced, `Range`-forced, and avoidable; and the state of
`ring_trace` TR49/TR50, which had already reached the avoidable one.

**Out of Scope:** This crate's own `const` surface, which is complete —
[`type/001`](../type/001_availables_const_surface.md). The allocation, which is
[`002`](002_a_vector_to_change_a_slices_type.md).

---

### CN51 — Four of `ring_seqno`'s Five Functions Could Be `const` Today, With Their Bodies Unchanged

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -E '^\s*pub (const )?fn' ring_seqno/src/lib.rs
command grep -rE '^\s*pub (const )?fn' ring_types/src/ | sed 's|ring_types/src/||'
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
error.rs:  pub const fn is_configuration( self ) -> bool
error.rs:  pub const fn is_transient( self ) -> bool
capacity.rs:  pub const fn new( slots : usize ) -> Result< Self, RingError >
capacity.rs:  pub const fn get( self ) -> usize
capacity.rs:  pub const fn mask( self ) -> usize
id.rs:  pub const fn next( self ) -> Self
id.rs:  pub const fn advanced_by( self, n : u64 ) -> Self
id.rs:  pub const fn distance_to( self, later : Self ) -> u64
id.rs:  pub const fn get( self ) -> usize
policy.rs:  pub const fn is_non_blocking( self ) -> bool
policy.rs:  pub const fn reports_failure( self ) -> bool
policy.rs:  pub const fn drops_silently( self ) -> bool
```

The first five lines are `ring_seqno`, the crate `ring_consume` calls for every
arithmetic decision: zero `const` of five. The twelve below them are
`ring_types`, the crate directly beneath it: twelve `const` of twelve, with no
bare `pub fn` among them. Every primitive `ring_seqno` builds on is already
`const`, and `ring_seqno` is where it stops.

That contrast is the whole finding, and it is one list read in two halves —
which is why it is not restated here as a second, numbered copy. It was, until
this correction: two more fenced blocks repeated the same twenty-two signatures
with line numbers attached, and by the time anyone re-read them the addresses
were eighteen lines out in `error.rs` and one out in `id.rs`. Nothing detected
it, because only the block labelled `Live output:` above is checked against a
re-run.

Four of the five bodies need no change at all. Compiled verbatim with `const`
added and evaluated in `const` position, so the compiler is the witness:

```rust
const CAP     : Capacity = match Capacity::new( 64 ) { Ok( c ) => c, Err( _ ) => panic!() };
const LAPS    : u64      = laps_between( Seq( 0 ), Seq( 640 ), CAP );
const CLAIM   : bool     = may_claim( Seq( 10 ), Seq( 0 ), CAP );
const FREE    : usize    = free_slots( Seq( 10 ), Seq( 0 ), CAP );
const PENDING : u64      = pending( Seq( 300 ), Seq( 100 ) );
```

```
all four evaluated at compile time:
  laps_between  = 10
  may_claim     = true
  free_slots    = 54
  pending       = 200
```

They compose only `distance_to` (`const`), `Capacity::get` (`const`),
`saturating_sub` (`const`), a division and a comparison. Nothing blocks them.

The fifth genuinely cannot:

```
error: `core::slice::<impl [T]>::iter` is not yet stable as a const fn
error[E0015]: cannot call non-const method `<Copied<Iter<'_, Seq>> as Iterator>::min`
```

So the split is 4 / 1, not 0 / 5, and the one exception has a real reason.

**The honest accounting of what this costs *this* crate is: nothing.**
`ring_consume` calls `pending` from `available()`, which loads atomics and could
never be `const` regardless ([`type/001`](../type/001_availables_const_surface.md)
CN47). Making `pending` `const` would not make one function here `const`.

What it costs is at the boundary — compile-time capacity arithmetic
(`const SLOTS : usize = free_slots( … )`, a `static_assert` that a configured
capacity admits a given batch) is unavailable to every caller in the family, for
no reason anyone recorded. This is filed as a workaround rather than a defect
because `ring_consume`'s response is simply to call the functions at runtime,
which is correct and costs it nothing; the finding is that the response is
forced rather than chosen.

**Cost:** none reachable here; a lost capability at the `ring_seqno` boundary.
Recorded because the audit that
[`type/001`](../type/001_availables_const_surface.md) passes cleanly for this
crate fails one level down, and the comparison is what makes it visible.

---

### CN52 — `.0` Is the Only Way Out of a `Seq`, and 15 Files Take It

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rE 'impl.*(From|Deref|Into).*Seq' ring_types/src/id.rs || echo "  none"
printf 'escapes across the family: %s in %s file(s)\n' \
  "$( command grep -rho '[a-z_]*\.0' ring_*/src/*.rs | wc -l )" \
  "$( command grep -rl '\.0' ring_*/src/*.rs | wc -l )"

# where they sit in the crate this finding compares against. addressed by
# content, not by line: `ring_batch` moved eleven lines under a concurrent
# edit during one session of this audit, and the addresses this block used to
# quote survived the move looking perfectly plausible
command grep '\.0' ring_batch/src/lib.rs

# `contains` and `overlaps` are both `pub const fn`. does the derived `Ord`
# they are accused of avoiding actually compile in that position?
probe_dir=$( mktemp -d )
cat > "$probe_dir/probe.rs" <<'RS'
#[ derive( Clone, Copy, PartialEq, Eq, PartialOrd, Ord ) ]
pub struct Seq( pub u64 );
// what ring_batch::contains writes
pub const fn via_field( a : Seq, b : Seq ) -> bool { a.0 >= b.0 }
// what a derived Ord would write in its place
pub const fn via_derived_ord( a : Seq, b : Seq ) -> bool { a >= b }
RS
rustc --edition 2021 --crate-type lib --out-dir "$probe_dir" "$probe_dir/probe.rs" 2>&1 \
  | command grep -E '^error\[|not `const`' | sed 's|^|  |'
rm -rf "$probe_dir"

# the one escape that is avoidable, against the const constructor that already
# does exactly what it open-codes
command grep -A3 'pub const fn end' ring_batch/src/lib.rs
command grep -A3 'pub const fn advanced_by' ring_types/src/id.rs
```

Live output:

```
  none
escapes across the family: 84 in 15 file(s)
    /// In a debug build, if `start.0 + count` overflows `u64` — unreachable via
        Seq(self.start.0 + self.count as u64)
        seq.0 >= self.start.0 && seq.0 < self.end().0
        (self.start.0..self.end().0).map(Seq)
            && self.start.0 < other.end().0
            && other.start.0 < self.end().0
  error[E0015]: cannot call non-const operator in constant functions
  note: impl defined here, but it is not `const`
    pub const fn end(&self) -> Seq {
        Seq(self.start.0 + self.count as u64)
    }

    pub const fn advanced_by(self, n: u64) -> Self {
        Self(self.0 + n)
    }
```

`Seq` implements no `From< Seq > for u64`, no `Deref`, no `Into`. The tuple field
is `pub` and it is the only exit. Eighty-four escapes across fifteen source files.

`ring_consume` takes it in exactly one expression, and cannot avoid it:

```rust
pub fn sequences( self ) -> impl Iterator< Item = Seq >
{
  ( self.start.0..self.end().0 ).map( Seq )
}
```

A `Range` needs an integer, `Seq` is not `Step`, and there is no conversion — so
`.0` out and `Seq` back in. One escape, structurally required, wrapped
immediately. That is the right way to use the hatch and `ring_claim` writes the
identical line for the identical reason.

`ring_batch` looked like the counter-example, and this finding said so from the
day it was written. Eight of its twelve escapes sit on the two comparison lines
printed above, in `contains` and `overlaps`, and `Seq` derives `PartialOrd, Ord`
([`type/001`](../type/001_availables_const_surface.md) CN47) — so
`seq >= self.start && seq < self.end()` looks available, shorter, and type-safe.

It is not available. Both methods are `pub const fn`, and a derived `PartialOrd`
is not a `const` impl. The probe above compiles the two forms side by side and
the derived one does not build:
`error[E0015]: cannot call non-const operator in constant functions`. Dropping to
the integer is what makes those methods `const` at all — the escape buys
compile-time evaluation, which is the same currency CN51 above spends fifteen
lines arguing `ring_seqno` should be collecting.

The finding was wrong in the direction that is hardest to notice: it accused
correct code, using a technique — compile it and see — that its own sibling
finding on this page already had running. Pointing CN51's method one crate
sideways refutes CN52 in two lines.

One escape in `ring_batch` **is** avoidable, and it is not among the eight:
`end`'s `Seq( self.start.0 + self.count as u64 )`. `Seq::advanced_by` is
`pub const fn` whose whole body is `Self( self.0 + n )` — both are printed
above. Same wrapping, same debug overflow panic, legal in `const` position, one
escape fewer. It is `ring_batch`'s change to make and is recorded here, not
made here.

So the distinction the family actually has is not legitimate-versus-gratuitous.
It is `const`-forced (eight), `Range`-forced (two crates, one line each,
re-wrapped immediately), and open-coded-where-a-constructor-exists (one). All
three read identically to `grep`, which is why counting them was never the same
as judging them.

The deletion condition for the `Range` use is narrow and worth naming: an
`impl Step for Seq` — which is nightly-only — or an inherent
`Available::sequences` that iterates by repeated `advanced_by` rather than by
`Range`. The latter is available on stable today and would remove this crate's
only escape.

**Cost:** none reachable here — this crate's escape is correct and immediately
re-wrapped. Recorded because the escape exists at all; kept because its first
attempt at separating good uses from bad got the separation backwards, and the
correction is what names the real one.

---

### CN55 — The Remedy for the One Avoidable Escape Was Declined by the Crate That Recorded It

CN52 missed the one `.0` escape in the family that is avoidable —
`ring_batch::end`'s `Seq( self.start.0 + self.count as u64 )`, where
`Seq::advanced_by` is a `pub const fn` doing that arithmetic under a name. That
much is not this finding, and is not new: `ring_trace`'s corpus already records
it as TR49 and TR50, naming the same expression and the same substitution.

What is new is what happened to that recommendation afterwards.

```sh
cd "$(git rev-parse --show-toplevel)"
# what TR49 says `end` writes out by hand, against what `end` writes
command grep -A3 -F 'pub const fn advanced_by' ring_types/src/id.rs
command grep -A3 -F 'pub const fn end( &self ) -> Seq' ring_trace/src/lib.rs
command grep -m1 -F 'Saturates rather than wrapping' ring_trace/src/lib.rs

# the crate TR50 aims the same substitution at, which still writes the operator
command grep -A3 -F 'pub const fn end( &self ) -> Seq' ring_batch/src/lib.rs

# every mention of saturation in the instance that recommends the substitution.
# printed rather than counted: the one hit is a link whose *filename* carries
# the word, which a count would let a reader mistake for an acknowledgement
command grep -o 'saturat[a-z]*' \
  ring_trace/docs/workaround/001_an_addition_the_types_crate_already_offers.md \
  | sed 's/^/  /' || echo "  none"
command grep -c 'saturating_add\|^\*\*Disposition:\*\*' \
  ring_trace/docs/workaround/001_an_addition_the_types_crate_already_offers.md \
  | sed 's/^/  saturating_add or Disposition lines: /'
```

Live output:

```
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
  pub const fn end( &self ) -> Seq
  {
    Seq( self.seq.0.saturating_add( self.count as u64 ) )
  }
  /// Saturates rather than wrapping: a bare `+` here would print a range that
  pub const fn end( &self ) -> Seq
  {
    Seq( self.start.0 + self.count as u64 )
  }
  saturating
  saturating
  saturating
  saturates
  saturating_add or Disposition lines: 1
```

TR49 states that `advanced_by`'s body "is the same `Self( self.0 + n )` that
`TraceEntry::end` writes out by hand," and prescribes
`self.seq.advanced_by( self.count as u64 )` as a direct substitution.
`TraceEntry::end` does not write that. It reads `saturating_add`, and its own
source comment gives the reason: a bare `+` prints a backwards range or panics
under debug assertions the moment a caller traces `ring_mpsc::UNSTAMPED`, which
is `Seq( u64::MAX )` and is the one `Seq` the family publishes by name. So
`ring_trace` reached its own recommendation, declined it, and solved the problem
the other way — and the prescribed substitution would now *undo* that, because
`advanced_by` is the bare `+`.

The finding's own quoted output already showed this. TR49's recipe prints
`end`'s doc comment, saturation sentence included, directly above the prose
saying `end` writes a bare addition. The block is regenerated and current; the
paragraph reading it is not. That is the same divergence this corpus keeps
finding between a command's output and the sentence introducing it, occurring
here inside the machinery built to catch it.

TR50 carries the wider claim — `ring_batch` and `ring_trace` writing the
identical expression "character for character" — and half of it is now false for
the same reason. `ring_batch` still writes the operator; the two are no longer
the same line.

None of this makes `advanced_by` the wrong answer for `ring_batch`. Whether
`Seq( u64::MAX )` reaches a *claim* range is a different question from whether it
reaches a trace, and `ring_batch`'s own BA44 already prices the release wrap. It
makes the recommendation unannotated: the crate that wrote it went the other
way, in source, for a stated reason, and the instance records no Disposition and
never names `saturating_add`.

CN52's own miss has a separate and duller cause, worth keeping. It searched for
a *syntax* — `.0` in a comparison — and graded what it found by whether a trait
method existed to replace it. That question answers "yes" for all eight forced
escapes, and they stay forced, because trait methods and `const fn` do not mix.
The avoidable site needs a different question: not *is there a trait for this*,
but *is there already a constructor doing exactly this arithmetic*. `ring_trace`
asked that one and found the site immediately.

**Cost:** reachable, in another crate's corpus. A reader who applies TR49 to
`ring_trace` reintroduces the panic TR41 is about; a reader who applies TR50 to
`ring_batch` removes one field access and leaves BA44 exactly where it was,
while the change reads like a fix.

**Deletion condition:** TR49 and TR50 re-read against `end`'s current body, with
a Disposition recording that `ring_trace` chose saturation over the named method
and why. The instance belongs to `ring_trace`; this crate has no stake in it
beyond having arrived from the other direction and found the two disagreeing.

**Disposition:** declined — the remedy belongs to `ring_trace`'s own corpus, not
to `ring_consume`. `ring_trace/docs/workaround/001_an_addition_the_types_crate_already_offers.md`'s
TR49 and TR50 are the instances that would carry a correction naming
`TraceEntry::end`'s `saturating_add`; this crate performs no substitution and
has no artefact of its own for a fix to land in.

---

## Both Workarounds, Summarised

| Constraint | Owner | This crate's response | Removable by |
|------------|-------|-----------------------|--------------|
| `ring_seqno` is not `const` | `ring_seqno` | call at runtime — costs nothing here | adding `const` to four bodies, unchanged |
| `Seq` has no conversion | `ring_types` | `.0` in one expression, re-wrapped immediately | `Step` (nightly) or an `advanced_by` loop |

Neither is a defect in `ring_consume`, and neither is recorded anywhere else.
CN55 is not a third row because it is not a constraint this crate absorbs: it is
a defect in another crate's *documentation* of the second constraint, found only
because auditing that constraint required classifying every escape in the family
rather than counting them, and the classification led to a finding two crates
away that had already made the same audit and been overtaken by its own code.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| workaround | [002](002_a_vector_to_change_a_slices_type.md) | the third constraint, and the only one that costs |
| type | [001](../type/001_availables_const_surface.md) | the audit this crate passes and `ring_seqno` fails |
| item | [001](../item/001_the_six_of_a_run.md) | `sequences`, the site of the one escape |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | where `pending` is called at runtime |
| `ring_trace` workaround | [001](../../../ring_trace/docs/workaround/001_an_addition_the_types_crate_already_offers.md) | TR49/TR50 — the same avoidable escape, reached first and overtaken by its own crate |

### Sources

| What | Where |
|------|-------|
| `ring_seqno`'s five | `ring_seqno/src/lib.rs`, the five `pub fn` items — none carries `const` |
| `ring_types`' twelve | `ring_types/src/{capacity,policy,error,id}.rs`, every `pub const fn` in them |
| This crate's escapes | `ring_consume/src/lib.rs`, `sequences`' `( self.start.0..self.end().0 )` and the doctest above it |
| The `const`-forced ones | `ring_batch/src/lib.rs`, the comparisons inside `pub const fn contains` and `pub const fn overlaps` |
| The avoidable one | `ring_batch/src/lib.rs`, `end`'s `Seq( self.start.0 + self.count as u64 )` |
| The constructor the recorded remedy names | `ring_types/src/id.rs`, `pub const fn advanced_by` |
| The finding that recorded it | `ring_trace/docs/workaround/001_an_addition_the_types_crate_already_offers.md`, TR49 and TR50 |
| The body that declined it | `ring_trace/src/lib.rs`, `end`'s `saturating_add` and the comment above it |

### Tests

| Claim | Verified by |
|-------|-------------|
| Four bodies are const-able unchanged | the four `const` items evaluated at compile time above |
| The fifth is not | the two compile errors quoted above |
| `ring_types` is 12/12 `const` | the recursive grep over its four source files |
| `Seq` has no conversion impl | the `From`/`Deref`/`Into` grep returning nothing |
| 84 escapes in 15 files | the two counting greps above |
| `Seq` derives `Ord` | `ring_types/src/id.rs`, the `derive` list on `Seq` — which is why the escapes look gratuitous |
| A derived `PartialOrd` is not `const` | the `rustc` probe above, printing `error[E0015]` for `seq >= self.start` in a `const fn` |
| `ring_batch::end` open-codes `advanced_by` | the two bodies printed side by side above |
| `ring_trace::end` does not, though TR49 says it does | `end`'s body and its saturation comment, printed in CN55's block |
