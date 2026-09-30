# Invariant: One Constant for the Whole Family

### Scope

- **Purpose**: State the restriction this crate's existence imposes on the other 32 `ring_*` crates — that there is one answer to "how big is a cache line" and it lives here — and record that the restriction is currently violated.
- **Responsibility**: State the restriction, name what enforces it and how weakly, record the live violation, and say what it costs.
- **In Scope**: Ownership of the cache-line number across the family; the second copy in [`ring_mpsc`](../../../ring_mpsc/readme.md).
- **Out of Scope**: Whether the number is correct, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md); the pair-separation property, which is [`invariant/001`](001_two_wrapped_fields_never_share_a_line.md).

### Invariant Statement

**The cache-line size appears once in the family, as
[`ring_align::CACHE_LINE`](../type/001_cache_line.md). No other crate declares
it, hardcodes it, or derives it independently.**

This is a stronger claim than [`invariant/001`](001_two_wrapped_fields_never_share_a_line.md)
and a different kind of claim. That one is about a layout the compiler produces
and is enforced by the compiler. This one is about *where a number is written*
across 33 crates, and nothing enforces it at all.

[`ring_cursor`'s readme](../../../ring_cursor/readme.md) states it as the
reason this crate exists:

> The padding *decision* is not here — it is `ring_align`'s single
> `CACHE_LINE` — and keeping it there is what stops the family from acquiring
> two independent answers to the same question.

**"Two independent answers to the same question" is this invariant, negated.**

### Enforcement Mechanism

| # | Mechanism | Strength |
|---|-----------|----------|
| Q1 | The constant is `pub` and the crate is a declared dependency of anything that needs it | Convention. Being available does not make a literal `64` unavailable |
| Q2 | `ring_cursor` holds the only padded type, so most crates never need the number | Real, and it is why the violation count is one rather than many |
| Q3 | Crate readmes state the ownership | Documentation |
| Q4 | A gate rejecting a bare `64` in a layout context anywhere in `ring_*` | **Does not exist** |
| Q5 | Type-level enforcement — e.g. an opaque `LineSize` newtype that cannot be constructed from a literal | **Not attempted**, and would not reach `#[ repr( align( … ) ) ]` anyway, which requires a literal (`E0693`, → [`type/001`](../type/001_cache_line.md)) |

**Q2 is doing all the real work**, and it is a property of the family's shape
rather than a mechanism: if only one crate needs padding, only one crate can
duplicate the number. That protection weakens as more crates grow their own
layout assertions — which is exactly how the violation below arrived.
`ring_cursor`'s own module doc (`ring_cursor/src/lib.rs:16-18`) names the
failure mode in advance, and names it precisely — the padding decision is made
once so that every cursor in the family inherits it *"without a second author
deciding 64 was probably fine."* A second author decided 64 was probably fine.

**Q4 is worth naming even though it does not exist,** because it is the shape
the enforcement would take, it is mechanically checkable, and running it by
hand today produces a short enough list to triage:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\b64\b' --include=*.rs ring_*/src/ \
  | command grep -viE 'u64|i64|f64|aarch64|x86_64|base64' \
  | command grep -v '^ring_align/' \
  | sed -E 's/:[0-9]+:/: /' | LC_ALL=C sort -u
```

Live output:

```
ring_atomic/src/lib.rs: //! with zero atomic operations" and feature 177's "a claim of 64 slots issues
ring_atomic/src/lib.rs: /// // One operation bought all 64 slots.
ring_atomic/src/lib.rs: /// let first = cell.fetch_add( 64, Ordering::AcqRel );
ring_atomic/src/lib.rs: /// zero for a thread-local accumulation, one for a 64-slot batch claim.
ring_batch/src/lib.rs: /// assert_eq!( batch.len(), 64 );
ring_batch/src/lib.rs: /// assert_eq!( cursor.counts().total, 1, "64 slots, one atomic operation" );
ring_batch/src/lib.rs: /// assert_eq!( cursor.load( Ordering::Acquire ), Seq( 64 ) );
ring_batch/src/lib.rs: /// let batch = claim( &cursor, 64, Ordering::AcqRel );
ring_batch/src/lib.rs: /// whether `count` is 1 or 64. The ordering is the caller's — see
ring_bench/src/lib.rs: /// let workload = Workload::new( RingConfig::new( 64 ).unwrap() )
ring_config/src/lib.rs:   /// assert!( RingConfig::new( 64 ).is_ok() );
ring_config/src/lib.rs: ///   .with_batch( 64 );
ring_cursor/src/lib.rs:   /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
ring_cursor/src/lib.rs: //! Alignment alone does not separate two cursors. A 64-aligned type of size 8
ring_cursor/src/lib.rs: //! `#[ repr( align( 64 ) ) ]` happens to round the size up too, so both hold —
ring_cursor/src/lib.rs: //! about a type; two fields being 64 bytes apart is the fact the promise was
ring_cursor/src/lib.rs: //! family inherits it without a second author deciding 64 was probably fine.
ring_cursor/src/lib.rs: /// assert_eq!( core::mem::align_of::< PaddedCursor >(), 64 );
ring_cursor/src/lib.rs: /// assert_eq!( core::mem::size_of::< PaddedCursor >(), 64 );
ring_flush/src/lib.rs: /// recording only "a flush occurred under policy `OnBatch( 64 )`" cannot detect
ring_index/src/lib.rs: /// rather than only after `2^64` publications. In a release build the
ring_mpsc/src/lib.rs:     claim.abs_diff( consume ) >= 64
ring_mpsc/src/lib.rs:   /// assert_eq!( ring.capacity().get(), 64 );
ring_mpsc/src/lib.rs:   /// it. Unpadded on purpose: [`PaddedCursor`] would make this array 64 times
ring_mpsc/src/lib.rs:   /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 64 ).unwrap() );
ring_spsc/src/lib.rs:   /// assert_eq!( ring.capacity().get(), 64 );
ring_spsc/src/lib.rs:   /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 64 ).unwrap() );
ring_tls/src/lib.rs: /// assert_eq!( cursor.counts().total, 1, "64 items, one atomic operation" );
ring_tls/src/lib.rs: /// assert_eq!( flush.claim().len(), 64 );
ring_tls/src/lib.rs: /// assert_eq!( flush.count(), 64 );
ring_tls/src/lib.rs: /// let mut buffer = TlsBuffer::with_capacity( 64 );
ring_trace/src/lib.rs:   /// assert_eq!( trace.entries()[ 0 ].count, 64, "one entry for the whole batch" );
ring_trace/src/lib.rs:   /// trace.record( TraceOp::Claim, Seq( 4 ), 64 );
ring_trace/src/lib.rs: /// `count` is what makes the log readable against a batch: a claim of 64 is one
ring_trace/src/lib.rs: /// entry saying 64, not 64 entries, because feature 177's whole point is that
ring_trace/src/lib.rs: /// let entry = TraceEntry { op : TraceOp::Claim, seq : Seq( 8 ), count : 64 };
```

That is a candidate list, not a verdict — most hits are capacities, batch
widths, and trace counts, where `64` is an unrelated round number. Triaged
against the 32 non-`ring_align` crates as of 2026-08-29, it yields exactly
three categories:

| Category | Hits | Verdict |
|----------|-----:|---------|
| `64` as a capacity, batch size, or trace count (`ring_batch`, `ring_tls`, `ring_trace`, `ring_atomic`, `ring_config`, `ring_bench`, `ring_flush`, and the `Capacity::new( 64 )` doc examples) | ~25 | Not a violation — unrelated concept, correctly excluded by D4's rule that coinciding values do not share a constant |
| `64` as a layout literal in a **doc example** (`ring_cursor:140`, `:141`, `:184`) | 3 | Borderline. Executable doctests asserting `size_of::< PaddedCursor >() == 64`. The crate's own integration test uses `CACHE_LINE` here and annotates it — *"the 64 above is `ring_align::CACHE_LINE`, not a coincidence"* — so the divergence is known and confined to examples |
| `64` as a layout literal in **executable, non-doc code** | **1** | **The violation** — `ring_mpsc:865`, below |

**One real hit in 32 crates**, which is Q2 working as described. The check is
cheap, the signal-to-noise is workable, and nothing runs it — it is the kind of
gate
[`bench_harness`'s gate directory](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
already houses several of, so its absence is a gap rather than an
impossibility.

### The Invariant Is Currently Violated

`ring_mpsc` carries a second copy of the number and does not depend on this
crate:

```rust
// ring_mpsc/src/lib.rs, in Producer::on_distinct_lines
claim.abs_diff( consume ) >= 64
```

```sh
cd "$(git rev-parse --show-toplevel)"
# `--include=*.rs` scopes this to code. Without it the scan also matches
# `ring_mpsc`'s own corpus, which discusses `ring_align` in prose — a doc
# citing the crate is not the crate being used, and counting it would let
# this corpus falsify its own finding by growing.
command grep -rn 'ring_align\|CACHE_LINE' --include=*.rs ring_mpsc/ | wc -l
sed -n '/\[dependencies\]/,/^\[/p' ring_mpsc/Cargo.toml # ring_align absent
# the control — the same pattern against the one crate that does use it, in code
command grep -rn 'ring_align\|CACHE_LINE' --include=*.rs ring_cursor/ | wc -l
```

Live output:

```
0
[dependencies]
ring_atomic = { path = "../ring_atomic" }
ring_store = { path = "../ring_store" }
ring_claim = { path = "../ring_claim" }
ring_config = { path = "../ring_config" }
ring_cursor = { path = "../ring_cursor" }
ring_gating = { path = "../ring_gating" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }

# `tests/mpsc_test.rs`'s `exhaustive` module, under `RUSTFLAGS="--cfg loom"`
# only. `ring_atomic` swaps `AtomicSeq` for an instrumented one under the same
# cfg, so what loom explores is this crate's own stamp protocol rather than a
# re-implementation of it.
[target.'cfg(loom)'.dev-dependencies]
18
```

**Expected: nothing, then a dependency list without `ring_align`, then a count
in the tens.** Only the third is a real expectation, and it is scoped to `*.rs`
deliberately: an unscoped count sweeps this crate's own `docs/` too, so it
climbs every time a document is written and a reader cannot tell a real change
from a prose edit. Against code alone the figure is stable, and zero is the
single reading that matters, because it means the pattern stopped matching
rather than that the violation was fixed.

**The control is what keeps the first line honest in the right direction.** "No
hits" here means the violation is still live, so a *fix* to `ring_mpsc` makes
this recipe visibly break — which is the behaviour worth having. What it cannot
detect unaided is the pattern going stale: rename `CACHE_LINE`, or rename this
crate, and the first line keeps printing nothing while saying something that is
no longer about anything.

The full finding — including that the same method name computes a *different
predicate* there — is
[`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) § The
Duplicate Already Exists. It is recorded rather than fixed, because the fix is
a source change in `ring_mpsc` and belongs to that crate's own change.

**Recording a live violation in the invariant that forbids it is deliberate.**
An invariant instance that describes only the intended state, while the
workspace contradicts it, is the more comfortable document and the less useful
one.

### Violation Consequences

| # | Violation | What is lost |
|---|-----------|--------------|
| D1 | A crate hardcodes `64` in a layout check | A platform port raising `CACHE_LINE` fixes every crate except that one, silently. This is the live case |
| D2 | A crate defines its own `const LINE : usize` | Same as D1, plus two names for one concept — the Term Collision failure at the code level |
| D3 | A crate computes the line size at runtime | Two answers that may *disagree at runtime*, with no build-time signal at all |
| D4 | A crate uses the number for something that is not a cache line — a buffer size, an alignment for a different reason | Coupling by coincidence: raising the constant for a port changes an unrelated size |

**D1 is the live one and it is the quiet kind.** The duplicate agrees with the
original today; it will stop agreeing on the first machine where the constant
has to change, and the crate that disagrees is the one whose tests were passing
all along.

**D4 is worth guarding against in the other direction.** The right response to
"I need a 64-byte alignment for an unrelated reason" is a second constant with
its own name and its own justification, not a second use of this one. Sharing a
number because the values coincide is how a port breaks something nobody
connected to cache lines.

### AL17 — The Family Holds Three Numbers, Not One

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the restriction: one declaration of the line size, family-wide --'
command grep -r 'pub const CACHE_LINE' */src/lib.rs
echo '  -- every bare 64 standing in for it, across the family --'
command grep -e 'addr() % 64' -e 'abs_diff( consume ) >= 64' \
  ring_cursor/src/lib.rs ring_mpsc/src/lib.rs
```

Live output:

```
  -- the restriction: one declaration of the line size, family-wide --
ring_align/src/lib.rs:pub const CACHE_LINE : usize = 64;
  -- every bare 64 standing in for it, across the family --
ring_cursor/src/lib.rs:  /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
ring_mpsc/src/lib.rs:    claim.abs_diff( consume ) >= 64
```

The declaration plus two literals. `ring_cursor` is the instructive one: its own
test file imports the constant at `tests/cursor_test.rs:42`, so the constant was
reachable — and a doctest deeper in the same crate's `src/` writes `% 64`
anyway.

**Finding.** The invariant did not fail because the constant was unavailable. It
failed in a file where the constant was already in use next door, which means
availability is not the mechanism that would have prevented it and a gate is.

**Disposition:** declined — the two violations this finding names sit in two
crates neither of which is `ring_align`: `ring_cursor/src/lib.rs:184` (a
doctest) and `ring_mpsc/src/lib.rs:865` (live code). Fixing either edits a
crate outside this disposition pass's own scope, and the remedy the finding
itself proposes — Q4, a family-wide gate rejecting a bare `64` in a layout
context — lives in `bench_harness/gate/`, a third crate also outside
scope. `ring_align` can name the violation, as this instance already does; it
cannot fix code it does not own or author a gate that belongs to a different
crate's own gate-authoring process.

---

### AL18 — The Costlier Copy Is in Running Code and Uses the Wrong Arithmetic

Of the two violations, `ring_cursor:184` is a doctest asserting a true thing
awkwardly. `ring_mpsc:865` is different on both counts: it is in running code,
and it is the *subtraction* form
([`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md)
calls that form wrong).

**Finding.** On a 128-byte-line port it keeps compiling, keeps passing, and
quietly asserts a separation half the size it claims. Two independent defects —
a duplicated constant and the wrong arithmetic — meet in one expression, and
each would have to be found by a different gate.

**Disposition:** declined — the same `ring_mpsc/src/lib.rs:865` expression
[`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md)
AL4 already declined, for the same reason: closing it means `ring_mpsc`
taking on a new `ring_align` dependency, an architectural call for that
crate's own change rather than this documentation pass, and this crate's own
[`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) § The
Duplicate Already Exists already measured that swapping only the arithmetic
form would not close the substantive gap either — the check stays
near-vacuous for `ring_mpsc`'s two cursors regardless of form, because they
are separate allocations rather than fields of one `PaddedCursor`, and that
section already files the real fix as `ring_mpsc`'s "own change, with its own
test and its own bug record."

---

### AL19 — This Crate Can Declare the Restriction and Cannot Enforce It

`ring_align` owns the number. It has no way to stop the other 32 crates from
spelling it out, and no gate in `bench_harness/gate/` greps the family
for a bare 64.

**Finding.** The invariant is a statement about intent that the two violations
above already contradict. The check is a single `grep` — this document's own
Regenerate block is that check, run by hand — so what is missing is not a
technique but a gate declaration, which is why the finding is recorded as
unenforced rather than as a design flaw.

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_one_consumer.md](../integration/001_one_dependency_one_consumer.md) | The declared-consumer count this invariant is measured against |
| [../integration/002_why_the_constant_lives_here.md](../integration/002_why_the_constant_lives_here.md) | Why the number is in this crate rather than in `ring_cursor`, which is the only crate that needs it |

### Invariants

| File | Relationship |
|------|--------------|
| [001_two_wrapped_fields_never_share_a_line.md](001_two_wrapped_fields_never_share_a_line.md) | The compiler-enforced companion — it holds regardless of whether this one does |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_constant_across_a_platform_port.md](../lifecycle/002_the_constant_across_a_platform_port.md) | The event that converts D1 from latent to actual |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_owner_for_a_magic_number.md](../pattern/002_one_owner_for_a_magic_number.md) | The general pattern this invariant is the family-specific statement of |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | The live violation in full, with all three of its divergences |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | The one declaration this invariant reserves, and the `E0693` bind that forces a second literal inside this crate itself |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/unsafe_allowlist.txt`](../../../bench_harness/gate/declared/ring/unsafe_allowlist.txt) | The same principle applied to a different permission: "a permission nobody exercises is a bound looser than the code actually is" |
| [`../../../README.md`](../../../README.md) | The family this invariant ranges over |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | No check covers this invariant. M1 and M2 are about the value and about `unsafe`; the Q4 grep is unwritten |
