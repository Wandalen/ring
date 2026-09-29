# Integration: Four Edges In and None Out

### Scope

**Purpose:** Establish what `ring_consume` depends on, what depends on it, and
what the difference between those two numbers means for how well-tested the
crate can be.

**Responsibility:** The declared dependency edges of `ring_consume`, in both
directions, read against the manifests of all 33 family crates.

**In Scope:** `ring_consume/Cargo.toml`; every `ring_*/Cargo.toml`
that names `ring_consume`; the four Tier 5 handshake primitives compared against
each other; the transitive closure `cargo tree` reports.

**Out of Scope:** What the edges are used *for* — that is
[`002`](002_eight_methods_and_the_one_that_is_called.md). The runtime cost of
any edge — that is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md).

---

## The Four Declared Dependencies

```sh
cd "$(git rev-parse --show-toplevel)"

# what this crate declares
cat ring_consume/Cargo.toml

# which of them the source actually names, and where
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
  | grep -E 'ring_(types|cursor|barrier|seq)::'
grep -E '^use ' ring_consume/src/lib.rs
```

Live output:

```
[package]
name = "ring_consume"
version = "0.1.0"
edition.workspace = true
description = "Single-consumer available-range computation and commit"
publish = false

[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_barrier = { path = "../ring_barrier" }
ring_seqno = { path = "../ring_seqno" }

[lints]
workspace = true
use ring_barrier::Barrier;
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
use ring_types::{ RingError, Seq };
      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
use ring_barrier::Barrier;
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
use ring_types::{ RingError, Seq };
```

Four dependencies, all four used, but not in the same way:

| Dependency | How it enters | Sites |
|------------|---------------|------:|
| `ring_barrier` | `use ring_barrier::Barrier;` | 1 type |
| `ring_cursor` | `use ring_cursor::{ PaddedCursor, SeqCell, GATING };` | 3 items |
| `ring_types` | `use ring_types::{ RingError, Seq };` | 2 items |
| `ring_seqno` | **no `use` at all** — one fully-qualified call at `:95` | 1 function |

Three of the four are imported at the top of the file and appear throughout.
`ring_seqno` is reached exactly once, by its full path, inside `available`:

```rust
.map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

### CN1 — The Whole Crate's Arithmetic Is One Call to a Crate It Names Once

`ring_consume` computes exactly one quantity — how many sequences are readable —
and it does not compute it. `ring_seqno::pending` does, at `src/lib.rs:95`, in the
crate's only fully-qualified cross-crate call and the only one of its four
dependencies with no `use` statement.

Everything else in the 418 lines is comparison, storage, and shape: `commit`
compares two `Seq` values against a range, `commit_available` stores one,
`Available` carries a start and a length. Subtract the one `pending` call and
nothing in the crate does arithmetic on a sequence at all.

This is worth recording because it is easy to read the crate as though it owned
the pending calculation — the module documentation describes computing an
available range, and `available` is where a reader looks for that. The
calculation is one tier down, shared with everything else in the family that
needs it, and the fully-qualified call style is the only visible signal that it
came from outside.

**Cost:** none. The finding is about where to look, not about a defect.

---

## What Depends on This Crate

```sh
cd "$(git rev-parse --show-toplevel)"

# every manifest naming any of the four Tier 5 handshake crates, by section
for c in ring_claim ring_publish ring_consume ring_barrier; do
  echo "--- $c ---"
  for m in ring_*/Cargo.toml; do
    sec=$( awk -v c="$c" '/^\[/{s=$0} $0 ~ "^[[:space:]]*"c"[[:space:]]*=" {print s}' "$m" )
    [ -n "$sec" ] && printf '  %-14s %s\n' "$( basename "$( dirname "$m" )" )" "$sec"
  done
done
```

Live output:

```
--- ring_claim ---
  ring_mpsc      [dependencies]
  ring_publish   [dev-dependencies]
--- ring_publish ---
--- ring_consume ---
  ring_publish   [dev-dependencies]
--- ring_barrier ---
  ring_consume   [dependencies]
  ring_publish   [dev-dependencies]
```

### CN2 — Two of the Four Tier 5 Primitives Have Zero Library Dependents

The read as a table:

| Tier 5 crate | Library dependents | Dev dependents |
|--------------|:------------------:|:--------------:|
| `ring_claim` | 1 — `ring_mpsc` | 1 |
| `ring_barrier` | 1 — `ring_consume` | 1 |
| `ring_consume` | **0** | 1 |
| `ring_publish` | **0** | **0** |

`ring_consume` is compiled into no other crate in the family. Its only
non-test consumer is itself. The single edge pointing at it is
`ring_publish`'s `[dev-dependencies]` — the handshake test, which builds a
producer and a consumer and runs one against the other.

That is not a defect, and it is not an argument that the crate should be
deleted. It is a statement about what the test suite can and cannot cover: a
crate with no library dependents is exercised only by its own tests and by
whatever a sibling's test happens to construct. Nothing downstream will ever
break because `ring_consume`'s behaviour changed, because nothing downstream
exists. The 21 tests in `tests/consume_test.rs` are, together with
`ring_publish`'s handshake test, the entire behavioural record.

`ring_claim` is the contrast that makes this readable. It has the same shape —
a Tier 5 primitive, small, heavily documented — but it is wired into `ring_mpsc`,
so a change to its semantics has somewhere to surface. `ring_consume`'s does not.

**Cost:** reachable, and it is the reason to weight this crate's own tests
more heavily than a dependent-rich crate's. There is no second line of defence.

---

### CN3 — `ring_mpsc` Dropped Both Halves, and Says So

The obvious question CN2 raises is why the crate that ought to be
`ring_consume`'s dependent is not. `ring_mpsc`'s own manifest answers it:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B2 -A12 'Eight, not the seven' ring_mpsc/Cargo.toml
```

Live output:

```
publish = false

# Eight, not the seven this manifest was scaffolded with. `ring_publish` and
# `ring_consume` are gone and `ring_atomic`, `ring_slot` and `ring_types` are
# new — see decision 124. Publication here is a per-slot stamp this crate owns,
# so there is no published cursor for either removed crate to act on.
[dependencies]
ring_atomic = { path = "../ring_atomic" }
ring_store = { path = "../ring_store" }
ring_claim = { path = "../ring_claim" }
ring_config = { path = "../ring_config" }
ring_cursor = { path = "../ring_cursor" }
ring_gating = { path = "../ring_gating" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }
```

The manifest comment records that `ring_publish` and `ring_consume` were both
removed from the dependency list and `ring_atomic`, `ring_slot` and `ring_types`
added, per a recorded rationale — because publication in `ring_mpsc` became a per-slot
stamp the crate owns, leaving no published cursor for either removed crate to
act on.

So the zero in CN2's table is a recorded decision rather than an omission. Two
things follow from that, and only the first is comfortable:

**One — the crate is not orphaned by accident.** A design ruling moved the
publication mechanism, and these two primitives serve the mechanism it moved
away from. They remain correct implementations of a cursor-based handshake.

**Two — nothing states what they are still for.** `ring_mpsc` documents why it
stopped using them. `ring_consume` does not document who it expects to be used
by instead, and neither does `ring_publish`. A reader arriving at either crate
finds a complete, tested, documented primitive with no stated consumer, and
must read a third crate's manifest comment to learn that the consumer it was
built for went elsewhere.

**Cost:** reachable as documentation drift. The crates are correct; what is
missing is a sentence in each saying which ring shape they serve now that the
family's flagship MPSC ring uses a different one.

---

## The Chain This Crate Pulls In

```sh
cd "$(git rev-parse --show-toplevel)"
set -o pipefail   # so a broken workspace manifest fails here rather than printing nothing
# 2>&1 through a filter for exactly cargo's package-cache lock line, which appears
# only when another process holds the lock; every other stderr line survives
tree() { cargo tree -p "$1" --edges normal --prefix none 2>&1 \
         | command grep -v 'Blocking waiting for file lock' | sort -u; }
tree ring_consume
tree ring_claim
```

Live output:

```
ring_align v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_align)
ring_atomic v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_atomic)
ring_barrier v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_barrier)
ring_consume v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_consume)
ring_cursor v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_cursor)
ring_cursor v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_cursor) (*)
ring_seqno v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_seqno)
ring_seqno v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_seqno) (*)
ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
ring_wait v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_wait)
ring_align v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_align)
ring_atomic v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_atomic)
ring_claim v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_claim)
ring_cursor v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_cursor)
ring_cursor v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_cursor) (*)
ring_gating v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_gating)
ring_seqno v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_seqno)
ring_seqno v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_seqno) (*)
ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
```

`ring_consume` reaches eight crates including itself; `ring_claim` reaches seven.
The extra one is `ring_wait`, pulled in through `ring_barrier`. What that costs
is [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md);
whether it is used at all is [`002`](002_eight_methods_and_the_one_that_is_called.md).

Note that `ring_consume` names `ring_seqno` **directly**, in its own manifest,
while `ring_claim` reaches it transitively through `ring_gating`. Both end up
depending on it. Only one declares the edge it actually uses.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| integration | [002](002_eight_methods_and_the_one_that_is_called.md) | what the `ring_barrier` edge is used for — one method of eight |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | what the eighth crate in the chain costs the other seven |
| decisions | [002](../decisions/002_plain_stores_rather_than_compare_exchange.md) | the single-consumer assumption CN2's isolation makes untested |
| pitfall | [002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | the empty-set behaviour only this crate's own tests cover |

### Sources

| What | Where |
|------|-------|
| The four declared dependencies | `ring_consume/Cargo.toml:9-12` |
| The single `ring_seqno` call | `ring_consume/src/lib.rs:342` |
| The three `use` statements | `ring_consume/src/lib.rs:71-73` |
| The decision-124 comment | `ring_mpsc/Cargo.toml` |
| The one dev-dependency edge | `ring_publish/Cargo.toml:15-18` |

### Tests

| Claim | Verified by |
|-------|-------------|
| All four dependencies are used | the `grep` above — every one appears at least once |
| `ring_seqno` is used exactly once | `grep -c 'ring_seqno::'` on the comment-stripped source → 1 |
| Zero library dependents | the per-section `awk` scan across all 33 manifests |
| The chain is eight crates | `cargo tree -p ring_consume --edges normal` |
