# Decision: The Backend Discrimination Surface Has No Caller

**Status:** open. The measurement is settled; what it implies is not — nothing
here is broken, and that is precisely why it is filed rather than fixed.

### Scope

- **Purpose**: Record the open question the item census raises — that `Backend`, `Ring::backend` and `Producer::try_clone` exist to let a caller discover its contract, and no library in the family calls any of them — and state what each disposal costs.
- **Responsibility**: The measurement, the three readings, and why this is a decision rather than a fix.
- **In Scope**: The three items, their eight dependents, and the argument in the module documentation that depends on them.
- **Out of Scope**: The measurement itself (→ [`../item/002`](../item/002_the_backend_discrimination_surface.md)); the `free_capacity` asymmetry (→ [`002`](002_free_capacity_keeps_one_signature_over_two_contracts.md)).

### The Measurement

Zero production call sites across thirty-two crates, for all three items. The
only consumers are `ring_factory`'s and `ring_handle`'s test files, and three doc
comments in crates explaining that they deliberately do not forward `try_clone`.

### Three Readings

| Reading | Consequence |
|---------|-------------|
| The surface is unnecessary | Delete all three. The `free_capacity` asymmetry then has no machine-checkable escape hatch at all, and the module documentation's central paragraph loses its remedy |
| The surface is necessary and unadopted | The eight dependents are each relying on "treat every reading as advisory", which is correct but undocumented per-crate. The fix is in the dependents, not here |
| The surface is right and the *need* is imagined | `free_capacity` is advisory everywhere in practice, so nobody ever needed to discriminate. The honest change is to document `free_capacity` as advisory unconditionally and let the SPSC guarantee be an unadvertised bonus |

**The third is the most likely and the least comfortable.** It says the
asymmetry the module documentation treats as this crate's hardest problem has
never actually bitten anyone, because every caller took the safe reading by
default.

### Why This Is Not Simply Fixed

Deleting `try_clone` removes the *only* way to obtain a second producer on an
MPSC-backed `ring_core::Ring` — the test
`four_threads_publishing_through_clones_lose_nothing` is the proof it works, and
`cloned_producers_share_one_ring` the proof it shares state. The item is unused
by libraries and load-bearing for the crate's own concurrency guarantee, and
those are not the same question.

### CO13 — The Only Consumers Are Two Test Files and Three Doc Comments

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Backend\|backend()\|try_clone' \
  ring_{bench,debug,factory,flush,handle,poll,shutdown,testkit}/{src,tests}/*.rs \
  2>/dev/null | sed 's/:  */:/'
```

Live output:

```
ring_bench/src/lib.rs://! `ring_core::Producer::try_clone` is the operation that would, and
ring_bench/tests/bench_test.rs:/// whose `Ends::split` yields one producer with no `try_clone` beside it.
ring_factory/tests/factory_test.rs:use ring_core::{Backend, Ring};
ring_factory/tests/factory_test.rs:assert_eq!(single.backend(), Backend::Spsc);
ring_factory/tests/factory_test.rs:assert_eq!(multi.backend(), Backend::Mpsc);
ring_handle/src/lib.rs://! | `try_clone` is **withheld** | `ring_core::Producer::try_clone` exists and can succeed on an MPSC backend. A `ring_handle::Producer` cannot be duplicated at all |
ring_handle/src/lib.rs:/// producer, which is the difference from `ring_core::Producer::try_clone`.
ring_handle/tests/handle_test.rs://! is no `Clone` and no `try_clone` — and a test cannot call a method that does
ring_handle/tests/handle_test.rs:let backend = ring.backend();
ring_handle/tests/ui_test.rs:/// | `producer_try_clones` | `ring_core`'s duplication is not forwarded | This crate — N2, and the `Deref` detector |
ring_handle/tests/ui_test.rs:cases.compile_fail("tests/ui/producer_try_clones.rs");
ring_testkit/src/lib.rs:/// # Backend-blind
```

Every `src/` hit is a doc comment, and all three say the same thing: `ring_bench`
naming the operation it does not perform, `ring_handle` twice explaining that
duplication is withheld. The test hits are `ring_factory` asserting the selector
returns the right `Backend` and `ring_handle` binding a `backend()` it then does
not branch on.

**Not one library asks which contract it was handed**, which is what the surface
exists for.

### CO14 — The Surface Is Unused by Libraries and Load-Bearing for This Crate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
grep -oE '^fn [a-z_]+' tests/core_test.rs | sed 's/^fn //' | grep -i 'clone\|thread\|backend'
```

Live output:

```
every_backend
every_backend
the_same_program_behaves_identically_on_every_backend
every_backend_the_build_offers_is_actually_exercised
every_backend_reports_the_capacity_it_was_configured_with
the_crossbeam_backend_ignores_the_producer_count
try_clone_refuses_at_spsc_and_permits_elsewhere
cloned_producers_share_one_ring
a_refused_record_comes_back_on_every_backend
drop_oldest_is_rejected_by_the_in_house_backends
four_threads_publishing_through_clones_lose_nothing
a_capacity_of_one_cycles_correctly_on_every_backend
```

Three of this crate's own tests depend on it, and one of them —
`four_threads_publishing_through_clones_lose_nothing` — is the crate's only
multi-threaded correctness assertion. Deleting `try_clone` deletes the only
mechanism by which an MPSC-backed `ring_core::Ring` can have more than one
producer at all.

So the measurement in CO13 says the *discrimination* use has no caller. It does
not say the *duplication* use is dead, and the two live behind one method.

### CO15 — Two Crates Document Withholding a Method Nobody Calls

`ring_handle/src/lib.rs:19` puts it in a divergence table — "`try_clone` is
**withheld**" — and again at line 104; `ring_bench/src/lib.rs:78` names it as
the operation a benchmark would perform and does not. Three doc comments across
two crates, describing the absence of a call that appears nowhere in the family.

The documentation is accurate and the emphasis is inherited from a design
discussion rather than from observed use. Recorded because it is evidence for
the third reading in the decision above: the discrimination need may be one the
family imagined rather than met.
