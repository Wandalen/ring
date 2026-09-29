# docs

Design documentation for `ring_atomic`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | One intrinsic per production method, and the read that is four reads |
| `api/` | Four signatures, one contract clause, and two attributes the trait declines |
| `data_structure/` | Eight bytes and forty, the missing `repr`, and what packing five counters costs |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Two edges in of which one is dead, five out that a re-export turns into twelve |
| `invariant/` | What upstream relies on, including the one property this crate declines to enforce |
| `item/` | Per-item contracts and coverage, constructor by constructor and method by method |
| `lifecycle/` | A cell from zero to drop, and the crate from task file to 376 lines |
| `non_functional_requirement/` | What the instrument costs, against no budget, and three optimizations measured |
| `pattern/` | The single creation site, and the substitute that is not a mock |
| `pitfall/` | Two defences that turn an anomaly into a plausible number |
| `type/` | What the trait requires against what its users need, and the report nobody keeps |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

13 definitions, 26 instances, 52 findings. Every finding is indexed in
[`definition/readme.md`](definition/readme.md), and every one is verified by a
command whose output is quoted at the point it is used.

Scope of this crate: atomic sequence helpers with explicit memory orderings.

### What 376 Lines With No Branch Is Documented For

`ring_atomic` declares one trait, two cells and one report. `AtomicSeq` is a
`u64`-wide atomic with a `Seq` newtype around every result; `CountingSeq` is the same
thing with four counters beside it; `OpCounts` is what those counters report. There is
no allocation, no `unsafe`, no `Drop`, no branch on any production path — every
`AtomicSeq` method compiles to one intrinsic and a newtype wrap
([`algorithm/001`](algorithm/001_one_intrinsic_or_two.md) § AT1).

What the crate is actually for is stated in its own module comment: **to be the one
place the family's sequence atomics are created, so that swapping them for `loom`'s
instrumented versions is a change to one file.** That claim holds for the swap and is
delivered — six `cfg` sites, four constructors, one hand-written `Default`, in
exchange for four other crates running model checks against these cells rather than a
re-implementation ([`workaround/001`](workaround/001_the_loom_seam_and_the_manifest_above_it.md)
§ AT49). It is the sentence's two neighbours that do not survive contact: "the one
place in 33 crates where an atomic is created" is false by ten raw atomics
([`pattern/001`](pattern/001_one_place_where_an_atomic_is_created.md) § AT37), and "no
other crate needs to know the seam exists" is true of siblings and false of the
workspace manifest that must legalise the `cfg` name (§ AT38).

A crate this thin carries a corpus this size because the interesting content is not in
what the code does. It is in what the code declines to require.

### The Four Threads Running Through the Corpus

**One — every reachable hazard lives in an undocumented decision.** The crate explains
three things at length: why a trait rather than a struct, why the constructors are
split by `cfg`, why `Default` is written out rather than derived
([`decisions/002`](decisions/002_a_trait_because_the_criteria_needed_two.md) § AT15,
[`item/001`](item/001_six_constructors_for_two_types.md) § AT25). None of the three is
a thing a caller could get wrong. Meanwhile `SeqCell` declines to require `Sync`, so a
`Cell< u64 >`-backed implementation satisfies it and `ring_batch::claim` accepts it
([`type/001`](type/001_what_the_trait_promises.md) § AT45); `fetch_add` declines
`#[ must_use ]`, so discarding a claim's only record compiles in silence (§ AT46);
and `counts()` declines to say it is four separate reads, so its contract reads as an
instant it never delivers ([`pitfall/001`](pitfall/001_the_snapshot_that_never_happened.md)
§ AT44). All three are one sentence or one bound away from being stated.

**Two — a defence converts an anomaly into a plausible number.** Past the `u64` wrap,
`Seq::distance_to`'s saturation reports an overrun ring as empty, and every gate in
the family then grants: `free_slots` says 64 of 64, `may_claim` true, `headroom` 64,
`check` `Ok(())` ([`pitfall/002`](pitfall/002_the_wrap_that_reads_as_an_empty_ring.md)
§ AT42). Under contention, `OpCounts::total` is computed from the same four torn reads
a suspicious caller would use it to check, so it certifies every impossible snapshot
(§ AT43). In both cases what reaches the caller is in range, well-typed, and wrong —
backpressure does not degrade, it inverts.

**Three — guard ordering decides the outcome, not guard strength.** Three defences sit
on the wrap chain and it passes all three. `Seq::next` panics on overflow in debug,
but production advances through `AtomicSeq::fetch_add`, which wraps identically in both
profiles — so the guard is on the arithmetic path while the traffic is on the atomic
one (§ AT41). `free_slots`' saturation would fail safe, but `distance_to`'s fails open
and runs first, so the safe clamp is unreachable (§ AT42). Both saturations carry
written rationales; neither rationale mentions the other.

**Four — counts written once and never recomputed.** Four in this crate's
neighbourhood alone: "the one place in 33 crates" against ten raw atomics (§ AT37),
`ring_cursor`'s "six manifests" against eight
([`integration/002`](integration/002_five_crates_downstream.md) § AT20), the module
comment naming `handshake_test.rs` as the loom seam's only user against four test
files, and the root manifest's "Only ring_atomic, ring_cursor and ring_publish"
against six crates ([`workaround/001`](workaround/001_the_loom_seam_and_the_manifest_above_it.md)
§ AT50). Each was true when written. None has a check that would notice the drift, and
each is one `grep` from being correct.

### What the Tests Cannot Reach

Three findings are about signals that read green without exercising anything. Both
concurrency tests — 60,000 operations between them — assert properties of `AtomicU64`
reached through a one-line delegation, so they cannot fail unless the standard library
is wrong ([`invariant/002`](invariant/002_every_increment_survives.md) § AT23).
Thirteen counting assertions in `ring_batch` and `ring_tls` rest on one twenty-line
single-threaded parity test over five fixed values, which never touches the wrap, a
zero-width advance, or any concurrency
([`pattern/002`](pattern/002_the_counting_cell_is_not_a_mock.md) § AT39). And all 17
tests *compile* under `--cfg loom` while every one of them panics at the first atomic
access, because none opens a `loom::model` — so `cargo build --tests` reports the loom
configuration healthy and nothing runs it
([`workaround/002`](workaround/002_what_the_seam_does_not_switch.md) § AT52).
