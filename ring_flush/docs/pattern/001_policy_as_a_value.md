# Pattern: Policy as a Value, Not a Call Site

### Scope

- **Purpose**: Name the practice this crate exists to apply — moving the flush decision out of the code that appends and into a value configured once — and account for what it costs.
- **Responsibility**: State the problem, the solution, where it applies, and its consequences.
- **In Scope**: Decision-as-value; the configure-once/consult-many shape.
- **Out of Scope**: Who calls the driver (→ [Driven, Not Self-Firing](002_driven_not_self_firing.md)); the specific variants (→ [The Policy Enum](../data_structure/001_the_policy_enum.md)).

### Problem

**A decision made at every call site is not one decision — it is as many
decisions as there are call sites, made by different people at different
times.**

Left unmanaged, flushing happens wherever someone thought to write it, so a
buffer's contents reach the ring at a point that varies by call site.

The specific difficulty is that **each individual decision is locally
reasonable.** A system about to do something slow flushes first, so its
commands are not stranded. A library flushes in teardown, so nothing is lost. A
hot loop flushes every thousand iterations, because that felt right. None is
wrong on its own terms; together they are not a design, and no single reader
ever sees them all.

**And the problem is invisible at review time.** A diff adding one flush call
shows one flush call. The property being destroyed — that publication timing is
a designed, global property — is not visible in any diff that destroys it.

### Solution

**Represent the decision as a value, configured where the ring is configured,
and consult it from exactly one place.**

| Aspect | Call-site form | Value form |
|--------|----------------|------------|
| Where the decision lives | Scattered across every writer | One configuration site per ring |
| How many exist | Unknown without a full-tree grep | One on SPSC, by construction; a convention only on MPSC backends (→ `FL37` below) |
| Changing it | Find every site; miss one | Change one value |
| Reviewing it | Impossible — no diff shows the whole | Reading one value |
| Testing it | Requires exercising every writer | Requires exercising three variants |
| Benchmarking it | Meaningless — no single policy is in effect | The reason the policy must be a value to be varied across benchmark runs |

**The last row is why this crate exists at all rather than being a convention.**
The benchmark must vary the policy across runs, since the flush point is one
of the few knobs that moves both throughput and latency at once. A knob that
is scattered across call sites cannot be varied across runs; it has to be a
value before it can be an experimental parameter.

**"How many exist" holds by backend, not by construction.** `Flusher::new`
takes a `Producer` by value, and `ring_core::Producer::try_clone` hands out
another one on the multi-producer backends — nothing prevents a second
`Flusher`, configured with a different policy, against the same ring. On
SPSC it is genuinely structural, since `try_clone` returns `None` there. The
reviewability the table promises is real on every backend; the impossibility
is SPSC-only.

**The value is an enum rather than a trait**, and that choice is not incidental
— it is what makes the policy comparable, serialisable into a benchmark
configuration, and exhaustively testable at three cases rather than at however
many implementations exist. The cost is that a consumer cannot supply a fourth
policy (→ [The Policy Surface](../api/001_the_policy_surface.md)'s
compatibility discussion).

### Applicability

| Situation | Applies | Why |
|-----------|---------|-----|
| A timing decision that must be uniform across writers | **Yes** | The canonical case — this crate |
| A decision that is an experimental parameter | **Yes** | It must be a value to be varied |
| A decision with a small, closed set of sensible answers | **Yes** | Enum stays exhaustive |
| A decision genuinely local to one call site | No | Centralising it adds indirection and removes context that mattered |
| A decision requiring caller-specific data at the moment of decision | No | The value would have to carry a closure, which defeats the comparability the pattern buys |
| A decision with an open set of implementations | No | Use a trait; accept losing exhaustiveness |
| A decision that is a correctness requirement, not a trade-off | No | Do not make it configurable at all — a policy implies the alternatives are all acceptable |

**The last row is the sharpest boundary and the easiest to cross by
accident.** Making something a policy says "any of these is fine, choose by
measurement." If one variant is actually wrong, expressing it as a policy hands
callers a way to be wrong that looks sanctioned. The three flush variants pass
this test — each is correct, with different latency and throughput — which is
what makes them a legitimate policy rather than a configuration hazard.

### Consequences

| # | Consequence | Direction |
|---|-------------|-----------|
| Q1 | The publication point becomes reviewable — one value, one place | **Positive.** This is [the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s P2, the only mechanism there that enforces by construction |
| Q2 | The policy becomes an experimental parameter | **Positive.** Required for benchmarking policy choice across runs |
| Q3 | A writer loses the ability to flush for its own reasons | **Positive by intent, negative in practice** — the writer that genuinely needed it now has no route, and will either add one (breaking the invariant) or accept latency it cannot control |
| Q4 | Exhaustiveness: three variants, three test cases | **Positive** |
| Q5 | A consumer cannot supply a fourth policy without amending this crate | **Negative**, and expensive because this crate is on the export Contract |
| Q6 | The decision is consulted per append, so its cost is on the hot path | **Negative** — constrains the implementation (→ [The Decision Costs Nothing on the Append Path](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)) |
| Q7 | Policy and *schedule* are separated, and only policy is a value | **Neutral, and the source of most confusion** — the driver's cadence is still a call-site property (→ [Driven, Not Self-Firing](002_driven_not_self_firing.md)) |

**Q3 is the real cost and it should not be argued away.** The pattern takes a
capability from writers on the grounds that they collectively misuse it. That
is a legitimate trade and it is still a loss — a writer with a genuine reason
to flush now has an unmet need, and the honest response is that the need should
become a policy variant if it recurs, not that the writer was wrong to have it.

**Q7 is where the pattern stops short of its own promise.** Making the policy a
value moves *what* out of the call sites. It does not move *when the driver
runs*, which remains wherever the consumer calls it. The pattern gets half the
property; the other half is the consumer's, and no crate in this family owns it.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_policy_surface.md](../api/001_the_policy_surface.md) | Q5's cost, stated as a compatibility guarantee |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | The value this pattern produces |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_point_is_designed_not_inherited.md](../invariant/002_publication_point_is_designed_not_inherited.md) | Q1 — this pattern is that invariant's only constructive enforcement |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) | Q6's constraint, stated with a threshold |

### Patterns

| File | Relationship |
|------|--------------|
| [002_driven_not_self_firing.md](002_driven_not_self_firing.md) | Q7 — the half of the property this pattern does not deliver |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_bench/readme.md`](../../../ring_bench/readme.md) | Varies the policy across runs, which is what requires it to be a value — Q2 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | Q4 — `every_entry_names_its_own_policys_trigger` iterates all three variants against their expected causes in one loop, and `the_parameterless_policies_need_no_validation` covers the two that take no argument. Exhaustive because the enum is closed |

### FL37 — "Exactly One, by Construction" Is a Property of One Backend and a Convention on the Other Two

What the comparison table claims, and what the constructor permits:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the row the pattern rests on --'
awk '/^### FL/{ exit } /^\| How many exist \|/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pattern/001_policy_as_a_value.md
echo '  -- what the constructor takes --'
awk -v n1="$( command grep -n -m1 -F '  pub fn new' ring_flush/src/lib.rs | cut -d: -f1 )" -v n2="$( command grep -n -m1 -F '  -> Result< Self, ConfigError >' ring_flush/src/lib.rs | cut -d: -f1 )" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- and whether a second producer for the same ring is obtainable --'
awk -v n1="$(( $( command grep -n -m1 -F '//! # What is uniform, and what only looks uniform' ring_core/src/lib.rs | cut -d: -f1 ) + 1 ))" -v n2="$( command grep -n -m1 -F '//! | [`Producer::free_capacity`] | **binding** | advisory | advisory |' ring_core/src/lib.rs | cut -d: -f1 )" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_core/src/lib.rs
command grep 'pub fn try_clone' ring_core/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the row the pattern rests on --
    39: | How many exist | Unknown without a full-tree grep | One on SPSC, by construction; a convention only on MPSC backends (→ `FL37` below) |
  -- what the constructor takes --
    408:   pub fn new
    409:   (
    410:     buffer : TlsBuffer< T >,
    411:     producer : Producer< 'a, T >,
    412:     policy : FlushPolicy,
    413:   )
    414:   -> Result< Self, ConfigError >
  -- and whether a second producer for the same ring is obtainable --
    37: //!
    38: //! | Property | [`ring_spsc`] | [`ring_mpsc`] | crossbeam |
    39: //! |---|---|---|---|
    40: //! | Producers permitted | exactly 1 | N | N |
    41: //! | [`Producer::try_clone`] | `None` | `Some` | `Some` |
    42: //! | [`Producer::free_capacity`] | **binding** | advisory | advisory |
      pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
```

The table's whole argument is the contrast between "Unknown without a full-tree
grep" and "Exactly one, by construction." The second half is not construction.
`Flusher::new` takes a `Producer` by value, and `ring_core::Producer::try_clone`
hands out another one — refusing on SPSC, succeeding on the two multi-producer
backends. Two `Flusher`s, two different policies, one ring: nothing in this
crate or its dependency prevents it, and the resulting publication timing is
whatever the two policies interleave into.

**The property the pattern promises therefore holds by backend, not by
construction.** On SPSC it is genuinely structural — `try_clone` returns `None`
and there is no second producer to build a second flusher from. On MPSC it is
exactly the convention the call-site form was criticised for: one configuration
site *if everyone agrees to have one*, discoverable only by the full-tree grep
the table's left column describes.

That does not damage the pattern; it damages the row. The honest form is that
the enum makes the decision *reviewable* wherever it is made, which is a real and
sufficient benefit, rather than that it makes a second decision impossible. The
stronger claim is the one a reader would rely on when deciding not to grep.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/pattern/001_policy_as_a_value.md
command grep -m2 'holds by backend' "$F"
```

Live output:

```
**"How many exist" holds by backend, not by construction.** `Flusher::new`
**The property the pattern promises therefore holds by backend, not by
```

**Disposition:** applied — the "How many exist" cell now names SPSC and
MPSC separately instead of claiming construction-level uniqueness family-wide,
and a new paragraph states the mechanism (`try_clone` returning `Some` on the
multi-producer backends) and the honest scope of the guarantee, matching the
finding's own "reviewable, not impossible" framing. Now prints: `holds by backend`

### FL38 — The Applicability Table's Sharpest Boundary Is Crossed by the Enum It Was Written For

The rule, and the variant that breaks it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the boundary the table calls the easiest to cross --'
awk '/^### FL/{ exit } /correctness requirement, not a trade-off|sharpest boundary/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pattern/001_policy_as_a_value.md | sed -E 's/^(.{0,126}).*/\1/'
echo '  -- the three variants --'
command grep -E '^  (OnFull|OnBarrier|OnBatch)' ring_flush/src/lib.rs | sed 's/^/    /'
echo '  -- what the constructor validates --'
awk '/^  pub fn new$/,/^    Ok\( Self/ { if ( /if |return Err/ ) printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- and what checks that an announced barrier is a real one --'
printf '    call sites of drive_at_barrier outside this crate: %s\n' \
  "$( command grep -rn 'drive_at_barrier' ring_*/src ring_*/tests 2>/dev/null \
       | command grep -v '^ring_flush/' | wc -l )"
```

Live output:

```
  -- the boundary the table calls the easiest to cross --
    76: | A decision that is a correctness requirement, not a trade-off | No | Do not make it configurable at all — a policy i
    78: **The last row is the sharpest boundary and the easiest to cross by
  -- the three variants --
      OnFull,
      OnBarrier,
      OnBatch( usize ),
  -- what the constructor validates --
    416:     if let FlushPolicy::OnBatch( n ) = policy
    418:       if n == 0
    420:         return Err( ConfigError::ZeroBatch );
    423:       if n > buffer.capacity()
    425:         return Err
  -- and what checks that an announced barrier is a real one --
    call sites of drive_at_barrier outside this crate: 0
```

The Applicability table's last row says a decision that is a correctness
requirement rather than a trade-off should not be made configurable at all,
"because a policy implies the alternatives are acceptable," and the paragraph
beneath calls it the sharpest boundary and the easiest to cross by accident.

**Two of the three variants are trade-offs and the third is not.** `OnFull` and
`OnBatch( n )` differ in latency and batch size; either is a defensible answer
and the constructor can check the one that has a domain — `n` must be nonzero and
must not exceed capacity. `OnBarrier` is different in kind: it is correct when
the consumer announces real barriers and silently degenerate when it does not
(→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md)'s F3). Its
alternatives are not all acceptable; one of them is a defect.

So the enum offers, on equal footing, two configuration choices and one
correctness obligation, and the value form gives a reader no way to tell them
apart. Nothing outside this crate calls `drive_at_barrier` today, which is why
the boundary has not yet cost anything.

**The table is not wrong and the crate is not wrong to have the variant** — the
requirement is real and the value form is still the right shape for the other
two. What is missing is the row's own consequence applied to its own subject:
selecting `OnBarrier` is a commitment the caller takes on, and it is spelled
exactly like the two selections that commit to nothing.
