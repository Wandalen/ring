# Integration: Who Reads a Cursor

### Scope

- **Purpose**: Enumerate the ten crates that depend on this one, classify what each takes, and use the split to explain why one of them forked a predicate this crate already owns.
- **Responsibility**: Give the consumer census with its regenerating command, separate the routes by which `SeqCell` arrives, and derive the structural condition under which sharing survives a manifest boundary.
- **In Scope**: The ten declaring crates; the `SeqCell` re-export's reach; `on_distinct_lines`'s two call paths.
- **Out of Scope**: What this crate depends on, which is [`integration/001`](001_four_dependencies_all_used.md).

### The Ten

```sh
cd "$(git rev-parse --show-toplevel)"
grep -l 'ring_cursor' */Cargo.toml | sed 's|ring/||;s|/Cargo.toml||'
```

Live output:

```
ring_barrier
ring_claim
ring_consume
ring_cursor
ring_debug
ring_gating
ring_mpsc
ring_publish
ring_shutdown
ring_spsc
ring_wait
```

Eleven manifests match; one is this crate's own `[package] name`. The ten
consumers:

| Crate | Takes | `SeqCell` route |
|-------|-------|-----------------|
| `ring_barrier` | `&[ PaddedCursor ]` | doctest only |
| `ring_claim` | `PaddedCursor`, `GATING` | via re-export |
| `ring_consume` | `PaddedCursor`, `GATING` | via re-export |
| `ring_debug` | `&CursorPair` | **direct from `ring_atomic`** |
| `ring_gating` | `Vec< PaddedCursor >` | doctest only |
| `ring_mpsc` | `PaddedCursor`, `GATING` | **direct from `ring_atomic`** |
| `ring_publish` | `PaddedCursor`, `GATING` | via re-export |
| `ring_shutdown` | `CursorPair` | neither |
| `ring_spsc` | `CursorPair`, `GATING` | via re-export |
| `ring_wait` | `CursorPair` | doctest only |

**Four of ten take the trait through this crate in library code.** That is the
re-export earning its place: four manifests that would otherwise carry a
`ring_atomic` edge to import one trait. Three more — `ring_barrier`,
`ring_gating`, `ring_wait` — take it only inside doc examples, a real use of
the re-export but not a manifest edge avoided, and `ring_shutdown` takes
neither route in library code. The two that go direct both have independent
business with the atomic layer — `ring_mpsc` uses `AtomicSeq` directly,
`ring_debug` reads cells the cursor layer does not expose — which is exactly
the condition `src/lib.rs:60-67` names as legitimate.

The count is easy to get wrong, so the check is run both ways — anchored, and
unanchored as its own control:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- anchored: manifests that actually declare the dependency --'
command grep -l '^ring_atomic' */Cargo.toml | sed 's:^:    :'
echo '  -- control: unanchored, which also matches a name and a comment --'
command grep -l 'ring_atomic' */Cargo.toml | sed 's:^:    :'
```

Live output:

```
  -- anchored: manifests that actually declare the dependency --
    ring_batch/Cargo.toml
    ring_cursor/Cargo.toml
    ring_debug/Cargo.toml
    ring_mpsc/Cargo.toml
    ring_tls/Cargo.toml
  -- control: unanchored, which also matches a name and a comment --
    ring_atomic/Cargo.toml
    ring_batch/Cargo.toml
    ring_cursor/Cargo.toml
    ring_debug/Cargo.toml
    ring_mpsc/Cargo.toml
    ring_spsc/Cargo.toml
    ring_tls/Cargo.toml
```

**Five manifests declare it; the unanchored form finds seven.** The two extras
are `ring_atomic`'s own manifest, matched on its `name =` line, and `ring_spsc`,
which discusses the crate in a comment without depending on it. Intersect the
anchored list with this crate's ten consumers and exactly two remain —
`ring_debug` and `ring_mpsc` — which is the "two exceptions" the paragraph above
counts.

### Two Shapes of Consumer

The `Takes` column splits cleanly, and the split predicts behaviour:

| Shape | Crates | Holds |
|-------|--------|-------|
| **Set** (2) | `ring_barrier`, `ring_gating` | Many cursors, no pairing — asks `slowest` |
| **Pair** (4) | `ring_debug`, `ring_shutdown`, `ring_spsc`, `ring_wait` | A `CursorPair` — asks the pair's own readings |
| **Loose** (4) | `ring_claim`, `ring_consume`, `ring_publish`, `ring_mpsc` | Individual cursors it wired together itself |

The **Loose** row is where this crate's abstractions stop reaching, and the
consequence is measurable.

**`ring_mpsc` imports `PaddedCursor` and `GATING` and never `CursorPair`** —
`src/lib.rs:189` is the whole of its cursor surface. It is the largest consumer
in the family and it holds none of this crate's composite types. That is the
seam the fork happened at, and the census predicts it without knowing anything
about `ring_mpsc`'s internals.

### The Predicate That Forked

`CursorPair::on_distinct_lines` answers "are these two cursors on different cache
lines". Two consumers ask it. They get different answers by different routes:

| Consumer | Code | Reaches `ring_align`? |
|----------|------|-----------------------|
| `ring_spsc:364` | `self.cursors.on_distinct_lines()` | **Yes** — through the `CursorPair` method |
| `ring_mpsc:865` | `claim.abs_diff( consume ) >= 64` | **No** — a literal, written locally |

Neither crate declares `ring_align`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -l '^ring_align' */Cargo.toml
# ring_cursor/Cargo.toml   — and nothing else
```

Live output:

```
ring_cursor/Cargo.toml
```

**So neither could call `ring_align::on_distinct_lines` directly. One of them
didn't need to.**

`ring_spsc` holds a `CursorPair`, and the predicate travels as a method on that
type — across the manifest boundary, at no cost, with no second literal.
`ring_mpsc` holds two cursors from two different owners (`self.claimer.cursor()`
and `self.ring.consumer_cursor()`), which is not a `CursorPair`, so no method
carries the predicate to it. It wrote `>= 64`.

### What That Says About Sharing

> A shared decision crosses a manifest boundary when it travels as a method on a
> type the consumer already holds. It does not cross as a free function, and it
> does not cross as a constant.

The family demonstrates all three cases at once:

| Decision | Vehicle | Crosses? | Evidence |
|----------|---------|:--------:|----------|
| The cache-line size | A method on `CursorPair` | ✅ | `ring_spsc:364` |
| The cache-line size | A free function in `ring_align` | ❌ | `ring_mpsc:865` wrote `64` |
| The gating ordering | A `const` in `ring_cursor` | ❌ | Three inline `Acquire`s — [`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md) |
| The `SeqCell` trait | A `pub use` re-export | ✅ | Eight of ten consumers |

**The two that cross are the two attached to a type.** A method rides on the
receiver; a re-export rides on the fact that the trait is needed to use the type
at all. A free function and a constant are attached to nothing, so reaching them
means a manifest edit, and a manifest edit for one line is a change nobody makes.

### The Two Predicates Are Not Even Equivalent

`ring_mpsc`'s local version is not a faithful copy:

```rust
// ring_align::on_distinct_lines
a / CACHE_LINE != b / CACHE_LINE

// ring_mpsc:865
claim.abs_diff( consume ) >= 64
```

Subtraction is **strictly stronger**: two addresses 64 or more apart are always
on different lines, but two addresses on different lines can be as little as 1
apart (63 and 64). So `ring_mpsc` under-reports — it would answer `false` for a
genuinely separated pair at 60 and 68.

For cache-aligned cursors both are multiples of 64, so today the two agree on
every input that actually occurs. **The divergence is latent, not active**, and
that is what makes it hard to notice: no test can distinguish them while the
alignment holds, and the alignment is the thing the predicate exists to check.

### What Nothing Checks

| # | Gap | Instrument that would |
|---|-----|-----------------------|
| C1 | That `ring_mpsc`'s copy agrees with `ring_align`'s | Nothing. Both are asserted true in their own suites, separately |
| C2 | That no eleventh consumer arrives with a third copy | Nothing — and `ring_batch` already carries the ordering equivalent without even depending on this crate |
| C3 | That the re-export stays reachable | The compiler, immediately — eight crates fail to build |
| C4 | That a **Loose**-shape consumer is not silently the wrong shape | Nothing. `ring_mpsc`'s two cursors could plausibly have been a `CursorPair` |

**C4 is the one worth acting on and this document does not act on it.** Whether
`ring_mpsc`'s claimer cursor and its ring's consumer cursor *should* be a
`CursorPair` is a question about `ring_mpsc`'s structure, not this crate's, and it
is recorded here only because this is where the evidence sits.

### CU18 — The Route Column Counts Doctests as Manifest Edges

| Route | Crates |
|-------|--------|
| via this crate's re-export | `ring_claim`, `ring_consume`, `ring_publish`, `ring_spsc` |
| direct from `ring_atomic` | `ring_batch`, `ring_debug`, `ring_mpsc`, `ring_tls` |
| doctests only | `ring_barrier`, `ring_gating`, `ring_wait` |
| neither | `ring_shutdown` |

The census in this document marks eight of ten consumers "via re-export".

**Finding.** Four take it through this crate. Two of the four counted as direct —
`ring_batch` and `ring_tls` — are not consumers of this crate at all, so they
could not have taken the re-export under any reading. The row that produced the
eight was a grep for the trait name rather than for the path it arrived by.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m1 'Four of ten take the trait' ring_cursor/docs/integration/002_who_reads_a_cursor.md
```

Live output:

```
**Four of ten take the trait through this crate in library code.** That is the
```

**Disposition:** applied — corrected the `SeqCell` route column and the "Eight
of ten" claim in this document's own consumer table (not this finding's frozen
text above): `ring_barrier`, `ring_gating` and `ring_wait` now read "doctest
only", `ring_shutdown` now reads "neither", and the summary line states four
of ten in library code — restricted to this crate's own ten consumers, since
`ring_batch` and `ring_tls` were never part of that count to begin with.
Now prints: `Four of ten take the trait through this crate in library code`

---

### CU19 — A Doctest Import Is a Real Import of a Different Thing

`ring_barrier`, `ring_gating` and `ring_wait` name `SeqCell` only inside doc
examples. A doctest compiles as a separate crate linked against the public
surface, so those imports are genuine — they exercise exactly the re-export.

**Finding.** The re-export earns its place in all three, and what it earns there
is not a manifest edge: a doctest already depends on the crate whose docs it
lives in. Counting the two kinds together is what produced CU5's "six" and CU18's
"eight", and separating them is the whole correction.

---

### CU20 — A Line-Numbered Citation Into a Crate Under Active Edit

```
      claim.abs_diff( consume ) >= 64
```

This document cites the forked predicate in `ring_mpsc` by line. A soundness fix
in that crate — moving `UnsafeCell` from around the buffer to around each slot —
moved it, and the same edit broke four of `ring_align`'s quoted recipes in one
afternoon.

**Finding.** A line number into an actively-edited file is a claim about a moving
target, and the corpus's own recipe checker turns that claim into a failing gate
on an unrelated change. The repair is to cite by pattern — the predicate text, an
`awk` range anchored on the signature — and to strip line numbers from any
cross-crate census where the number is incidental. That is now house practice and
this finding is why.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The readings the **Pair**-shape consumers take |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_surface_that_forwards.md](../api/001_the_surface_that_forwards.md) | The re-export, and its eight users measured |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | The constant that does not cross, measured across the same ten |

### Integrations

| File | Relationship |
|------|--------------|
| [001_four_dependencies_all_used.md](001_four_dependencies_all_used.md) | The `ring_align` edge whose uniqueness creates the bottleneck |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_fold_two_questions.md](../pattern/002_one_fold_two_questions.md) | The two **Set**-shape consumers, and the fold they share |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_reading_that_consults_one_cursor.md](../pitfall/002_a_reading_that_consults_one_cursor.md) | What a consumer that wires its own cursors risks |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:60-69` | The re-export's argument, and the exception it names |
| `ring_cursor/src/lib.rs:422-441` | `CursorPair::on_distinct_lines` — the vehicle that crosses |
| `ring_spsc/src/lib.rs:349-351` | The consumer that used it |
| `ring_mpsc/src/lib.rs:860-866` | The consumer that wrote `64` instead |
| `ring_align/src/lib.rs:138` | The free function neither could reach |

### Tests

| File | Relationship |
|------|--------------|
| `ring_spsc/tests/spsc_test.rs:217` | Asserts separation via the shared method |
| `ring_mpsc/tests/mpsc_test.rs:288` | Asserts separation via the forked literal — C1, the two never compared |
| `ring_align/tests/align_test.rs:96-101` | The boundary cases that distinguish the two predicates |
| `tests/cursor_test.rs:66-92` | This crate's own use of the predicate |
