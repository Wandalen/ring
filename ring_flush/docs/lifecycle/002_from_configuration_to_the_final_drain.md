# Lifecycle: From Configuration to the Final Drain

### Scope

- **Purpose**: Give a flusher's own arc — bound once, driven many times, drained once — and settle the teardown phase where all three policies must behave identically.
- **Responsibility**: State the phases, their transitions, their dependencies, and the cleanup obligations, including the one the crate deliberately does not discharge.
- **In Scope**: Binding, steady state, teardown; the final drain.
- **Out of Scope**: One consolidation cycle's internals (→ [The Consolidation Cycle](001_the_consolidation_cycle.md)); thread registration, which is `ring_tls`'s.

### Lifecycle Phases

| Phase | Name | What holds |
|-------|------|-----------|
| L1 | **Unbound** | A `FlushPolicy` value exists. It is valid syntactically and unvalidated against any buffer |
| L2 | **Bound** | A `Flusher` owns a policy and a buffer; validation has passed ([`api/001`](../api/001_the_policy_surface.md)'s A2) |
| L3 | **Steady** | Appends accumulate; drives consult the policy; consolidation cycles run as triggered |
| L4 | **Draining** | `drain_final` has been called; the buffer is being emptied regardless of policy |
| L5 | **Drained** | Nothing staged. Further appends are a question this crate has not settled |
| L6 | **Dropped** | The `Flusher` is gone. **Anything still staged is lost** |

**L1 is a real phase and not a formality.** A `FlushPolicy` is `Copy` and
always constructible, so it can be passed around, stored in configuration, and
logged before it is ever checked against a buffer's capacity. A policy in hand
carries no guarantee it is valid for the buffer it is about to meet
(→ [`type/001`](../type/001_flush_policy.md)'s N2, which can only be checked at
binding).

### Phase Transitions

| From | To | Trigger | Notes |
|------|----|---------|-------|
| L1 | L2 | `Flusher::new( buffer, producer, policy )` succeeds | Validation runs here. The destination is bound at the same moment as the policy (→ [`api/001`](../api/001_the_policy_surface.md)'s A2) |
| L1 | L1 | `Flusher::new` fails | The policy value survives; the binding does not |
| L2 | L3 | First append or first drive | No ceremony; the phases differ only in whether anything has accumulated |
| L3 | L3 | Every consolidation cycle | The loop; see [The Consolidation Cycle](001_the_consolidation_cycle.md) |
| L3 | L4 | `drain_final` | **One-way.** The policy stops governing |
| L4 | L5 | The drain completes | |
| L4 | L3 | The drain is rejected — ring full | **Retry is the caller's.** Nothing here loops |
| L5 | L6 | Drop | Clean |
| L3 | L6 | Drop without `drain_final` | **Lossy, silent** |
| L5 | L3 | An append after draining | **Unspecified.** See below |

**The L3 → L6 transition is the one that loses data** and it is reachable by
doing nothing wrong except forgetting a call. It is the price of
[`pattern/002`](../pattern/002_driven_not_self_firing.md)'s no-`Drop` commitment
and it is stated rather than mitigated, because every mitigation reintroduces a
publication point nobody scheduled.

**The L4 → L3 transition matters more than it looks.** A final drain into a
full ring cannot complete, and this crate does not wait
(the family's tick path never parks). So teardown may need several
calls, and a consumer that calls `drain_final` once and drops is in the lossy
case despite having done the right thing. **`drain_final` returning `Rejected`
is a call to action, not a completion**, and
[`FlushOutcome`](../type/002_flush_outcome.md) is what makes that visible.

### The final drain, and why it is not a policy

**At teardown all three policies must behave identically: empty the buffer.**

| Policy | Steady-state behaviour | At `drain_final` |
|--------|----------------------|-----------------|
| `OnFull` | Fires when full | Fires regardless |
| `OnBarrier` | Fires when announced | Fires with no announcement |
| `OnBatch( n )` | Fires at `n` | Fires at any count ≥ 1 |

**So `drain_final` ignores the policy, which makes it look exactly like the
`flush_now()` that [`api/001`](../api/001_the_policy_surface.md) refuses to
expose.** The distinction is real but it is a matter of naming and discipline,
not of types:

| | `flush_now()` | `drain_final()` |
|---|---|---|
| Callable from | Anywhere, any time | Teardown, by the owner |
| How often | Unbounded | Once |
| Effect on the invariant | Publication point becomes a call-site property — the invariant is broken | One publication point, at a phase boundary the consumer chose |
| Enforced by | — | **Nothing but its name and this document** |

**That last cell is the honest assessment.** Nothing prevents `drain_final`
being called in a loop from a hot path, at which point it is `flush_now()` with
a more responsible name. A type-level guard — consuming `self`, returning a
`Drained` type — would enforce the once-only property and is not obviously
right, because the L4 → L3 retry case requires calling it again after a
rejection. **Consuming `self` and returning it on rejection** is the shape that
satisfies both, and it is recorded in [`decisions/`](../decisions/readme.md)
rather than settled here.

### Dependencies

| Dependency | Phases | Why |
|------------|--------|-----|
| The consumer | L2, L3, L4 | Binds, drives, and drains. Every transition except drop is a call it makes |
| [`ring_tls`](../../../ring_tls/readme.md) | L2–L5 | Owns the buffer being bound and emptied |
| [`ring_core`](../../../ring_core/readme.md) | L3, L4 | The ring; its fullness is what makes L4 → L3 reachable |
| [`ring_shutdown`](../../../ring_shutdown/readme.md) | L4 | **Not a dependency.** Owns `close`/`reset`/`drain_all`; whether `drain_final` should be its call rather than this crate's is open |

**`ring_shutdown`'s row is a genuine unresolved seam.** `ring_shutdown` owns
`drain_all`, which is plausibly this phase at family scope — one call
draining every registered buffer rather than one flusher draining its own.
If so, `drain_final` here is the per-buffer primitive that crate sequences,
which is the same relationship this crate has with `ring_tls`. Nothing read
settles it.

### Cleanup Requirements

| # | Requirement | Status |
|---|-------------|--------|
| W1 | `drain_final` empties the buffer regardless of policy | This crate's, discharged |
| W2 | A rejected final drain retains the records | Discharged — same guarantee as any rejection (O3) |
| W3 | The caller retries a rejected final drain | **The caller's.** Not enforceable here |
| W4 | No flush on drop | Deliberate |
| W5 | Staged records at drop are lost | **Accepted.** W3's failure mode |
| W6 | The flush log is dropped with the flusher | Trivial; instrumented builds only |

**W3 is the obligation this crate states and cannot enforce**, and it is the
third instance of that same shape in this crate's documentation — after the
barrier announcement ([`api/002`](../api/002_the_driver_surface.md)) and direct
buffer access ([the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s
P4). **A crate that is a decision rather than a mechanism accumulates these:**
it can be correct in everything it does and still depend on a consumer doing
three things it cannot check.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_policy_surface.md](../api/001_the_policy_surface.md) | L1 → L2; where `flush_now()` is refused and pointed here |
| [../api/002_the_driver_surface.md](../api/002_the_driver_surface.md) | L3 and L4's calls; D3 is `drain_final` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | Its V5 — why the final drain must be an explicit call rather than a `Drop` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_the_consolidation_cycle.md](001_the_consolidation_cycle.md) | The inner loop L3 repeats; its U5 is this instance's W5 |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_driven_not_self_firing.md](../pattern/002_driven_not_self_firing.md) | W4's commitment, and why W5 is accepted rather than fixed |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | What makes L4 → L3 visible to a caller |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/docs/lifecycle/001_thread_registration_and_teardown.md`](../../../ring_tls/docs/lifecycle/001_thread_registration_and_teardown.md) | The buffer's own teardown, whose loss window this arc sits inside |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | L4 → L3 — `a_refused_final_drain_keeps_the_records`: `Rejected { staged : 4 }`, four records still staged, nothing in the ring, and the retry the caller owes returns the same thing rather than losing them |

### FL30 — A Question Closed by Measurement Is Still Open in Every Place It Was Asked

Three prose statements of one question, and the register that answered it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- where the question is still stated as open --'
for f in ring_flush/docs/*/[0-9][0-9][0-9]_*.md
do
  awk '/^### FL/{ exit }
       /has not settled|An append after draining|Called after `drain_final`/ {
         printf "%s:%d: %s\n", FILENAME, NR, substr( $0, 1, 74 ) }' "$f"
done | sed 's|ring_flush/docs/||'
echo '  -- what the register says --'
command grep -E '^\| P5 \|' ring_flush/docs/decisions/readme.md | sed -E 's/^(.{0,118}).*/\1/'
echo '  -- and what pins the answer --'
command grep -E '^fn (a_driver_still_works_after_a_final_drain|a_second_final_drain_is_an_empty_trigger)' \
  ring_flush/tests/flush_test.rs
```

Live output:

```
  -- where the question is still stated as open --
api/002_the_driver_surface.md:98: | Called after `drain_final` | Open — see [`decisions/`](../decisions/read
lifecycle/002_from_configuration_to_the_final_drain.md:18: | L5 | **Drained** | Nothing staged. Further appends are a question this c
lifecycle/002_from_configuration_to_the_final_drain.md:41: | L5 | L3 | An append after draining | **Unspecified.** See below |
  -- what the register says --
| P5 | Appends after `drain_final` | **Answered by measurement** — the flusher is reusable, and it is now a test rathe
  -- and what pins the answer --
fn a_driver_still_works_after_a_final_drain()
fn a_second_final_drain_is_an_empty_trigger()
```

The answer exists and is tested. Three prose rows across two instances still ask
the question, one of them phrased as this crate having "not settled" it.

**The closure travelled to the register and to the suite and stopped there.**
That is the natural reach of a closing edit: a decision is closed *in* the
decisions register, and tests are written *in* the test file. The rows that
raised the question live in the documents where the question came up — a state
table, a transition table, an error table — and none of them is the sort of place
a person goes to record an answer.

**The asymmetry is structural, not careless.** A question can be raised anywhere
a document notices it, so questions scatter. An answer has one canonical home, so
answers concentrate. Nothing walks from the concentrated answer back out to the
scattered questions, and there is no index of "documents that asked this" to walk
along — the deferrals point *forward*, to `decisions/`, and that link is not
traversable in reverse.

The reusable shape: **a forward-pointing deferral is a one-way edge, and closing
a decision needs the reverse edge.** Every one of these three rows correctly
names where the answer would live. None of them is reachable from there.
