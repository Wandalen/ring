# integration

Both sides of this crate's position in the dependency graph — the four edges it
declares, and the ten crates that declare it.

### Overview Table

| ID | Name | Side | Count |
|----|------|------|------:|
| 001 | [Four Dependencies, All Used](001_four_dependencies_all_used.md) | Inbound — what this crate imports | 4 |
| 002 | [Who Reads a Cursor](002_who_reads_a_cursor.md) | Outbound — what imports this crate | 10 |

### Why Both

The crate sits at a genuine narrowing. `ring_align` has exactly one consumer in
the family — this one — so the padding decision reaches thirty-odd crates through
a single edge. That makes the outbound side worth documenting separately from the
inbound side: 001 explains why the bottleneck exists, 002 measures what it costs.

The measurement is the useful part. Two consumers needed the same cache-line
predicate; the one holding a `CursorPair` got it for free through a method, and
the one holding loose cursors wrote `>= 64` by hand. Neither could reach
`ring_align` directly, so the difference is entirely about whether a *type* was
available to carry the decision across.

That result generalises past this crate, and 002 states it as a rule.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this crate declares --'
command grep -E '^ring_' ring_cursor/Cargo.toml
echo '  -- and what declares it --'
command grep -l '^ring_cursor' */Cargo.toml | sed 's|ring/||;s|/Cargo.toml||' | tr '\n' ' '
echo
```

Live output:

```
  -- what this crate declares --
ring_types = { path = "../ring_types" }
ring_seqno = { path = "../ring_seqno" }
ring_atomic = { path = "../ring_atomic" }
ring_align = { path = "../ring_align" }
  -- and what declares it --
ring_barrier ring_claim ring_consume ring_debug ring_gating ring_mpsc ring_publish ring_shutdown ring_spsc ring_wait 
```

**Four in, ten out.** The `^` anchor is what keeps the second arm honest: this
crate's own manifest carries `ring_cursor` on its `name =` line, which an
unanchored pattern would count as a dependency on itself.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU17 | The four edges | n/a — observation | Every declared dependency is used on the `use` lines of `src/lib.rs`, and `ring_align` supplies exactly one attribute through one wrapper. That makes it the edge `cargo +nightly udeps` would report the instant the wrapper is replaced by a literal — the one dependency whose removal is also the invariant's violation |
| CU18 | The consumer census | **wrong doc** | The `SeqCell` route column marked eight of ten consumers "via re-export". In library code four take it through this crate, four take it directly from `ring_atomic` — and two of those four, `ring_batch` and `ring_tls`, are not consumers of this crate at all — while `ring_shutdown` imports the trait from neither. **Disposition: applied** — the route column and summary now state four of ten |
| CU19 | `ring_barrier`, `ring_gating`, `ring_wait` | n/a — observation | These three name `SeqCell` only inside doc examples. A doctest compiles as an external crate against the public surface, so the re-export genuinely earns its place there — but the claim it earns is about three crates' doctests, not their library code, and the census did not separate the two |
| CU20 | Citations into `ring_mpsc` | n/a — drift | This document cites the forked predicate by line number into `ring_mpsc`. An unrelated soundness fix in that crate moved it, and the same edit broke four of `ring_align`'s quoted recipes in one afternoon. A line-numbered citation into an actively-edited crate is a claim about a moving target |
