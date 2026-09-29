# algorithm

Two algorithms, and neither is about sequences. `AtomicSeq` has none — each of its
four methods is one `AtomicU64` intrinsic with a newtype wrapped around the
result, so the production path cannot be slower than a bare atomic and has
nothing of its own to get wrong. Everything computational in the crate is in the
other implementation: `CountingSeq` increments a counter and delegates, which
costs two hardware atomics per logical operation and turns a plain read into a
read-modify-write.

The second algorithm is the one that reads those counters back. `counts` takes
four separate `Relaxed` loads and sums them into one struct, and `reset_counts`
writes four separate stores. Both are correct for one thread and neither says so.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_intrinsic_or_two.md) | One Intrinsic, or Two | Both implementations of the four operations, and what the instrument costs |
| [002](002_counts_is_four_reads_not_one.md) | `counts` Is Four Reads, Not One | The snapshot that is not one, measured, and the test that cannot see it |

## The Production Path Is a Rename

`impl SeqCell for AtomicSeq` is twenty-six lines for four methods, and three of
`compare_exchange`'s six are `.map( Seq )`, `.map_err( Seq )`, and a brace. There
is no branch, no assertion, no fallback, and no ordering picked on a caller's
behalf anywhere in it.

That is the design working. `AtomicSeq` exists so the family has exactly one
place where an atomic is *created* — which is what makes the `loom` seam a
one-line switch and what makes the ordering argument checkable in one file — not
so it can do something `AtomicU64` could not.

## Every Cost in This Crate Belongs to the Instrument

`CountingSeq` was built for two acceptance criteria that are negative claims
about atomic traffic. Measured, it issues two atomics where the production cell
issues one: 1.90× on `fetch_add`, 3.42× on `load`, and 1.82× with eight threads
on one cell.

`load` is the worse ratio for a reason that is not volume. A pure atomic read can
complete on a shared cache line; prefixing it with a counter `fetch_add` makes it
take the line exclusively. The shim does not add an operation to a read, it turns
the read into a write — the exact traffic class the criteria are counting.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- both implementations of the same four operations --'
command grep -n '^impl SeqCell for' ring_atomic/src/lib.rs
echo '  -- every atomic operation the crate body issues --'
command grep -nE '(self\.[a-z_0-9]+)\.(load|store|fetch_add|compare_exchange)\(' ring_atomic/src/lib.rs
echo '  -- and the five fields they land on --'
command grep -m1 -A7 -F 'pub struct CountingSeq' ring_atomic/src/lib.rs
```

The census above lists fifteen; the sixteenth is `AtomicSeq::compare_exchange`,
whose receiver and method sit on separate lines (`:183-184`) and so does not
match a single-line pattern.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT1 | `ring_atomic` | n/a — observation | Every `AtomicSeq` method is one intrinsic with a newtype around the result — no branch, no assertion, no ordering chosen — so the production path has no algorithm of its own to be wrong about |
| AT2 | `ring_atomic` | **measured cost** | `CountingSeq` issues two hardware atomics per logical operation and turns a pure read into a read-modify-write — 1.90× on `fetch_add`, 3.42× on `load`, 1.82× at eight threads — while certifying a count of logical operations for two criteria phrased in fences |
| AT3 | `ring_atomic` | **latent hazard** | `counts` builds one struct from four separately-timed `Relaxed` loads, so it can report a state that never existed — 11,575 impossible snapshots in 2,000,000, the widest by 5,456 |
| AT4 | `ring_atomic` | n/a — doc gap | `reset_counts` has the same seam on the write side; both methods' doc comments read as descriptions of an instant, and the one concurrent test touches a single counter so neither seam is reachable from the suite |
