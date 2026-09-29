# docs

Design documentation for `ring_batch`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | One `fetch_add` at any batch size, and a gate that is three steps and not part of it |
| `api/` | Twelve items, seven attributes, and the one bare statement that burns sequences |
| `data_structure/` | Sixteen bytes with no niche, and the same struct written again three tiers up |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Four dependencies with two written down, one dependant, and a design corpus above both |
| `invariant/` | Properties that must hold for every input, one of which cannot break |
| `item/` | Per-item contracts and coverage, method by method |
| `lifecycle/` | No destructor, no rollback, and the empty claim as a legal state everywhere |
| `non_functional_requirement/` | What a batch buys at two thread counts, and the heap the crate never touches |
| `pattern/` | The range object written four times, and a split into two functions |
| `pitfall/` | Two ways to outrun a ring with well-typed code |
| `type/` | What the types commit to, and the one call the signature cannot refuse |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

13 definitions, 26 instances, 53 findings. Every finding is indexed in
[`definition/readme.md`](definition/readme.md), and every one is verified by a
command whose output is quoted at the point it is used.

Scope of this crate: batch claim objects spanning a sequence range.

### What One Struct and Three Functions Is Documented For

`ring_batch` is 314 lines, of which 212 are doc comment, 16 are blank, and 86 are
code. It declares one sixteen-byte `Copy` struct with eight `const` methods, three
free functions, no `unsafe`, no allocation, no `Drop`, and no state. Its entire
executable content is one `fetch_add`, one comparison, one `+`, and two lazy
folds.

What is left is what the crate is actually for: **turning `count` slots into one
atomic operation, so the fences that make the handshake correct are paid per
operation and not per item.** That claim is this crate's own headline claim, and
it holds — measured, a batch of sixty-four costs 11.49 ns against a
single item's 5.93 ns on one thread, and at eight threads is *cheaper* than a
single-item claim ([`non_functional_requirement/001`](non_functional_requirement/001_what_one_batch_actually_buys.md)
§ BA34). A crate this thin carries a corpus this size because the amortisation is
the easy half: `fetch_add` returns the old value, so publishing the claim and
advancing the cursor are the same instruction and nothing can be unclaimed
([`algorithm/001`](algorithm/001_one_fetch_add_whatever_the_count.md) § BA1). The
hard half is everything that instruction leaves for someone else.

### The Three Threads Running Through the Corpus

**One — the corrected version already exists three tiers up, and neither crate
knows it.** Six of the fifty-three findings are about `ring_claim`, which defined
`Claim { start, len }` against this crate's `BatchClaim { start, count }` —
character-identical `sequences` bodies, all eight shared operations agreeing
([`data_structure/002`](data_structure/002_the_struct_ring_claim_wrote_again.md)
§ BA11, [`pattern/001`](pattern/001_the_range_object.md) § BA38). It is not the
duplication that matters but the direction: every guard this crate lacks, that one
has. A type-level `#[ must_use ]` whose message states the irreversibility rule
`ring_batch` never writes (§ BA39); a `Claimer` that owns its producer cursor,
making the aliased-cursor call unrepresentable
([`pattern/002`](pattern/002_two_functions_where_one_would_have_hidden_it.md)
§ BA41); a `compare_exchange` retry loop with a four-line comment naming the exact
race this crate leaves open
([`pitfall/001`](pitfall/001_the_window_between_the_gate_and_the_advance.md)
§ BA43); a `claim_up_to` for the caller this crate's `Full` variant cannot help
([`decisions/002`](decisions/002_two_errors_not_one.md) § BA16); and a
`sequences( self )` that needs no edition-2024 capture bound
([`workaround/001`](workaround/001_the_capture_bound_edition_2024_made_necessary.md)
§ BA51). There is no dependency edge in either direction and no prose in either
crate naming the other — and the Tier 5 duplicate is the one with two dependents
and both ring implementations behind it, while the Tier 2 original reaches one
leaf crate (§ BA12).

**Two — every way this crate goes wrong compiles clean.** All four hazards pass
`-D warnings`. `claim( &cursor, 8, order );` as a bare statement advances the
cursor and orphans eight sequences, because `#[ must_use ]` is on the seven
accessors where dropping the result does nothing and on none of the three
functions where it does
([`api/001`](api/001_twelve_items_seven_must_use.md) § BA5, § BA6).
`claim_gated( &cell, &cell, .. )` type-checks and grants six of six requests on a
ring of four, because `free_slots( at, at, cap )` is always the full capacity —
and that aliased call is `free_slots`' own doctest, written down as correct
([`type/001`](type/001_the_ring_that_can_gate_against_itself.md) § BA46). The gate
and the advance are separate operations, so under sixteen producers the cursor
passes `consumer + capacity` in about one round in two hundred, by a whole batch
each time (§ BA42). And `end()`'s `+` panics in debug and wraps in release, after
which `len()` reports 1 while every method routing through `end()` behaves as
though the claim were empty, with no `# Panics` section anywhere in the crate
([`pitfall/002`](pitfall/002_the_addition_with_no_panics_section.md) § BA44) — a
surprise only because `ring_types::Seq::next` documents saturation seventeen lines
above a `distance_to` that actually saturates (§ BA45). Nothing catches any of it:
no `loom` in the manifest while five other crates have it, and a patched copy with
both gating loads set to `Relaxed` passes all 21 integration tests and 10 doctests
([`decisions/001`](decisions/001_ordering_is_the_callers_except_where_it_is_not.md)
§ BA14).

**Three — the tests assert the property that cannot break and skip the one that
can.** Disjointness follows from `fetch_add` alone and survives the gated path
unchanged — 16,000 sequences per run, zero claimed twice, through both entry
points over twenty runs each — which is why its test cannot fail, and why the
comment calling it "the property a claim protocol must never violate" points a
reader at the wrong risk
([`invariant/001`](invariant/001_disjointness_is_free.md) § BA22, § BA23). The
capacity bound is the invariant that actually breaks, and no test asserts it: both
threaded tests call `claim`, and all thirteen `claim_gated` references sit above
the contention section
([`invariant/002`](invariant/002_ascending_not_contiguous.md) § BA25). The same
shape reaches the docs — `overlaps` says it exists so a whole-run test can assert
disjointness, and that test names the same property and says it uses a per-sequence
`HashSet` instead because it is stronger
([`item/001`](item/001_the_method_whose_reason_was_declined.md) § BA26) — and the
costs: this crate's own stated claim is a number, `ring_bench` exists to produce numbers,
and no file under `ring_bench/` mentions `ring_batch`
([`integration/002`](integration/002_the_feature_is_planned_its_problems_are_addressed.md)
§ BA21). Every figure in this corpus is a probe written for it.

### Reading Order

| If you want | Start at |
|-------------|----------|
| what the crate does, in one instruction | [`algorithm/001`](algorithm/001_one_fetch_add_whatever_the_count.md) |
| what a batch actually buys, measured at two thread counts | [`non_functional_requirement/001`](non_functional_requirement/001_what_one_batch_actually_buys.md) |
| the crate three tiers up that already solved this one | [`pattern/001`](pattern/001_the_range_object.md) |
| the gate that can be handed one cursor twice | [`type/001`](type/001_the_ring_that_can_gate_against_itself.md) |
| the ways correct-looking code outruns the ring | [`pitfall/001`](pitfall/001_the_window_between_the_gate_and_the_advance.md) |
| every finding at once, indexed and ranked by severity | [`definition/readme.md`](definition/readme.md) |
