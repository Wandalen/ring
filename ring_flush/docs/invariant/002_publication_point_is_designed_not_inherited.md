# Invariant: The Publication Point Is Designed, Not Inherited

### Scope

- **Purpose**: Turn the flush policy's "If Missing" clause into a standing restriction — the moment a buffer's contents become visible must be a property of a configured policy, never of which call site happened to be written first.
- **Responsibility**: State the restriction, name what enforces it, and record what is lost when publication order becomes an accident of authorship.
- **In Scope**: Ownership of the publication moment; the single-point requirement.
- **Out of Scope**: The ordering *within* a flushed batch, which is `ring_batch`'s contiguous claim; cross-thread merge order, which is a separate concern from where a single buffer publishes.

### Invariant Statement

**The point at which a thread-local buffer's contents become visible in the
ring is determined by a configured `FlushPolicy`, and by nothing else.**

Losing this invariant has a direct consequence:

> Flushing happens wherever someone thought to write it, so a buffer's contents
> reach the ring at a point that varies by call site. The publication order
> stops being a property of the design and becomes a property of the code that
> happened to be written first.

**That last sentence is the invariant, negated.** This crate exists to make the
publication point a designed property. Everything else it does — three
variants, a driver, a log — is machinery in service of that one restriction.

**It is a stronger claim than [trigger exclusivity](001_a_policy_fires_only_at_its_trigger.md).**
That invariant says a policy fires only when it should. This one says nothing
*other than a policy* decides publication at all. A system could satisfy the
first perfectly — every policy firing exactly on cue — while a second code path
flushes on its own schedule beside it, and this invariant would be broken while
the other held.

### Enforcement Mechanism

| # | Mechanism | Strength |
|---|-----------|----------|
| P1 | This crate owns the seal/drain/reset sequence (→ [`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)) | Real, but only for callers who route through it |
| P2 | Policy is a value a consumer configures once, not a call a consumer makes repeatedly (→ [`pattern/001`](../pattern/001_policy_as_a_value.md)) | Real — it moves the decision out of the call site by construction |
| P3 | `ring_flush` is on the export Contract, so a consumer names it deliberately | Weak. Being nameable does not make it unavoidable |
| P4 | `ring_tls`'s primitives are public | **Negative** — this actively works against the invariant (→ [`invariant/001`](001_a_policy_fires_only_at_its_trigger.md)'s E3) |
| P5 | A gate checking that no crate outside `ring_flush` calls `ring_tls`'s drain | **Does not exist.** G5 checks manifests, not call sites |

**P2 is the only mechanism that enforces rather than merely encourages.** A
policy held as a value cannot be "where someone thought to write it" — there is
one place it lives and one place it is consulted. The rest of the table is
convention, and P4 is a standing invitation to break it.

**P5 is worth naming even though it does not exist,** because it is the shape
the enforcement would take: a check that `ring_tls`'s consolidator surface has
exactly one caller in the workspace. That is mechanically checkable — it is a
grep over `Cargo.toml` reverse-dependencies plus a source scan — and it is the
kind of thing
[`bench_harness`'s gate directory](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
already houses six of. Its absence is a gap, not an impossibility.

### Violation Consequences

| # | Violation | What is lost |
|---|-----------|--------------|
| C1 | A system flushes its own buffer before a call it knows is slow | The publication point now depends on that system's internal knowledge. Reordering two systems changes the visible order of unrelated commands |
| C2 | A library flushes defensively in its own teardown | Order becomes a function of drop order, which is declaration order, which is not a design decision anyone made |
| C3 | Two policies are configured for the same buffer | Undefined — whichever is consulted first wins, and "first" is call-site order, the exact property this invariant excludes |
| C4 | The driver is called from two places at different cadences | The effective policy is the union of two schedules, which matches neither |
| C5 | A flush is added inside `ring_tls` itself, for its own reasons | This crate's policy becomes advisory. Nothing here can detect it |

**C1 is the realistic one and it does not look like a violation while it is
being written.** A system that knows it is about to block, flushing so its
commands are not stranded, is behaving considerately. It is also making the
publication point a property of that system's implementation — and the next
person to reorder the schedule inherits a dependency nobody documented.

**C3 and C4 are the same defect at different scopes** and they are the reason
[the driver surface](../api/002_the_driver_surface.md) has to be explicit about
who calls it. A policy is a value; a *schedule* is not, and this crate owns
only the first.

**C5 is not hypothetical — see `FL24` below.** `ring_tls` ships
`flush_into( cursor, order )`, a public, documented method that fuses claim and
drain against a caller-supplied cursor and publishes without consulting any
`FlushPolicy`. Every call site today is `ring_tls`'s own test suite, publishing
into a test-owned cursor — no product code path reaches it, so C5's condition
holds and its consequence has not landed. The table has no column for that
middle state; this note is it.

**The determinism this invariant protects is not this crate's to guarantee.**
It supplies the mechanism by which publication becomes designed; whether the
design is actually deterministic depends on the consumer calling the driver at
a consistent point. That is the same division
[`ring_handle`'s barrier lifecycle](../../../ring_handle/docs/lifecycle/002_the_barrier_holds_the_consumer.md)
draws — the crate makes the property expressible, the scheduler makes it true.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) | P1 — the sequence this crate owns, and the only route that respects the invariant |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | Why P5's check would live in `bench_harness` rather than here |

### Invariants

| File | Relationship |
|------|--------------|
| [001_a_policy_fires_only_at_its_trigger.md](001_a_policy_fires_only_at_its_trigger.md) | The narrower companion — satisfiable while this one is broken |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) | P2 — the only mechanism in the table that enforces by construction |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/readme.md`](../../../ring_tls/readme.md) | The buffer whose publication point this invariant fixes as a designed property |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | P2 is demonstrated by `the_bound_policy_never_changes` (configured once, unchanged by driving) and `appending_never_publishes` (the decision is not at the call site). C1–C5 remain undetectable from here, as this instance said — with one correction: **C2 was reachable and now is not.** `dropping_a_driver_with_records_staged_publishes_nothing` asserts this crate never becomes the library that flushes in its own teardown |

### FL23 — The Gate Directory Tripled and the One Gate This Invariant Specified Is Still Absent

P5 names a check, argues it is easy, and cites the directory that would house it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the gates that exist --'
ls bench_harness/gate/g*.sh | sed 's|.*/g|    g|' | sort -t g -k2 -n | tr '\n' ' '; echo
printf '    count: %s\n' "$( ls bench_harness/gate/g*.sh | wc -l )"
echo '  -- what P5 says about that directory --'
awk '/^### FL/{ exit } /already houses/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/invariant/002_publication_point_is_designed_not_inherited.md
echo '  -- and whether any of them looks at a call site rather than a manifest --'
printf '    gates naming ring_tls at all: %s\n' \
  "$( command grep -l 'ring_tls' bench_harness/gate/g*.sh 2>/dev/null | wc -l )"
```

Live output:

```
  -- the gates that exist --
    g1_coverage.sh     g2_docs.sh     g3_features.sh     g4_manual.sh     g5_export_surface.sh     g6_unsafe.sh     g7_determinism.sh     g8_oracle_form.sh     g9_lint.sh     g10_pinned_math.sh     g12_mutation.sh     g13_survey_freshness.sh     g14_corpus_shape.sh     g15_corpus_recipes.sh     g16_corpus_citations.sh     g17_corpus_vocabulary.sh     g18_family_coverage.sh     g19_measured_columns.sh     g20_corpus_disposition.sh     g21_corpus_addressing.sh     g22_exemption_expiry.sh 
    count: 21
  -- what P5 says about that directory --
    55: already houses six of. Its absence is a gap, not an impossibility.
  -- and whether any of them looks at a call site rather than a manifest --
    gates naming ring_tls at all: 0
```

Twenty-one gates. P5 says the directory "already houses six of" this kind of
thing, which was true when it was written, and the growth to twenty-one happened
without the one gate this invariant specified being among them. The three most
recent — `g20_corpus_disposition.sh`, `g21_corpus_addressing.sh`, and
`g22_exemption_expiry.sh` — were added while someone was reading this very
document, which is the sharpest available restatement of the finding: a plan can
add gates while looking straight at a specification for one it is not adding.

**P5 is unusually well-specified for an absent mechanism.** It names the check
("`ring_tls`'s consolidator surface has exactly one caller in the workspace"),
argues it is mechanically decidable, and describes the implementation as a
reverse-dependency grep plus a source scan. That is more design than several of
the twenty-one gates needed. It closes with "its absence is a gap, not an
impossibility," which is correct and has now been correct across fifteen
additions.

**No gate names `ring_tls`**, so the absence is not partial — nothing in the gate
directory looks at this crate's enforcement problem from any angle. The gates
that were added grade manifests, features, determinism, lint, mutation, corpus
shape, corpus recipes, corpus citations, corpus vocabulary, corpus disposition,
corpus addressing, family coverage, measured columns and exemption expiry. Every one is a property of the repository's *form*; P5 is a
property of its *call graph*, and no gate reads one.

The reusable shape: **a well-specified absent mechanism competes for attention
with under-specified present ones and loses, because nothing enumerates
specifications that were never built.** Twenty-one files say what is checked. One
sentence in one instance says what is not, and it is filed under the invariant it
would protect rather than beside the gates it would join.

### FL24 — C5 Is Written as a Hazard and the Method It Describes Is Public, Documented, and Exercised Fourteen Times

C5 imagines a flush added inside `ring_tls`; `ring_tls` shipped one:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- C5, as written --'
awk '/^### FL/{ exit } /^\| C5 \|/{ printf "    %s\n", substr( $0, 1, 112 ) }' \
  ring_flush/docs/invariant/002_publication_point_is_designed_not_inherited.md
echo '  -- ring_tls'\''s own publication surface --'
command grep -E '^  pub fn (drain|flush_into)' ring_tls/src/lib.rs
echo '  -- call sites of the fused form, family-wide --'
printf '    ring_tls/src:    %s\n'   "$( command grep -c 'flush_into(' ring_tls/src/lib.rs )"
printf '    ring_tls/tests:  %s\n'   "$( command grep -c 'flush_into(' ring_tls/tests/tls_test.rs )"
printf '    anywhere else:   %s\n'   "$( command grep -rl 'flush_into(' ring_*/src ring_*/tests 2>/dev/null | command grep -cv 'ring_tls' )"
```

Live output:

```
  -- C5, as written --
    | C5 | A flush is added inside `ring_tls` itself, for its own reasons | This crate's policy becomes advisory. No
  -- ring_tls's own publication surface --
  pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_
  pub fn flush_into< C >( &mut self, cursor : &C, order : Ordering ) -> Flush< '_, T >
  -- call sites of the fused form, family-wide --
    ring_tls/src:    2
    ring_tls/tests:  14
    anywhere else:   0
```

`flush_into( cursor, order )` fuses the claim and the drain against a
caller-supplied cursor. It publishes. It is public, it carries a doc example, and
`ring_tls`'s own suite calls it fourteen times.

**C5 says "a flush is added inside `ring_tls` itself, for its own reasons" and
files the consequence as "nothing here can detect it."** Both halves are already
true in the present tense. The method is not a risk this invariant is watching
for; it is the state of the workspace, and it has been since before this instance
was written — `integration/001` discusses it at length, under the heading of why
this crate uses `drain()` instead.

**The interesting part is that the invariant is not actually violated.** Every
`flush_into` call site is in `ring_tls`'s own tests, publishing into a cursor
those tests own, with no `ring_flush` policy in the picture — no *product* code
publishes outside a policy. So C5's condition holds and its consequence has not
landed, which is a distinction the row has no column for: it lists violations and
what they cost, with nowhere to record "present, exercised, and not yet harmful."

The reusable shape: **a hazard table written in the conditional cannot express a
condition that is already met.** C5 reads as a warning, and warnings are checked
by asking "has this happened yet?" — the wrong question for a row whose subject
shipped, in a sibling crate, before the warning was written.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m1 'not hypothetical' ring_flush/docs/invariant/002_publication_point_is_designed_not_inherited.md
```

Live output:

```
**C5 is not hypothetical — see `FL24` below.** `ring_tls` ships
```

**Disposition:** applied — a note now sits between the C3/C4 paragraph and
the determinism paragraph, stating C5's real status: `flush_into` exists,
is public, and is exercised, but every call site today is confined to
`ring_tls`'s own tests, so the condition holds and the consequence has not
landed. This gives the table the "present, exercised, not yet harmful" middle
state the finding says it has no column for, without adding a gate or a new
table column — P5 (the call-site-checking gate FL23 already covers, filed
separately) remains the mechanism that would need to exist to detect a future
product-code violation; this disposition only corrects what the prose claims
about the present. Now prints: `not hypothetical`
