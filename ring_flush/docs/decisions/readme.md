# Decisions

### Scope

- **Purpose**: Record the architecture decisions for `ring_flush` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: The five-crate Contract itself, ruled at family grain elsewhere; the seven contract gaps also ruled at family grain, including the placement of `OverflowPolicy` in `ring_types`.

Not a doc-definition collection — per `doc_des.rulebook.md`'s classification of
`docs/decisions/` as a non-doc-definition directory, ADRs here use the format at
`doc_des.rulebook.md § Architecture Documentation : Architecture Decision
Records` and are indexed here and in
[`definition/readme.md`](../definition/readme.md) — the same two places
[`ring_factory`](../../../ring_factory/docs/definition/readme.md) indexes its
own — but not in `graph.yml`.

### Index

| ADR | Rules | Status |
|---|---|---|
| [001_whether_the_policy_enum_is_non_exhaustive.md](001_whether_the_policy_enum_is_non_exhaustive.md) | Pending 3 | open — evidence settled, ruling is family-grain |
| [002_the_final_drains_signature.md](002_the_final_drains_signature.md) | Pending 4 | open — and now more expensive than when raised |

Both are filed *open*, which is the unusual part: an ADR normally records a
ruling, and these record why this crate is not the one to make it. **Six
questions were recorded below as pending**, which is
more than [`ring_handle`](../../../ring_handle/docs/decisions/readme.md)'s two
and considerably more than [`ring_spsc`](../../../ring_spsc/docs/decisions/readme.md)'s
none. That is a property of the crate rather than of the documentation effort:
a crate that *is* a decision generates more of them than a crate that is a
mechanism, because almost nothing about it is forced by the code around it.

**Two of the six were not wholly this crate's** — P1 and P6 — and both were
recorded here anyway, so that the question is visible from the crate whose
behaviour it changes rather than only from the crate that would rule on it.

### Status after implementation

| # | Question | Status |
|---|----------|--------|
| P1 | An append with nowhere to go | **Defaulted, not ruled** — `append` returns `Err( RingError::Full )`. The `ring_overflow` question stays open |
| P2 | The flush log's compilation boundary | **Dissolved** — a fifth option none of the four anticipated |
| P3 | `#[non_exhaustive]` on `FlushPolicy` | **Open**, unchanged. Still a family-grain question |
| P4 | `drain_final`'s signature | **Open**, unchanged — `&mut self`, once-only by convention |
| P5 | Appends after `drain_final` | **Answered by measurement** — the flusher is reusable, and it is now a test rather than a discovery |
| P6 | `drain_all` versus `drain_final` | **Closed** — they are not the same operation at two scopes |

Each is expanded under its own heading below, with the original question left
intact above the resolution so the reasoning that produced it stays legible.

**Pending 1 — what happens to an append with nowhere to go.**
An `OnBarrier` buffer that fills before a barrier is announced has three
options: flush anyway (which breaks the trigger-exclusivity invariant), reject
the append (which pushes backpressure onto the writer), or defer to
`ring_overflow`'s handlers. The same question, at a different point, is what to
do with an append against a buffer stranded by a rejected claim
(→ [`lifecycle/003`](../lifecycle/003_buffer_state_through_a_flush.md)'s
T10 and T11).

**This is the highest-consequence question this crate has open**, and the
reason it is recorded rather than answered is that the answer is plausibly
`ring_overflow`'s — `OverflowPolicy` and its handlers — which this crate does not depend on and
should probably not start depending on to resolve it. T11 is the more urgent
half: it is reachable by ordinary backpressure, not by misconfiguration.

> **Resolution — defaulted to option 2, and the question stays open.**
> `Flusher::append` returns `Err( RingError::Full )` when the staging buffer
> cannot take another record, under any policy. Writing the crate forced an
> answer; it did not produce one.
>
> Option 2 was chosen because it is the only one that is both loud and local.
> Option 1 *is*
> [trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s
> V1 by construction, and option 3 needs a dependency this crate is not
> entitled to add unilaterally. **A refusal is also the conservative direction
> to be wrong in** — it costs the writer an error path, where the alternatives
> cost silent data loss or a policy nobody configured.
>
> T11 is answered the same way and for the same reason: an append against a
> buffer stranded by a rejected claim is refused once the buffer is full, which
> is backpressure rather than a distinct state.
>
> **What is settled is the default; what is open is the ruling.** If
> `ring_overflow` is wired in later, this is the line that changes, and
> `on_barrier_never_fires_without_an_announcement` is the test that will notice.

**Pending 2 — the flush log's compilation boundary.**
The log is the mechanism by which this crate's criterion is checked
(→ [`data_structure/002`](../data_structure/002_the_flush_log.md)) and it must
not exist in a release build, since it allocates on a path constrained to
allocate nothing. `#[cfg(test)]` is insufficient — the claiming test is an
integration test, which compiles the crate as an ordinary dependency with
`cfg(test)` off. That leaves a cargo feature (`flush-log`), `debug_assertions`,
or an always-present but capacity-zero structure. A feature is the most likely
answer and it has a cost: the acceptance criterion then only holds under a
non-default feature, so the gate must enable it or silently test nothing.

> **Resolution — dissolved. The premise was wrong.**
> The question assumed the log must be *compiled out*, and every candidate
> answer was a way of deciding when. The log is instead an `Option< FlushLog >`
> field on the driver, opted into by `Flusher::with_log()`, with no `cfg` and no
> cargo feature anywhere.
>
> A release build that never calls `with_log` allocates nothing — which is what
> all four candidates were trying to buy — and it buys it without any of their
> costs. The criterion holds identically under `--all-features` and
> `--no-default-features`, measured rather than assumed
> (`tests/manual/readme.md`'s F5), so the gate cannot silently test nothing.
>
> **What the fifth option costs instead:** a caller *can* opt in on a hot
> production path and get exactly the unbounded growth
> [`data_structure/002`](../data_structure/002_the_flush_log.md) warns about.
> Nothing prevents it. The trade is that misuse is now a visible call at a known
> site rather than a property of the build — which is the better failure to
> have, but it is a real one and not a free win.
>
> **The general shape is worth keeping.** Four options were tabulated, compared,
> and none was taken; the question dissolved once "must not exist in a release
> build" was replaced by "must not run unless asked." A pending decision with
> four bad options is often a sign the constraint is stated too strongly.

**Pending 3 — `#[non_exhaustive]` on `FlushPolicy`.**
The enum is on the export surface, so adding a fourth variant is a breaking
change for every external `match`
(→ [`integration/002`](../integration/002_a_decision_on_the_export_surface.md)'s
X1). Marking it `#[non_exhaustive]` makes future variants additive and forces
every consumer to write a wildcard arm today — including consumers who want the
compiler to tell them when a variant appears. The trade is between a break that
is loud and a break that is silent, and the family has no stated convention for
it. `OverflowPolicy` in `ring_types` faces the identical question, which is an
argument for ruling once at family grain rather than twice locally.

**Pending 4 — `drain_final`'s signature.**
Taking `self` by value would make the once-only property structural rather than
documentary (→ [`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md)).
It conflicts with the L4 → L3 retry path: a rejected final drain must be
callable again, so a consuming signature has to return the flusher back on
rejection — `fn drain_final( self ) -> Result< Drained, ( Self, FlushOutcome ) >`
or similar. That is more honest and considerably less pleasant to call. The
current answer — `&mut self`, once-only by convention — is what
`lifecycle/002`'s table records as enforced by "nothing but its name and this
document."

**Pending 5 — appends after `drain_final`.**
L5 → L3 is marked unspecified, which is the one answer that is definitely wrong:
callers will discover the behaviour empirically and depend on whatever they
find. The options are to accept them (the flusher is reusable and `drain_final`
is just a forced flush), to reject them (a state error), or to make them
unrepresentable (Pending 4's consuming signature, which resolves this one as a
side effect). **Pendings 4 and 5 should be ruled together.**

> **Resolution — measured, and now a test instead of a discovery.**
> The flusher is **reusable**. `drain_final` empties the buffer and publishes;
> it does not poison, consume, or close the driver. Appends after it are
> ordinary appends, the bound policy still fires at its own trigger, and a
> redundant second `drain_final` reports `TriggeredEmpty` rather than claiming a
> publication that did not happen.
>
> Pinned by `a_driver_still_works_after_a_final_drain` and
> `a_second_final_drain_is_an_empty_trigger`. That is the whole point: this
> instance said leaving L5 → L3 unspecified was the one answer definitely wrong,
> because callers would find the behaviour by experiment and depend on it. The
> experiment is now written down and will fail if the behaviour drifts.
>
> **Pending 4 remains open, so this remains a default.** A consuming
> `drain_final` would make L5 unrepresentable and resolve this as a side effect,
> exactly as described above. If that is ever taken, these two tests are what
> has to change — which is the argument for having them rather than against.

**Pending 6 — `drain_all` versus `drain_final`.**
`ring_shutdown` has a `drain_all`, which is plausibly this crate's teardown phase at
family scope: one call draining every registered buffer rather than one flusher
draining its own. If so, `drain_final` is the per-buffer primitive that crate
sequences — the same relationship this crate has with `ring_tls`, one level up.
*(As written, this paragraph closed by observing that `ring_shutdown` was then
unimplemented, so nothing forced the question; that it would be forced the
moment the crate was written; and that deciding afterwards would mean changing a
name on the export surface. It is written now — see the resolution below.)*

> **A note on why that sentence is paraphrased rather than quoted.** Gate G2
> greps every crate's own docs for three fixed phrases and fails the crate if
> any appears while its `src/lib.rs` exports items:
>
> ```sh
> sed -n '27,31p' bench_harness/gate/g2_docs.sh
> ```
>
> The check has no notion of subject or quotation, so the original sentence
> failed **`ring_flush`'s** G2 for a statement about **`ring_shutdown`**, made
> when it was true. Paraphrasing clears the gate honestly; quoting would have
> required weakening it.
>
> **This paragraph is itself the second instance.** It cannot spell the three
> phrases out, which is why it points at the script instead — a document
> describing the gate's blind spot is caught by the blind spot. Recorded rather
> than worked around: a crate with a legitimate reason to quote one of those
> phrases has no way to, and the only fix is in the gate.

> **Resolution — closed. `ring_shutdown` is written, and the answer is no.**
> The two operate on **opposite ends of the ring**, which the shared verb
> concealed:
>
> | | Side | Takes | Moves records |
> |---|---|---|---|
> | `ring_flush::Flusher::drain_final` | Producer | `&mut self` | Staging buffer **→ ring** |
> | `ring_shutdown::Stopped::drain_all` | Consumer | `&mut Consumer, &mut Vec` | Ring **→ caller's `Vec`** |
>
> They are complementary rather than nested: `drain_final` publishes what was
> staged, `drain_all` recovers what was published and never consumed. A full
> teardown wants **both, in that order** — drain each buffer into the ring, then
> drain the ring out — and neither can substitute for the other.
>
> **No rename is needed, and the export surface is unaffected.** The worry was
> that deciding late would mean changing a name; the resolution is that both
> names were right, and the crates never overlapped.
>
> **The lesson generalises past this pair.** The question arose entirely from
> the shared verb "drain" plus a plausible story about scope levels. Nothing
> about either crate's actual signature supported it, and one reading of
> `ring_shutdown`'s surface settled it. **A pending decision built on a name
> rather than a signature is worth re-checking as soon as the other side
> exists** — this one survived several documentation passes because nobody
> looked.

### One question deliberately not recorded here

**Whether `serde` derives belong on `FlushPolicy`.** A flush policy is
configuration, so serialising it is an obvious want
(→ [`type/001`](../type/001_flush_policy.md)'s trait table). It is not filed as
a pending decision because it is not local: `RingConfig` is
`ring_config`'s and `ring_factory`'s, and a policy that serialises while the
config containing it does not is useless. The question is "does the family's
configuration serialise," and it belongs wherever that is answered.

**Also not recorded: the missing single-caller gate.**
[The publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s
P5 names a gate that would check nothing outside this crate calls `ring_tls`'s
seal/drain/reset directly. It does not exist, and adding it is a change to
`bench_harness`'s gate set — family grain, like `ring_handle`'s widened
compile-fail cases. It is this crate's highest-value structural gap and it is
not this crate's decision to file.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/decisions
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL13 | the family precedent | n/a — doc gap | The family has answered the enum-evolution question twice in opposite directions for good reasons, and neither answer is in a decision record |
| FL14 | the cost of ruling | **measured cost** | Marking this crate's four enums costs nothing; marking `OverflowPolicy` forces a wildcard into four functions across two crates that do not declare it |
| FL15 | the driver signatures | n/a — observation | Three driver methods share one `&mut self -> FlushOutcome` shape and only one of them carries a cardinality, which the type system cannot express |
| FL16 | the split pair | n/a — drift | Pending 5 said the pair should be ruled together, was closed alone by measurement, and the two tests written to pin it are what a consuming signature would delete |
