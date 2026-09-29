# Invariant: Received Never Exceeds Reported Never Exceeds Offered

### Scope

- **Purpose**: State the one count ordering that holds for every candidate under every overflow policy, and establish that nothing stronger is available.
- **Responsibility**: State the property, why it holds per candidate, and how it is checked.
- **In Scope**: The two inequalities; the reasons each holds; the stronger property that does not hold and the candidate that breaks it.
- **Out of Scope**: What the gap between the terms means (→ [`data_structure/002`](../data_structure/002_three_counts_that_are_not_interchangeable.md)); the counters' own consistency (→ [`invariant/002`](002_the_counters_are_written_outside_the_clock.md)).

### Invariant Statement

**For every candidate, under every workload and every overflow policy:**

```text
received  ≤  reported  ≤  offered
```

**And nothing stronger.** In particular `reported == received` does *not* hold —
it is a property of the configuration, reported by `Outcome::conserved`, not a
property of a correct run.

#### Why each inequality holds

**`reported ≤ offered`** — a runner cannot report taking a record it was never
given. Structurally guaranteed for four of the six runners: they iterate
`workload.records_of( i )` exactly once and increment only on a success, so the
count is bounded by the iteration length. The other two — `run_contract_ring`
and `run_off_the_shelf` — hand a `Range< Record >` straight to
`try_push_batch` and add its return value; the bound there is
`ring_handle::Producer::try_push_batch`'s own documented contract (consume at
most what the iterator yields), not this crate's iteration (→ BN21). No
candidate has a retry loop
(→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md), on
why the staged candidate deliberately does not retry a rejection).

**`received ≤ reported`** — the drain cannot produce a record the write path
never accepted. This one is *not* structural in this crate; it rests on every
candidate's own correctness, and it is therefore the more interesting half:

| Candidate | Why the drain cannot exceed the pushes |
|---|---|
| `MutexQueue` | One `VecDeque`, pushed under the lock, drained after the scope joins |
| `ContractRing`, `OffTheShelf` | `try_push_batch` returns how many it consumed from the iterator; the consumer sees only published slots |
| `TlsOverRing` | `Flusher` reports what it published; staged-but-unflushed records never reach the ring |
| `DirectSpsc`, `DirectMpsc` | Per-record `try_push`, counted on `Ok` only |

#### The stronger property that does not hold

**`reported == received` fails, legitimately, for `ContractRing` and
`OffTheShelf` under `OverflowPolicy::DropNewest`** — the family's default. A
full ring returns `Ok` for a discarded record, so the middle term equals the
last while the first is a fraction of it:

```text
offered 256  reported 256  received 16      # contract_ring, 16-slot ring
```

This was asserted as an invariant in the crate's first version and it broke on
the first run against the default policy. The distinction that resolves it — a
property the run must have, versus a property of the configuration the run is
reporting on — is
[`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md)'s
subject.

### Enforcement Mechanism

**Asserted directly, per candidate, on the fixture where the terms actually
diverge** — the cramped one. A roomy fixture satisfies the ordering trivially
(all three equal) and would pass with the counts wired to any single source:

```rust
assert!( outcome.reported() <= 256 );
assert!( outcome.received() <= outcome.reported() );
assert!( outcome.received() <= 16 );          // the ring holds 16
```

### Violation Consequences

**A violation would mean a ring produced a record nobody wrote**, which is a
defect in the candidate rather than in this harness — and asserting the
inequality here is how this crate would notice one. That is the invariant's real
job: it is a cross-check on five other crates, phrased as a property of this
one's output.

**`dropped()` and `silently_discarded()` are both defined as differences of
these terms**, so the invariant is what keeps either from underflowing. Both are
`usize` subtractions; a violation of the ordering is a panic in release as well
as debug, which is the intended behaviour — a harness that produced a wrapped
`dropped` count would report `18446744073709551360` and be obviously broken
rather than subtly so.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | The shared shape that makes `reported ≤ offered` structural |
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | `is_lossless`, which is well-defined only because the ordering holds |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | The three terms and the derived quantities that subtract them |

### Invariants

| File | Relationship |
|------|--------------|
| [002_the_counters_are_written_outside_the_clock.md](002_the_counters_are_written_outside_the_clock.md) | The other property, about when rather than what |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) | The stronger property that does not hold, and why the harness assumed it did |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `drops_silently()` — the predicate that decides whether the two inequalities can be strict at once |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_cramped_run_drops_and_the_drop_is_counted_from_the_drain` asserts both inequalities for every candidate on the fixture where they diverge |

### BN21 — The "Structural" Half Is Not Structural for the Two Runners That Need It Most

`reported ≤ offered` is claimed above as *structurally guaranteed*, on the
grounds that "every runner iterates `workload.records_of( i )` exactly once and
increments only on a success, so the count is bounded by the iteration length".
Two of the six do not iterate it at all:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- which runners iterate records_of(), which do not --'
awk '/^fn run_[a-z_]+\(/ { n = $2; sub( /\(.*/, "", n ); has = "no"; next }
     n != "" && /records_of\(/ { has = "yes" }
     n != "" && /^\}/ { printf "    %-20s iterates records_of() : %s\n", n, has; n = "" }' src/lib.rs
echo '  -- what bounds the count in the other two --'
command grep -n 'try_push_batch' src/lib.rs | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and whose function that is --'
command grep -n 'pub fn try_push_batch' ../ring_handle/src/lib.rs ../ring_core/src/lib.rs \
  | sed -E 's/^(.{0,96}).*/\1/' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- which runners iterate records_of(), which do not --
    run_mutex_queue      iterates records_of() : yes
    run_contract_ring    iterates records_of() : no
    run_tls_over_ring    iterates records_of() : yes
    run_direct_spsc      iterates records_of() : yes
    run_direct_mpsc      iterates records_of() : yes
    run_off_the_shelf    iterates records_of() : no
  -- what bounds the count in the other two --
        reported += producer.try_push_batch( &mut records );
        reported += producer.try_push_batch( &mut records );
  -- and whose function that is --
    ../ring_handle/src/lib.rs:  pub fn try_push_batch( &mut self, records : &mut impl Iterator< 
    ../ring_core/src/lib.rs:  pub fn try_push_batch( &mut self, records : &mut impl Iterator< It
```

`run_contract_ring` and `run_off_the_shelf` build a `Range< Record >` inline and
hand it to `try_push_batch`, adding the returned count. Nothing in this crate
bounds that return value. The bound is `ring_handle::Producer::try_push_batch`'s
own contract — *consume at most what the iterator yields, return how many* — and
that function belongs to `ring_handle`, which is not even a declared dependency
of this crate (→ [`integration/002`](../integration/002_the_only_consumer_of_two_contract_names.md)).

**The document already knows this and says it in the other direction four lines
below.** The `received ≤ reported` table's row for `ContractRing, OffTheShelf`
reads "`try_push_batch` returns how many it consumed from the iterator" — an
explicit delegation to another crate's contract, presented under the inequality
that the prose calls *non-structural*. So one paragraph claims the first
inequality is structural in this crate and the table under the second inequality
records that the same two runners get theirs from elsewhere. Both are describing
the same call.

The correction is small and the reason it matters is not: **"structurally
guaranteed" and "guaranteed by a dependency's documented contract" are different
strengths of claim**, and this invariant's whole stated job is to be "a
cross-check on five other crates". A cross-check that inherits its bound from one
of the five crates it is checking is not checking that crate — it is agreeing
with it.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 "not this crate's iteration" ring_bench/docs/invariant/001_received_never_exceeds_reported_never_exceeds_offered.md
```

Live output:

```
most what the iterator yields), not this crate's iteration (→ BN21). No
```

**Disposition:** applied — the "Why each inequality holds" paragraph now
names the split directly: four runners get the bound from their own iteration,
`run_contract_ring` and `run_off_the_shelf` get it from `ring_handle`'s
documented contract instead, matching what the `received ≤ reported` table
four lines below already said about the same two runners.
Now prints: `not this crate's iteration`

### BN22 — The Underflow the Invariant Protects Against Wraps Rather Than Panics, in the Profile Nobody Configured

Violation Consequences argues the ordering is what keeps `dropped()` and
`silently_discarded()` from underflowing, and that this is safe because "a
violation of the ordering is a panic in release as well as debug". Rust panics on
integer overflow in release only when `overflow-checks` is on, and it is not:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two subtractions --'
command grep -A 3 'pub const fn dropped\|pub const fn silently_discarded' ring_bench/src/lib.rs \
  | command grep -E 'self\.|fn ' | sed 's/^/    /'
echo '  -- overflow-checks settings --'
printf '    occurrences in the workspace manifest      : %s\n' "$( command grep -c 'overflow-checks' Cargo.toml )"
printf '    occurrences in ring_bench/Cargo.toml : %s\n' "$( command grep -c 'overflow-checks' ring_bench/Cargo.toml )"
printf '    occurrences anywhere under ring_*    : %s\n' "$( command grep -rho 'overflow-checks' ring_*/Cargo.toml | wc -l )"
```

Live output:

```
  -- the two subtractions --
      pub const fn dropped( &self ) -> usize
        assert!( self.received <= self.offered, "received exceeded offered" );
        self.offered - self.received
      pub const fn silently_discarded( &self ) -> usize
        assert!( self.received <= self.reported, "received exceeded reported" );
        self.reported - self.received
  -- overflow-checks settings --
    occurrences in the workspace manifest      : 0
    occurrences in ring_bench/Cargo.toml : 0
    occurrences anywhere under ring_*    : 0
```

Both **were** `usize` subtractions in `const fn` bodies with no guard — the
`assert!` above each is the fix, quoted here from the source it now guards. In a
debug build the ordering's violation was a panic, as claimed. In a release build
it wrapped, and the value produced was the one the same sentence offers as the
*counterfactual*: `18446744073709551360`. The paragraph named the wrapped result
as what does not happen, and it was what happened in the profile a benchmark
crate is most likely to be run under.

**The reasoning was right and the premise was inherited.** "Release panics on
overflow" is true of a project that sets `overflow-checks = true`, and false by
default; the workspace sets it nowhere. Nothing in this crate could have caught
that — no test runs in release, and a `debug_assert` would be identical to the
current behaviour.

**The invariant is fine. Its enforcement claim is what is wrong**, and the
distinction is the useful one: the ordering does hold, so the underflow does not
occur, so nothing observable is broken today. What is broken is the argument for
why a violation would be *loud*, and that argument is what licenses leaving both
subtractions unguarded.

The general shape: **a claim about what a language does under a build profile is
a claim about configuration, not about the language** — and it is the kind of
claim that reads as a fact, is repeated confidently, and is checked by nothing
because it is not about the code.

**Disposition:** applied — the guard was written rather than the setting turned
on. Both accessors now open with an unconditional `assert!` on their half of the
ordering, so the panic is a property of this crate rather than of a profile
nobody configured; `overflow-checks` is still absent everywhere, and that is now
irrelevant to the claim. Turning it on workspace-wide was the alternative and was
rejected: it changes the arithmetic of 33 crates to fix an argument made in one,
and it would leave the enforcement claim resting on a setting a downstream
profile can still override. `both_ordering_subtractions_are_guarded_unconditionally`
pins both guards, additionally asserts that neither has been softened to a
`debug_assert!` — which would reinstate exactly the profile dependence this
finding names — and asserts this crate's manifest still does not set
`overflow-checks`, so if that ever changes the test says to revisit the
mechanism. It was proven able to fail by downgrading one guard to a
`debug_assert!`. What this does not buy: the guards cannot be reached from a
test. `Outcome`'s fields are private and `run` is the only constructor, so no
suite can build a violating value — the test checks that the guards are present,
not that they fire, and the finding's own point stands that the ordering holding
is what makes this moot today. Now prints: `occurrences anywhere under ring_*    : 0`

### BN53 — The Same Subtraction Has a Third Call Site, and BN22's Guard Does Not Reach It

BN22 guarded `Outcome::dropped` and `Outcome::silently_discarded` — the two
accessors that expose `offered - received` and `reported - received` after a
run finishes. `run` itself computes the first of those two subtractions
earlier, inline, to feed `RingStats::record_drop`, and that call site is not
either accessor:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two accessors BN22 guarded --'
command grep -A2 'pub const fn dropped\|pub const fn silently_discarded' ring_bench/src/lib.rs \
  | command grep -E 'assert!|fn ' | sed 's/^/    /'
echo '  -- the third call site, before this fix (git HEAD) --'
git show HEAD:ring_bench/src/lib.rs | command grep -B1 'stats.record_drop' | sed 's/^/    /'
echo '  -- the third call site, after this fix (working tree) --'
command grep -B1 'stats.record_drop' ring_bench/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the two accessors BN22 guarded --
      pub const fn dropped( &self ) -> usize
        assert!( self.received <= self.offered, "received exceeded offered" );
      pub const fn silently_discarded( &self ) -> usize
        assert!( self.received <= self.reported, "received exceeded reported" );
  -- the third call site, before this fix (git HEAD) --
      assert!( received <= offered, "received exceeded offered" );
      stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
  -- the third call site, after this fix (working tree) --
      assert!( received <= offered, "received exceeded offered" );
      stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
```

**That before/after contrast has since collapsed.** The fix was committed
after this section was written, so `git show HEAD` now returns the same
guarded line as the working tree — both blocks above read identically. The
historical narrative below still describes what actually changed; it is no
longer demonstrated live by this recipe, only asserted.

Before this fix, `run` built `RingStats` from the unguarded subtraction and
only *afterward* constructed the `Outcome` whose two accessors BN22 had
already hardened. A violation reaching that earlier line would have wrapped
`( offered - received ) as u64` to `18446744073709551615` and handed it to
`record_drop` — corrupting `RingStats` — before either guarded accessor ever
ran, in the same silent-instead-of-loud failure shape BN22 named. A standalone
probe reproducing the exact expression outside this crate confirmed the
wraparound value empirically before this fix was written, and confirmed the
guarded form panics instead on the same inputs.

**BN22's own scope note said as much without drawing the conclusion**: its
fix pinned "both ordering subtractions" — naming the two it guarded — and its
test is titled `both_ordering_subtractions_are_guarded_unconditionally`, a
title that is accurate about what it checks and silent about there being a
third subtraction elsewhere performing the same arithmetic on the same two
variables one scope up.

**Disposition:** applied — `run` now asserts `received <= offered` immediately
before the `record_drop` call, with the identical message as
`Outcome::dropped`'s own guard. `the_record_drop_input_is_guarded_before_the_subtraction_runs`
pins the guard's text and its position ahead of the call it protects, and — as
with BN22 — cannot make the guard fire: `Outcome`'s fields stay private and
`run` stays the only constructor, so the ordering holds on every reachable
path and the new assertion is, today, exactly as moot as the two it matches.
That is the same tradeoff BN22 accepted, extended to the call site BN22's own
title described but did not cover. Now prints:
`assert!( received <= offered, "received exceeded offered" );`
