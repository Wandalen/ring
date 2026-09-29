# Lifecycle: Policy Arming and Firing

### Scope

- **Purpose**: Show that the three policies are not three instances of one machine — `OnFull` and `OnBarrier` are stateless, and `OnBatch( n )` only appears to carry a counter — and settle where its arming state actually lives and what resets it.
- **Responsibility**: Enumerate the per-variant states, transitions, and the invariants that hold across all three despite their different shapes.
- **In Scope**: The policy's own condition, independent of the buffer's; the accumulation state and why it is derived rather than stored.
- **Out of Scope**: Buffer condition (→ [Buffer State Through a Flush](003_buffer_state_through_a_flush.md)); the enum's layout (→ [The Policy Enum](../data_structure/001_the_policy_enum.md)).

### States

**Three machines, not one.** Presenting them as one machine with a shared state
set is the error this instance exists to prevent — it invites a counter field on
every variant, which is exactly the shape
[`data_structure/001`](../data_structure/001_the_policy_enum.md) refuses.

| Variant | States | Carries state? |
|---------|--------|----------------|
| `OnFull` | **A1 Idle** only | No |
| `OnBarrier` | **A1 Idle** only | No |
| `OnBatch( n )` | **A2 Arming**, **A3 Armed** | **Yes, but not its own** — the count is the buffer's `len()`, read rather than stored |

| # | State | Applies to | Condition |
|---|-------|-----------|-----------|
| A1 | **Idle** | `OnFull`, `OnBarrier` | The decision is a function of the argument alone. There is nothing to remember |
| A2 | **Arming** | `OnBatch( n )` | `staged < n`. Appends move toward the threshold |
| A3 | **Armed** | `OnBatch( n )` | `staged >= n`. The next consultation fires |

**A1 is not a degenerate case of A2 and A3; it is the absence of the axis.**
`OnFull` asks the buffer whether it is full and `OnBarrier` asks the caller
whether a barrier was announced. Neither question has a history. Modelling them
as "always in A3" would be true and misleading — it suggests something is being
tracked.

**A3 is transient in practice and must be a real state anyway.** A buffer that
reaches `n` and is not driven stays armed indefinitely: the policy fires when
consulted, and consultation is the caller's
([`pattern/002`](../pattern/002_driven_not_self_firing.md)). An armed policy is
therefore a buffer holding `>= n` records that nobody has collected — a
legitimate resting condition, not a bug.

### Transitions

| # | From | To | Trigger | Variant |
|---|------|----|---------|---------|
| P1 | A1 | A1 | Any consultation | `OnFull`, `OnBarrier` |
| P2 | A2 | A2 | An append leaving `staged < n` | `OnBatch` |
| P3 | A2 | A3 | An append reaching `staged >= n` | `OnBatch` |
| P4 | A3 | A3 | A further append | `OnBatch` — **stays armed**, does not double-fire on one drive |
| P5 | A3 | A2 | A completed flush; `staged` returns to 0 | `OnBatch` |
| P6 | A3 | A2 | A **rejected** flush | **Does not happen.** See below |
| P7 | A2/A3 | A2 | `drain_final` | The policy stops governing ([`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md)) |

**P6 is the transition that must not exist**, and it is the one an
implementation is most likely to add by accident. A rejected flush leaves the
records staged ([`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)'s
O3) — so `staged` is still `>= n` and the policy is still armed. Disarming on
rejection would mean the next `n` records accumulate on top of the stranded ones
before firing again, so the flush would carry `2n`. The counter is derived from
what is staged, not incremented separately, which makes P6 unreachable rather
than merely forbidden.

**That derivation is the whole design and it is worth stating as such:** there
is no counter field. `OnBatch( n )`'s state is `buffer.len() >= n`, evaluated at
consultation. P2–P5 describe how that expression's value changes, not a variable
being maintained. This is what keeps the policy `Copy`, 16 bytes, and free of
the reset bug P6 names.

> **This instance was right and its sibling was wrong, and the disagreement
> survived every documentation pass.**
> [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md) had a
> Step 3 that incremented a separate `usize` per append — the exact counter
> field this section says does not exist. Both instances were written before the
> code, both were reviewed, and the contradiction between them was never
> flagged.
>
> The implementation followed `algorithm/001`, because a procedure describing
> the append path is the natural thing to implement an append path from. The
> counter was then found redundant by an injection probe aimed at something
> else entirely (`tests/manual/readme.md`'s F3) and deleted, which is how the
> code arrived at what *this* instance had specified all along.
>
> **The lesson is about cross-instance consistency, not about the counter.** A
> contradiction between two instances of the same crate's own documentation is
> invisible to every gate: G2 checks that docs exist and build, G3 that a
> feature is cited, and nothing compares two instances against each other. The
> reconciliation is manual, and this one took an unrelated probe to force.

**P4's "stays armed" clause matters for the log.** One drive against an armed
`OnBatch( 64 )` buffer holding 200 records flushes 200 in one claim and writes
one log entry, not three. The acceptance criterion's assertion is on the entry,
so a implementation that fired three times would fail M1 — but only if the
scenario overshoots `n`, which a test written to the threshold exactly would
never do.

### Behavioral Invariants

| # | Invariant | Holds because |
|---|-----------|---------------|
| J1 | A policy's state is a pure function of `( policy, buffer.len(), argument )` | No stored counter; nothing accumulated across calls |
| J2 | Consultation never mutates the policy | `FlushPolicy` is `Copy` and consulted by value |
| J3 | `OnFull` and `OnBarrier` have no reachable state | A1 is their only state |
| J4 | A rejected flush leaves the policy armed | J1 — arming is derived from `staged`, which a rejection preserves |
| J5 | Arming is monotone in `staged` between flushes | Appends only add; nothing removes without a flush |
| J6 | Two policies over the same buffer disagree only on the trigger, never on the count | J1 — they read the same `staged` |

**J6 is the one that permits the same buffer to serve rings with different
latency needs** without a combinatorial state space. Several policies can be evaluated against one buffer
because none of them owns state — which is a property J1 gives away for free and
a counter-field design would have destroyed.

**J2 is checkable, and as built it is structural twice over.** This section
proposed `fn should_flush( self, ... ) -> bool` on the policy, taking `self` by
value. What exists is `fn trigger( &self, at_barrier : bool ) -> Option< FlushCause >`
— a private method on `Flusher`, not on `FlushPolicy` — reading
`self.policy` by `Copy` and returning a cause rather than a bool.

The placement differs from the proposal and the guarantee is the same or
stronger: `FlushPolicy` has no methods that could mutate it, so J2 holds because
there is nowhere to violate it from. `the_bound_policy_never_changes` asserts
the observable consequence — driving a flusher four times leaves `policy()`
equal to what was bound.

**Returning `Option< FlushCause >` rather than `bool` is the one place the
implementation improved on this section.** A bool answers *whether* to flush; a
cause answers *why*, which is what the log needs
(→ [`data_structure/002`](../data_structure/002_the_flush_log.md)). With a bool,
the cause would have to be re-derived at the recording site from the policy —
two places computing one fact, which is exactly the divergence the flush log's
own derivation rule forbids.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | The consultation these transitions describe; its per-variant table is P1–P3 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | Why there is no counter field — its last refused shape is P6's cause |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | P4's no-double-fire clause, as a standing restriction |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) | J2's source — a value, not an object with a lifecycle |

### State Machines

| File | Relationship |
|------|--------------|
| [003_buffer_state_through_a_flush.md](003_buffer_state_through_a_flush.md) | The orthogonal axis; its B5 is where J4 is observable |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_batch/readme.md`](../../../ring_batch/readme.md) | The claim width `n` is chosen against — the `2n` overshoot P6 would cause is a claim-width problem |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | P5 — `batches_are_consecutive_not_cumulative` drives `OnBatch( 2 )` through three rounds and asserts each fires at exactly 2, never at 1 and never carrying 4. It was named for a counter and renamed when the counter was deleted; the assertion is unchanged, which is the point |
| `tests/flush_test.rs` | P4 — `an_overshooting_batch_publishes_everything_staged_in_one_claim`: eight appends under `OnBatch( 4 )` with no drive between them publish as **one** entry of eight. P6 — `a_rejected_flush_leaves_the_policy_armed`: two rejections, then the next drive fires with the same records. Written as this row specified |

### FL32 — Four Corpus Checkers Were Added Since and Every Verdict They Can Emit Is Structural

The block above says a contradiction between two instances is invisible to every
gate. Four gates that read the corpus itself have been added since:

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate
echo '  -- the corpus gates --'
ls g1[4-7]_corpus_*.sh | sed 's/^/    /'
echo '  -- every verdict their checkers can emit --'
command grep -hoE "'[A-Z]{3,8} |f'[A-Z]{3,8} |f'[A-Z]{3,8}'" corpus/*.py \
  | tr -d "f'" | sort -u | tr '\n' ' ' | sed 's/^/    /'; echo
echo '  -- and whether any checker opens two instances to compare them --'
printf '    checkers reading a second document for comparison: %s\n' \
  "$( command grep -lE 'compare.*instance|cross_instance|other_doc' corpus/*.py 2>/dev/null | wc -l )"
```

Live output:

```
  -- the corpus gates --
    g14_corpus_shape.sh
    g15_corpus_recipes.sh
    g16_corpus_citations.sh
    g17_corpus_vocabulary.sh
  -- every verdict their checkers can emit --
    AGREE  DEFS  DISP  DUP  EVID  EXIT  FIND  FORM  GAP  INDEX  INST  LINEADDR  LINK  PREFIX  PROBE  README  REGEN  STALE  SUBJ  TEST  TIER  UNQUOTED  WHERE  WHY  
  -- and whether any checker opens two instances to compare them --
    checkers reading a second document for comparison: 1
```

The one hit is `recipes.py` itself, matched on its own comment — *"compared
exactly like an instance file's"* — not a second document read; the checker
still compares one recipe's freshly captured output against its own quote,
never against another file.

Eighteen distinct verdicts across four checkers, and every one is a property of
a document's *form*: a definition is missing, a count is short, an id repeats or
skips, a link does not resolve, a cited test does not exist, a readme has no
regenerate block, a recipe exits non-zero.

**`STALE` is the closest any of them comes and it points the other way.** It
re-runs a recipe and compares the freshly captured output against the output
quoted beneath it — a document checked against the world, which is exactly the
right instrument and exactly not the one this block asked for. Two documents
disagreeing about the same fact produce no `STALE`, because neither is quoting
anything.

**That is why every contradiction recorded in this crate had to be written by
hand.** A finding that two instances disagree is authored one recipe at a time,
by someone who already suspected it. `STALE` catches drift automatically because
the ground truth is executable; a cross-instance contradiction has no executable
ground truth, only two sentences, and no checker can tell which one is wrong.

**The block above is therefore still accurate, and its reason has sharpened.**
It read as an observation about a gap that nobody had got to yet. Four checkers
later it is better described as a boundary: the corpus can be checked
mechanically against code and against itself structurally, and its *claims* can
only be checked by re-deriving them — which is a recipe, which is a finding,
which is manual by construction.

The reusable shape: **automatic checking reaches exactly as far as there is
something executable to compare against.** Everything past that line is somebody
writing a `sh` block, and the corpus's growth in checkers moved the line without
crossing it.
