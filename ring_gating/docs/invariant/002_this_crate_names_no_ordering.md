# Invariant: This Crate Names No Ordering

### Scope

- **Purpose**: Establish that `ring_gating` performs no atomic operation, names no memory ordering, and contains no `unsafe`, and show why each absence is load-bearing rather than incidental.
- **Responsibility**: Prove all three mechanically, trace what the crate gets in exchange, and record that its thread-safety could not have been asserted even deliberately.
- **In Scope**: What this crate does not do about concurrency.
- **Out of Scope**: What `ring_cursor` does instead — see [`integration/001`](../integration/001_three_dependencies_and_two_dependents.md).

### The Invariant

> `ring_gating` contains no atomic load, no `Ordering::` of any kind, no import
> of `SeqCell`, and no `unsafe`. Every concurrency decision it depends on is made
> in `ring_cursor` and inherited unmodified.

### The Three Absences

```sh
cd "$(git rev-parse --show-toplevel)"
# A1 — no ordering, no atomic trait. Doc lines excluded: the doctests
#      legitimately name Ordering::Release when driving a cursor.
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -oE "Ordering::[A-Za-z]+|GATING|SeqCell" | sort | uniq -c
# control — the identical expression one layer down, where the names do appear
grep -vE "^[[:space:]]*(///|//!)" ring_cursor/src/lib.rs \
  | grep -oE "Ordering::[A-Za-z]+|GATING|SeqCell" | sort | uniq -c

# A2 — no unsafe of any kind
grep -c 'unsafe' ring_gating/src/lib.rs || true

# A3 — no local alignment or cache-line concern
grep -cE 'align|CACHE_LINE|(^|[^a-z0-9_])64([^0-9]|$)' ring_gating/src/lib.rs || true
# control — the identical expression over the crate that holds the attribute
grep -cE 'align|CACHE_LINE|(^|[^a-z0-9_])64([^0-9]|$)' ring_align/src/lib.rs
```

Live output:

```
      8 GATING
      1 Ordering::Acquire
      2 SeqCell
0
0
24
```

A1 is the crate's manual check M2, and its expected result is stated there with
the reason:

> **Expected:** **no output at all.** No `Ordering::`, no `GATING`, and no
> `SeqCell` — this crate no longer performs an atomic read, so it has no ordering
> to name and no reason to import the trait that would let it. A hit of any kind
> means a load came back into a crate whose job is arithmetic over an answer
> someone else read.

The exclusion of doc lines is necessary, not cosmetic: five doctest lines write
`cursor.store( Seq( … ), Ordering::Release )` to set up a scenario —
`grep -n '///.*Ordering::' ring_gating/src/lib.rs` lists them at 63, 66,
192, 218 and 317. A grep over the whole file reports `Ordering::Release` five
times and reads as a violation. This is the same failure that broke three of `ring_cursor`'s seven
manual checks on first run — a grep over a Rust file reads documentation as if it
were code.

### Why A1 Is Load-Bearing

The crate's own manual plan states the failure mode better than a restatement
would:

> an `Ordering::Relaxed` substituted for the gating read produces a suite that
> passes on x86 — where the hardware supplies the acquire semantics the code
> failed to ask for — and a data race on aarch64, which is the exact shape of bug
> this family exists to make impossible.

So the invariant is not "this crate is tidy". It is: **there is exactly one place
in the family where a gating read's ordering is decided, and a second copy is how
one of them ends up `Relaxed`.** That place is `ring_cursor::GATING`, and
`ring_gating` reaches it only through `ring_cursor::slowest`, which does the
loading itself.

The history is visible in the manual plan's own preamble — M1 through M3 were
rewritten when the fold moved out of this crate:

> They had asserted that this crate iterated its cursors correctly and named
> `GATING` twice; both are now false, and both were replaced by the stronger claim
> the move makes available — that this crate does none of it at all.

An earlier version of this crate *did* name the ordering, twice. The invariant is
therefore a property that was achieved, not one that was always true, which is
the strongest kind of reason to keep checking it.

### A2 — `unsafe` Is Denied, and This Crate Did Not Opt Out

The workspace denies it:

```toml
# Cargo.toml — [workspace.lints.rust]
unsafe-code = "deny"
```

Exactly two crates opt back in, and both are ring backends that must dereference
an `UnsafeCell`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'allow( *unsafe_code' ring_*/src/*.rs
# ring_mpsc/src/lib.rs:191:#![ allow( unsafe_code ) ]
# ring_spsc/src/lib.rs:169:#![ allow( unsafe_code ) ]

grep -r 'unsafe impl' ring_*/src/*.rs
# ring_mpsc/src/lib.rs:353:unsafe impl< S : Send > Sync for Ring< S > {}
# ring_spsc/src/lib.rs:282:unsafe impl< S : Send > Sync for Ring< S > {}
```

Live output:

```
ring_mpsc/src/lib.rs:#![ allow( unsafe_code ) ]
ring_spsc/src/lib.rs:#![ allow( unsafe_code ) ]
ring_mpsc/src/lib.rs://! is the fact [`Ring`]'s `unsafe impl Sync` argument rests on
ring_mpsc/src/lib.rs:unsafe impl< S : Send > Sync for Ring< S > {}
ring_spsc/src/lib.rs:unsafe impl< S : Send > Sync for Ring< S > {}
```

Two opt-outs, two `unsafe impl`s, and `ring_gating` is neither.

**The consequence is stronger than "the crate happens not to use `unsafe`":
`GatingSet`'s `Send` and `Sync` could not have been asserted even deliberately.**
Every thread-safety property the type has is auto-derived from its fields —
`Vec< PaddedCursor >` and `Capacity` — and if either field were not `Sync`, the
compiler would reject this crate's own concurrent test rather than let a hand-written
`unsafe impl` paper over it.

That test is what depends on it:

```rust
// tests/gating_test.rs:405-427, as originally written
std::thread::scope( | scope |
{
  scope.spawn( ||
  {
    for position in 0..CAPACITY as u64
    {
      set.cursor( 0 ).unwrap().store( Seq( position ), Ordering::Release );
    }
  } );

  for _ in 0..10_000
  {
    let headroom = set.headroom( producer );
    …
  }
} );
```

`&set` crosses into the spawned closure while the main thread reads it, which
compiles only if `GatingSet : Sync`. Nothing in this crate says so. See
[`type/002`](../type/002_send_and_sync_without_unsafe.md).

### What the Absences Buy

| Absence | Bought |
|---------|--------|
| No `Ordering::` | The family's gating ordering is stated once and cannot drift here |
| No `SeqCell` import | The crate cannot perform a load even by accident — the method is not in scope |
| No `unsafe` | Thread-safety is a compiler conclusion, not an author's assertion |
| No alignment concern | False sharing is `ring_align`'s property, inherited through `PaddedCursor` |

The second row is the one worth noticing. `SeqCell` is a trait, and its methods
are unreachable without importing it. So A1's stronger half — no `SeqCell` — makes
A1's weaker half — no `Ordering::` — structurally difficult rather than merely
checked: to name an ordering usefully, the crate would first have to add an
import that M2 also catches.

### What Would Break It

| Change | Detected by |
|--------|-------------|
| Inlining `ring_cursor::slowest`'s body here | M1 (a `.iter()` or `.min()` appears) **and** M2 (an `Ordering::` appears) |
| Adding a `headroom_relaxed` fast path | M2 |
| Taking a raw `&[ AtomicU64 ]` instead of `&[ PaddedCursor ]` | M2, via the `SeqCell` import it would need |
| Adding `unsafe impl Send for GatingSet` | The compiler — `unsafe-code = "deny"`, no opt-out here |

The last row is the only one enforced by the build rather than by a manual
reading. That asymmetry is worth stating plainly: **three of the four are
defended by greps a person runs, and one is defended by the compiler.** Making
M1/M2 a build step would close it, and nothing here does that today.

### GT25 — Zero in the Library, Five in the Examples

```
non-doc lines naming an Ordering : 0
doc lines naming one             : 5
      63, 66, 192, 218, 317  -> all Ordering::Release
```

The invariant is stated over the crate. Doc examples are part of the crate's
test surface and are compiled and run, so what they name is worth separating
from what the library names.

**Finding.** The crate names no ordering in any line the compiler builds into the library — measured zero — and names `Ordering::Release` five times inside doc examples. Those compile and run under `cargo test --doc`, so the invariant as stated holds for the library and not for everything this file causes to execute

---

### GT26 — Kept by the Call Graph, Not by the Source

```
ring_cursor::GATING : the ordering, one crate over
this crate imports  : ring_cursor::{ slowest, PaddedCursor }   -- not GATING
a direct cursor.load( .. ) here would need an ordering to compile
```

Nothing forbids naming an ordering in this crate. What prevents it is that no
code here loads a cursor directly, which is a fact about the current
implementation rather than a constraint on future ones.

**Finding.** By delegation. The ordering lives in `ring_cursor::GATING`, which this crate does not import, and arrives inside `ring_cursor::slowest`. A future direct cursor read here would have to name an ordering to compile, and nothing would flag it — the invariant is a property of the current call graph, not a constraint on the source

Manual check M2 above is now also an automated test, so the same omission
fails `cargo test` rather than depending on a human remembering to run the
grep. `2>/dev/null` below drops cargo's own build-progress chatter
(compile/lock/timing lines on stderr) — those vary with unrelated concurrent
builds and elapsed wall-clock time, so quoting them made this block
unreproducible on its own terms, not just stale:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo test -p ring_gating --test gating_test crate_names_no_ordering_in_any_non_doc_line 2>/dev/null
```

Live output:

```

running 1 test
test crate_names_no_ordering_in_any_non_doc_line ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 0.00s
```

**Disposition:** applied — added `crate_names_no_ordering_in_any_non_doc_line`
to `tests/gating_test.rs`, automating manual check M2's own recipe (doc-line
exclusion and all) as a `#[ test ]`. A future direct cursor read naming an
ordering now fails `cargo test` instead of relying on someone running M2 by
hand; the crate's 23 unit tests plus 12 doctests re-verified passing (`cargo
test -p ring_gating --all-features`, 2026-09-04). Now prints:
`test crate_names_no_ordering_in_any_non_doc_line ... ok`

---


### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The fields whose auto-derived properties the type inherits |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_two_dependents.md](../integration/001_three_dependencies_and_two_dependents.md) | Where the loading actually happens |

### Invariants

| File | Relationship |
|------|--------------|
| [001_the_bound_is_the_minimum_and_only_the_minimum.md](001_the_bound_is_the_minimum_and_only_the_minimum.md) | C6, which this invariant is the enforcement of |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_gate_must_never_over_report.md](../non_functional_requirement/002_the_gate_must_never_over_report.md) | What the concurrent test does and does not establish |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_send_and_sync_without_unsafe.md](../type/002_send_and_sync_without_unsafe.md) | The auto-derivation in full |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:42-43` | Two imports, neither of them `SeqCell` |
| `Cargo.toml` § `[workspace.lints.rust]` | `unsafe-code = "deny"` |
| `ring_cursor/src/lib.rs:89` | `GATING` — the family's one statement of the ordering |
| `tests/manual/readme.md` § M1, § M2 | The two structural checks, and the rewrite that strengthened them |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:394-428` | The concurrent test, which compiles only because `GatingSet : Sync` |
| `tests/gating_test.rs:329-339` | Cursor reads through `SeqCell`, done by the *test*, not the crate |
