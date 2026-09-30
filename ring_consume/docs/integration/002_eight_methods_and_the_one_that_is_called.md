# Integration: Eight Methods and the One That Is Called

### Scope

**Purpose:** Establish how much of `ring_barrier`'s surface `ring_consume`
actually uses, and what the unused remainder costs the crates that carry it.

**Responsibility:** The `ring_consume` → `ring_barrier` edge, measured method by
method, and the `ring_barrier` → `ring_wait` edge it drags in behind it.

**In Scope:** `Barrier`'s nine public methods; every call site of each across all
33 crates' `src/` and `tests/`; the dependency `ring_wait` enters through.

**Out of Scope:** Whether the edge should exist at all — it must, `available`
cannot be computed without a frontier. What `ring_wait` costs in properties
rather than in calls — that is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md).

---

## The Only Library Consumer Calls One Method

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# which crates' library code imports Barrier at all
command grep -rl 'ring_barrier::Barrier\|use ring_barrier' ring_*/src/*.rs

# every method call on ring_consume's barrier field, across line breaks
perl -0777 -ne 'while(/\.\s*barrier\s*\n?\s*\.\s*(\w+)/g){print "  .$1()\n"}' \
  ring_consume/src/lib.rs
```

Live output:

```
ring_barrier/src/lib.rs
ring_consume/src/lib.rs
  .frontier()
```

Two files import `Barrier`, and one of them is `ring_barrier` defining it. So
`ring_consume` is the **only** library consumer of `ring_barrier` in the family
— which [`001`](001_four_edges_in_and_none_out.md) already established from the
manifests, and which this confirms from the source.

That single consumer calls one method. `Barrier` has nine.

### CN4 — Eight of Nine `Barrier` Methods Have No Library Caller

```sh
cd "$(git rev-parse --show-toplevel)"
for m in over dependencies len is_empty cursor frontier available admits wait_for; do
  printf '  %-14s %s\n' "$m" \
    "$( grep -rhE "Barrier::$m|barrier\s*\.\s*$m\s*\(" \
         ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null | wc -l )"
done

# and where those hits live
for f in ring_publish/tests/handshake_test.rs \
         ring_consume/tests/consume_test.rs \
         ring_barrier/tests/barrier_test.rs; do
  printf '  %-40s %s\n' "$( echo "$f" | sed 's|ring/||;s|/tests/|/|' )" \
    "$( perl -0777 -ne 'my %s; while(/(?:Barrier::(\w+)|\bbarrier\s*\.\s*(\w+)\s*\()/g){ $s{$1//$2}=1 } print join(" ",sort keys %s)' "$f" )"
done
```

Live output:

```
  over           86
  dependencies   1
  len            2
  is_empty       2
  cursor         4
  frontier       11
  available      12
  admits         5
  wait_for       7
  ring_publish/handshake_test.rs           over
  ring_consume/consume_test.rs             over
  ring_barrier/barrier_test.rs             admits available cursor dependencies frontier is_empty len over wait_for
```

Read as a table, with `src` meaning library code rather than a test:

| `Barrier` method | Called from `src` | Exercised outside `ring_barrier` |
|------------------|:-----------------:|:--------------------------------:|
| `frontier` | ✔ — `ring_consume:341` | ✔ |
| `over` | ✘ — constructed only by tests | ✔ — 2 test files |
| `dependencies` | ✘ | ✘ |
| `len` | ✘ | ✘ |
| `is_empty` | ✘ | ✘ |
| `cursor` | ✘ | ✘ |
| `available` | ✘ | ✘ |
| `admits` | ✘ | ✘ |
| `wait_for` | ✘ | ✘ |

Every method is covered — `ring_barrier`'s own test file exercises all nine, so
this is not a coverage gap. It is a *use* gap, and the distinction matters:
seven of the nine have never been called by anything except the tests that were
written to call them. A method exercised only by its own crate's tests has had
its behaviour asserted but not its design validated; nothing has yet tried to
use it and found the signature awkward, the return type wrong, or the
semantics not quite what a caller needed.

`over` is the interesting middle case. Two sibling test files construct a
`Barrier` with it, so its ergonomics have been exercised from outside. No
library code anywhere constructs one — every `Barrier` in the family that is
not in a test is one that `ring_consume` received as a constructor argument
and never made.

**Cost:** reachable, and it is a design-confidence cost rather than a
correctness one. The eight unused methods may be exactly right. Nothing has
tested that claim in the only way that tests it.

---

## The Dependency Behind the Method Nobody Calls

`wait_for` is the reason `ring_barrier` depends on `ring_wait`. `cargo tree`'s
`2>/dev/null` below drops a message cargo prints when it has to wait on
another concurrent cargo process for the package-cache lock — real but
non-reproducible, since whether it fires depends on unrelated work running at
the same moment, not on anything this crate does:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'ring_wait' ring_barrier/Cargo.toml ring_barrier/src/lib.rs
command grep -m1 -B1 -A19 -F '/// How far a consumer may read, given what it depends on.' ring_barrier/src/lib.rs
cargo tree -p ring_consume --edges normal --prefix none 2>/dev/null | sort -u
```

Live output:

```
ring_barrier/Cargo.toml:ring_wait = { path = "../ring_wait" }
ring_barrier/src/lib.rs://! Depends on `ring_types`, `ring_cursor`, `ring_wait`.
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;

/// How far a consumer may read, given what it depends on.
///
/// Borrows its dependencies rather than owning them: the cursors belong to
/// whoever advances them — a publisher, an upstream consumer — and a barrier
/// that owned copies would be reading positions nobody was writing.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_barrier::Barrier;
/// use ring_cursor::{ PaddedCursor, SeqCell };
/// use ring_types::Seq;
///
/// let published = [ PaddedCursor::default() ];
/// published[ 0 ].store( Seq( 5 ), Ordering::Release );
///
/// let barrier = Barrier::over( &published );
/// assert_eq!( barrier.frontier(), Some( Seq( 5 ) ) );
/// assert_eq!( barrier.available( Seq( 2 ) ), 3, "sequences 2, 3 and 4" );
/// ```
#[ derive( Debug, Clone, Copy ) ]
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
```

### CN5 — `ring_consume`'s Chain Is One Crate Longer Than `ring_claim`'s, for a Method It Never Calls

`ring_consume` reaches eight crates; `ring_claim` reaches seven. The extra crate
is `ring_wait`, and the path to it is:

```
ring_consume → ring_barrier → ring_wait
```

`ring_barrier` needs `ring_wait` for exactly one of its nine methods —
`wait_for`, which takes a `WaitKind` and a spin count and blocks until the
barrier admits a range. That method has zero library callers anywhere in the
family ([CN4](#cn4--eight-of-nine-barrier-methods-have-no-library-caller)),
and `ring_consume` in particular never calls it: the crate's whole design is
the non-blocking half of the handshake, and a caller who wants to wait composes
the wait itself.

So `ring_consume` — a crate that never blocks, allocates no `WaitKind`, and
mentions `ring_wait` nowhere in its manifest or source — nonetheless compiles
`ring_wait` into every binary that links it, because a method it does not call
on a type it uses for one other purpose needs it.

What that costs is not compile time, which is negligible for a crate this
small. It is two properties, and both are measured in
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md):
`ring_wait` is the only crate in the eight-crate chain containing a `std::`
path, and the only one containing a blocking construct. `ring_claim`'s
seven-crate chain has neither. The two Tier 5 siblings are therefore not
equivalent in the properties they can claim, and the entire difference is one
transitive edge reached through an uncalled method.

A feature gate on `ring_barrier::wait_for` would restore both properties to
`ring_consume` at the cost of one `cfg` and one optional dependency. Nothing in
the family currently proposes that, because nothing currently measures either
property.

**Cost:** reachable. It costs `ring_consume` the `no_std`-cleanliness and the
never-blocks guarantee that its sibling holds, for a capability it does not use.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| integration | [001](001_four_edges_in_and_none_out.md) | the manifest-level view of the same edges |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | the two properties CN5's edge costs, measured |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | what `frontier` — the one method called — is used to compute |
| pitfall | [002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | what `frontier` returns when the barrier is empty, and why that is not what a `GatingSet` would return |

### Sources

| What | Where |
|------|-------|
| `Barrier`'s nine public methods | `ring_barrier/src/lib.rs:86-288` |
| The one call site | `ring_consume/src/lib.rs:341` |
| `wait_for`, the `ring_wait` seam | `ring_barrier/src/lib.rs:285` |
| The eight-crate chain | `cargo tree -p ring_consume --edges normal` |

### Tests

| Claim | Verified by |
|-------|-------------|
| `ring_consume` calls one `Barrier` method | the multi-line `perl` scan → `.frontier()` alone |
| `ring_consume` is the only library importer | `grep -rln` over all `ring_*/src/*.rs` |
| All nine methods are covered by tests | `ring_barrier/tests/barrier_test.rs` exercises all nine |
| `wait_for` has zero library callers | the per-method count above — every hit is in a test file |
