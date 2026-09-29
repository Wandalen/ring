# Decisions

### Scope

- **Purpose**: Record the architecture decisions for `ring_factory` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: The five-crate Contract itself, which is ruled at family grain rather than by this crate; `ring_config`'s choice to clamp rather than reject, which is that crate's and is already made.

Not a doc-definition collection — per `doc_des.rulebook.md`'s classification of
`docs/decisions/` as a non-doc-definition directory, ADRs here use the format at
`doc_des.rulebook.md § Architecture Documentation : Architecture Decision
Records` and are indexed only in this file, not in `definition/readme.md` or
`graph.yml`.

### Index

| ADR | Rules | Status |
|---|---|---|
| [001 — The Owner Is The Return Value](001_the_owner_is_the_return_value.md) | Pendings 1, 2, 3, 4 | accepted 2026-08-28 |
| [002 — Two Doors, Not One That Routes](002_two_doors_not_one_that_routes.md) | Pendings 9, 10 | accepted 2026-08-28 |

**Ten pendings stood here; two remain.** Eight were closed when this crate was
implemented, and the manner of their closing is the finding worth keeping:

| Closed by | Pendings | How |
|---|---|---|
| Code written **below** this crate, while the question sat open | 2, 6 | `ring_handle` shipped `Split`; `ring_registry` shipped `get_mut` |
| ADR 001, following from that code | 1, 3, 4 | The owner is the return value, so the factory needs no field and the door re-exports |
| ADR 002 | 9, 10 | Two doors, and the refusal names the other one |
| Implementation, by the branch that removes rather than adds | 7 | Two manifest lines went |

**This inverts the pattern this file recorded when the count was ten.** It said
then: "a skeleton accumulates decisions from its dependencies faster than it
resolves its own." True while it was a skeleton — Pendings 9 and 10 both arrived
from `ring_core` being written. But the same dependency traffic runs the other
way once the crates below are finished, and it runs *faster*: two pendings
arrived from below over the family's build-out, and two were **answered** from
below in a single sitting, taking three more with them as consequences.

**The asymmetry is that arriving questions are one-at-a-time and answers come in
clusters.** A dependency that refuses a policy creates exactly one question. A
dependency that ships a *shape* — `Split` owning its ring — answers every
question that was waiting on that shape at once. Pendings 1, 2 and 3 were
recorded as "three that are one question", correctly, and one commit in another
crate resolved all three.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^\*\*Pending [0-9]+ —' ring_factory/docs/decisions/readme.md
command grep -cE '^\*\*Closed [0-9]+ —' ring_factory/docs/decisions/readme.md
```

Live output:

```
2
8
```

Both greps now agree with a looser `grep -c '^\*\*Pending'` / `'^\*\*Closed'`,
because the paragraph that used to introduce Pendings 1–3 as one question — and
that inflated the loose count by one — is gone with them.

### Closed

#### The three that were one question — all closed by `ring_handle` shipping

**Closed 1 — is the registry held or passed?** **Passed.** A `Factory` holding a
`Registry` would own every ring it ever built, since
`ring_registry::Registry< T >` owns the `Split< T >` values in it and `build` now
produces exactly such a value. `Factory` stays `pub struct Factory;` —
[`type/001`](../type/001_factory.md)'s fieldless definition, ruled rather than
provisional. → [ADR 001](001_the_owner_is_the_return_value.md).

**Closed 2 — which backend shape does `ring_handle` use?** **D2**, and it was
decided by that crate being written rather than by anyone ruling here.
`Split< T >` owns a `ring_core::Ring< T >` by value; `ends`/`split` borrow. The
cost [`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md)
scored as *severe* did not materialise, because it enumerated two possible
owners and the answer was a third — the return value.
→ [ADR 001](001_the_owner_is_the_return_value.md).

**Closed 3 — where does lookup live?** **On `ring_registry`, re-exported here.**
The question presupposed that routing the named registry *through* the factory meant
reimplementing its surface; it does not. `pub use ring_registry::Registry;`
satisfies this family's own Contract ruling without a second implementation of `get_mut` to keep
in step. → [ADR 001](001_the_owner_is_the_return_value.md).

#### The rest

**Closed 4 — `RingConfig` is not on the export Contract.** Option W3:
`pub use ring_config::RingConfig;`. W2 (move it into `ring_types`) remains the
more honest long-term shape and is not taken, because it changes two shipped
crates to buy a property the re-export already delivers.
→ [ADR 001](001_the_owner_is_the_return_value.md).

**Closed 6 — what does the registry retain?** `ring_registry` shipped, and it
retains the whole `Split< T >`, lending `&mut Split< T >` from `get_mut`.
The producer/consumer partition survives: a borrow yields one `Ends`, and one `Ends`
yields one producer and one consumer. The feared reading — a registry handing out
`Producer` values, issuing a second producer for a ring that already has one — is
not what was built and cannot be reached from what was.

**Closed 7 — `ring_stats` and `ring_tls` are declared and unused.** The second
branch: **two manifest lines went.** `RingConfig` has no field that would
configure staging or counters, so there was nothing for a build to do with
either, and inventing surface to justify a manifest line is backwards. Restoring
them is a one-line change once the config fields exist; the deviation from
the crate's own original manifest assignment is commented in place in `Cargo.toml`. `cargo +nightly udeps` is the
check that this stayed true.

**Closed 9 — how does `RingError::PolicyUnsupported` reach the caller?**
`BuildError::Unsupported( RingError )`, relayed and never re-decided here.
→ [ADR 002](002_two_doors_not_one_that_routes.md).

**Closed 10 — is the optional crossbeam backend reachable through the factory?**
Yes, through a second door — `build_crossbeam`, documented as outside the
one-door promise rather than routed inside it. Routing on the policy field would
have made one config produce different rings under different cargo features.
→ [ADR 002](002_two_doors_not_one_that_routes.md).

### Pending

Two remain. Both are questions this crate can pose and cannot answer alone.

**Pending 5 — does the factory resolve `WaitKind`, or pass the record down?**
[`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)'s
W3-versus-W4. Wait strategies are `ring_wait`'s to assign; this crate's own design
says `RingConfig` is the only constructor input; nothing says which crate turns
the discriminant into a waiter.

**The question narrowed while this crate was written, and it narrowed the
answer's value along with it.** `ring_wait` shipped, and its surface is free
functions taking a `WaitKind` per call:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^pub fn' ring_wait/src/lib.rs
```

Live output:

```
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

`pause`, `wait_until`, `wait`, `for_space`, `for_data` — every one of them takes
the kind as an argument. **There is no waiter object to construct**, so W3
("the factory resolves it") has nothing to resolve *into*; it would amount to
the factory validating a discriminant and discarding it. The remaining live
question is smaller than the one recorded: not *who builds the waiter* but
*who carries the kind from the config to the call site*, since a `Split` does
not carry its config and `RingConfig` is `Copy` so the caller still has theirs.

**It is still not answered by omission.** The current code ignores the field,
and "the caller keeps their config" is a real answer that happens to produce the
same code — which is precisely why it needs saying rather than assuming.
`ring_wait` is deliberately not a dependency of this crate, and the reason is
now positive rather than incidental: there is nothing to depend on it *for*.

**Pending 8 — should `Ring::with_config` exist?**
[`invariant/002`](../invariant/002_construction_is_the_only_path.md)'s L2. Both
backends expose it and both read one field of five. It is not obviously wrong —
a per-backend constructor using the applicable part of the record is defensible
— but its *name* claims more than it does, and it is the one construction path
that reads as compliance while bypassing the factory. Three answers: delete it,
rename it (`with_capacity_from`), or document the four omissions in its doc
comment. **This is `ring_spsc`/`ring_mpsc`'s to rule and is recorded here
because this is the crate whose invariant it breaks.**

**That question got more expensive while this file was being written.** It was
recorded when `with_config` had zero callers, where all three answers cost the
same. `ring_core::Ring::new` now calls it on both branches, so a rename touches
a caller and a deletion needs a replacement — the doc-comment answer is the only
one that is still free. **Delay changed the answer's price, not the question**,
which is the argument for ruling the cheap ones early.

#### Three questions deliberately not recorded here

**`ring_handle` should be a declared dependency.** ✅ **Fixed, not decided.** It
was reachable only through `ring_registry`, which is not enough to *name* a type,
and `build`'s return type is `Split< S >`. A one-line manifest fix with an
obvious answer and no trade-off — a bug. `ring_types` turned out to need the same
fix for the same reason, since `BuildError::Unsupported` carries a `RingError`.
Both are now declared, with the deviation from the original manifest assignment commented in place.
→ [`integration/001`](../integration/001_declared_edges_and_the_reached_closure.md)'s
requirement 1.

**Whether `FlushPolicy` belongs in `RingConfig`.** `ring_flush` is a peer on the
Contract with its own policy surface, so a consumer configures a ring and a
flush policy through two doors. Merging them would make `RingConfig` depend on
`ring_flush` and make every ring carry a staging decision it may not use. The
current split is the obvious default and nothing has pushed against it; it
becomes a question only if someone finds the two-door arrangement confusing in
practice.

**Whether `build` should return the effective config.**
[`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)'s
mitigation 2. It is cheap and would close the sweep-labelling gap — but the same
gap closes at zero cost by having the harness read `cfg.batch()` back out after
construction, since `RingConfig` is `Copy` and the caller keeps their copy
(→ [`lifecycle/003`](../lifecycle/003_config_state_through_a_build.md)'s
B5). **A decision is not needed for a problem that has a free solution
elsewhere**, and adding state to the handle pair to solve it would work against
`ring_handle`'s design of carrying nothing.

### Sources

| File | Relationship |
|------|-----------------|
| [`ring_handle/docs/decisions/readme.md`](../../../ring_handle/docs/decisions/readme.md) | Where Pending 2 is ruled |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/decisions
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC13 | `ring_factory` | n/a — observation | The ruling is longer than the crate it rules: ADR 001 is 197 lines against `src/lib.rs`'s 76 non-doc lines, and produces two `pub use` lines and two `Ok( Split::new( ring ) )` statements |
| FC14 | `ring_factory` | n/a — observation | The decision was forced before it was taken — `ring_handle::Split::split` borrowing `&'a mut self` fixed the owner as the return value while the question was still recorded as open |
| FC15 | `ring_factory` | **latent hazard** | The two doors have the same signature, so the ruling is enforced by a name: `build` and `build_crossbeam` differ only in identifier, and a caller who picks the wrong one gets a different backend with no type error |
| FC16 | `ring_factory` | n/a — doc gap | Eight of ten pendings are closed and the two left are not this ruling's residue; the residue it did create — `build_crossbeam`'s expiry condition — appears in no pending, no task, and no gate |
