# Pitfall: The Amortised Flush Has No Ring

### Scope

- **Purpose**: Record that `ring_tls`'s batch-claiming flush cannot reach a `ring_core::Ring`, so the fixture's `Flush` step pushes one record at a time.
- **Responsibility**: The type mismatch, what it costs, and why it is not worked around here.
- **In Scope**: `Step::Flush`, `TlsBuffer::{ drain, flush_into }`.
- **Out of Scope**: Whether `ring_core` should expose a cursor — [`../decisions/readme.md`](../decisions/readme.md) Pending 3.

### The mismatch

`ring_tls::TlsBuffer::flush_into` is the amortised path, and its own docs name
the feature it exists for:

> One `fetch_add` regardless of how many items are staged — the amortisation
> feature 175 is about.

Its signature is:

```rust
pub fn flush_into< C >( &mut self, cursor : &C, order : Ordering ) -> Flush< '_, T >
where C : SeqCell
```

It claims a contiguous sequence run **against a `SeqCell`** — an
`ring_atomic::AtomicSeq` or equivalent. And a `ring_core::Ring` exposes no
cursor on its own public surface: its ends offer `try_push`, `try_push_batch`,
`try_clone`, `free_capacity`, `is_full`, `try_recv`, `try_recv_batch`, `len`,
`is_empty` — none of type `SeqCell`. A cursor exists three hops down its own
dependency tree (`ring_core` → `ring_spsc` → `ring_cursor` → `ring_atomic`,
whose `PaddedCursor` is what the SPSC backend actually uses) but is not
re-exported, so there is nothing of that type this crate can reach to hand it.

**So the two crates have exactly one join, and it is the slow one:**
`TlsBuffer::drain()` yields the staged items, and each goes into the ring
through an ordinary `try_push`.

### What the fixture does

```rust
let staged : Vec< u32 > = staging.drain().collect();
for record in staged
{
  match guard.try_push( record ) { .. }
}
```

| Choice | Why |
|---|---|
| `drain()` and not `flush_into` | `flush_into` needs a cursor the ring does not have |
| `try_push` per record and not `try_push_batch` | `try_push_batch` consumes the record it fails to place and reports only a count, so a partial flush would lose one record *silently* — the exact failure this crate exists to detect. Per-record refusal hands the record back and lets it be counted |
| Collect before the loop | `drain()` borrows the buffer for the iterator's life, and the guard's push cannot run inside that borrow |

### What it costs

One `fetch_add` per record instead of one per batch — the amortisation
`flush_into` was built for, given up. **In a fixture that is the right trade**: the fixture
measures *what happened to the records*, and a batched claim would make a
partial flush report a count rather than a record-by-record outcome. The
per-record path is slower and says more.

It is still a real gap in the family, not a preference of this crate. A
consumer that stages into a `TlsBuffer` and publishes into a `ring_core::Ring`
pays the same cost, with no fixture's excuse.

### Consequences

| # | Consequence | Where it lands |
|---|---|---|
| S1 | `Flush` empties the buffer whether or not the ring takes the records | Mirrors `flush_into`'s own contract, so the two paths agree on at least this |
| S2 | Refusals during a flush are counted, never returned to the buffer | `staged_at_end` is 0 after any `Flush` |
| S3 | `ring_tls` gains a caller it did not have | Before this crate, `flush_into` had no caller anywhere in the family — and it still does not |

**S3 is the uncomfortable one.** `ring_tls`'s headline operation remains
uncalled by anything in the family. This crate got as close as the types allow
and still could not use it.

### Evidence

| # | Claim | Test |
|---|---|---|
| T1 | A flush empties the buffer even when the ring refuses | `a_full_staging_buffer_refuses_before_the_ring_is_reached` |
| T2 | Staging refusals and ring refusals are counted separately | same test |
| T3 | A flush into a closed ring is refused as closed, not as full | `a_flush_into_a_closed_ring_is_refused_as_closed` |
| T4 | Staging without flushing leaves the records staged and counted | `records_staged_and_never_flushed_are_still_accounted_for` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'flush_into calls outside ring_tls: %s\n' "$( cat ring_bench/src/*.rs ring_bench/tests/*.rs ring_flush/src/*.rs ring_flush/tests/*.rs ring_testkit/src/*.rs ring_testkit/tests/*.rs 2>/dev/null | command grep -c '\.flush_into(' || true )"
printf 'flush_into calls inside ring_tls:  %s\n' "$( cat ring_tls/src/*.rs ring_tls/tests/*.rs | command grep -c '\.flush_into(' || true )"
printf 'crates depending on ring_tls:      %s\n' "$( command grep -l 'ring_tls *=' ring_*/Cargo.toml | sed 's|/Cargo.toml||' | tr '\n' ' ' )"
printf 'of those, also on ring_core:       %s\n' "$( for m in $( command grep -l 'ring_tls *=' ring_*/Cargo.toml ) ; do command grep -q 'ring_core *=' "$m" && echo x ; done | wc -l )"
# Note: the two commands above are scoped to ring_*/Cargo.toml only. Workspace-wide
# (module + ring + spike), ring_tls has a fourth dependent outside this glob:
# /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path, which also depends on ring_core.
printf 'the bound flush_into needs:        %s\n' "$( command grep -rn 'pub trait SeqCell' ring_*/src/lib.rs )"
printf 'impls of it in the family:         %s\n' "$( command grep -rhc 'impl SeqCell for' ring_*/src/lib.rs | paste -sd+ | bc )"
printf 'ring_core dependencies:            %s\n' "$( command grep -oE '^ring_[a-z_]+' ring_core/Cargo.toml | sort -u | tr '\n' ' ' )"
printf 'ring_spsc dependencies:            %s\n' "$( command grep -oE '^ring_[a-z_]+' ring_spsc/Cargo.toml | sort -u | tr '\n' ' ' )"
printf 'ring_cursor dependencies:          %s\n' "$( command grep -oE '^ring_[a-z_]+' ring_cursor/Cargo.toml | sort -u | tr '\n' ' ' )"
printf 'public fns on ring_core:           %s\n' "$( command grep -cE '^  pub (const )?fn ' ring_core/src/lib.rs )"
printf 'of them exposing a cursor:         %s\n' "$( command grep -E '^  pub (const )?fn ' ring_core/src/lib.rs | command grep -cE 'SeqCell|[Cc]ursor|Seq' || true )"
```

Live output:

```
flush_into calls outside ring_tls: 0
flush_into calls inside ring_tls:  16
crates depending on ring_tls:      ring_bench ring_flush ring_testkit 
of those, also on ring_core:       3
the bound flush_into needs:        ring_atomic/src/lib.rs:108:pub trait SeqCell : Sync
impls of it in the family:         3
ring_core dependencies:            ring_config ring_mpsc ring_overflow ring_slot ring_spsc ring_types 
ring_spsc dependencies:            ring_store ring_config ring_cursor ring_slot ring_types 
ring_cursor dependencies:          ring_align ring_atomic ring_seqno ring_types 
public fns on ring_core:           16
of them exposing a cursor:         0
```

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_three_edges_and_the_one_that_is_missing.md](../integration/001_the_three_edges_and_the_one_that_is_missing.md) | The `ring_tls` edge, and how much of it is reachable |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/src/lib.rs`](../../../ring_tls/src/lib.rs) | `flush_into` and its `SeqCell` bound |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | The ends, and the absence of a cursor among them |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | T1–T4 |

### TK45 — the amortised path is exercised only by the crate that defines it

S3 says `flush_into` had no caller before this crate and still has none. Measured
across the family, the count of `.flush_into(` call sites outside `ring_tls` is
**zero**; inside `ring_tls` it is **fifteen**, split between its own doc examples
and its own test suite.

The population that could adopt it is small and fully accounted for.
`ring_tls` has exactly four dependents workspace-wide — `ring_bench`, `ring_flush`,
`ring_testkit`, and `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path` — and **all four** also depend on
`ring_core`. Every crate in the family that holds a `TlsBuffer` also holds a `Ring`,
and not one of them has found a way to put the two together through the operation
that exists for precisely that join.

So the "gap in the family" this document names is not one crate's inconvenience
sampled once. It is three out of three, each arriving at `drain()` independently,
against an operation whose own crate documents as the amortisation `flush_into`
provides. What this file adds to that is the reason — a `SeqCell` bound with no
satisfying value on the ring side — which is the same reason for all three and is
written down in only one of them.

### TK46 — "exposes no cursor at all" is true of the surface and false of the structure

The mismatch section's argument turns on a scarcity claim: `flush_into` wants a
`C : SeqCell`, and *"a `ring_core::Ring` exposes no cursor at all"*. The
enumeration that follows is complete and correct — `ring_core` has sixteen public
functions and **zero** of them name a cursor or a `SeqCell`.

The claim is about the surface. Underneath it, the trait is not scarce and it is
not distant. `SeqCell` is declared at `ring_atomic/src/lib.rs:108` and has
**three** implementors in the family. And `ring_atomic` is inside `ring_core`'s
own dependency tree, three hops down: `ring_core` → `ring_spsc` → `ring_cursor`
→ `ring_atomic`. One of the three implementors — `ring_cursor::PaddedCursor` — is
the cursor type the SPSC backend `ring_core` is built on actually uses.

So the correct statement is not that the ring has no cursor. It is that the ring
*has* a cursor, two crates inside itself, and deliberately does not expose it —
which is a different fact with different consequences. The scarcity reading makes
the join look impossible and the fix look like new construction. The
encapsulation reading makes it an access-control question, which is what Pending
3 is really asking and what a reader of this file currently has no way to see.

The document is right about what it measured and reaches a conclusion its
measurement does not support. Sixteen public functions expose nothing of that
type; that establishes that the join is unavailable, not that the material for it
is absent.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -m1 -F 'nothing of that type this crate can reach to hand it' docs/pitfall/003_the_amortised_flush_has_no_ring.md
```

Live output:

```
re-exported, so there is nothing of that type this crate can reach to hand it.
```

**Disposition:** applied — the mismatch section now says the ring exposes no
cursor on its own public surface rather than "at all", and names where a
cursor does exist (`ring_cursor::PaddedCursor`, three hops down `ring_core`'s
own dependency tree) but is not re-exported — reframing the gap as
encapsulation rather than scarcity, matching what Pending 3 is actually asking.
Now prints: `nothing of that type this crate can reach to hand it`
