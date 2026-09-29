# integration

Four edges in, none out, and a public surface that almost nothing calls. This
crate imports `ring_types`, `ring_cursor`, `ring_barrier` and `ring_seqno`, and no
library in the family imports it. That is not a gap in the graph — it is where
the graph currently ends, and both instances here are about what that costs.

The first traces the edges themselves and finds that the crate's entire
arithmetic contribution is one call to `ring_seqno::pending`, a crate it names
exactly once. The second walks the other direction, through `Barrier`'s nine
public methods, and finds that eight of them have no caller outside their own
crate's tests — including `wait_for`, the one method that drags `std` and a
blocking loop into a chain that would otherwise have neither.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Four Edges In and None Out](001_four_edges_in_and_none_out.md) | CN1, CN2, CN3 — the whole dependency graph, two Tier 5 primitives with no library dependents, and a `ring_mpsc` doc describing a decision it has since reversed |
| 002 | [Eight Methods and the One That Is Called](002_eight_methods_and_the_one_that_is_called.md) | CN4, CN5 — `Barrier`'s nine, the one `ring_consume` uses, and the chain that is one crate longer than `ring_claim`'s for a method nobody calls |

### The Graph, in Full

```
ring_types ──┬──▶ ring_cursor ──▶ ring_barrier ──┐
             │                                   ├──▶ ring_consume
             ├──▶ ring_seqno ──────────────────────┘
             │
             └──▶ (ring_wait) ◀── ring_barrier
```

`ring_wait` is parenthesised because `ring_consume` never names it. It arrives
transitively, through `ring_barrier`, solely because `Barrier::wait_for` exists
— and `Barrier::wait_for` has no library caller anywhere in the family. The
crate's `std` exposure and its only blocking construct both enter through a door
nobody opens.

### Why the Missing Outgoing Edge Matters

`ring_consume` and `ring_publish` are the two Tier 5 primitives with zero
library dependents. `ring_claim` and `ring_barrier` each have exactly one —
`ring_mpsc` and `ring_consume` respectively. The split is by dependents, not by
what the crates do, and it does not follow the read/write pairing: one consume
crate and one publish crate sit on each side of it.

The asymmetry decides where confidence has to come from: with no consumer crate
exercising it in anger, everything this crate promises is promised by its own
test suite and by `ring_publish/tests/handshake_test.rs`, which is the single
place all four operations meet. That test reaches `ring_consume` through a
`[dev-dependencies]` edge, which is why the crate can have a caller there and
still have no library dependent — the two are different questions, and only the
manifest section distinguishes them.

That one test is doing a great deal of load-bearing work, and it lives in
another crate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

echo '  -- out: the declared edges --'
command grep -E '^ring_[a-z_]* = ' ring_consume/Cargo.toml | sed 's/ = .*//;s/^/    /'

echo '  -- in: library dependents of each Tier 5 primitive --'
for c in ring_consume ring_claim ring_publish ring_barrier; do
  who=$( awk -v w="$c" '/^\[/{ s = $0 } $0 ~ w" = " && s == "[dependencies]" { print FILENAME }' \
         ring_*/Cargo.toml | command grep -v "/$c/" | sed 's|ring/||;s|/Cargo.toml||' | tr '\n' ' ' )
  printf '    %-14s %s\n' "$c" "${who:-(none)}"
done

echo '  -- every use in the crate, against those edges --'
command grep '^use ' ring_consume/src/lib.rs | sed 's/^/    /'
printf '    ring_seqno:: named in src/: %s time(s)\n' \
  "$( command grep -c 'ring_seqno::' ring_consume/src/lib.rs )"

echo "  -- Barrier's nine, and the library calls of each --"
# `ring_barrier` has exactly one library dependent, listed above, so every
# library call of a `Barrier` method is in that one crate's `src/` -- which is
# what makes this census exact rather than a guess. It matters because four of
# the nine names (`len`, `is_empty`, `cursor`, `available`) are also carried by
# `Vec`, by slices, and by `Consumer` itself, so a family-wide grep for them
# counts unrelated calls by the thousand. Doc comments are excluded: a doctest
# is a test that happens to live in `src/`.
for m in over dependencies len is_empty cursor frontier available admits wait_for; do
  lib=$( awk -v m="$m" '
    /^[ \t]*(\/\/\/|\/\/!)/ { next }
    { line = ( prev ~ /barrier$|barrier\)$/ ) ? prev $0 : $0
      if ( line ~ ( "(barrier|Barrier)[^;]*[.:]" m "\\(" ) ) n += 1
      prev = $0 }
    END { print n + 0 }' ring_consume/src/lib.rs )
  # `over` is an associated function, spelled `Barrier::over(` -- matching it
  # as a method call would report a false zero for the one constructor.
  case $m in
    over ) tpat="Barrier::$m(" ;;
    * )    tpat="\.$m(" ;;
  esac
  tst=$( command grep -rc -- "$tpat" ring_barrier/tests/*.rs 2>/dev/null \
         | awk -F: '{ s += $2 } END { print s + 0 }' )
  printf '    %-13s library: %-3s ring_barrier tests: %s\n' "$m" "$lib" "$tst"
done

echo '  -- what ring_wait costs, and who reaches it --'
printf '    std:: in ring_wait/src: %s   ring crates naming ring_wait: %s\n' \
  "$( command grep -c 'std::' ring_wait/src/lib.rs )" \
  "$( command grep -l 'ring_wait' ring_*/Cargo.toml 2>/dev/null | command grep -vc '^ring_wait/' )"
```

Live output:

```
  -- out: the declared edges --
    ring_types
    ring_cursor
    ring_barrier
    ring_seqno
  -- in: library dependents of each Tier 5 primitive --
    ring_consume   (none)
    ring_claim     ring_mpsc 
    ring_publish   (none)
    ring_barrier   ring_consume 
  -- every use in the crate, against those edges --
    use ring_barrier::Barrier;
    use ring_cursor::{ PaddedCursor, SeqCell, GATING };
    use ring_types::{ RingError, Seq };
    ring_seqno:: named in src/: 1 time(s)
  -- Barrier's nine, and the library calls of each --
    over          library: 0   ring_barrier tests: 33
    dependencies  library: 0   ring_barrier tests: 1
    len           library: 0   ring_barrier tests: 3
    is_empty      library: 0   ring_barrier tests: 2
    cursor        library: 0   ring_barrier tests: 5
    frontier      library: 1   ring_barrier tests: 20
    available     library: 0   ring_barrier tests: 9
    admits        library: 0   ring_barrier tests: 5
    wait_for      library: 0   ring_barrier tests: 10
  -- what ring_wait costs, and who reaches it --
    std:: in ring_wait/src: 2   ring crates naming ring_wait: 2
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN1 | `ring_consume` | n/a — observation | The crate's whole arithmetic is one call to `ring_seqno::pending`, from a crate it names once |
| CN2 | family | n/a — coverage | `ring_consume` and `ring_publish` are the two Tier 5 primitives with zero library dependents, so their own suites carry all the weight |
| CN3 | `ring_mpsc` | n/a — drift | `ring_mpsc` documents dropping both halves of the handshake, and the doc describes a decision the code has since moved past |
| CN4 | `ring_barrier` | n/a — coverage | Eight of `Barrier`'s nine public methods have no library caller; seven have no caller outside their own crate's tests either |
| CN5 | `ring_wait` | n/a — doc gap | The chain is one crate longer than `ring_claim`'s, carrying `std` and a blocking loop, entirely for `wait_for` — which nothing calls |
