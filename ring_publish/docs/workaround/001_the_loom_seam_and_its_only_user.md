# Workaround: The `loom` Seam and Its Only User

### Scope

- **Purpose**: Record the `cfg(loom)` seam this crate's reached-test depends on, why it is a workaround rather than a design, and the two documents that still name this crate as its sole user.
- **Responsibility**: State what the seam replaces, show the whole-family census of who reads the cfg, check the two claims of exclusivity against it, and name what the seam costs.
- **In Scope**: `cfg(loom)` as it reaches `ring_publish` — one manifest block and one test file.
- **Out of Scope**: The four dev-dependencies the same test file needs — see [`workaround/002`](002_four_dev_dependencies_that_look_like_a_cycle.md).

### What the Seam Is

A memory model weaker than any real machine cannot be obtained from real atomics,
so `loom` supplies its own and the code under test must be made to use them. The
family does this in exactly one place — `ring_atomic/src/lib.rs:64-67`:

```rust
#[ cfg( loom ) ]
use loom::sync::atomic::{ AtomicU64, AtomicUsize };
#[ cfg( not( loom ) ) ]
use core::sync::atomic::{ AtomicU64, AtomicUsize };
```

Four lines, and `ring_atomic:46-52` states why they suffice:

> This is the family's only such switch, and it is here for the same reason
> the orderings are: this crate is the one place in 33 crates where a
> *sequence* atomic is created. Every cursor, gating set, claim and barrier
> reaches its atomic through [`AtomicSeq`], so the `loom` switch here reaches
> all of them — the counting instrument reaches only the callers generic
> enough to accept [`CountingSeq`] in its place — and no other crate needs to
> know the seam exists.

It is a **workaround** and not a design because nothing about the ring wants two
atomic implementations. The switch exists because loom's atomics cannot be
obtained any other way — a verification tool's requirement reaching into
production source. `ring_atomic` confines it to four lines so that the cost is
paid once, which is the best available outcome, not a good one.

### What It Costs, Stated at the Site

`ring_atomic:54-57`:

> loom's atomics carry model state and have no `const` constructor, so
> [`AtomicSeq::new`] and [`CountingSeq::new`] — and, downstream, `ring_cursor`'s
> two — are `const` only in an ordinary build.

That cost reaches this crate. `Publisher::new` is **not** `const`
(`src/lib.rs:99-103`) while `cursor()` and `is_published`'s siblings are, and the
reason is three crates away: a `const fn` chain that bottoms out in
`AtomicSeq::new` cannot survive the loom build
([`api/001`](../api/001_six_methods_and_no_caller.md)).

So the seam's price is visible in this crate's public API, in a way no reader of
this crate alone could explain.

### PB43 — Two Documents Name This Crate as the Seam's Only User, and Both Are Stale by Three Crates

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'cfg( *loom' ring_*/src/*.rs ring_*/tests/*.rs | sed 's|ring/||'
grep -rl 'loom' ring_*/Cargo.toml | sed 's|ring/||'
```

Live output:

```
ring_atomic/src/lib.rs:#[cfg(loom)]
ring_atomic/src/lib.rs:    #[cfg(loom)]
ring_atomic/src/lib.rs:    #[cfg(loom)]
ring_cursor/src/lib.rs:    #[cfg(loom)]
ring_cursor/src/lib.rs:    #[cfg(loom)]
ring_testkit/src/lib.rs://! and it is already a `cfg(loom)` dev-dependency of `ring_atomic`,
ring_mpsc/tests/mpsc_test.rs:#[cfg(loom)]
ring_publish/tests/handshake_test.rs:#[cfg(loom)]
ring_publish/tests/publish_test.rs:// `#![ cfg( loom ) ]` its `exhaustive` module carries. `--cfg loom` swaps
ring_spsc/tests/spsc_test.rs://! simply narrow enough that sampling it 100 000 times does not open it. The `#[ cfg( loom ) ] mod exhaustive` at the bottom
ring_spsc/tests/spsc_test.rs:#[cfg(loom)]
ring_testkit/tests/exhaustive_test.rs://! **The whole file is `cfg( loom )`.** Under `--cfg loom`, `ring_atomic`
ring_testkit/tests/exhaustive_test.rs:#![cfg(loom)]
ring_testkit/tests/testkit_test.rs:// The inverse of `exhaustive_test.rs`'s `#![ cfg( loom ) ]`. `--cfg loom` swaps
ring_atomic/Cargo.toml
ring_mpsc/Cargo.toml
ring_publish/Cargo.toml
ring_spsc/Cargo.toml
ring_testkit/Cargo.toml
```

The two claims:

| Where | Says |
|-------|------|
| `Cargo.toml:226-227` | *"Only ring_atomic, ring_cursor and ring_publish read the cfg — see ring_atomic's module documentation on the seam."* |
| `ring_atomic/src/lib.rs:59-60` | *"`ring_publish/tests/handshake_test.rs` is what uses it, and is run with `RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test`."* |

The census:

| Crate | Reads `cfg(loom)` in `src/` | Reads it in `tests/` | Declares `loom` |
|-------|:---------------------------:|:--------------------:|-----------------|
| `ring_atomic` | ✔ `:64,203,364` | — | `[dependencies]` — a **real** one |
| `ring_cursor` | ✔ `:165,274` | — | — |
| **`ring_publish`** | **—** | ✔ `:60,237` | `[dev-dependencies]` |
| `ring_mpsc` | — | ✔ `:900` | `[dev-dependencies]` |
| `ring_spsc` | — | ✔ `:924` | `[dev-dependencies]` |
| `ring_testkit` | — | ✔ `:30` — the **whole file** | `[dev-dependencies]` |

Six crates touch the seam, not three. Five declare `loom`, not two. And this
crate's `src/` reads the cfg **zero** times — the `ring_publish` in the workspace
comment's list is doing different work from the `ring_atomic` and `ring_cursor`
beside it, which read it in their libraries.

Both documents were true when written. `handshake_test.rs` was the first loom
model in the family — it is the four-operation handshake's reached-test, and the seam was built
for it — and `ring_mpsc`, `ring_spsc` and `ring_testkit` each added one later,
each correctly, each under `[target.'cfg(loom)'.dev-dependencies]`, and none
updated the two sentences that said they would be the only ones.

Neither is a defect that breaks anything:

| Claim | Consequence of it being stale |
|-------|-------------------------------|
| `Cargo.toml:226` | none — `check-cfg` is workspace-wide and covers all six regardless of the comment |
| `ring_atomic:59` | a reader looking for loom models finds one of four |

The second is the one worth fixing, because it is the seam's own documentation
and it is where a reader would go. It is `ring_atomic`'s to fix, not this
crate's.

**Disposition:** declined — the stale sole-user sentence is `ring_atomic`'s own
module doc at `ring_atomic/src/lib.rs:59-60`; the neighboring stale-count
claim in the same paragraph's family (the root manifest's "Only ring_atomic,
ring_cursor and ring_publish" comment) is what `ring_atomic`'s own AT50 already
declined from the manifest side, and this is the same pattern from the doc side
— `ring_atomic`'s to fix, not this crate's.

### PB44 — This Crate's Own Check Verifies Two Manifests Out of Five

`tests/manual/readme.md § P6` guards the seam from this side:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E "^#\[ cfg\( (not\( )?loom" ring_publish/tests/handshake_test.rs
command grep -E "^\[target|^loom" ring_publish/Cargo.toml ring_atomic/Cargo.toml
```

Live output:

```
#[ cfg( loom ) ]
#[ cfg( not( loom ) ) ]
ring_publish/Cargo.toml:[target.'cfg(loom)'.dev-dependencies]
ring_publish/Cargo.toml:loom = "0.7"
ring_atomic/Cargo.toml:[target.'cfg(loom)'.dependencies]
ring_atomic/Cargo.toml:loom = "0.7"
```

Its expectation is exact and correct: two `cfg` gates on the test file, and
`loom` declared only under `[target.'cfg(loom)']` in both manifests — a
dev-dependency here, a real one in `ring_atomic`. The rationale is worth quoting
because it names the failure precisely:

> An ordinary `cargo build` must never resolve loom at all; if it appears under
> a plain `[dependencies]`, the seam has leaked into the shipped crate.

Two things about its scope, and they pull in opposite directions:

1. **It checks two manifests, and five declare `loom`.** A leak in `ring_mpsc`,
   `ring_spsc` or `ring_testkit` would pass P6 untouched. That is correct scope
   for a check living in `ring_publish` — a crate's manual checks should not
   police three crates it does not depend on — but it does mean the property
   *"loom never reaches a shipped build"* is not established family-wide by
   anything.
2. **It is the only check of its kind anywhere.** Widening it would be the wrong
   fix; the right one is the same check in each of the other three crates'
   manual plans, which is their work and not this crate's.

The Run Record at `tests/manual/readme.md:172` shows P1–P6 last verified
2026-08-28, 6/6 as expected.

### Why Both Harnesses, and Not Just the Model

The seam buys the exhaustive half of a two-harness file, and
`tests/handshake_test.rs:37-44` argues neither half is redundant:

> Real threads run the true code on the true hardware, at sizes loom could never
> enumerate (thousands of items, several producers) — but they only ever sample
> the interleavings the scheduler happens to pick, and on x86 the hardware
> supplies orderings the code failed to ask for. `loom` runs a deliberately tiny
> case — one claim, one drain — and checks *every* interleaving of it against a
> memory model weaker than any real machine, so it catches the missing `Release`
> that x86 would hide. Neither subsumes the other: one has scale without
> coverage, the other coverage without scale.

The two are mutually exclusive by construction — `#[ cfg( loom ) ] mod
exhaustive` at `:60`, `#[ cfg( not( loom ) ) ] mod threaded` at `:237` — because
a build with both live would run loom's model on real threads, which is P6's
stated hazard.

The consequence for CI is that **the loom half does not run by default**. An
ordinary `cargo test -p ring_publish` compiles `mod threaded` and skips
`mod exhaustive` entirely, so the criterion's *"asserted over every interleaving"*
is satisfied only when someone types the `RUSTFLAGS` line. Nothing in the
repository types it automatically.

### What the Seam Actually Establishes Here

| Under | Runs | Establishes |
|-------|------|-------------|
| `--cfg loom` | `mod exhaustive`, 2 tests | every interleaving of one claim and one drain, capacity 2 |
| ordinary | `mod threaded`, 8 tests | scale — 20 000 items, three producers — sampled, not exhaustive |

Two tests against eight, and the two are the ones the acceptance criterion names.
`tests/manual/readme.md § P1` is what makes them evidence: it mutates the source
so publication happens at claim time and records that the model catches it
(`left: 0`), and separately records that mutation 1 **passes all eight threaded
tests un-`loom`ed** — the two harnesses' asymmetry, measured rather than argued.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The `const` this crate cannot have, and why |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_from_claim_to_visibility.md](../lifecycle/001_a_slot_from_claim_to_visibility.md) | The instrument the loom model builds inside the seam |
| [../lifecycle/002_the_four_operation_handshake.md](../lifecycle/002_the_four_operation_handshake.md) | How much of the criterion the two tests actually cover |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_named_ordering_constant.md](../pattern/002_the_named_ordering_constant.md) | The pairing only the model can check |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | The bug the seam exists to catch |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_two_derives_and_the_ones_that_are_absent.md](../type/002_two_derives_and_the_ones_that_are_absent.md) | The two `AtomicU64`s the seam swaps between |

### Workarounds

| File | Relationship |
|------|--------------|
| [002_four_dev_dependencies_that_look_like_a_cycle.md](002_four_dev_dependencies_that_look_like_a_cycle.md) | The other thing the same test file costs the manifest |

### Sources

| File | Relationship |
|------|--------------|
| `ring_atomic/src/lib.rs:43-66` | The seam, its rationale, its cost, and the stale sole-user sentence |
| `ring_atomic/src/lib.rs:64-67` | The four lines the whole family's instrumentation rests on |
| `ring_publish/Cargo.toml:21-22` | `loom` under `[target.'cfg(loom)'.dev-dependencies]`, never a plain one |
| `Cargo.toml:614-619` | `check-cfg` declared once workspace-wide, and the stale three-crate list |
| `ring_cursor/src/lib.rs:166,281` | The other library that reads the cfg, for the `const` split |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs:32-50` | Why both harnesses, and the command that runs the second |
| `tests/handshake_test.rs:60,237` | The two gates that make them mutually exclusive |
| `tests/manual/readme.md § P6` | Two manifests checked, five declare `loom` |
| `tests/manual/readme.md § P1` | The mutation that passes eight threaded tests and fails the model |
