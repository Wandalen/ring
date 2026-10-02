# ring_debug manual testing

Six stages. Each states a **prediction before it is run**, so a wrong prediction
is recorded as a finding rather than quietly corrected into agreement with the
result.

Four of the six exist because
[`docs/non_functional_requirement/001`](../../docs/non_functional_requirement/001_absent_unless_called.md)'s
C1, C2 and C4 are explicitly **not testable from inside the crate**. A test here
cannot observe what other crates depend on. They are greps, and this is where
they live.

Run from the repository root.

---

## M1. Does anything in the family depend on this crate?

```bash
grep -rln 'ring_debug' */Cargo.toml
```

**Prediction:** exactly one line, `ring_debug/Cargo.toml`. C1 holds.

**Result (2026-08-28):** as predicted, one line.

---

## M2. Does any family `src/` mention this crate?

```bash
command grep -rn --include=*.rs 'ring_debug' */src/ | command grep -v '^ring_debug/'
```

**Prediction:** empty. C2 holds, and the dependency arrow points one way.

**Result (2026-08-28):** as predicted, empty.

**Re-verified (2026-09-27). No longer empty, and the root set had also drifted.**
The recipe above was widened from the original `*/src/`-only form to cover every
crate root this repository's history defined, five in total. It also switched to
`command grep`, the convention applied family-wide. Re-run at either scope, it
now finds two lines:
`ring_spsc/src/lib.rs:556-557`, a doc comment citing
`ring_debug/docs/invariant/002` and `ring_debug/docs/pattern/001` by path. This
is a documentation cross-reference, not a `use` or a call. C1 (still one
manifest, extended-roots confirmed) is what guarantees that no functional
dependency exists. But C2 as written is a text grep that cannot tell a citation
from a real dependency, so "empty" is not the invariant it looks like. Read the
prediction as "no functional dependency," which still holds.

---

## M3. Does anything here run implicitly?

```bash
grep -n 'impl Drop\|\<static \|lazy_static\|OnceLock\|\<ctor\>' ring_debug/src/lib.rs
```

**Prediction:** no match. C4 holds by construction, because the crate is eight
public items and one private `const`.

**Result (2026-08-28):** as predicted, no match. The only `const` is `OBSERVE`,
which is an `Ordering`, not state.

**Re-verified (2026-09-11). The recipe itself had drifted.** Re-running the
command exactly as it was recorded (bare `ctor`, no word boundary) now matches
`src/lib.rs:212`, a comment added since 2026-08-28 as part of DB9's `checked_sub`
rationale: `// the contradictory case say so instead of rendering a number.` The
match is `ctor` inside "contra**dic-tor**y", which is English prose and not an
implicit-invocation mechanism. C4 still holds. A full read confirms that no
`impl Drop`, `static`, `lazy_static`, `OnceLock`, or ctor-hook exists anywhere in
the crate. But the recipe as written can no longer prove it, because a bare
substring match on `ctor` is satisfied by any comment containing "contradictory",
"factor", "director", "victor", etc. The pattern above is now `\<ctor\>`
(word-bounded). It still catches `#[ctor]`/`ctor::ctor`-style real usage and
rejects the prose false positive. The bare `static ` term has the same latent
exposure (`ecstatic`, `hydrostatic`, ... contain it as a substring). It is not
yet live against the current file, but the same pass tightened it to `\<static `
rather than leave a matching hazard beside the one just fixed. It still catches
a real `static FOO: ...` declaration and rejects both probe words. The crate's
own Regenerate blocks are immune to this drift class, because they re-run and
re-print their own output on every read. A plain `**Result:**` line does not,
and this is the one recipe in this file that had gone stale.

---

## M4. Can the strongest check be pointed at a live `ring_core::Ring`?

```bash
grep -n 'pub fn cursors\|-> *&*CursorPair' ring_core/src/lib.rs \
  ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
grep -n 'pub fn position' ring_core/src/lib.rs
```

**Prediction:** no. `ring_core` has no `CursorPair` at all, and the backends keep
theirs private. Both greps come back empty, which confirms
[`docs/integration/001`](../../docs/integration/001_reaching_the_cursors_of_a_live_ring.md)'s
boundary from the source rather than from the doc.

**Result (2026-08-28):** as predicted, both empty. `ring_core`'s ends expose
`free_capacity`, `is_full`, `len`, `is_empty` and nothing else positional.

---

## M5. Is the crate fully covered?

```bash
cargo tarpaulin -p ring_debug --all-features --out Stdout 2>&1 \
  | grep 'ring_debug/src'
```

**Prediction:** 100%. The public API is small and every branch has a named test.

**Result (2026-08-28):** `59/59`. Reached on the first measurement, without a
coverage-driven test added afterwards.

**Re-verified (2026-09-11):** `63/63`, still 100%, so the prediction holds. The
numerator and denominator both moved (the suite grew from 22 to 26 tests and
`src/lib.rs` grew with it, per DB9's `checked_sub` refactor), and coverage stayed
full.

---

## M6. Are the three values reachable from a `ring_spsc` ring, even though a `CursorPair` is not?

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
only obstacle. That would mean `check` taking `&CursorPair` is a narrower
signature than it needs to be, and a public `(Seq, Seq, Capacity)` form would
make the crate usable one layer lower than it is now.

**Result (2026-08-28): the prediction was half right, and the half that was
wrong is the more useful half.**

`ring_spsc` exposes `position()` on both ends, so for the single-producer
backend the three values are reachable and the signature is the only obstacle.
`ring_mpsc` does **not**. Its producer end exposes `claimed()` and its consumer
`position()`, and `claimed` is the claim cursor, not the publish cursor. The
publish side is `committed()` and `published_through()` on the `Ring`, and
`published_through` returns `Option< Seq >` because there may be no published
record at all.

So the two backends do not present the same pair of sequences, and a
`(Seq, Seq, Capacity)` entry point would silently invite the `ring_mpsc` caller
to pass `claimed()`. That cursor is legitimately ahead of the publish cursor, so
the check would report D1 on a healthy ring. **The narrow signature prevents
this.** Taking a `CursorPair` means the caller cannot assemble a wrong pair, and
the reachability cost is the price of that.

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
| M6 | Data reachable from a backend | ⚠️ only from `ring_spsc`; `ring_mpsc` differs, see above |

**Predictions wrong: 1 of 6.** M6 assumed the two backends presented the same
pair of cursors. They do not, and the difference turned a proposed API widening
into an argument against it.

**Environment:** `2026-08-28`, Linux 6.8.0, `cargo nextest`, `cargo tarpaulin`.
22 tests + 3 doc tests, all passing. Re-verified `2026-09-11`: 28 tests + 3 doc
tests (1 ignored), all passing. The DB9 `checked_sub` refactor and its regression
coverage added 4. Two more closed DB38's T6 gap
(`a_watch_that_faulted_reports_ok_once_the_ring_recovers`) and the untested
genuinely-full `check_ends` boundary (`check_ends_on_a_genuinely_full_ring`). No
stage's substantive verdict changed.
