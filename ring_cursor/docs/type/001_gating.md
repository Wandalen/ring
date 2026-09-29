# Type: `GATING`

### Scope

- **Purpose**: State what a consumer may rely on from `GATING`, what it pairs with, and the one thing its documentation claims that it does not deliver.
- **Responsibility**: Give the declaration, the `Release` stores it forms edges with, the external assertions that check it, and the boundary of the promise.
- **In Scope**: `pub const GATING : Ordering` at `src/lib.rs:71-89`; its pairing partners in `ring_spsc`, `ring_mpsc`, `ring_publish`.
- **Out of Scope**: Why it is a constant rather than a parameter, which is [`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md); the reads that use it, which are [`algorithm/001`](../algorithm/001_the_slowest_fold.md) and [`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md).

### The Declaration

```rust
pub const GATING : Ordering = Ordering::Acquire;
```

One line, no generics, no target conditionals. A `const` rather than a `static`,
so it inlines at every use and costs nothing.

### What a Consumer May Rely On

| # | Promise | Enforced by |
|---|---------|-------------|
| P1 | The value is `Ordering::Acquire` | The doctest at `src/lib.rs:85-88`, and two consumers' own suites |
| P2 | It is `pub` and stable across builds | No `#[ cfg ]` on the declaration — unlike `PaddedCursor::new`, which has a `loom` seam |
| P3 | Every read `ring_cursor` itself performs uses it | `tests/manual/readme.md` M3 — exactly one non-doc `Ordering::Acquire` in this crate, the declaration |
| P4 | Reading a cursor at `GATING` establishes happens-before with the matching `Release` store | The C++20 memory model, via `core::sync::atomic` |

**P3's scope is this crate.** The doc comment reaches further — see the boundary
below.

### What It Pairs With

`Acquire` is half of an edge. The other halves live in the crates that *write*
the cursors this one reads:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^\s*(pub )?const [A-Z_]+ : (core::sync::atomic::)?Ordering' --include=*.rs ring_*/src/
```

Live output:

```
ring_claim/src/lib.rs:const CLAIM_SUCCESS : core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;
ring_consume/src/lib.rs:const COMMIT : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_cursor/src/lib.rs:pub const GATING : Ordering = Ordering::Acquire;
ring_debug/src/lib.rs:const OBSERVE : Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const PUBLISH : Ordering = Ordering::Release;
ring_mpsc/src/lib.rs:pub const OBSERVE : Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const COMMIT : Ordering = Ordering::Release;
ring_mpsc/src/lib.rs:pub const OWN : Ordering = Ordering::Relaxed;
ring_publish/src/lib.rs:const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_spsc/src/lib.rs:pub const OWN : Ordering = Ordering::Relaxed;
ring_spsc/src/lib.rs:pub const HANDOFF : Ordering = Ordering::Release;
```

The family names ten ordering constants across six crates. `GATING` is the only
one this crate owns; the rest are what it pairs with:

| Constant | Crate | Value | Visibility | Relationship to `GATING` |
|----------|-------|-------|:----------:|--------------------------|
| **`GATING`** | `ring_cursor:89` | `Acquire` | pub | — |
| `HANDOFF` | `ring_spsc:213` | `Release` | pub | **Pairs.** A producer's slot writes, made visible to the peer's `GATING` load |
| `COMMIT` | `ring_mpsc:271` | `Release` | pub | **Pairs.** A consumer's drain, released to the producers' `GATING` load |
| `PUBLISH` | `ring_mpsc:237` | `Release` | pub | **Pairs.** The producer's publication |
| `PUBLISH` | `ring_publish:67` | `Release` | private | **Pairs, in one call** — `compare_exchange( start, end, PUBLISH, GATING )` |
| `CLAIM_SUCCESS` | `ring_claim:76` | `AcqRel` | private | **Pairs, in one call** — `compare_exchange( current, next, CLAIM_SUCCESS, GATING )` |
| `OBSERVE` | `ring_mpsc:250` | `Acquire` | pub | Same value, different question — the consumer reading the *producer* cursor |
| `OBSERVE` | `ring_debug:73` | `Acquire` | private | Same value — a diagnostic's read |
| `OWN` | `ring_spsc:198` | `Relaxed` | pub | **Not** a pair — a thread reading back its own cursor |
| `OWN` | `ring_mpsc:287` | `Relaxed` | pub | The same, in the multi-producer ring |

Two observations the census makes visible and no single crate does:

**Three names are used twice** — `PUBLISH`, `OBSERVE`, `OWN` — each by two
crates, and each pair agrees on its value. Nothing enforces that agreement; the
two `PUBLISH` constants are declared in different crates with different
visibility and have never been compared by anything.

**`OWN` is the instructive one.** It exists to make explicit that the *same
cursor* is read two different ways depending on who is reading: `Relaxed` by its
sole writer, `GATING` by everyone else. A reader who assumes one ordering per
cursor concludes one of the two is a bug; both crates document the distinction
against `GATING` by name for exactly that reason.

Note that `ring_publish` and `ring_claim` write `core::sync::atomic::Ordering`
in full, so a grep for the short form silently misses them — the census command
above matches both spellings deliberately.

### Checked From Outside

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'ring_cursor::GATING, Ordering::Acquire' --include=*.rs
```

Live output:

```
ring_mpsc/tests/mpsc_test.rs:    assert_eq!( ring_cursor::GATING, Ordering::Acquire );
ring_mpsc/src/lib.rs:/// assert_eq!( ring_cursor::GATING, Ordering::Acquire );
ring_spsc/tests/spsc_test.rs:    assert_eq!( ring_cursor::GATING, Ordering::Acquire, "and reads the peer's with an acquire" );
ring_spsc/src/lib.rs:/// assert_eq!( ring_cursor::GATING, Ordering::Acquire );
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::GATING, Ordering::Acquire );
```

| File | Assertion |
|------|-----------|
| `ring_mpsc/tests/mpsc_test.rs:782` | `assert_eq!( ring_cursor::GATING, Ordering::Acquire );` |
| `ring_spsc/tests/spsc_test.rs:869` | `assert_eq!( ring_cursor::GATING, Ordering::Acquire, "and reads the peer's with an acquire" );` |
| `ring_mpsc/src/lib.rs:285` | The same assertion inside `OWN`'s doctest |
| `ring_spsc/src/lib.rs:196` | The same, inside `OWN`'s doctest |

**Four external assertions of a one-line constant.** That looks redundant and is
not: each consumer is pinning the half of *its own* happens-before edge that
lives in another crate. If `GATING` were weakened, the failure would surface in
the crates whose correctness depends on it rather than only here — which is
where a reader debugging a lost write would be looking.

### The Boundary of the Promise

The doc comment claims more than the crate can deliver:

> The ordering **every gating read in the family** uses.

Measured family-wide, four crates independently state the same `Acquire`
decision — `ring_cursor`, `ring_batch` (inline, twice), `ring_debug`, and
`ring_mpsc`. All four chose the same value, so nothing is incorrect today. But
the constant's guarantee stops at this crate's boundary, and a consumer reading
P3 as a family-wide invariant is reading a hope.

The measurement, the reason the count is four, and the placement that would make
the claim true are in
[`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md).

### What Would Break If This Changes

| Change | Who breaks | How loudly |
|--------|-----------|------------|
| `GATING` → `Relaxed` | Every producer's headroom check; every consumer's readability check | **Silently.** No test asserts a happens-before edge; the four value assertions would fail, which is the only alarm |
| `GATING` → `SeqCst` | Nothing correctness-wise | Loudly in `ring_bench` — the padding measurement would be dominated by fences |
| Renamed | 10 consumer crates, at compile time | Loudly |
| Made private | `ring_claim`, `ring_publish`, `ring_consume`, `ring_spsc`, `ring_mpsc` — all import it by name | Loudly |

**The first row is the dangerous one and the four value assertions are the whole
defence.** A happens-before violation does not fail a test; it produces a torn
read under contention on some machines, some of the time. Asserting the constant
is a proxy, and it is the only proxy available at unit-test scope — `loom` is
the instrument that would check the edge itself.

### CU45 — The One Exported Constant Has a Type This Crate Does Not Own

```
89:pub const GATING : Ordering = Ordering::Acquire;
```

`Ordering` is `core`'s. A constant is not a newtype: it carries no `must_use`, no
distinct identity, and no invariant of its own.

**Finding.** The decision `GATING` encodes is enforced entirely by callers
choosing to name it. There is no mechanism — no wrapper, no lint, no test — that
distinguishes `GATING` from `Ordering::Acquire` written out, which means the
constant is documentation with a compiler-checked spelling rather than an
enforced constraint.

---

### CU46 — Naming the Right Ordering Is Easy; Writing a Wrong One Is Not Hard

Four of the five crates that import `GATING` also import `PaddedCursor`, whose
`SeqCell` impl accepts whatever ordering it is passed.

**Finding.** So every crate that has the constant also has, in the same `use`
line, the type that ignores it. Substituting `Ordering::Relaxed` at any load is a
one-word edit that compiles, passes every test in this crate, and breaks the
gating guarantee the constant exists to state. The type system is not carrying
this decision; the convention is.

**Disposition:** declined — closing this by construction would mean
`PaddedCursor`'s `SeqCell` impl refusing any ordering but `GATING`, which
breaks the family's legitimate divergent-ordering uses this same file already
documents: `ring_spsc`'s `OWN` (`Relaxed`, a thread reading its own cursor)
and `HANDOFF`/`OBSERVE` pairings read the identical `PaddedCursor` type at
orderings other than `GATING` by design (see `api/002` CU7 and this crate's
own [`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md),
which records ordering flexibility on the raw cursor as deliberate, not an
oversight). The risk itself is already fully documented in this same file —
CU45 and the "What Would Break If This Changes" table above — so there is no
missing sentence left to add; a fix here would have to over-constrain the
type against its own recorded design.

---

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | Why it is a constant, and the four-site measurement of its claim |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_slowest_fold.md](../algorithm/001_the_slowest_fold.md) | The fold that reads at this ordering |
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The six loads that read at this ordering |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | The half of the surface this constant governs |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | The consumers that import it by name |

### Types

| File | Relationship |
|------|--------------|
| [002_padded_cursor.md](002_padded_cursor.md) | The type whose reads this ordering governs |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:38-51` | The module-level argument for fixing rather than parameterising |
| `ring_cursor/src/lib.rs:71-89` | The declaration |
| `ring_spsc/src/lib.rs:177-213` | `OWN` and `HANDOFF`, the pairing partners |
| `ring_mpsc/src/lib.rs:239-287` | `OBSERVE`, `COMMIT`, `OWN` |
| `ring_publish/src/lib.rs:164` | `compare_exchange( start, end, PUBLISH, GATING )` — both halves in one call |

### Tests

| File | Relationship |
|------|--------------|
| `src/lib.rs:85-88` | The doctest |
| `ring_mpsc/tests/mpsc_test.rs:782` | External assertion |
| `ring_spsc/tests/spsc_test.rs:869` | External assertion |
| `tests/manual/readme.md` M3 | Exactly one non-doc `Ordering::Acquire` in this crate |
