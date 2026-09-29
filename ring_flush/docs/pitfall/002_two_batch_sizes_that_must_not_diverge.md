# Pitfall: Two Batch Sizes That Must Not Diverge

### Scope

- **Purpose**: Record that `OnBatch( n )` and `ring_batch`'s claim width are two different numbers that read as one, are tuned by different people against different criteria, and have no shared test.
- **Responsibility**: Name the trap, the failures it produces, and the mitigations available from this crate.
- **In Scope**: The `n` in `OnBatch( n )`; the 64 in `ring_batch`'s acceptance criterion; the relationship between them.
- **Out of Scope**: `ring_batch`'s own claim mechanics; the throughput/latency trade itself.

### Trap

**Two numbers in this family are both called the batch size.**

| | `OnBatch( n )` | `ring_batch`'s claim width |
|---|---|---|
| Owned by | This crate | `ring_batch` |
| Means | How many records accumulate before a flush is triggered | How many slots one claim reserves in one fence |
| Tuned against | Tail latency — how long the unluckiest record waits | Fence amortisation — one fence per claim rather than per slot |
| Named in the acceptance table as | "`OnBatch(n)` … fires at exactly its stated trigger" | "A claim of 64 slots issues one fence, not 64" |
| Chosen by | Whoever configures the policy, per ring | Whoever calls `claim()`, at every call site — no stored value exists to be "chosen once" (→ `FL43` below) |

**They are related but not equal, and the relationship is not stated
anywhere.** A flush of `n` records has to be claimed from the ring; if the
claim width is 64 and `n` is 100, the flush becomes two claims and two fences,
which is precisely the cost `OnBatch` was tuned to amortise. If `n` is 10, the
claim is under-filled and the fence is paid for a tenth of its capacity.

**The trap is that both are "the batch size" in conversation** and a reader who
tunes one has no signal that the other exists. `ring_batch`'s row names 64
concretely; this crate's row leaves `n` symbolic. Nothing connects them.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| G1 | `n` is set larger than the claim width | Each flush costs ⌈n / width⌉ fences. `OnBatch` measures worse than expected and the cause is attributed to the policy rather than the mismatch |
| G2 | `n` is set smaller than the claim width | Fence cost per record rises. The benchmark shows batching helping less than hard problem 122 predicts, and the prediction is blamed |
| G3 | There is no single claim width to change — call sites can pass different widths within the same build | Every `OnBatch` measurement is silently incomparable unless every call site's width is recorded alongside it. **Nothing records which width was in effect for a result** (→ `FL43` below) |
| G4 | `n` exceeds the buffer's capacity | `OnBatch` can never fire; the buffer fills and either overflows or falls back to `OnFull` behaviour (→ [trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s V2) |
| G5 | Someone "helpfully" defines `OnBatch`'s default as the claim width | The two numbers become coupled by a default rather than by a stated rule, and the coupling is invisible at every call site that accepts the default |

**G3 is the one that corrupts the benchmark's output rather than the
program.** Measured verdicts across this family depend on being reproducible;
a verdict that depends on an unrecorded parameter from a neighbouring crate is
not reproducible. The result would still be *reported*, which is worse than it
failing.

**G4 is a configuration error that presents as a behavioural one.** It belongs
in [`FlushPolicy`'s validation rules](../type/001_flush_policy.md), not in
runtime fallback — a policy that quietly degrades to a different policy is
[trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s
V2 exactly.

**G5 is the tempting fix that makes the problem permanent.** A default that
couples the two silently is worse than an explicit `n` that is sometimes wrong,
because the wrongness becomes undetectable at the point where it is introduced.

**G5 has already happened, just not by a default.** `ring_bench` binds
`OnBatch`'s `n` to the staging buffer's capacity by construction, not by a
`FlushPolicy::default()` — the exact coupling G5 warns against, arriving one
level removed from where the warning expected it (→ `FL44` below).

### Mitigation

1. **State the relationship rather than encoding it.** `n` and the claim width
   are independent parameters whose *product* structure matters; the honest
   form is a documented rule ("prefer `n` a multiple of the claim width"), not
   a default that hides the choice.
2. **Validate `n` against buffer capacity at construction**, closing G4 as a
   configuration error with a loud failure rather than a silent behavioural
   change.
3. **Record the claim width alongside every `OnBatch` benchmark result**,
   closing G3. This is a `ring_bench` obligation, not one this crate can
   discharge — but it is this crate's to *state*, because this crate owns the
   parameter that becomes meaningless without it.
4. **Cite this instance from `ring_batch`'s side too**, if and when that crate's
   docs are written — the coupling is symmetric and a reader tuning the claim
   width has the same blind spot in the other direction.

**Mitigation 3 is the load-bearing one and it is not enforceable from here.**
This crate can name the dependency between a parameter it owns and a number it
does not; it cannot make the benchmark record it. That is the same shape as
[`ring_tls`'s S2b seam](../../../ring_tls/docs/integration/001_family_dependency_seam.md) —
a property depended on across a crate boundary with no test spanning both —
and it is recorded here so that the two instances can be found together.

**Mitigation 3 also assumes the benchmark can tell the three policies apart;
today it cannot.** `ring_bench`'s `Workload` drives `OnBatch`'s `n`, the
staging buffer's capacity, and the ring's own config from one scalar field —
when that scalar sets both `n` and the capacity, `OnBatch` and `OnFull` fire
on the same condition and publish at the same instants, distinguishable only
by the `cause` label written to the log (→ `FL44` below). Recording the claim
width does not surface this: the confusion is between two of *this* crate's
own policies, not between this crate's parameter and `ring_batch`'s.

**Nothing mitigates G1 and G2 except measurement.** Which value of `n` is right
for a given claim width is a trade that has to be chosen rather than
inherited, and measurement is how it gets chosen. The pitfall is not that the
number is unknown; it is that there are two numbers and only one of them is
visible from either side.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | `OnBatch` is the one variant carrying a number, which is why it is the one with this problem |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | `ring_batch` sits in the transitive closure, reachable and untested-against |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | G4's fallback is its V2 — a policy silently becoming another policy |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md) | G3 makes its measurements incomparable across runs |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_flush_policy.md](../type/001_flush_policy.md) | Mitigation 2 — where G4 is closed as validation |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_batch/readme.md`](../../../ring_batch/readme.md) | The other batch size — the claim width, whose acceptance criterion names 64 |
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row and `ring_batch`'s side by side — one names a number, the other leaves it symbolic |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | Mitigation 2 — `an_unusable_batch_size_is_refused_at_binding` (both error cases) and `a_batch_size_equal_to_capacity_is_accepted` (the boundary is inclusive). G1–G3 remain uncovered: they span two crates and a benchmark, and no test in this crate can see either |

### FL43 — The Second Batch Size Is Not a Number `ring_batch` Owns; It Is an Argument Its Callers Pass

The crate the trap is half about, measured for the parameter the trap says it owns:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this instance attributes to ring_batch --'
awk '/^### FL/{ exit } /claim width/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pitfall/002_two_batch_sizes_that_must_not_diverge.md \
  | sed -E 's/^(.{0,116}).*/\1/' | head -5
echo '  -- ring_batch entire public surface --'
command grep -E '^pub (fn|struct)|^  pub (const )?fn ' ring_batch/src/lib.rs | sed 's/^/    /'
echo '  -- any stored or constant width in the crate --'
printf '    const declarations: %s\n' "$( command grep -cE '^ *(pub )?const [A-Z]' ring_batch/src/lib.rs )"
printf '    struct fields:      %s\n' \
  "$( awk '/^pub struct BatchClaim/{ i = 1; next } i && /^}/{ exit } i && /:/{ printf "%s ", $1 }' ring_batch/src/lib.rs )"
echo '  -- and where 64 actually appears --'
command grep -E '\b64\b' ring_batch/src/lib.rs | sed -E 's/^(.{0,104}).*/    \1/'
```

Live output:

```
  -- what this instance attributes to ring_batch --
    5: - **Purpose**: Record that `OnBatch( n )` and `ring_batch`'s claim width are two different numbers that read 
    14: | | `OnBatch( n )` | `ring_batch`'s claim width |
    24: claim width is 64 and `n` is 100, the flush becomes two claims and two fences,
    36: | G1 | `n` is set larger than the claim width | Each flush costs ⌈n / width⌉ fences. `OnBatch` measures wors
    37: | G2 | `n` is set smaller than the claim width | Fence cost per record rises. The benchmark shows batching h
  -- ring_batch entire public surface --
    pub struct BatchClaim
      pub const fn new( start : Seq, count : usize ) -> Self
      pub const fn start( &self ) -> Seq
      pub const fn len( &self ) -> usize
      pub const fn is_empty( &self ) -> bool
      pub const fn end( &self ) -> Seq
      pub const fn contains( &self, seq : Seq ) -> bool
      pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
      pub const fn overlaps( &self, other : &Self ) -> bool
    pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
    pub fn claim_gated< P : SeqCell, C : SeqCell >
    pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
  -- any stored or constant width in the crate --
    const declarations: 0
    struct fields:      start count 
  -- and where 64 actually appears --
    /// whether `count` is 1 or 64. The ordering is the caller's — see
    /// let batch = claim( &cursor, 64, Ordering::AcqRel );
    /// assert_eq!( batch.len(), 64 );
    /// assert_eq!( cursor.counts().total, 1, "64 slots, one atomic operation" );
    /// assert_eq!( cursor.load( Ordering::Acquire ), Seq( 64 ) );
```

The trap's table gives `ring_batch`'s claim width a row of its own — owned by
that crate, chosen "once," by "whoever implements the claim." No such number
exists. `claim( cursor, count, order )` takes the width as a per-call argument,
`BatchClaim` stores whatever it was given, and the crate declares no constant.
Every appearance of 64 in `ring_batch` is in a doc comment illustrating a call.

**That does not dissolve the trap; it relocates it, and the new location is
worse.** The two numbers still have to agree for `OnBatch` to amortise the fence
it was tuned against. But one of them is a configured value with a validated
domain and exactly one binding site, and the other is an argument supplied afresh
at each call — which is precisely the shape
[`pattern/001`](../pattern/001_policy_as_a_value.md) exists to argue against.
This crate put its half of the coupling in value form and the other half is
still in call-site form.

**G3 is the failure that changes most.** As written it describes the claim width
changing during tuning, invalidating earlier `OnBatch` measurements — a single
edit somebody could notice. What is actually reachable is two call sites passing
different widths in the same build, so there is no "the claim width" for a
benchmark result to be invalidated against. The mitigation the instance asks for
— record the width alongside every result — is right and is not sufficient:
there may be more than one width to record.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/pitfall/002_two_batch_sizes_that_must_not_diverge.md
command grep -m1 'no stored value exists to be' "$F"
command grep -m1 'There is no single claim width to change' "$F"
```

Live output:

```
| Chosen by | Whoever configures the policy, per ring | Whoever calls `claim()`, at every call site — no stored value exists to be "chosen once" (→ `FL43` below) |
| G3 | There is no single claim width to change — call sites can pass different widths within the same build | Every `OnBatch` measurement is silently incomparable unless every call site's width is recorded alongside it. **Nothing records which width was in effect for a result** (→ `FL43` below) |
```

**Disposition:** applied — the Trap table's "Chosen by" cell no longer claims
`ring_batch` chooses its claim width once; it states the measured fact that
`claim()` takes the width as a per-call argument with no stored constant. The
Failure table's G3 row is corrected to match: the reachable failure is two
call sites disagreeing within one build, not a single value drifting over
time. Declined to change `ring_batch` itself (the "thorough fix" of giving it
an actual owned, validated width constant) — that is a new-API decision on a
different crate, out of this doc-corpus scope; the finding's own "not
enforceable from here" framing for the sibling `FL44`/Mitigation 3 applies
equally here. Now prints: `no stored value exists to be`

### FL44 — The Only Benchmark Binds One Scalar to Three Roles, and Two of the Three Policies Then Fire at the Same Moment

What the benchmark's own harness configures:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the one scalar, and everything it drives --'
command grep -E 'self\.config = self\.config\.with_batch|batch : usize|batch : 32' ring_bench/src/lib.rs | sed 's/^/    /'
awk -v n1="$( command grep -n -m1 -F '  let buffer = TlsBuffer::< Record >::with_capacity( workload.batch() );' ring_bench/src/lib.rs | cut -d: -f1 )" -v n2="$( command grep -n -m1 -F '    .map_err( RunError::Flush )?;' ring_bench/src/lib.rs | cut -d: -f1 )" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_bench/src/lib.rs
echo '  -- the two triggers it binds --'
awk -v n1="$( command grep -n -m1 -F '      FlushPolicy::OnFull => if self.buffer.is_full() { Some( FlushCause::Full ) } else { None },' ring_flush/src/lib.rs | cut -d: -f1 )" -v n2="$( command grep -n -m1 -F '      FlushPolicy::OnBatch( n ) => if self.buffer.len() >= n { Some( FlushCause::Batch ) } else { None },' ring_flush/src/lib.rs | cut -d: -f1 )" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- and whether the claim width is recorded anywhere in the benchmark --'
printf '    ring_bench mentions of "claim width": %s\n' \
  "$( command grep -rcio 'claim width' ring_bench/src ring_bench/tests 2>/dev/null | cut -d: -f2 | paste -sd+ | bc )"
printf '    ring_bench mentions of ring_batch:    %s\n' \
  "$( command grep -rc 'ring_batch' ring_bench/src ring_bench/tests 2>/dev/null | cut -d: -f2 | paste -sd+ | bc )"
```

Live output:

```
  -- the one scalar, and everything it drives --
      pub fn with_batch( mut self, batch : usize ) -> Result< Self, WorkloadError >
        self.config = self.config.with_batch( batch );
  -- the two triggers it binds --
    604:       FlushPolicy::OnFull => if self.buffer.is_full() { Some( FlushCause::Full ) } else { None },
    605:       FlushPolicy::OnBarrier => if at_barrier { Some( FlushCause::Barrier ) } else { None },
    606:       FlushPolicy::OnBatch( n ) => if self.buffer.len() >= n { Some( FlushCause::Batch ) } else { None },
  -- and whether the claim width is recorded anywhere in the benchmark --
    ring_bench mentions of "claim width": 0
    ring_bench mentions of ring_batch:    0
```

`Workload` holds one `batch` field, defaulting to 32. `with_batch` writes it to
the field *and* into the ring's own config. The run then uses it a third time, as
the staging buffer's capacity, and a fourth, as `OnBatch`'s `n`.

**With `n` equal to the buffer's capacity, `OnBatch` and `OnFull` are the same
policy.** `OnFull` fires on `is_full()`; `OnBatch( n )` fires on `len() >= n`.
When `n` is the capacity those conditions are the same condition, so the two
policies publish at the same instants, move the same records, and differ only in
the `cause` written to the log. The benchmark that exists to compare three
publication policies is configured so that two of them cannot be told apart by
anything except a label.

**This is G5 arriving through a route the instance did not anticipate.** G5 warns
against defining `OnBatch`'s default *as* the claim width — coupling by default
rather than by decision — and [`type/001`](../type/001_flush_policy.md)
accordingly withholds `Default` from `FlushPolicy`. The coupling arrived anyway,
in the consumer, where no trait obligation could stop it, and it is tighter than
G5 imagined: not two numbers made equal but one number spent four times.

Mitigation 3 asked `ring_bench` to record the claim width beside every `OnBatch`
result. It names neither the width nor the crate, which is consistent — it has no
claim width to record, because the number it passes everywhere is this crate's
`n`.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/pitfall/002_two_batch_sizes_that_must_not_diverge.md
command grep -m1 'G5 has already happened, just not by a default' "$F"
command grep -m1 'Mitigation 3 also assumes the benchmark can tell' "$F"
```

Live output:

```
**G5 has already happened, just not by a default.** `ring_bench` binds
**Mitigation 3 also assumes the benchmark can tell the three policies apart;
```

**Disposition:** applied — added a note after the G5 commentary stating that the coupling G5 warns against has already occurred in `ring_bench`, by construction rather than by a `FlushPolicy::default()`, and a second note after the Mitigation-3 paragraph stating that recording the claim width does not surface this benchmark-internal conflation between `OnBatch` and `OnFull`. Both notes forward-reference this finding rather than restate its evidence. Declined to change `ring_bench`'s configuration itself (giving `Workload` four independent scalars instead of one) — that is a benchmark redesign on a different crate with its own tuning tradeoffs, out of this doc-corpus scope; recorded here per Mitigation 4's own precedent of stating a cross-crate coupling this crate cannot enforce. Now prints: `G5 has already happened`
