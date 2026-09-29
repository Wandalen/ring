# Manual Testing — ring_debug

Six stages. Each states a **prediction before it is run**, so a wrong prediction
is recorded as a finding rather than quietly corrected into agreement with the
result.

Four of the six exist because
[`docs/non_functional_requirement/001`](../../docs/non_functional_requirement/001_absent_unless_called.md)'s
C1, C2 and C4 are explicitly **not testable from inside the crate** — a test here
cannot observe what other crates depend on. They are greps, and this is where
they live.

Run from the repository root.

---

## M1 — Does anything in the family depend on this crate?

```bash
grep -rln 'ring_debug' */Cargo.toml
```

**Prediction:** exactly one line, `ring_debug/Cargo.toml`. C1 holds.

**Result (2026-08-28):** as predicted — one line.

---

## M2 — Does any family `src/` mention this crate?

```bash
command grep -rn --include=*.rs 'ring_debug' */src/ | command grep -v '^ring_debug/'
```

**Prediction:** empty. C2 holds — the dependency arrow points one way.

**Result (2026-08-28):** as predicted — empty.

**Re-verified (2026-09-27) — no longer empty, and the root set had also drifted.**
The recipe above is widened from the original `*/src/`-only form to cover every
crate root this repository's history defined — five in total — and switched to
`command grep`, per the same convention applied family-wide. Re-run at either
scope, it now finds two lines:
`ring_spsc/src/lib.rs:556-557`, a doc comment citing
`ring_debug/docs/invariant/002` and `ring_debug/docs/pattern/001` by path. This
is a documentation cross-reference, not a `use` or a call — C1 (still one
manifest, extended-roots confirmed) is what actually guarantees no functional
dependency exists. But C2 as written is a text grep with no way to tell a
citation from a real one, so "empty" is not the invariant it looks like, and
the prediction should be read as "no functional dependency," which still holds.

---

## M3 — Does anything here run implicitly?

```bash
grep -n 'impl Drop\|\<static \|lazy_static\|OnceLock\|\<ctor\>' ring_debug/src/lib.rs
```

**Prediction:** no match. C4 holds by construction — the crate is eight public
items and one private `const`.

**Result (2026-08-28):** as predicted — no match. The only `const` is `OBSERVE`,
which is an `Ordering`, not state.

**Re-verified (2026-09-11) — the recipe itself had drifted.** Re-running the
command exactly as it was recorded (bare `ctor`, no word boundary) now matches
`src/lib.rs:212`, a comment added since 2026-08-28 as part of DB9's `checked_sub`
rationale: `// the contradictory case say so instead of rendering a number.` The
match is `ctor` inside "contra**dic-tor**y" — English prose, not an implicit-
invocation mechanism. C4 still holds (confirmed by full read: no `impl Drop`,
`static`, `lazy_static`, `OnceLock`, or ctor-hook exists anywhere in the crate),
but the recipe as written can no longer prove it, because a bare substring match
on `ctor` is satisfied by any comment containing "contradictory", "factor",
"director", "victor", etc. Pattern above tightened to `\<ctor\>` (word-bounded) —
confirmed to still catch `#[ctor]`/`ctor::ctor`-style real usage while rejecting
the prose false positive. The bare `static ` term has the identical latent
exposure (`ecstatic`, `hydrostatic`, ... contain it as a substring) — not yet
live against the current file, but tightened to `\<static ` on the same pass
rather than leaving a matching hazard sitting beside the one just fixed;
confirmed to still catch a real `static FOO: ...` declaration while rejecting
both probe words. This is the same drift class the crate's own
Regenerate blocks are otherwise immune to (those re-run and re-print their own
output on every read); a plain `**Result:**` line does not, and this is the one
recipe in this file that had gone stale.

---

## M4 — Can the strongest check be pointed at a live `ring_core::Ring`?

```bash
grep -n 'pub fn cursors\|-> *&*CursorPair' ring_core/src/lib.rs \
  ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
grep -n 'pub fn position' ring_core/src/lib.rs
```

**Prediction:** no. `ring_core` has no `CursorPair` at all, and the backends keep
theirs private. Both greps empty, confirming
[`docs/integration/001`](../../docs/integration/001_reaching_the_cursors_of_a_live_ring.md)'s
boundary from the source rather than from the doc.

**Result (2026-08-28):** as predicted — both empty. `ring_core`'s ends expose
`free_capacity`, `is_full`, `len`, `is_empty` and nothing else positional.

---

## M5 — Is the crate fully covered?

```bash
cargo tarpaulin -p ring_debug --all-features --out Stdout 2>&1 \
  | grep 'ring_debug/src'
```

**Prediction:** 100%. The surface is small and every branch has a named test.

**Result (2026-08-28):** `59/59`. Reached on the first measurement, without a
coverage-driven test added afterwards.

**Re-verified (2026-09-11):** `63/63` — still 100%, prediction still holds. The
numerator and denominator both moved (the suite grew from 22 to 26 tests and
`src/lib.rs` grew with it, per DB9's `checked_sub` refactor) but full coverage
was not lost in the process.

---

## M6 — Are the three values reachable from a `ring_spsc` ring, even though a `CursorPair` is not?

The interesting question M4 leaves open. `ring_spsc::Producer` and `Consumer`
both expose `position() -> Seq`, and `Ring::capacity()` exposes the third value.
So the *data* `check` needs may be reachable from a backend even where the
*type* it takes is not.

```bash
# Written as tests/zz_probe_reach.rs, run, then deleted — see below.
grep -n 'pub fn position' ring_spsc/src/lib.rs
grep -n 'pub fn position\|pub fn capacity' ring_mpsc/src/lib.rs
```

**Prediction:** the data is reachable from `ring_spsc` and the API shape is the
only obstacle — which would mean `check` taking `&CursorPair` is a narrower
signature than it needs to be, and a public `(Seq, Seq, Capacity)` form would
make the crate usable one layer lower than it currently is.

**Result (2026-08-28): the prediction was half right, and the half that was
wrong is the more useful half.**

`ring_spsc` exposes `position()` on both ends — so for the single-producer
backend the three values are reachable and the signature is indeed the only
obstacle. `ring_mpsc` does **not**: its producer end exposes `claimed()` and its
consumer `position()`, and `claimed` is the claim cursor, not the publish
cursor — `committed()` and `published_through()` on the `Ring` are the publish
side, and `published_through` returns `Option< Seq >` because there may be no
published record at all.

So the two backends do not present the same pair of sequences, and a
`(Seq, Seq, Capacity)` entry point would silently invite the `ring_mpsc` caller
to pass `claimed()` — a cursor that is legitimately ahead of the publish cursor,
which would report D1 on a perfectly healthy ring. **The narrow signature is
load-bearing**: taking a `CursorPair` means the caller cannot assemble a wrong
pair, and the reachability cost is the price of that.

Recorded as the reason Pending 1 in
[`docs/decisions/readme.md`](../../docs/decisions/readme.md) proposes
`ring_core::position()` rather than a looser signature here.

---

## Run Record

| Stage | Question | 2026-08-28 |
|---|---|---|
| M1 | Nothing depends on this crate | ✅ one line |
| M2 | No family `src/` mentions it | ✅ empty |
| M3 | Nothing runs implicitly | ✅ no match |
| M4 | Strongest check unreachable from `ring_core` | ✅ confirmed from source |
| M5 | Fully covered | ✅ 59/59 |
| M6 | Data reachable from a backend | ⚠️ only from `ring_spsc`; `ring_mpsc` differs — see above |

**Predictions wrong: 1 of 6.** M6 assumed the two backends presented the same
pair of cursors. They do not, and the difference turned a proposed API widening
into an argument against it.

**Environment:** `2026-08-28`, Linux 6.8.0, `cargo nextest`, `cargo tarpaulin`.
22 tests + 3 doc tests, all passing. Re-verified `2026-09-11`: 28 tests + 3 doc
tests (1 ignored), all passing — 4 grew from the DB9 `checked_sub` refactor and
its regression coverage, 2 more added closing DB38's T6 gap
(`a_watch_that_faulted_reports_ok_once_the_ring_recovers`) and the untested
genuinely-full `check_ends` boundary (`check_ends_on_a_genuinely_full_ring`); no
stage's substantive verdict changed.
