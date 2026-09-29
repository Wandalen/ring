# Workaround: A Vector That Changed a Slice's Type

### Scope

**Purpose:** Record the allocation that was on this crate's read path as what
it actually was — a signature adaptation, not a computation — state the
condition under which it could be deleted, and record what happened when that
condition was met from a direction this document did not consider.

**Responsibility:** `ring_cursor::slowest`'s `Vec`; why it existed; what it was
believed to buy; what removing it actually cost.

**In Scope:** The call chain from `Consumer::available` to `ring_seqno::slowest`
as it stood and as it stands now; the measured allocation; the zero-allocation
equivalent that shipped.

**Out of Scope:** The cost figures themselves, which are
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md).
The other two inherited constraints, which are
[`001`](001_five_functions_none_const.md).

---

## The Chain

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs | grep -A3 'pub fn frontier'
grep -vE "^[[:space:]]*//" ring_cursor/src/lib.rs  | grep -A4 'pub fn slowest'
grep -vE "^[[:space:]]*//" ring_seqno/src/lib.rs     | grep -A3 'pub fn slowest'
```

Live output:

```
  pub fn frontier( &self ) -> Option< Seq >
  {
    ring_cursor::slowest( self.dependencies )
  }
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}

pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

Three crates, one question — *what is the slowest cursor?* — until commit
`b7e075ca`, which cut the last link. The Live output above is the shape after
that cut: `ring_cursor::slowest` folds the cursors itself, and
`ring_seqno::slowest` is still there, still correct, and no longer reached from
this path at all.

```rust
// the chain as this document found it, before commit b7e075ca

// ring_consume::Consumer::available
self.barrier.frontier().map_or( 0, | f | ring_seqno::pending( f, position ) )

// ring_barrier::Barrier::frontier
ring_cursor::slowest( self.dependencies )

// ring_cursor::slowest — the two lines this document is about
let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
ring_seqno::slowest( &positions )

// ring_seqno::slowest
cursors.iter().copied().min()
```

Those two middle lines are now one, and it is the fourth line's own body moved
up a crate:

```rust
// ring_cursor::slowest, since commit b7e075ca
cursors.iter().map( | c | c.load( GATING ) ).min()
```

`Consumer::available` still calls `ring_seqno::pending`, so this crate's own edge
into `ring_seqno` is untouched. What went away is one call, in a crate in the
middle — which turns out to be the distinction the rest of this document was
missing.

### CN53 — The Allocation Existed to Change a Slice's Element Type, and Nothing Else

`ring_seqno::slowest` takes `&[ Seq ]`. `ring_cursor::slowest` had
`&[ PaddedCursor ]`. There is no way to reinterpret one as the other — a
`PaddedCursor` is 64 bytes of padded atomic, a `Seq` is 8 bytes of plain integer
— so the `Vec` materialised the loaded values into a contiguous buffer purely so
that a slice of the right element type existed to pass.

It computed nothing. The entire computation was one line further down:
`.min()`.

Measured at the time, release build, against an inline equivalent. Both blocks
below are frozen: the probe was a scratch binary, and neither it nor the numbers
it printed exist any more.

```rust
let via_crate = ring_cursor::slowest( &published );
let inline    = published.iter().map( | c | c.load( Ordering::Acquire ) ).min();
```

```
ring_cursor::slowest     -> Some(Seq(350)), 1 alloc
inline .map(..).min()    -> Some(Seq(350)), 0 alloc
same answer: true
empty:  crate None / inline None
```

Identical results including the empty case, which is the one that would most
plausibly have diverged — `Vec::collect` on an empty iterator and
`Iterator::min` on an empty iterator both yield the `None` the caller wants, so
even the edge case was unchanged.

That comparison is the whole argument, and it turned out to be the whole fix as
well: the `inline` line above *is* the body now. It was applied two crates away,
in commit `b7e075ca`, and nothing on this page was consulted or updated when it
landed.

```sh
cd "$(git rev-parse --show-toplevel)"

body=$( awk '/^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_cursor/src/lib.rs )
printf 'allocations left in the adapter body: %s\n' \
  "$( printf '%s\n' "$body" | command grep -cE 'Vec<|collect\(\)' )"
printf 'calls left from ring_cursor::slowest into ring_seqno: %s\n' \
  "$( printf '%s\n' "$body" | command grep -c 'ring_seqno::' )"
# the trailing filter drops `//`, `///` and `//!` lines: a doc comment that
# mentions this function is not a call to it
printf 'ring_seqno::slowest callers outside ring_seqno: %s\n' \
  "$( grep -rn 'ring_seqno::slowest' --include='*.rs' */ \
      | grep -v '^ring_seqno/' | grep -vE ':[[:space:]]*//' | wc -l )"
```

Live output:

```
allocations left in the adapter body: 0
calls left from ring_cursor::slowest into ring_seqno: 0
ring_seqno::slowest callers outside ring_seqno: 0
```

One heap allocation, on every `available()`, on every `commit()`, on every poll
of an idle ring, to obtain a slice of a different element type so that a
one-line function in a lower crate could be called. Now none — and the third
line is the part worth pausing on: the function the adaptation existed to reach
now has no caller anywhere outside its own crate.

This is the same site
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
measured at 1000 allocations per 1000 calls and now measures at zero, and the
same one `ring_claim`'s corpus recorded as CL55. What this instance added was
the *reason* — it was not an algorithmic cost, it was a signature-matching cost,
and the algorithm it was matching a signature for was `.min()`. That reason is
also why the fix was four lines: nothing had to be reimplemented, because
nothing was ever being computed.

**Cost:** was reachable, and was the crate's largest per-call cost. Recorded
here rather than only as a number because the number is easy to defend
("computing a minimum over a set costs something") and the true statement was
that the minimum cost nothing and the plumbing cost everything.

**Disposition:** applied — `ring_cursor/src/lib.rs` now holds the
`inline` line from the comparison above, and
`ring_consume/tests/allocation_test.rs` asserts the six read-path call
shapes at zero so it cannot return unnoticed. Both dispositions on this page
were first written `declined`, on the ground that the site belonged to another
crate's corpus and a `ring_consume`-only pass could not touch it — which was
true, and did not stop the change from landing from the other side. Now prints:
`allocations left in the adapter body: 0`

---

### CN54 — What It Bought Was One Call, Not the Edge, and the Option Ranked Worst Is the One That Shipped

The `Vec` was not an oversight — it was read here as the thing preserving the
`ring_cursor` → `ring_seqno` call. Delete it and `ring_cursor::slowest` becomes:

```rust
cursors.iter().map( | c | c.load( GATING ) ).min()
```

…which is correct, allocation-free, and no longer calls `ring_seqno` at all. The
question the code was answering was therefore read as *should `ring_cursor`
depend on `ring_seqno` for this?*, and it answered yes at the cost of an
allocation per call.

That framing was one word too broad, and the word is *depend*. The manifest
edge was never what the allocation was holding up:

```sh
cd "$(git rev-parse --show-toplevel)"

printf 'ring_seqno in ring_cursor/Cargo.toml: %s\n' \
  "$( command grep -c '^ring_seqno' ring_cursor/Cargo.toml )"
# no `-n`: the prose below names the three functions, not their line numbers,
# and an absolute line number in a quoted block goes stale on the next edit
echo '  -- what ring_cursor still calls ring_seqno for, after the fold left --'
command grep 'ring_seqno::' ring_cursor/src/lib.rs | sed 's/^ *//' | sed 's/^/    /'
printf '    calls: %s\n' "$( command grep -c 'ring_seqno::' ring_cursor/src/lib.rs )"
```

Live output:

```
ring_seqno in ring_cursor/Cargo.toml: 1
  -- what ring_cursor still calls ring_seqno for, after the fold left --
    ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
    ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
    calls: 3
```

The dependency is still declared and still used, three times, by
`SharedCursors`. Removing the fold cost one call out of four. Everything below
was written as though it cost the edge.

Three ways out, with what each gives up, and what became of each:

| Option | Allocation | Keeps the `ring_seqno` edge | Cost | What happened |
|--------|-----------:|:-------------------------:|------|---------------|
| inline the `.min()` in `ring_cursor` | 0 | ✘ | the layering claim, for this one function | **shipped, `b7e075ca`** |
| `ring_seqno::slowest` takes `impl Iterator< Item = Seq >` | 0 | ✔ | a signature change; `ring_seqno` gains a generic | not taken |
| stack buffer for `n <= 8`, `Vec` beyond | 0 in practice | ✔ | a branch and a size limit | not taken |

The ✘ in the first row is the mistake, and it is the reason that row was ranked
last. The edge survives; what row 1 actually gave up was one call and the claim
that this particular fold was shared. The second option was called *the one that
keeps everything* — `ring_seqno::slowest`'s body is already
`cursors.iter().copied().min()`, so a signature taking the iterator directly
would have deleted the adaptation at both ends and left the body shorter, and it
would have been const-hostile in the same way it already is
([`001`](001_five_functions_none_const.md) CN51), so nothing was lost there
either. It was ranked first on a criterion that turned out not to be in play,
and the option ranked last on that same criterion is what shipped.

`ring_seqno::slowest` paid for it. It kept its signature, its body and its
documentation, and lost its only caller outside its own tests — which is the
outcome the ranking above was built to avoid, arrived at by protecting it.

Two things kept this from being a straightforward recommendation at the time,
and both belong on the record because one of them is why it took an unrelated
commit to resolve:

**The allocation fired when there was nothing to read.** Measured against an
empty barrier the crate allocated zero — `frontier()` returns `None` without
calling through — but against a *non-empty* barrier with nothing pending it
allocated every time
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)).
An idle consumer polling a live ring paid the full cost to learn that nothing
happened, which is the worst shape for a cost to have.

**No test measured it.** The crate's test suite asserted behaviour, and the
behaviour was correct with or without the `Vec`. `tests/manual/readme.md`'s six
checks (`§ N1`–`§ N6`) cover atomics, orderings, and commit ranges; none counts
allocations. So the workaround was invisible to everything except a deliberate
probe — which is also why its removal was invisible, in the other direction, for
as long as it was. That half is closed: `tests/allocation_test.rs` now measures
the read path on every run.

**Deletion condition:** `ring_seqno::slowest` accepting an iterator, or
`ring_cursor::slowest` doing its own `.min()`. The second was taken. It removed
the allocation from `available`, `available_up_to`, `commit`, and
`commit_available` at once, and from `ring_claim`'s claim path in the same
change, exactly as predicted.

**Cost:** was reachable. The workaround was deliberate, its purpose defensible,
its price the crate's largest per-call cost. What this section got wrong was not
the cost or the fix but the price of the fix — it priced the cheapest option as
the loss of a dependency, and the dependency was never for sale.

**Disposition:** applied — `ring_cursor::slowest` does its own `.min()`, the
second of the two deletion conditions named above. This section's ranking is
kept exactly as written, including the ✘ that is wrong, because the finding now
worth recording is that a correct measurement and a correct fix can sit in the
same paragraph as a mis-priced tradeoff, and only the tradeoff survives to
misinform. Now prints: `calls: 3`

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| workaround | [001](001_five_functions_none_const.md) | the two constraints that cost nothing |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | the measured figures |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | the call site |
| integration | [001](../integration/001_four_edges_in_and_none_out.md) | the four edges this chain crosses |
| pitfall | [002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | the empty case, where it never fired |

### Sources

| What | Where |
|------|-------|
| The `Vec`, while it existed | `ring_cursor::slowest`, `ring_cursor/src/lib.rs` |
| What replaced it | the same function, one line, quoted above |
| `ring_seqno::slowest`'s body, unchanged throughout | `ring_seqno/src/lib.rs`, `fn slowest` |
| `frontier`'s delegation | `ring_barrier/src/lib.rs`, `fn frontier` |
| The call site | `ring_consume/src/lib.rs`, `fn available` |
| The test that now pins the read path at zero | `ring_consume/tests/allocation_test.rs` |

### Tests

| Claim | Verified by |
|-------|-------------|
| One allocation per call, while the `Vec` was there | the frozen counting-allocator probe above, 1 vs 0 |
| The inline form gave identical answers | `Some(Seq(350))` from both, `None` from both on empty |
| The inline form is now the shipped body | the `awk` extract above, `allocations left in the adapter body: 0` |
| `ring_seqno::slowest` is one iterator chain | `ring_seqno/src/lib.rs`, `fn slowest`, body quoted above |
| The `ring_cursor` → `ring_seqno` edge outlived the fold | `calls: 3`, and the manifest line, in the second recipe |
| Zero allocations on every read-path call, now | `no_read_of_the_available_range_allocates`, six rows with a control arm |
| No test measured it then | `§ N1`–`§ N6` cover atomics and ranges; none counts allocations |
