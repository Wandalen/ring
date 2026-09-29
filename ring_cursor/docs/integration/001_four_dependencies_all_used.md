# Integration: Four Dependencies, All Used

### Scope

- **Purpose**: Account for each of this crate's four dependency edges by the items it actually imports, and record the one edge no other crate in the family has.
- **Responsibility**: Give the per-edge item census with the command that regenerates it, show what each edge buys, and name what would happen if any were dropped.
- **In Scope**: `ring_types`, `ring_seqno`, `ring_atomic`, `ring_align` as declared in `ring_cursor/Cargo.toml`.
- **Out of Scope**: The crates that depend on *this* one, which is [`integration/002`](002_who_reads_a_cursor.md).

### The Manifest

```toml
[dependencies]
ring_types  = { path = "../ring_types" }
ring_seqno    = { path = "../ring_seqno" }
ring_atomic = { path = "../ring_atomic" }
ring_align  = { path = "../ring_align" }
```

Four edges, no dev-dependencies, no optional features, no `default-features = false`
anywhere. It is the plainest manifest in the family.

### What Each Edge Buys

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^use ring_' ring_cursor/src/lib.rs
grep 'ring_seqno::' ring_cursor/src/lib.rs
```

Live output:

```
use ring_align::{ on_distinct_lines, CacheAligned };
use ring_atomic::AtomicSeq;
use ring_types::{ Capacity, Seq };
    ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
    ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
```

| Edge | Items imported | Sites | Would dropping it compile? |
|------|----------------|------:|----------------------------|
| `ring_types` | `Capacity`, `Seq` | many | No — `Seq` is in every signature |
| `ring_atomic` | `AtomicSeq`, `SeqCell` (re-exported) | many | No — `AtomicSeq` is the payload |
| `ring_seqno` | `free_slots`, `pending`, `may_claim` | **3** | No — but each is a one-line arithmetic body that could be inlined |
| `ring_align` | `CacheAligned`, `on_distinct_lines` | **5** | No — but `#[ repr( align( 64 ) ) ]` would replace it in one line |

**The last two rows are the interesting ones.** Both edges are load-bearing today
and both are *cheap to remove by reimplementation* — which is precisely the shape
of a dependency that quietly disappears in a refactor that looks like
simplification. They are handled as invariants rather than as facts:
[`invariant/002`](../invariant/002_the_number_64_never_appears_here.md) for the
`ring_align` edge, and `tests/manual/readme.md` M5 for the `ring_seqno` edge.

### The `ring_seqno` Edge Is Three Calls, and Was Four

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every call across this edge --'
# `-n` is deliberately absent: this section names the callees, never their
# addresses, and four line numbers here go stale on the next edit above them
command grep 'ring_seqno::' ring_cursor/src/lib.rs
printf '    calls: %s\n' "$( command grep -c 'ring_seqno::' ring_cursor/src/lib.rs )"
echo '  -- and the callee that is no longer among them --'
command grep 'ring_seqno::slowest' ring_cursor/src/lib.rs \
  || echo '    (ring_seqno::slowest — the fourth call, gone with the Vec that needed it)'
```

Live output:

```
  -- every call across this edge --
    ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
    ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
    calls: 3
  -- and the callee that is no longer among them --
    (ring_seqno::slowest — the fourth call, gone with the Vec that needed it)
```

Every one is the same shape: read both cursors at `GATING`, hand the two `Seq`
values to a pure function, return what it says. **This crate performs no sequence
arithmetic of its own** — it performs the *reads*, and `ring_seqno` performs the
subtraction. That division is the whole reason both crates exist:

| | Owns |
|---|---|
| `ring_seqno` | What two positions mean — wrapping, laps, capacity |
| `ring_cursor` | When and how those positions are obtained — atomically, at `GATING`, from cache-separated cells |

A reviewer removing the `ring_seqno` edge would have to move
`consumer.distance_to( producer )` into three method bodies here. Nothing would
break; the crate would just start owning arithmetic it deliberately does not
own.

**The fourth call left by exactly that route, and nobody called it a dependency
change.** `slowest` used to `collect()` cursor positions into a `Vec` purely so
it could hand a `&[ Seq ]` to `ring_seqno::slowest`; removing the allocation
removed the reason for the call, and the fold now runs `min()` in place
([`algorithm/001`](../algorithm/001_the_slowest_fold.md) § CU1). The edge
survived because three other calls hold it up. Had `slowest` been the only one,
a change filed as an allocation fix would have severed a declared dependency
silently — and `ring_seqno::slowest` itself now has no caller outside its own
tests, which is the same fact seen from the other end.

### The `ring_align` Edge Is Unique in the Family

```sh
cd "$(git rev-parse --show-toplevel)"
grep -l '^ring_align' */Cargo.toml
```

Live output:

```
ring_cursor/Cargo.toml
```

**One consumer. This crate is it.** `ring_align` exists to state the cache-line
size once for a family of 33 crates, and exactly one of them can name it.

That is not a defect — it is the intended tiering, and it works: every other
crate inherits the alignment through `PaddedCursor` without ever seeing
`CACHE_LINE`. But it makes this edge the single point through which the family's
padding decision reaches the family, and it means `ring_align`'s reachability is
entirely mediated by whether a caller is holding one of *this* crate's types.

The consequence is measurable, and it has already bitten once — see
[`integration/002`](002_who_reads_a_cursor.md) for `ring_mpsc`'s forked copy of
the line predicate, and for the structural reason it forked.

### The Four Are Not Interchangeable Layers

They form two pairs, not a stack:

| Pair | Supplies | Knows about |
|------|----------|-------------|
| `ring_types` + `ring_seqno` | The *value* — a `Seq`, and what two of them mean | Nothing about memory |
| `ring_atomic` + `ring_align` | The *cell* — an atomic, and where it sits | Nothing about ring semantics |

`ring_cursor` is the join. Neither pair depends on the other:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'ring_align' ring_atomic/Cargo.toml   # 0
grep -c 'ring_atomic' ring_align/Cargo.toml   # 0
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
0
0
```

Both are `0`, and that mutual ignorance is what forces
[`pattern/001`](../pattern/001_the_forwarding_newtype.md)'s newtype: with neither
dependency crate able to name the other, the trait impl has nowhere legal to live
except here.

### What Nothing Checks

| # | Gap | Instrument that would |
|---|-----|-----------------------|
| E1 | That the `ring_align` edge stays load-bearing | `tests/manual/readme.md` M6, a human reading; `cargo +nightly udeps` at level 4 after the fact |
| E2 | That the `ring_seqno` edge stays load-bearing | M5 — whose expected value drifted and then drifted back, see below |
| E3 | That no fifth edge is added without argument | Nothing. Adding a dependency is a one-line manifest edit |
| E4 | That the two pairs stay mutually ignorant | Nothing — a cycle would be caught by cargo, but a one-directional edge would not |

**E2 had a live defect that has since self-corrected by coincidence, which is a
worse finding than a defect that stays broken.**

| Line | Text | Kind | Status |
|-----:|------|------|--------|
| `readme.md:100` | "**Expected:** three `ring_seqno::` calls" | The check's own expectation | **Correct today** — was briefly stale while the count was four, see below |
| `readme.md:146` | "Three `ring_seqno::` calls (`free_slots`, `pending`, `may_claim`) at lines 267, 286, 316" | A dated Run Record | Correct — a record of what was true on 2026-08-28 |

Neither row is a live defect now. A run record is a historical observation and
is *supposed* to freeze; an expectation is a prediction about the present, and
this one happens to be right again — not because anyone revised it.

The original mismatch is visible in the record itself: the three calls it names
are `free_slots`, `pending`, and `may_claim`. A fourth — `ring_seqno::slowest`,
called from `slowest` before that function's own `Vec`-collecting allocation was
removed (above) — existed for a time after M5 was last run, and the expectation
was never revised to say "four" while it did. That fourth call is gone now, for
a reason unrelated to this check: the allocation fix in `slowest` deleted the
call along with the `Vec` that fed it, restoring the count to three by accident.

**The check passes today, but the pass in between would have been the
untrustworthy kind.** Its pass condition is the *second* command's silence — no
local restatement of the lap boundary — and that command finds nothing today.
While the fourth call existed, a reader running M5 would have seen four where
three were promised and had to work out whether the code drifted or the check
did; today the same reader sees three and has no way to tell, from the check
alone, that it was ever wrong in between. `grep -c` against a stated minimum,
rather than an exact literal, would not have drifted at all — and would not
have silently un-drifted either, which is the more useful property here.

### CU17 — The Edge Whose Removal Is Also the Invariant's Violation

```
9:ring_types = { path = "../ring_types" }
10:ring_seqno = { path = "../ring_seqno" }
11:ring_atomic = { path = "../ring_atomic" }
12:ring_align = { path = "../ring_align" }
```

Three of the four supply types used in signatures. `ring_align` supplies one
attribute, through one wrapper, at one line —
`pub struct PaddedCursor( CacheAligned< AtomicSeq > );`.

**Finding.** Replace that wrapper with `#[ repr( align( 64 ) ) ]` and the crate
still compiles, the tests still pass, and `ring_align` becomes an unused
dependency. So `cargo +nightly udeps` is a second-order detector for
[`invariant/002`](../invariant/002_the_number_64_never_appears_here.md) — and it
runs at verification level 4, which ordinary work does not reach.

---

### Integrations

| File | Relationship |
|------|--------------|
| [002_who_reads_a_cursor.md](002_who_reads_a_cursor.md) | The other side of the graph, and what the `ring_align` bottleneck costs there |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_number_64_never_appears_here.md](../invariant/002_the_number_64_never_appears_here.md) | E1, stated as a restriction on this crate's source |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | Why the mutual ignorance of `ring_atomic` and `ring_align` forces a local type |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_obvious_implementation_forks_the_constant.md](../pitfall/001_the_obvious_implementation_forks_the_constant.md) | What removing the `ring_align` edge looks like from the inside |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The type built from `ring_atomic` and `ring_align` together |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/Cargo.toml` | The four edges |
| `ring_cursor/src/lib.rs:56-58` | The three `use` lines that consume them |
| `ring_cursor/src/lib.rs:370, 389, 419` | Every `ring_seqno::` call |
| `ring_align/Cargo.toml` | One edge, to `ring_types` — and none to `ring_atomic`, which is what forces the newtype |
| `ring_cursor/tests/manual/readme.md:88-102, 146` | M5's expectation and its Run Record — E2 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` M5 | E2 — the `ring_seqno` edge check, with a stale expected value |
| `tests/manual/readme.md` M6 | E1 — the `ring_align` edge check |
| `tests/cursor_test.rs:33-37` | The test file importing `CACHE_LINE` through this crate's own dependency |
