# docs

Design documentation for `ring_spsc`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The claim-and-publish and drain procedures — steps, orderings, and the four atomic operations the single-producer cardinality removes |
| `api/` | The two caller surfaces — a publishing guard and a committing batch — and why the shape questions they once left open were cheap to leave open |
| `data_structure/` | Two fields where the multi-producer sibling needs more — and the per-slot state deliberately not carried |
| `decisions/` | Two open ADRs, why the three shape questions never became a third, and the one decision recorded at family grain instead |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Five dependencies, four pointed absences, and the export boundary this crate's capability crosses while the crate itself does not |
| `invariant/` | The cardinality precondition every saving is purchased with, and the synchronization budget it buys |
| `item/` | What actually reaches the items, and two ordering constants where the sibling has five |
| `lifecycle/` | The ring's phases and handle arcs, and the contiguous-prefix property that eliminates per-slot stamps |
| `non_functional_requirement/` | The correctness-floor obligation carried on the family's behalf, and this crate's own binary Reached condition |
| `pattern/` | The practice this crate exists to embody — prove the degenerate configuration first — with an honest account of when it does not pay |
| `pitfall/` | The trap this crate's own success creates: seven properties true here and false one cardinality up |
| `type/` | The two values the correctness arguments are written in — a single-writer position, and a derived quantity whose contract is stronger here than anywhere else in the family |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

The instances sit at three grains. `invariant/` and
`non_functional_requirement/` document the **contract grain** — this crate's
cardinality precondition, its no-lock budget, and the binary Reached condition
[the acceptance table](../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
grades it by. `algorithm/`, `data_structure/`, `pattern/`, and `pitfall/`
document the **mechanism grain** that follows from that precondition.
`api/`, `lifecycle/`, `type/`, and `integration/` document
a **surface grain** — what a caller touches, when, and across which crate
boundary.

**The surface grain is cheaper to leave open here than in an exported crate,
and the docs spend that budget deliberately.** `ring_spsc` is *not* one of the
family's exported crates; this crate's "externally-facing API" means reachable
through `ring_factory` → `ring_handle`, never separately importable. So the
three candidate producer shapes in
[`api/001`](api/001_producer_surface.md), the borrow-versus-copy drain in
[`api/002`](api/002_consumer_surface.md), and the cursor initialization value
in [`lifecycle/001`](lifecycle/001_ring_construction_and_teardown.md) were all
marked undecided on purpose: each was a refactor question whose blast radius
stops at `ring_core` and `ring_handle`
(→ [`integration/002`](integration/002_reached_through_the_export_surface.md)).
The exact same class of question is a public contract question in
[`ring_tls`](../../ring_tls/docs/readme.md), which sits *on* the export list.

**All three are now closed in their own instances, and none needed an ADR** —
which is what the budget was for. Writing the code answered them faster and
more concretely than deliberation would have; see
[`decisions/readme.md`](decisions/readme.md) for what that cost and what it
bought.

**The crate's whole thesis is what is missing from its dependency list.** It
declares five crates — storage, configuration, cursors, slot views, shared
newtypes — and none of `ring_gating`, `ring_claim`, `ring_publish` or
`ring_consume`. Each of those four exists for a question single-producer does
not ask: a gating minimum over a one-member set, a CAS loop over a cursor only
one thread writes, a per-slot stamp recording an order the cursor already
carries, a bound computed through a barrier when the peer cursor *is* the
bound. Every other instance here is a consequence of those four absences, which
is why these docs argue with
[`ring_mpsc`](../../ring_mpsc/docs/readme.md)'s rather than paraphrase them.

One of the two problems these docs were written around is now solved and the
other is now enforced, and both are worth stating as outcomes rather than
quietly deleting. The shared-crate contention risk in
[`integration/001`](integration/001_family_dependency_seam.md) — that
`ring_claim`, `ring_publish` and `ring_consume`, being shared with `ring_mpsc`,
might offer only contended variants and make this crate pay for contention it
does not have while nothing failed — dissolved when the three were read: there
was no uncontended variant to ask for, because each crate exists for a
multi-producer question. Not depending on them removes the risk structurally.
What replaces it is the mirror obligation, checked by `tests/manual/readme.md`
S6: the absences must stay absent.

The cardinality precondition in
[`invariant/001`](invariant/001_exactly_one_producer_one_consumer.md) is no
longer merely stated either. `split` takes `&mut self`, so a second pair does
not compile; neither end is `Clone`; neither is `Sync`. All three are asserted
as `compile_fail` blocks in the library, because a soundness argument resting
on "exactly one producer" is worth nothing if a second one is constructible.

**`item/` exists, and the trigger this paragraph reserved for creating it is
what fired.** `item_des.rulebook.md` catalogs Rust items defined in this
crate's own tree, with Representation, Kind, Definition location, File Usage
Table, Crate Usage Table and a Caller or Callee Tree per function. The argument
against writing them was that the public types and their methods already carry
every one of those sections in the rustdoc on the declarations themselves,
which cannot drift from the code because it is attached to it — so duplicating
it into `item/` would create a second place to update and a second place to be
wrong.

That argument bounded itself: the trigger to revisit was not "the crate has
items" — it had them — but a consumer needing the cross-crate Usage Tables,
which rustdoc does not give, and it named `ring_core`'s composition of both
backends as the question that would raise one. Both instances are that question
rather than a restatement of the declarations.
[`item/001`](item/001_sixty_items_and_what_actually_reaches_them.md) records
what reaches these items from outside, which is a fact about the composition
and not one this crate's own source carries, and
[`item/002`](item/002_two_ordering_constants_where_the_sibling_has_five.md)
reads the same seam against `ring_mpsc`'s five.

Measured. The six this paragraph used to claim matched neither figure below —
there are five public types, and the census of types *and* methods it was
attached to returns thirty:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'public types (struct/enum/trait): %s\n' \
  "$( command grep -cE '^\s*pub (struct|enum|trait) ' ring_spsc/src/lib.rs )"
printf 'the old census, types + methods:  %s\n' \
  "$( command grep -cE '^\s*pub (fn|struct|enum|trait|type|const) ' ring_spsc/src/lib.rs )"
printf 'instances in docs/item:           %s\n' \
  "$( ls ring_spsc/docs/item/[0-9][0-9][0-9]_*.md | wc -l )"
```

Live output:

```
public types (struct/enum/trait): 5
the old census, types + methods:  30
instances in docs/item:           2
```

### Related Crates

Five declared dependencies, one composition point above, and one sibling this
crate is defined by contrast with. The four dependencies `ring_mpsc` carries
and this crate does not are the subject of
[`integration/001`](integration/001_family_dependency_seam.md).

| Crate | Relationship |
|-------|--------------|
| [`ring_store/readme.md`](../../ring_store/readme.md) | Dependency — the one-time slot allocation, holding no cursor and no ordering state |
| [`ring_cursor/readme.md`](../../ring_cursor/readme.md) | Dependency — the padded cursor whose `align_of == 64` contract keeps the two ends off one cache line |
| [`ring_claim/readme.md`](../../ring_claim/readme.md) | **Deliberately absent.** Its claim is a CAS loop over a cursor several threads write; one producer's claim is a `Relaxed` load of a cursor only it writes |
| [`ring_publish/readme.md`](../../ring_publish/readme.md) | **Deliberately absent.** It publishes *with holes* — a per-slot stamp and a highest-contiguous scan. One producer completes in claim order, so the cursor already carries what the stamp would record |
| [`ring_consume/readme.md`](../../ring_consume/readme.md) | **Deliberately absent.** Its commit computes a bound through `ring_barrier` over a consumer set; the set has one member and the bound is the peer cursor, read directly |
| [`ring_config/readme.md`](../../ring_config/readme.md) | Dependency — `RingConfig`, of which only `capacity` is read; `producer_count` is rejected above, at `ring_factory` |
| [`ring_slot/readme.md`](../../ring_slot/readme.md) | Dependency — `Slot`, `TypedSlot`, `BytesSlot`: the per-slot view, carrying no synchronisation of its own |
| [`ring_types/readme.md`](../../ring_types/readme.md) | Dependency — `Seq`, `Capacity`, `RingError`. `Capacity` is why an invalid capacity is unrepresentable here rather than rejected here |
| [`ring_gating/readme.md`](../../ring_gating/readme.md) | **Deliberately absent.** Not an oversight and not deferred — its two halves degenerate to nothing at one producer, by construction |
| [`ring_core/readme.md`](../../ring_core/readme.md) | Composition point — selects this crate, `ring_mpsc`, or the crossbeam backend behind one surface |
| [`ring_handle/readme.md`](../../ring_handle/readme.md) | Where this crate's cardinality invariant is *enforced* rather than stated — a `Producer: Clone` there would compile, pass its own tests, and permit the race |
| [`ring_mpsc/docs/readme.md`](../../ring_mpsc/docs/readme.md) | The general case this crate is the degenerate configuration of. Not a competitor: [`pitfall/001`](pitfall/001_spsc_correctness_does_not_transfer.md) exists because reasoning flows between them too easily |
