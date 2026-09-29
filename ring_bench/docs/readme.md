# docs

Design documentation for `ring_bench`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | One workload driven through six runners, and the filter that runs before the comparison |
| `api/` | Two entry points, one of which returns a table and never a ranking |
| `data_structure/` | A description with two fields called "producers", and a result with three counts that are not interchangeable |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally |
| `definition/` | Module Index — every definition, instance, decision and finding in this crate, in one place |
| `integration/` | Six declared edges, three that had to be added, and the Contract crate this one uses hardest and never names |
| `invariant/` | The ordering every candidate must satisfy, and the region the counters must stay outside of |
| `item/` | Seven nouns and forty verbs, catalogued as a set — where twenty-five accessors and one false doc sentence become visible |
| `lifecycle/` | Seven phases from a description to a verdict, and the six states one candidate occupies inside them |
| `non_functional_requirement/` | The crate's own stated comparison criterion, and the isolation requirement it does not state |
| `pattern/` | The measurement as a value, and the refusal as a row |
| `pitfall/` | A ceiling imposed by a door, a counter that would change what it counts, and an `Ok` that means nothing |
| `type/` | The candidate enumeration and the four ways a run can be refused |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

**This crate produces a verdict, and the documentation is shaped by that.** The
other 32 crates of the family are judged by whether their behaviour matches
their specification. This one is judged by whether the number it prints is
*about the thing it names* — a distinction that has no analogue in a data
structure crate, and that turns out to be where every one of this crate's
findings lives.

The instances sit at three grains. `non_functional_requirement/` and
`invariant/` document the **verdict grain** — what must be true for a
measurement to mean anything, and what
[the acceptance table](../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
grades this comparison by. `algorithm/`, `data_structure/`, `pattern/` and
`pitfall/` document the **mechanism grain**: how one workload becomes six
outcomes. `api/`, `lifecycle/`, `type/` and `integration/`
document the **surface grain** — what a caller touches and across which crate
boundary.

**Three findings dominate, and all three were established by running the
harness rather than by reading anything.**

**`Ok` is not evidence a record was kept, and the harness that believes it
inverts its own verdict.** `OverflowPolicy::default()` is `DropNewest`, so the
family's own documented idiom — a bare `RingConfig::new( n )` — produces a ring
whose `try_push` returns `Ok` for a discarded record. The first working version
of this crate counted those and reported `contract_ring` as **lossless at 256
records in a 16-slot ring**; it would also have reported it as *fast*, and
correctly, because discarding is the cheapest thing a queue can do. Worked out
in [`pitfall/003`](pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md). The
trap is named in
[`ring_shutdown/docs/pitfall/002`](../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md)
as a *delivery* hazard; this crate is where it was paid for as a *verdict* one,
which is a strictly worse failure because a lost record is visible and a wrong
recommendation is not.

**The Contract door caps at one producer two structures that have no cap.**
`ring_factory::build` returns a `ring_handle::Split`, whose `Ends::split` yields
one producer with no `try_clone` beside it. So the in-house MPSC ring and
crossbeam's `ArrayQueue` — both genuinely multi-producer — are single-producer
when reached the way the export Contract says to reach them. The comparison
requires the candidates "under the same producer counts" and through one door that is
unsatisfiable. [`pitfall/001`](pitfall/001_the_door_caps_what_the_structure_does_not.md)
carries it; `Candidate::DirectMpsc` exists so the gap is *measured* rather than
asserted, which is
[`decisions/001`](decisions/001_five_candidates_for_four_named_paths.md).

**Two names on the same five-crate export Contract do not compose.**
`ring_factory::build` produces a `ring_handle::Split`; `ring_flush::Flusher::new`
consumes a `ring_core::Producer`; nothing on the Contract converts one to the
other and `ring_flush` re-exports neither. The staged candidate therefore builds
its ring one level *below* the door the Contract names — the first place in the
family where following the Contract and building the thing are incompatible.
→ [`integration/001`](integration/001_declared_edges_and_the_three_that_were_missing.md).

**A fourth finding is smaller and settles someone else's open question.**
`ring_spsc::Ring::with_config` and `ring_mpsc::Ring::with_config` read capacity
and ignore `overflow`, so the identical `RingConfig` that `ring_factory::build`
refuses outright builds a working ring one level down.
[`ring_factory/docs/decisions/readme.md`](../../ring_factory/docs/decisions/readme.md)'s
Pending 8 records this as a suspicion — "its *name* claims more than it does" —
and `the_direct_doors_ignore_the_policy_the_contract_door_refuses` is the
measurement it was waiting for. **This crate is the first that reaches both
doors with one config**, which is why the evidence appears here and not there.

**A fifth is about the suite rather than the crate, and coverage found it where
no assertion did.** Both workload fixtures offered 256 records in batches of 32,
which divides exactly — so the staged candidate's closing `drain_final` had
nothing to publish and its tail was never exercised. A staged path that
abandoned its partial final batch would have been reported lossless by every
test in the file and lossy only on dimensions nobody had written.
`a_partial_final_batch_is_published_rather_than_abandoned` closes it.
**Two fixtures chosen for readability shared an arithmetic property neither was
chosen for**, and a suite whose fixtures agree on an accident tests less than
its test count says.

**A corollary inherited from
[`ring_factory`](../../ring_factory/docs/readme.md) and honoured here:** every
count in these instances ships with the command that regenerates it, because a
finding resting on a dependency closure is perishable while the family is being
implemented. This crate adds the terminal case — **it is the last of the 33, so
the closure below it is finished and no claim here is a prediction about an
unwritten crate:**

```sh
cd "$(git rev-parse --show-toplevel)"
( for c in ring_*/; do
  [ "$( cat "$c"src/*.rs | command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' )" = 0 ] \
    && basename "$c"
done ) || true
```

Live output: empty (no crate has zero public declarations)

Every retraction `ring_factory` warned would be needed has been made; this crate
needed none, because there was nothing left below it to be wrong about.

`item/` exists, **and this readme argued that it should not.** The argument was
about drift: every declaration carries rustdoc under `#![ deny( missing_docs ) ]`,
so a parallel catalogue would be a second place for the same text to say the same
thing, and one of the two would eventually be wrong.

**The argument confused what a catalogue restates with what a catalogue finds,
and the two instances demonstrate the difference rather than asserting it.**
`#![ deny( missing_docs ) ]` guarantees that each declaration has a comment. It
guarantees nothing about the *set* — and every property either instance records
is a property of the set. [`item/001`](item/001_seven_nouns_and_the_one_never_compared.md)
finds that `RunError` is `Copy` only because three other crates are, one of which
reserves the right to add variants with `#[ non_exhaustive ]`;
[`item/002`](item/002_forty_verbs_twenty_five_of_them_const.md) finds that
`with_batch`'s doc comment makes a claim about six *other* declarations and is
true of four of them. Neither is visible from any single rustdoc page, and the
lint that guarantees those pages exist cannot read what they say.

The general shape, recorded because it will recur: **a lint that requires every
declaration to be documented creates exactly the conditions under which a
cross-declaration claim goes unchecked** — the claim has to live somewhere, it
lands in whichever comment is nearest, and it is then guarded by a rule that
counts comments.

Nothing in `item/` is a restatement of a rustdoc page. The catalogue tables are
the enumeration the findings are read off, and the drift risk the original
argument named is real but bounded: both tables ship with the command that
regenerates them.

### Related Crates

| Crate | Relationship |
|-------|--------------|
| [`ring_factory`](../../ring_factory/readme.md) | The Contract door. Two candidates go through it, and it is where the producer ceiling comes from — and it **re-exports `RingConfig`**, which is how a workload is described |
| [`ring_tls`](../../ring_tls/readme.md) | The staging buffer of the `TlsOverRing` candidate |
| [`ring_flush`](../../ring_flush/readme.md) | The publisher of that candidate — and the crate whose `ConfigError` had to gain `Display` before this one could compile |
| [`ring_stats`](../../ring_stats/readme.md) | Its counters, written once per run from totals after the clock stops |
| [`ring_spsc`](../../ring_spsc/readme.md) | `DirectSpsc`'s backend, reached without the Contract, to price the dispatch above it |
| [`ring_mpsc`](../../ring_mpsc/readme.md) | `DirectMpsc`'s backend — the only in-house candidate with no producer ceiling, and the reason a multi-producer comparison exists at all |
| [`ring_core`](../../ring_core/readme.md) | **Added, not declared.** `ring_flush::Flusher::new` takes a `ring_core::Producer`, so the staged candidate cannot be constructed without naming it |
| [`ring_slot`](../../ring_slot/readme.md) | **Added, not declared.** `TypedSlot< T >` is the only `Slot` implementor, so the two direct candidates cannot name their own ring type without it |
| [`ring_types`](../../ring_types/readme.md) | **Added, not declared.** `RingStats::record_drop` takes an `OverflowPolicy` |
| [`ring_handle`](../../ring_handle/readme.md) | Not a dependency, and the source of the largest finding — its `Ends::split` is what caps the Contract candidates at one producer |
| [`bench_harness`](../../bench_harness/readme.md) | Owns the acceptance table and the six gates this crate is graded by |
