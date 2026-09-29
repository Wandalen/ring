# workaround

External constraints `ring_bench` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

| File | Responsibility |
|------|-----------------|
| [`001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md`](001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md) | W1 — the clock's resolution is the platform's, and the tie-break is a list |
| [`002_a_feature_that_cannot_be_negated_at_the_use_site.md`](002_a_feature_that_cannot_be_negated_at_the_use_site.md) | W2 — an additive Cargo feature forks every construct that enumerates |

**This readme argued that no workarounds existed, and the two instances above
are the refutation.** The argument is preserved below because the reasoning was
sound and the conclusion was still wrong, which is the useful part.

### The Argument That Was Made

This crate is implemented and it did meet external constraints — three of them,
all inside its own family. None became a workaround:

| Constraint met | Where it went instead | Why not a workaround |
|---|---|---|
| `ring_flush::Flusher::new` takes a `ring_core::Producer`, which nothing on the export Contract produces | [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md) | The staged candidate builds below the Contract. That is not a way *around* the gap — it is the measurement of it, and hiding it in an adapter would have deleted the finding |
| `ring_handle::Producer` has no `try_clone`, capping every Contract-reached ring at one producer | [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | Reported as a refusal row per candidate. A harness that synthesised extra producers would report a comparison the Contract cannot actually deliver |
| `Ring::with_config` silently ignores the `overflow` field that `ring_factory::build` refuses | [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md), and `ring_factory`'s own decisions Pending 8 | Asserted as behaviour rather than normalised. The divergence between two doors onto one structure is evidence the factory's open question needs |

**That reasoning still holds.** A benchmark harness that works around the
awkwardness of the thing it measures stops measuring it. Every constraint above
would have had a tidy local fix — an adapter, a synthetic producer, a policy
normaliser — and each fix would have converted a fact about the family into an
implementation detail of this crate.

### Why It Concluded Wrongly

The argument's last step was: entries arrive here only if a constraint
originates *outside* the family; the three constraints examined are all inside
it; therefore nothing qualifies.

The premise is right and the survey was incomplete. It enumerated the
constraints this crate had *worked around* and found them all internal — but a
workaround entry records a constraint the crate **absorbs**, which includes
those it absorbs by design and never fought. Two such constraints were in the
crate from its first commit, and both come from outside the workspace entirely:

- **W1** — `std::time::Instant`'s resolution is the operating system's. The
  crate's one ordering is a comparison of two readings of it.
- **W2** — Cargo features are additive and crate-global, so an optional
  candidate cannot be excluded at a use site.

Neither has a family crate anywhere in it. Both have a stated compensation, a
measurable cost, and a deletion condition — everything a workaround row needs —
and both were invisible to a survey that asked "what did this crate route
around?" rather than "what is this crate paying for?"

The general shape, worth carrying to the other 32 crates: **a survey of what was
worked around finds only the constraints somebody fought.** The ones absorbed
silently at design time cost just as much and never appear on that list, because
absorbing them left no scar to search for.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/workaround
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN49 | `Outcome::write_nanos` | n/a — coverage | Nineteen source mentions, three test mentions and zero assertions — the compensation is exact, and it leaves the crate's headline ranking with the coverage of a value the crate has declared untestable |
| BN50 | `Comparison::fastest` | **latent hazard** | A tie in `min_by_key` is resolved by `Candidate::ALL`'s declaration order, whose first entry is the mutex baseline, so a clock too coarse to separate two paths reports the control as winner |
| BN51 | `Candidate::producer_ceiling` | **misleading doc** | The doc's summary sentence counts "the four bounded ones" over a table that has three rows unless `--features crossbeam` is passed, and the suite carries no `cfg` that could check it |
| BN52 | `Candidate::ALL` | **latent hazard** | The list is declared twice by hand under opposite `cfg`s with identical doc text, nothing asserts the two agree, and that list is also the undocumented tie-break of BN50 |
