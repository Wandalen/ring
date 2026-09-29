# Decisions

### Scope

- **Purpose**: Record architecture decisions for `ring_spsc` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: The crate's existence and its place in the family, which is a family-level decision; the contract gaps ruled at family grain, including this crate's export position.

ADRs here use the format at `doc_des.rulebook.md § Architecture Documentation :
Architecture Decision Records`. They are indexed both below and in
[`definition/readme.md`](../definition/readme.md), which counts every instance
under every definition and would otherwise report a total that does not match
the directory.

### Index

| ID | Decision | Status | Turns on |
|----|----------|--------|----------|
| [001](001_the_switching_cost_argument_undercounts_its_own_blast_radius.md) | The switching-cost argument undercounts its own blast radius | **Open** | Whether the Decision Gate ruling that kept three shape questions unfiled survives a corrected blast radius, and whether export position measures coupling at all |
| [002](002_the_loom_seam_runs_through_a_crate_this_manifest_never_names.md) | The loom seam runs through a crate this manifest never names | **Open** | Whether a build-configuration dependency invisible in the manifest should be made visible |

### The Three Shape Questions, and Why None Became an ADR

Three questions were explicitly undecided in the instances — the producer's
three candidate shapes ([`api/001`](../api/001_producer_surface.md)), the
borrow-versus-copy drain ([`api/002`](../api/002_consumer_surface.md)), and the
cursor initialization value
([`lifecycle/001`](../lifecycle/001_ring_construction_and_teardown.md)).

None passed the Decision Gate, because the gate tests *switching cost* and
the family's own layering rules set that cost deliberately low: `ring_spsc` is
internal, so changing any of the three reaches only its in-repo consumers
(→ [`integration/002`](../integration/002_reached_through_the_export_surface.md)).
A question answerable later by a two-crate refactor is a question to leave open
and decide against real code, not one to spend an ADR on now.

**Which consumers, exactly, is what 001 above reopens — and the answer is three,
not two.** The argument as originally written named `ring_core` and
`ring_handle`. `ring_bench`, which depends on this crate and calls into it, was
not named; `ring_handle`, which was, declares no dependency and imports nothing,
yet pins this crate's `Producer` declaration verbatim in a `trybuild` fixture
that runs on every ordinary test. So the count is wrong, the names are wrong,
and the one coupling nothing in the dependency graph can show is the one the
argument's own criterion — export position — is structurally unable to measure.
The conclusion below stands as a ruling; the reasoning that reached it has not
been re-taken against the measured set.

**All three are now closed, and closing them without an ADR is the outcome
this section predicted rather than a gap it failed to catch.** Each was settled
in its own instance at implementation time — a publishing guard with a fused
`try_push` over it; a borrowed batch that commits on drop; both cursors at
`Seq::ZERO` — and each is recorded where the reasoning lives rather than in a
separate record that would duplicate it. The cheap-to-reverse call was correct:
the producer shape in particular went from "three candidates" to one in the
time it took to discover that `Slot` has no generic by-value setter, which no
amount of ADR deliberation would have surfaced earlier than writing the code
did.

**The comparison with [`ring_tls`](../../../ring_tls/docs/decisions/readme.md)
is the useful one.** That crate's equivalent open question — its append
signature — is a public contract question, because `ring_tls` is on the
five-crate export list. Same class of uncertainty, two crates, and the
Decision Gate separates them correctly.

**One question here would have passed the gate, and it was answered at family
grain as this section said it should be.**
[`integration/001`](../integration/001_family_dependency_seam.md)'s
Compatibility Requirement 2 — whether `ring_claim`, `ring_publish` and
`ring_consume` grow uncontended variants or this crate implements those three
steps itself — had real switching cost and a silent failure mode, and was not
local: the answer binds `ring_mpsc` too.

It resolved a third way, which is the part worth recording. Reading the three
crates showed each exists for a question single-producer does not ask — a CAS
loop, a per-slot stamp, a minimum over a set — so there was no uncontended
variant to grow. This crate implements the three steps itself, in five atomic
operations, and does not depend on any of them; the manifest that declared all
three was scaffolding, not a design. `ring_mpsc` is unaffected, so the binding
that made this non-local never materialised.

The one decision this crate did need was also not local, and is on the record:
This crate's own design sites the `unsafe` here rather than in `ring_store`
or `ring_slot`, because the invariant making it sound spans slot storage *and*
the cursor protocol.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/decisions
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP13 | the `ring_handle` fixture | **latent hazard** | A pinned `.stderr` in `ring_handle` copies this crate's `Producer` declaration verbatim, so reformatting it reddens a test two hops away with no dependency edge to explain why. |
| SP14 | the first reach scan | n/a — observation | The scan that reported two consumers looped over a four-crate list written from memory, leaving twenty-eight crates unexamined for the right answer. |
| SP15 | the consumer set | n/a — observation | Three consumers break three ways, and the two the argument omitted are the two whose failure does not name the file that caused it. |
| SP16 | `--cfg loom` | n/a — unenforced | The flag changes a type two crates away and appears in no build file as a declared configuration. |
| SP17 | the loom comment | n/a — duplication | Both manifests carry the same `ring_atomic` argument; only `ring_mpsc` declares the dependency it argues from. |
