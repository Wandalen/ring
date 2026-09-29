# docs

Design documentation for `ring_factory`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | One boolean choosing between two unrelated types, and five fields consumed in a fixed order |
| `api/` | One total function and one fallible one, and the argument type a compliant consumer cannot name |
| `data_structure/` | A 32-byte record in, the pair's owner out, and what each end got wrong |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Five declared edges, a closure that has moved three times, and the two re-exports that close the Contract |
| `invariant/` | The property the sweep rests on, and the restriction that does not currently hold |
| `item/` | The nine declarations, split into the verbs and the nouns |
| `lifecycle/` | A configuration becoming a ring and a name becoming a registration — as phases, and as the states each passes through |
| `non_functional_requirement/` | This crate's own stated criterion, and the performance-isolation one it omits |
| `pattern/` | Configuration as a value, and construction as its only consumer |
| `pitfall/` | A field corrected before arrival and a field arriving intact with nothing to act on it |
| `type/` | A fieldless service and a two-variant error, one variant of which arrived from a dependency |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

**This crate is a door, and the documentation is shaped by that.** It is one of
the five names on the family's export Contract, and the only one that exists to
make other crates unnecessary: this family's own Contract ruling holds that the SPSC,
MPSC, and named-registry capabilities are reached *through* this surface rather than by
importing `ring_spsc`, `ring_mpsc` or `ring_registry` directly. The other four
Contract names are things a consumer holds or vocabulary they use;
`ring_factory` is the only verb.

The instances sit at three grains. `invariant/` and
`non_functional_requirement/` document the **contract grain** — what must be
true, and what
[the acceptance table](../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
grades this crate by. `algorithm/`, `data_structure/`, `pattern/` and
`pitfall/` document the **mechanism grain**: how one record becomes one ring.
`api/`, `lifecycle/`, `type/` and `integration/` document the
**surface grain** — what a caller touches, in what order, and across which
crate boundary.

**Three findings dominate, and none was visible from the feature document
alone.**

**The acceptance criterion grades the value the ring was built with, not the
one the caller asked for.** This crate's own acceptance criterion requires that "observable behaviour
matches every field, asserted one field at a time", and two of the five fields
are silently corrected by `ring_config`'s setters before `build` is reachable.
The requested value is not stored anywhere, so the assertion compares the
corrected value against itself. Worked out in
[`pitfall/001`](pitfall/001_the_criterion_grades_the_clamped_value.md); the
sharpest case is a manifest field left empty, which clamps to one producer and
selects the SPSC backend — changing the primitive under measurement, not a
parameter.

**Two of the five fields cannot be honoured to the precision the record
expresses** — and once the crate was implemented and measured, **three of the
five turned out to be unobservable through its output entirely.** `wait` names a
strategy in `ring_wait`, which is not in this crate's dependency closure at all;
`producers` chooses a backend and its value above 1 never reaches the ring;
`batch` is read by nobody in the closure. The first two are in
[`algorithm/002`](algorithm/002_assembling_a_ring_from_a_validated_record.md);
`wait`'s trap — an exhaustive four-arm `match` that reads as complete handling
over a field nothing can act on — is
[`pitfall/002`](pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md).
The measurement is `only_two_of_five_config_fields_are_observable_through_the_factory`,
and it asserts each absence rather than noting it, so any of the three reaching
the surface later breaks a test instead of quietly invalidating a paragraph.

**`producers` is the one whose unobservability is a choice rather than a gap.**
It genuinely changes the backend — `Backend::Spsc` versus `Backend::Mpsc`,
asserted one level down by `the_backend_does_change_with_producer_count_where_it_can_still_be_seen`
— and `ring_handle::Split` exposes no way to ask which is underneath. Without
that control test, "the two rings look identical" would read as the factory
ignoring the field, which would be a bug rather than encapsulation.

**"Construction is the only path" is documented aspiration, not current
behaviour, and it moved further from true during the writing.** Both backends
expose `Ring::new` and `Ring::with_config`, and the second takes the whole
`RingConfig` and reads one field of it — `ring_mpsc`'s own doctest sets
`producers` to 4, never reads it, and passes. `ring_core` was then implemented
and called `with_config` on both branches, giving the leak its first in-family
caller, on the sanctioned path.
[`invariant/002`](invariant/002_construction_is_the_only_path.md) enumerates the
four leaks and why gate G5 cannot see any of them;
[`pattern/002`](pattern/002_one_way_in.md) records why a chokepoint documented
before it is built accumulates the routes it was meant to prevent.

**A fourth finding arrived after the other three and overturned part of them.**
`ring_core::Ring::new` refuses `OverflowPolicy::DropOldest` — a value
`ring_config` accepts without complaint — so the unnamed `build` path is
fallible after all. Five instances asserted the opposite from a complete trace
of every candidate refusal; the trace was sound and its conclusion was drawn
from a crate that was then a skeleton.
[`type/002`](type/002_build_error.md) carries the finding and the retraction.

**A fifth arrived with the implementation and is the largest of them.** The
specified return type, `HandlePair< S >`, **cannot be written**: both handles
borrow from an `Ends` that borrows from a `Split`, so a struct holding all three
is self-referential. `build` returns the owner instead —
[`decisions/001`](decisions/001_the_owner_is_the_return_value.md).
[`data_structure/002`](data_structure/002_the_handle_pair_as_output.md) had
scored this shape's cost as *severe* on the grounds that only two owners were
possible and both were bad; the third was the return value. **Two documents in
this crate made the same mistake independently** — that one, and
[`integration/002`](integration/002_the_crate_the_export_surface_routes_through.md),
which read "route the named registry through this crate" as requiring a forwarding
method when a re-export satisfies it. Enumerating candidates and scoring each
carefully is not the same as checking whether the list is complete, and a table
with a *Cost* column invites the first and never prompts the second.

**The counterweight is that the crate's ten pending decisions closed eight at
once, and mostly from below.** `ring_handle` shipping `Split` answered three
questions that had been recorded as one; `ring_registry` shipping `get_mut`
answered a fourth. **Arriving questions come one at a time and answers come in
clusters**, because a dependency that refuses a policy creates exactly one
question while a dependency that ships a *shape* answers every question waiting
on that shape simultaneously. Recorded in
[`decisions/readme.md`](decisions/readme.md).

**A corollary inherited from
[`ring_flush`](../../ring_flush/docs/readme.md), honoured here, and now with a
second half:** a finding that rests on a dependency closure is perishable while
the family is being implemented, so every count in these instances ships with
the command that regenerates it. `ring_flush`'s closure moved from 24 to 20
during its own authoring; this crate's moved from 22 to 21 during its own.

**The second half is that a finding resting on an *unimplemented* crate is
perishable in a way no recipe catches.** A count can be regenerated; "nothing
can fail here" cannot, because the evidence for it is an absence. Nine of the
thirty-three crates were empty when this was written; **none are now** — the
hazard is fully spent for this crate and the retractions it caused are all in
place:

```sh
cd "$(git rev-parse --show-toplevel)"
# Counted rather than listed: an empty listing and a listing that never ran
# print the same nothing, and the claim above is that the count is zero.
printf 'crates with no declarations at all: %s\n' \
  "$( for c in ring_*/; do
        [ "$( cat "$c"src/*.rs | command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' )" = 0 ] \
          && basename "$c"
      done | wc -l )"
```

Live output:

```
crates with no declarations at all: 0
```

Every claim in these instances about what a skeleton will or will not do is a
prediction, and the instances say so where it matters rather than reading as
settled.

**`item/` was absent for two reasons and both have since gone.** The first was
ordering — `item_des.rulebook.md` requires a Defining Crate with real
declarations, and this crate had none. The second was family consistency: no
crate in the family had an `item/`. Both are now measurably false:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'declarations the old census matched: %s\n' \
  "$( command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' ring_factory/src/lib.rs )"
printf 're-exports it silently dropped:      %s\n' \
  "$( command grep -cE '^pub use ' ring_factory/src/lib.rs )"
printf 'family crates with docs/item:        %s of %s\n' \
  "$( ls -d ring_*/docs/item 2>/dev/null | wc -l )" "$( ls -d ring_*/ | wc -l )"
```

Live output:

```
declarations the old census matched: 6
re-exports it silently dropped:      2
family crates with docs/item:        33 of 33
```

The census the earlier version of this paragraph quoted returns six and is wrong
twice over — it counts `fn fmt`, a private method inside a trait impl, and misses
both `pub use` re-exports, because the pattern has no `use` alternative. **The
two errors do not cancel**, which is what the earlier version of this paragraph
claimed: one removes a single declaration and the other adds two, so the census
reads one short rather than square. The real figure is seven —
[`item/001`](item/001_three_verbs_one_of_them_conditional.md)'s three verbs and
[`item/002`](item/002_four_nouns_two_of_them_somebody_elses.md)'s four nouns —
and both instances' own titles say so, which is how the arithmetic was caught.

**The non-duplication objection survives and is answered rather than ignored.**
Every declaration does carry rustdoc under `#![ deny( missing_docs ) ]`, so an
`item/` tree that restated signatures would be a second place for the same text
to drift. These two instances state what rustdoc structurally cannot: which
declarations are compiled out by default, which refusal set is a union rather
than a forward, and what each noun costs in bytes.

### Related Crates

| Crate | Relationship |
|-------|--------------|
| [`ring_config`](../../ring_config/readme.md) | The input. Implemented, 13 items, the one dependency nothing could remove — and **re-exported** as `RingConfig`, since a Contract-bound consumer could not otherwise name `build`'s only argument |
| [`ring_core`](../../ring_core/readme.md) | The composed backend `build` assembles. **Implemented — 26 items**, and it owns the backend branch, the `DropOldest` refusal, and its own optional third backend |
| [`ring_handle`](../../ring_handle/readme.md) | The output type. **Now declared directly** — it had to be before `build` could name its own return type. Implemented, 20 items, and its `Split` is what ruled shape D2 |
| [`ring_registry`](../../ring_registry/readme.md) | This crate's named-registry storage. Implemented; **re-exported here** as `Registry`, which is how lookup reaches a Contract-bound consumer without a forwarding method |
| [`ring_spsc`](../../ring_spsc/readme.md) | One branch. Implemented, and exposes two constructors this crate is meant to displace |
| [`ring_mpsc`](../../ring_mpsc/readme.md) | The other branch. Same |
| [`ring_stats`](../../ring_stats/readme.md) | **No longer declared.** `RingConfig` has no field that would configure counters, so there was nothing for a build to do with it — one manifest line removed rather than surface invented to justify it |
| [`ring_tls`](../../ring_tls/readme.md) | **No longer declared**, same reason — which also took `ring_batch` out of the closure, since `ring_tls` was its only route in |
| [`ring_wait`](../../ring_wait/readme.md) | **Not in the closure, now for a positive reason.** Its surface is free functions taking a `WaitKind` per call — there is no waiter object to construct, so there is nothing here to depend on it *for* |
| [`ring_flush`](../../ring_flush/readme.md) | A Contract peer, not a dependency — a consumer configures a ring and a flush policy through two doors |
| [`bench_harness`](../../bench_harness/readme.md) | Owns the acceptance table, gate G5, and the export surface declaration |
