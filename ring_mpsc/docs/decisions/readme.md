# Decisions

### Scope

- **Purpose**: Record architecture decisions for `ring_mpsc` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: The crate's existence and shared, family-neutral shape, which is a family-level decision, not a local ADR; the concrete ring pattern, which remains an open question at the family level, not this crate's to decide unilaterally either.

ADRs here use the format at `doc_des.rulebook.md § Architecture Documentation :
Architecture Decision Records`. They are indexed both below and in
[`definition/readme.md`](../definition/readme.md), which counts every instance
under every definition and would otherwise report a total that does not match
the directory.

### Index

| ID | Decision | Status | Turns on |
|----|----------|--------|----------|
| [001](001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md) | `ring_core` sits on the unsafe allowlist without unsafe | **Open** | Whether an allowlist entry for a crate that uses no unsafe is a gate that cannot fail |
| [002](002_the_observation_surface_kept_without_a_caller.md) | The observation surface kept without a caller | **Open** | Whether seven accessors reached only by this crate's own tests are a test affordance that leaked or an API awaiting its consumer |

**Both were raised by the item census rather than by a design discussion.** 001
is not this crate's alone to settle — the allowlist is a repository-level
artifact whose ruling lives elsewhere, so what is filed here is the evidence and
the reading, not the change.

The trade-off a reader might expect instead — which ring pattern to implement —
is a family-level question, because the answer is shared with another
prospective consumer's own merge mechanism and is not this crate's alone to
decide.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/decisions
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP13 | the allowlist | n/a — inconsistency | `ring_core` is declared as permitted to opt out of `unsafe-code = "deny"` and contains no `unsafe` and no opt-out attribute. |
| MP14 | `G6` | **latent hazard** | The gate's scan pattern omitted the house spacing, so its justification half never ran for any crate. |
| MP15 | `workaround/readme.md` | n/a — doc gap | The allowlist requires each named crate to justify its opt-out in its own `docs/workaround/readme.md`; this crate's said "None" and named a dependency list that was wrong. |
| MP16 | the observation surface | n/a — unadopted | Every public method that reports state rather than changing it has zero production callers; every method that changes state has some. |
| MP17 | the observation surface | n/a — coverage | Making the seven private would make three of this crate's own assertions unwritable where the family requires tests to live. |
