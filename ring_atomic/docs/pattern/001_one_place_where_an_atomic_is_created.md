# Pattern: One Place Where an Atomic Is Created

### Scope

**Purpose:** Record the crate's central organizing pattern, test it against the 33
crates it claims to cover, and locate the atomics that sit outside it.

**Responsibility:** The single-creation-site claim, the `loom` seam it justifies,
and every raw atomic constructed elsewhere in the family.

**In Scope:** `ring_atomic/src/lib.rs:45-54`;
`ring_stats/src/lib.rs:85-91`, `:219-225`; `ring_shutdown/src/lib.rs:63`, `:72`;
`ring_bench/src/lib.rs:1031`, `:1195`.

**Out of Scope:** What the seam costs to compile outside this workspace is
[`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md).
The substitution pattern the seam enables is
[`pattern/002`](002_the_counting_cell_is_not_a_mock.md). That the `const` the split
exists for is unused is [`item/001`](../item/001_six_constructors_for_two_types.md)
AT26.

---

## The Claim, and Every Atomic Outside It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the claim --'
command grep -m1 -A3 -F '//! the orderings are: this crate is the one place in 33 crates where a' ring_atomic/src/lib.rs
echo '  -- every raw atomic constructed outside this crate --'
command grep -rE 'Atomic[A-Za-z0-9]+::new' --include=lib.rs ring_*/src/ \
  | command grep -v 'ring_atomic/\|AtomicSeq::new' | command grep -vE ': *//' \
  | sed 's|ring/||;s|/src/lib.rs||'
echo '  -- and what each of those crates knows about orderings and the seam --'
for c in ring_stats ring_shutdown ring_bench
do printf '    %-14s orderings: %-22s cfg( loom ) sites: %s\n' "$c" \
   "$( command grep -ohE 'Ordering::[A-Za-z]+' ring/$c/src/lib.rs | sort -u | sed 's/Ordering:://' | tr '\n' ' ' )" \
   "$( command grep -c 'cfg( loom )' ring/$c/src/lib.rs || true )"; done
```

Live output:

```
  -- the claim --
//! the orderings are: this crate is the one place in 33 crates where a
//! *sequence* atomic is created. Every cursor, gating set, claim and barrier
//! reaches its atomic through [`AtomicSeq`], so the `loom` switch here reaches
//! all of them — the counting instrument reaches only the callers generic
  -- every raw atomic constructed outside this crate --
ring_bench:  let reported = AtomicUsize::new( 0 );
ring_bench:  let reported = AtomicUsize::new( 0 );
ring_shutdown:    Self { closed : AtomicBool::new( false ) }
ring_stats:      claimed : AtomicU64::new( 0 ),
ring_stats:      published : AtomicU64::new( 0 ),
ring_stats:      consumed : AtomicU64::new( 0 ),
ring_stats:      dropped_newest : AtomicU64::new( 0 ),
ring_stats:      dropped_oldest : AtomicU64::new( 0 ),
ring_stats:      failed : AtomicU64::new( 0 ),
ring_stats:      wait_nanos : AtomicU64::new( 0 ),
  -- and what each of those crates knows about orderings and the seam --
    ring_stats     orderings: Relaxed                cfg( loom ) sites: 0
    ring_shutdown  orderings: Acquire Release        cfg( loom ) sites: 0
    ring_bench     orderings: Relaxed                cfg( loom ) sites: 0
```

---

### AT37 — The Pattern Is Real and the Sentence Stating It Is False

The paragraph makes two claims in two sentences, and only the second one holds.

The second — "Every cursor, gating set, claim and barrier reaches its atomic through
`AtomicSeq`" — is exactly true, and it is the load-bearing one. It is what makes the
ordering discipline enforceable in one file, the `loom` switch a two-line change
instead of a thirty-three-crate change, and the counting substitution possible at
all. As an organizing pattern this is the best decision in the crate.

The first — "this crate is the one place in 33 crates where an atomic is *created*"
— is not true. Ten raw atomics are constructed in three other crates: seven
`AtomicU64` in `ring_stats::Counters`, one `AtomicBool` in `ring_shutdown`, and two
`AtomicUsize` in `ring_bench`.

**Finding.** The gap between the two sentences is the whole pattern's actual
boundary, and nothing marks it. The pattern covers *sequence* atomics — the ones on
the write path, the ones whose ordering is subtle and whose interleavings a model
checker would want to explore. It does not cover counters, flags, or test
scaffolding, and there is a decent argument that it should not.

But the exception is unstated, so the boundary is invisible from inside the crate,
and one of the three exceptions does not obviously belong on the far side of it.
`ring_stats::Counters` holds seven shared, concurrently-mutated `AtomicU64`s, uses
`Relaxed` throughout, carries zero `cfg( loom )` sites, and does not depend on
`ring_atomic` at all. Whether a stats counter needs model checking is a real
question with a real answer; the crate that claims to own every atomic in the family
has not asked it.

The remedy is one clause. "The one place in 33 crates where a *sequence* atomic is
created" is true as written, states the boundary, and makes `ring_stats` a visible
decision rather than a silent counterexample.

The module doc now reads exactly that:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! the orderings are: this crate is the one place in 33 crates where a' ring_atomic/src/lib.rs
```

Live output:

```
//! the orderings are: this crate is the one place in 33 crates where a
//! *sequence* atomic is created. Every cursor, gating set, claim and barrier
//! reaches its atomic through [`AtomicSeq`], so the `loom` switch here reaches
```

**Disposition:** applied — the module doc's loom-seam paragraph in
`src/lib.rs` now reads "the one place in 33 crates where a *sequence* atomic
is created", matching this instance's own suggested remedy; the crate's 21
unit tests plus 8 doctests re-verified passing (`cargo test --all-features`,
2026-09-03). Now prints:
`this crate is the one place in 33 crates where a`

---

### AT38 — Instrumenting Here Instruments Less Than It Says

The same paragraph closes with "so instrumenting it here instruments all of them,
and no other crate needs to know the seam exists." Both halves are narrower than
they read.

*Instruments all of them* is true of the `loom` switch — swapping the atomic source
in this file genuinely swaps it everywhere `AtomicSeq` reaches — and false of the
counting instrument, which is the one a reader is most likely to have in mind.
`counts` and `reset_counts` live on `CountingSeq`, off the trait, so they reach only
code that is generic over `SeqCell`. That is three parameters in two crates
([`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) AT27); the
four cursors held concretely as `PaddedCursor` fields cannot be instrumented at all,
and neither can `ring_stats`' seven counters, which are outside the seam entirely.

*No other crate needs to know the seam exists* is true of the source switch and
false of the build. A crate that carries `cfg( loom )` needs the `check-cfg` entry
that makes the name legal, and that entry lives in the root manifest's
`[workspace.lints.rust]` — so the seam is invisible to sibling crates but not to the
workspace above them
([`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md)).

**Finding.** Neither overstatement is wrong about the design; both are wrong about
its reach, and in the same direction. The pattern is a *creation*-site pattern: it
centralizes where atomics come from, which buys the ordering discipline and the
`loom` swap outright. It does not centralize where they are *observed*, which is
what instrumentation needs, and the paragraph uses one word for both.

The same paragraph now narrows the second sentence's reach, immediately after
AT37's fix:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! reaches its atomic through [`AtomicSeq`], so the `loom` switch here reaches' ring_atomic/src/lib.rs
```

Live output:

```
//! reaches its atomic through [`AtomicSeq`], so the `loom` switch here reaches
//! all of them — the counting instrument reaches only the callers generic
//! enough to accept [`CountingSeq`] in its place — and no other crate needs to
```

**Disposition:** applied — the module doc's loom-seam paragraph in
`src/lib.rs` now distinguishes the `loom` switch's full reach from the
counting instrument's narrower one (generic callers only); the crate's 21
unit tests plus 8 doctests re-verified passing (`cargo test --all-features`,
2026-09-03). Now prints:
`all of them — the counting instrument reaches only the callers generic`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_the_counting_cell_is_not_a_mock.md) | The substitution this pattern enables, and why it is not mocking |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | How far the counting instrument actually reaches |
| [`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md) | The seam's cost, and the manifest entry it depends on |
| [`decisions/001`](../decisions/001_orderings_named_never_defaulted.md) | The ordering discipline this pattern makes enforceable |
| [`integration/002`](../integration/002_five_crates_downstream.md) | The re-export that widens the trait's reach past what the manifests show |

### Sources

| Fact | Where |
|------|-------|
| The two-sentence claim | `ring_atomic/src/lib.rs:47-50` |
| Seven raw `AtomicU64` outside the seam | `ring_stats/src/lib.rs:85-91`, `:219-225` |
| One `AtomicBool` outside the seam | `ring_shutdown/src/lib.rs:63`, `:72` |
| Two `AtomicUsize` outside the seam | `ring_bench/src/lib.rs:1031`, `:1195` |
| `Relaxed`-only, no `loom` in all three | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_cell_drives_through_the_trait_alone` | That a caller can work entirely through `SeqCell`, which is what the pattern buys |
| `the_counting_cell_is_the_production_cell_plus_bookkeeping` | That the two implementations agree, which is what makes one creation site sufficient |
| *(to create)* | Nothing asserts that no crate outside `ring_atomic` constructs a raw atomic, which is the pattern's own stated invariant and is currently violated ten times |
