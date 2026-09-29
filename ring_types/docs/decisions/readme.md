# Decisions

### Scope

- **Purpose**: Record the architecture decisions for `ring_types` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: The five-crate export Contract itself, which this crate does not define; the placement of `WaitKind` and `OverflowPolicy` here as names only, already settled (→ [`../pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)).

ADRs here use the format at `doc_des.rulebook.md § Architecture Documentation :
Architecture Decision Records`. The family indexes `decisions/` in the Module
Index alongside its other definitions — `ring_barrier` does, and this crate now
does too — so the two filed ADRs appear in [`../definition/readme.md`](../definition/readme.md)
with every other instance.

### Index

**Two filed as ADRs, and five questions recorded below as pending.** Every one
of them surfaced while writing the twenty-eight instances against the
implementation rather than before it — which is the expected shape for a crate
that was implemented first and documented second.

| # | ADR | Supersedes |
|---|-----|------------|
| 001 | ["One Error Type" Is a Rule the Family Does Not Keep](001_one_error_type_is_a_rule_the_family_does_not_keep.md) | P2, which named one counterexample of five |
| 002 | [The Two Name Errors, and the Two Crates That Redeclared Them](002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md) | P3, whose `#[ non_exhaustive ]` cost argument does not hold |

**Three of the five are not wholly this crate's** — P3, P4 and P5 — and all
three are recorded here anyway, so the question stays visible from the crate
whose constraint caused it rather than only from the crate that would act on it.

| # | Question | Status | Raised by |
|---|----------|--------|-----------|
| P1 | `NonZeroUsize` for `Capacity`'s field | **Open** — deferred, not declined | [`pattern/002`](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md), [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md) |
| P2 | Is "one error type for the family" a rule or an observation? | **Superseded by [`001`](001_one_error_type_is_a_rule_the_family_does_not_keep.md)** — the family has six, not two | [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md) |
| P3 | What becomes of `NameTaken` and `NameUnknown` | **Superseded by [`002`](002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md)** — three crates spell the collision, and `#[ non_exhaustive ]` makes deletion cheap rather than costly | [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md) |
| P4 | Should `ring_factory` re-export `RegistryError`? | **Open** — one line, in a crate this one does not own | [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md) |
| P5 | A gate asserting tier 0's `[dependencies]` stays empty | **Open** — the invariant with no mechanism | [`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md) |

**Pending 1 — `NonZeroUsize` for `Capacity`'s field.**
`Capacity` wraps a plain `usize` and rejects zero in its constructor. A
`NonZeroUsize` would move half the invariant into the type: the zero case would
become unrepresentable, `RingError::CapacityZero` would lose its reason to
exist, and `mask()`'s freedom from underflow would be structural rather than
argued in a comment.

Three things make it a decision rather than an obvious improvement. It costs a
variant that is currently constructed exactly once but is part of a
`#[ non_exhaustive ]` public enum, so removing it is a semver event for a crate
on the export Contract. It threads an `Option` through a `const fn` constructor
that is currently two `if`s and a return. And it buys nothing against the
*second* half of the invariant — a `NonZeroUsize` of 6 is still not a power of
two, so `Capacity::new` keeps its second test and its second error either way.

**The question is what the invariant is for.** If it is for the compiler, the
change is worth the churn. If it is for the fifteen consumer crates, they
already never re-check, so it buys nothing they can observe.

**Pending 2 — is "one error type for the family" a rule or an observation?**
*Superseded by [`001`](001_one_error_type_is_a_rule_the_family_does_not_keep.md),
which measures six error enums rather than the two this entry assumed. The
reasoning below stands; the count it rests on does not.*
`src/error.rs`'s module documentation opens by stating that the family has one
error type rather than one per crate. Measured against the workspace, that is
false: `ring_registry` declares `RegistryError`, and it did so because the
`Copy` requirement here forbids the `String` payload its only failure needs
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).

Three readings, and they are not equivalent:

| Reading | Consequence |
|---------|-------------|
| It is a rule and `ring_registry` violates it | The registry's error must be folded in, which means dropping `Copy` or dropping the name from the message |
| It is a rule scoped to the ring path | The wording needs "on the ring path" and the exception becomes documented rather than contradictory |
| It is an observation that has since expired | The sentence should be corrected to describe what is actually true |

**The second is almost certainly right** and the first is actively harmful —
unifying the two types would either put an allocation on the tick path or
delete the name from a name-collision error. The pending part is that nothing
has ruled it, so a future reader meets a stated rule and a visible exception
with no note connecting them.

**Pending 3 — what becomes of `NameTaken` and `NameUnknown`.**
*Superseded by [`002`](002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md),
which finds a third declaration in `ring_factory` and shows the
`#[ non_exhaustive ]` argument below runs backwards — the attribute is what makes
deletion cheap.*
Both variants are declared, classified, formatted and tested; nothing in the
workspace constructs either. The only three references outside `src/` are in
this crate's own suite, asserting that they classify and display correctly
(→ [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md)).

They exist for registration failures, and registration declined them for the
reason P2 describes. So the options are:

- **Delete both.** Honest, and a semver event on a `#[ non_exhaustive ]` enum.
- **Keep and document.** State on the type that they are reserved for a
  registry that shares this error, and that no such registry exists.
- **Keep and make them reachable** by re-exporting `RegistryError` (P4) and
  leaving these unused — the status quo, undocumented.

**This is genuinely open**, and it is downstream of P2: if the "one error type"
sentence is scoped to the ring path, these two variants are on the wrong side of
that scope and deleting them is coherent. If it is a rule the family means to
honour, they are the placeholder for honouring it.

**Pending 4 — should `ring_factory` re-export `RegistryError`?**
`ring_factory` is on the five-crate export Contract and re-exports `Registry`.
It does not re-export `RegistryError`. A Contract-bound consumer can therefore
call `Registry::register`, receive an `Err`, and have no way to name the type
they are holding without importing `ring_registry` — which the Contract forbids
(→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md), R1/R4).

The fix is one `pub use` line. It is recorded here rather than as an
implementation task because it is not obviously the right fix: re-exporting the
error makes the Contract's surface two error types wide, which is exactly the
outcome P2 says the family may or may not want. **Doing it settles P2 by
accident, in the direction of "the rule is scoped", without anyone ruling it.**

**Pending 5 — a gate asserting tier 0's `[dependencies]` stays empty.**
[`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md) states the
property the family's acyclicity rests on and reports that nothing enforces it.
Six gate scripts already run under `bench_harness/gate/`; a seventh
asserting this crate's dependency table is empty is a few lines.

**What makes it a decision rather than a task is the exception it would have to
encode.** The violation that will actually happen is `error_tools` replacing the
hand-written `Display` impl — the workspace's own house convention, applied by
someone doing the right thing everywhere else. A gate that fails on it either
blocks a convention the rest of the workspace follows, or carries a
`ring_types`-shaped exemption that has to be justified in the gate script. The
question is which, and the answer belongs to whoever owns the convention rather
than to this crate.

### Not decisions

Two things surfaced during documentation that are **fixes, not trade-offs**, and
are recorded in their instances rather than here so they do not inflate this
index:

- **`Seq::next`'s doc comment was wrong about release builds — fixed.** The
  one-sentence correction landed in `src/id.rs`
  (→ [`pitfall/001`](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)).
- **`OverflowPolicy::ALL` had no per-variant `contains` assertion — fixed.**
  `overflow_policy_has_no_overwrite_variant` now carries the same loop
  `WaitKind::ALL`'s test does, so a duplicated entry no longer passes every test
  in the workspace
  (→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md), T6, closed).

Both belong to `src/` and `tests/`, which the concurrent implementation session
owns. Neither has a second option worth weighing.

**One further question was ruled from outside rather than filed here.** Whether
this crate declares `docs/item/` at all was left open by
[`ring_flush`](../../../ring_flush/docs/definition/readme.md)'s module index on
family-consistency grounds; it has since been directed that every crate's `docs/`
carry one. It is therefore settled, and `item/` is declared
(→ [`../item/readme.md`](../item/readme.md)).


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/decisions
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  8
# rows in the table below:  8
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY1 | `ring_types` | **misleading doc** | `src/error.rs:1` states this is the one error type the whole ring family returns; the workspace declares six, five of them in other crates |
| TY2 | `ring_types` | n/a — doc gap | This crate's own P2 entry names `ring_registry` as the single counterexample to the one-error-type claim; four more exist and none is named anywhere in the corpus |
| TY3 | ring family | n/a — observation | Not one of the six error enums carries an `impl From` for any other; `ring_bench::RunError` unions three of them by hand-written variant instead |
| TY4 | `ring_factory` | n/a — observation | A Contract-bound consumer receives `RingError` from the ring-building crate only inside `BuildError::Unsupported`, and cannot name `RegistryError` at all |
| TY5 | `ring_types` | n/a — inconsistency | Two `#[ non_exhaustive ]` attributes across 23 public enums — `RingError` marked, `WaitKind` and `OverflowPolicy` beside it not — and no crate matches on a `RingError` value at all, so it currently costs nothing and buys nothing |
| TY6 | ring family | n/a — duplication | `RingError::NameTaken` is constructed nowhere while `ring_factory::BuildError::NameTaken` and `ring_registry::RegistryError::NameTaken` are each constructed and each public |
| TY7 | `ring_types` | n/a — coverage | No lookup path in `ring_registry` — the crate whose lookup it names — mentions `RingError::NameUnknown` |
| TY8 | `ring_registry` | n/a — unadopted | The only one of the 31 declared dependents that names no item from `ring_types` in `src/`; the manifest edge outlived the decision that created it |
