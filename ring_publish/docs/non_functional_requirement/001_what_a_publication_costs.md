# Non-Functional Requirement: What a Publication Costs

### Scope

- **Purpose**: State the crate's resource cost exactly — bytes, atomic operations, allocations — and record that none of it has ever been measured, or can be by the apparatus the family built.
- **Responsibility**: Give the per-operation cost from the code, the per-instance cost from the type, show the family's benchmark crate does not reach this crate, and name the properties that are true in fact but unclaimed in code.
- **In Scope**: Static, countable cost of `Publisher` and its six methods.
- **Out of Scope**: The spin's cost, which is unbounded and belongs to the caller — see [`non_functional_requirement/002`](002_what_the_spin_costs.md).

### Per Instance

| Property | Value | Established by |
|----------|-------|----------------|
| `size_of::< Publisher >()` | **64 bytes** | one `PaddedCursor` field, nothing else |
| `align_of::< Publisher >()` | **64 bytes** | inherited from `PaddedCursor`'s `repr( align )` |
| Live state | **8 bytes** — one `Seq`, which is `pub struct Seq( pub u64 )` | `ring_types/src/id.rs:25` |
| Padding | 56 bytes, 87.5% | the difference |
| Heap allocations | **zero**, ever | no `Vec`, `Box`, `String` or `alloc` in `src/lib.rs` |
| Construction cost | one 64-byte zeroed write | `Self::default()` |

`ring_cursor/tests/cursor_test.rs:50-60` asserts the 64/64 figures for
`PaddedCursor` directly; `Publisher` is a single-field wrapper around it, so both
carry through.

The 56 bytes of padding are the point, not waste —
[`data_structure/001`](../data_structure/001_one_padded_cursor_and_nothing_else.md)
§ PB13 records the arithmetic and the case where the family judged it *not* worth
paying (`ring_mpsc`'s per-slot stamps, where 64 bytes × capacity would dominate).

### Per Operation

| Method | Atomic ops | Ordering | Non-atomic work | Can retry |
|--------|-----------:|----------|-----------------|:---------:|
| `new` / `default` | 0 | — | one zeroed write | — |
| `cursor` | **0** | — | a field offset — `const`, compiles away | — |
| `published` | 1 load | `Acquire` | — | — |
| `is_published` | 1 load | `Acquire` | one `u64` comparison | — |
| `try_publish` | 1 `compare_exchange` | `Release` success / `Acquire` failure | one `u64` add | — |
| `publish` | *k* `compare_exchange` | same | one add per attempt, *k−1* `spin_loop` hints | **yes, unbounded** |

Everything except `publish` is O(1) with a constant of one. `publish` is the only
row whose cost is not a number, and
[`non_functional_requirement/002`](002_what_the_spin_costs.md) is about that row
alone.

`is_published` costing the same as `published` is worth stating: the comparison is
free relative to the load, so the derived method is not more expensive than the
one it derives from. A caller asking about *n* sequences pays *n* `Acquire` loads
regardless of which it calls — the batching answer is
`ring_consume::Consumer::available`, which reads the frontier once and returns a
whole range
([`item/001`](../item/001_the_three_readings_of_the_cursor.md)).

`PUBLISH` being `Release` rather than `SeqCst` is the one ordering choice with a
cost consequence, and `src/lib.rs:60-67` states where it is free and where it is
not:

> On x86 and aarch64 this is free in the store itself; the annotation is what
> makes it correct everywhere else.

So on the two architectures anyone runs this on, the crate's entire ordering
apparatus costs nothing at runtime and everything at review time.

### PB29 — The Family Built a Benchmark Crate With Five Candidates, and This Crate Is In None of Them

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A24 -F 'pub enum Candidate' ring_bench/src/lib.rs
command grep 'path = ' ring_bench/Cargo.toml
# no crate in the family has one, so this prints nothing and exits nonzero —
# the empty result is the evidence, so it must not end the block
ls -d ring_*/benches 2>/dev/null || true
```

Live output:

```
pub enum Candidate
{
  /// `Mutex< VecDeque< Record > >` — the baseline every other candidate has to
  /// beat to justify its existence. Bounded by the workload's capacity so that
  /// it competes under the same back-pressure as the rings rather than
  /// absorbing the whole load.
  MutexQueue,
  /// The in-house ring reached the way the workstream Contract says to reach
  /// it: `ring_factory::build`, returning a `ring_handle::Split`.
  ContractRing,
  /// Thread-local staging over the in-house ring — `ring_tls::TlsBuffer`
  /// accumulating, `ring_flush::Flusher` publishing on a batch policy.
  TlsOverRing,
  /// `ring_spsc` driven directly, two levels below the Contract. Prices the
  /// dispatch `ring_core` and `ring_handle` add: same ring, same single
  /// producer, no wrapper.
  DirectSpsc,
  /// `ring_mpsc` driven directly. The only in-house candidate with no producer
  /// ceiling, and the reason the comparison can be run at more than one
  /// producer at all.
  DirectMpsc,
  /// The off-the-shelf concurrent queue, through `ring_factory`'s second door.
  #[ cfg( feature = "crossbeam" ) ]
  OffTheShelf,
}
ring_factory = { path = "../ring_factory" }
ring_tls = { path = "../ring_tls" }
ring_flush = { path = "../ring_flush" }
ring_stats = { path = "../ring_stats" }
ring_spsc = { path = "../ring_spsc" }
ring_mpsc = { path = "../ring_mpsc" }
ring_core = { path = "../ring_core" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }
```

`ring_bench::Candidate` has five variants by default and six with the
`crossbeam` feature:

| Candidate | What it runs | Reaches `ring_publish` |
|-----------|--------------|:----------------------:|
| `MutexQueue` | the baseline the ring is measured against | no |
| `ContractRing` | `ring_factory::build` → `ring_handle::Split` | no |
| `TlsOverRing` | `ring_tls::TlsBuffer` + `ring_flush::Flusher` | no |
| `DirectSpsc` | `ring_spsc` driven directly | no |
| `DirectMpsc` | `ring_mpsc` driven directly | no |
| `OffTheShelf` *(feature-gated)* | crossbeam, through `ring_factory` | no |

`ring_bench/Cargo.toml` declares nine path dependencies — `ring_factory`,
`ring_tls`, `ring_flush`, `ring_stats`, `ring_spsc`, `ring_mpsc`, `ring_core`,
`ring_slot`, `ring_types` — and `ring_publish` is not among them. The third
command returns nothing: **there is no `benches/` directory anywhere in the
33-crate family.**

This is not an omission that a later benchmark run would fix. The two candidates
that publish anything — `DirectSpsc` and `DirectMpsc` — publish through *stamps*,
which is the mechanism built specifically instead of this crate
([`decisions/001`](../decisions/001_refused_rather_than_reordered.md)), and
`ContractRing` reaches them through `ring_factory` → `ring_core`, which declares
neither `ring_publish` nor `ring_consume`
([`integration/002`](../integration/002_the_two_crates_that_declined.md) § PB5).
Every path through the measurement apparatus routes around this crate, because
every path through the *family* does.

So every figure in the tables above is a count read off the source, and none is
a measurement. That is the honest status, and it is stable: no benchmark can
reach this code until some candidate uses it.

### PB30 — Core-Only In Fact, Unclaimed In Code, Family-Wide

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '\bstd::' ring_publish/src/lib.rs
grep -ohE 'core::[a-z_:]+' ring_publish/src/lib.rs | sort -u
grep -rl 'no_std' ring_*/src/*.rs
for f in ring_*/src/lib.rs; do grep -qE '\bstd::' "$f" || echo "$f"; done
```

Live output:

```
core::hint::spin_loop
core::sync::atomic::
ring_overflow/src/lib.rs
ring_stats/src/lib.rs
ring_types/src/lib.rs
ring_align/src/lib.rs
ring_atomic/src/lib.rs
ring_barrier/src/lib.rs
ring_batch/src/lib.rs
ring_store/src/lib.rs
ring_claim/src/lib.rs
ring_config/src/lib.rs
ring_consume/src/lib.rs
ring_core/src/lib.rs
ring_cursor/src/lib.rs
ring_debug/src/lib.rs
ring_event/src/lib.rs
ring_factory/src/lib.rs
ring_flush/src/lib.rs
ring_gating/src/lib.rs
ring_handle/src/lib.rs
ring_index/src/lib.rs
ring_poll/src/lib.rs
ring_publish/src/lib.rs
ring_seqno/src/lib.rs
ring_shutdown/src/lib.rs
ring_slot/src/lib.rs
ring_stats/src/lib.rs
```

`src/lib.rs` names exactly two `core::` paths and no `std::` path at all:

| Path | Used for |
|------|----------|
| `core::sync::atomic::Ordering` | the `PUBLISH` constant's type |
| `core::hint::spin_loop` | the one line inside `publish`'s loop |

And **25 of the 33 crates name no `std::` path in their `lib.rs`, while three
declare `#![no_std]`.** The attribute reaches `ring_types`, `ring_stats` and
`ring_overflow` and stops there; `ring_publish` is not among them, though
`ring_types` is in its dependency closure.

Two cautions on reading this, both of which matter:

1. **Absence of a `std::` path is not `no_std` compatibility.** `Vec`, `String`
   and `Box` reach code through the prelude with no `std::` prefix, so this grep
   cannot distinguish a genuinely core-only crate from one using the prelude. For
   *this* crate the stronger check passes too — no `Vec`, `Box`, `String` or
   `alloc` token appears in `src/lib.rs` — but that says nothing about the other
   24.
2. **The dependencies decide it, not this crate.** `ring_publish` depends on
   `ring_types` and `ring_cursor`; both are in the no-`std::`-path list, and
   `ring_types` declares the attribute at `ring_types/src/lib.rs:27` while
   `ring_cursor` does not — so the chain is broken at `ring_cursor` and nothing
   is guaranteed transitively.

   **Correction (2026-09-20):** this bullet read "neither declares the attribute
   either" until the census above was retargeted at `ring/` and re-run, which is
   what surfaced the three declaring crates the paragraph above now names. The
   conclusion is unchanged — one undeclared link is enough to break the chain —
   but the reason is now one crate, not two.

The property is settled by compiling, not by grepping:

```sh
cd "$(git rev-parse --show-toplevel)"
# cargo's status lines carry a build-directory lock notice and per-crate
# progress, both of which depend on what else happens to be compiling — the
# diagnostic underneath them is the answer
cargo build -p ring_publish --target thumbv7em-none-eabi 2>&1 \
  | grep -vE '^ *(Blocking|Compiling|Building|Updating|Locking|Finished)'
```

Live output:

```
error[E0463]: can't find crate for `core`
  |
  = note: the `thumbv7em-none-eabi` target may not be installed
  = help: consider downloading the target with `rustup target add thumbv7em-none-eabi`

For more information about this error, try `rustc --explain E0463`.
error: could not compile `ring_types` (lib) due to 1 previous error
```

which has never been run against this crate — no CI job, no manual check, and no
entry in `tests/manual/readme.md` names a bare-metal target. Without the
attribute the crate links `std` regardless of whether it uses it, so the build
would need `#![no_std]` added before it could even be a meaningful test; what the
command settles is whether adding it would then compile. Recording this as unclaimed
rather than as a defect is deliberate: no consumer has asked for a no-`std`
build, and adding `#![no_std]` to one Tier 5 crate whose dependencies do not
declare it would be a claim the build could not honour.

### What Is Not Costed Here

| Cost | Whose | Where |
|------|-------|-------|
| The slot write between claim and publish | the caller's | [`lifecycle/001`](../lifecycle/001_a_slot_from_claim_to_visibility.md)'s 2 → 3 window |
| Waiting for a predecessor | the caller's, unbounded | [`non_functional_requirement/002`](002_what_the_spin_costs.md) |
| The consumer's barrier read | `ring_barrier`'s | its own docs |
| Cache-line traffic between producers | the hardware's | unmeasured; the 64-byte padding is the mitigation |

The last row is the one a benchmark would speak to and nothing does. Several
producers hammering one `compare_exchange` on one cache line is the canonical
contention pattern, and the padding prevents *false* sharing with neighbouring
data — not *true* sharing between producers competing for the same cursor, which
is inherent to the design and is exactly what `ring_mpsc`'s stamps eliminate.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The six methods costed above |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | Where the 64 bytes and the 87.5% come from |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The alternative whose cost profile is the opposite of this one |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | Why no benchmark candidate routes through here |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_readings_of_the_cursor.md](../item/001_the_three_readings_of_the_cursor.md) | Zero, one, one — the read side of the table above |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_what_the_spin_costs.md](002_what_the_spin_costs.md) | The one row above that is not a number |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_named_ordering_constant.md](../pattern/002_the_named_ordering_constant.md) | Why `Release` and not `SeqCst`, and what that saves |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:57-67,84-88` | The two `core::` paths, the ordering constant, and the single field |
| `ring_bench/src/lib.rs:418-442` | The five candidates, none of them this crate |
| `ring_bench/Cargo.toml` | Nine path dependencies, and the argument for each of the last three |
| `ring_types/src/id.rs:25` | `Seq( pub u64 )` — the eight live bytes |
| `ring_cursor/docs/invariant/001_one_cursor_one_line.md` | The 64/64 invariant this crate inherits |

### Tests

| File | Relationship |
|------|--------------|
| `ring_cursor/tests/cursor_test.rs:50-60` | `align_of` and `size_of` asserted at 64 |
| `tests/publish_test.rs:36-43` | Construction, through the only observable it produces |
| `tests/handshake_test.rs:497-561` | The largest run in the repository — three producers × 3 000, timed by nothing |
