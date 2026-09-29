# Non Functional Requirement: The Crate Costs Nothing at Runtime

### Scope

- **Purpose**: State the requirement that a crate sitting on the family's hottest path add no instructions, and account honestly for the cost it does impose — which is memory, is large in proportion, and is paid whether or not it buys anything.
- **Responsibility**: State the requirement, give the evidence, quantify the memory cost with real numbers, and name where the cost is paid for nothing.
- **In Scope**: Instruction cost, allocation, branching, `const`-evaluability, memory overhead, build cost.
- **Out of Scope**: Whether the padding improves throughput, which is [`non_functional_requirement/001`](001_the_structural_claim_is_testable.md) N2; the layout itself, which is [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md).

### The Requirement

**Nothing this crate contributes may cost an instruction on the write path.**

The crate exists to make a hot path faster. A wrapper that added a bounds
check, a branch, or an indirection to every cursor read would be spending on
the same path it is trying to protect, and the accounting would be
unmeasurable against the false-sharing effect it removes.

### The Evidence

| Property | Holds | Why |
|----------|:-----:|-----|
| No allocation | yes | Nothing in the crate calls into `alloc`; `CacheAligned` is `Sized` and owns its payload by value |
| No branch on the wrapper's operations | yes | `new`, `get`, `get_mut`, and `into_inner` are each a single expression: `Self( value )`, `&self.0`, `&mut self.0`, `self.0` |
| No indirection | yes | The payload is at offset 0 and `get` returns a borrow of it — there is no pointer to follow that would not have been followed anyway |
| No `unsafe` | yes | `#[ repr( align( 64 ) ) ]` is a safe attribute; verified by `tests/manual/readme.md` M2, recorded ✅ on 2026-08-28 |
| Every operation `const` | **all but one** | `new`, `get`, `get_mut`, and `on_distinct_lines` are `const fn`. `into_inner` cannot be (`E0493`, → [`type/002`](../type/002_cache_aligned.md)) |
| No monomorphisation blowup | yes | One generic parameter, no trait bounds, no methods that generate code beyond a field access |

**The `const` property is checkable rather than assertable**, and it is
stronger than "cheap" — a `const fn` that a caller invokes on literals is
evaluated at compile time and costs nothing at all. Verified on
`rustc 1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cd "$(git rev-parse --show-toplevel)"
cat > ./-probe.rs <<'EOF'
pub const CACHE_LINE : usize = 64;
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
{
  a / CACHE_LINE != b / CACHE_LINE
}
const SEPARATED : bool = on_distinct_lines( 63, 64 );
fn main() { const _ : () = assert!( SEPARATED ); }
EOF
rustc --crate-name probe --edition 2021 --emit=metadata -o ./-probe.rmeta ./-probe.rs 2>&1 \
  && echo '    const context: accepted'
echo '  -- control: the same assertion over two addresses the predicate calls equal --'
sed -i 's/63, 64/64, 65/' ./-probe.rs
rustc --crate-name probe --edition 2021 --emit=metadata -o ./-probe.rmeta ./-probe.rs 2>&1 \
  | command grep -oE 'evaluation of .* failed' | head -1 | sed 's:^:    :'
rm -f ./-probe.rs ./-probe.rmeta
```

Live output:

```
    const context: accepted
  -- control: the same assertion over two addresses the predicate calls equal --
    evaluation of `main::_` failed
```

The control is what makes the first line worth printing. An accepted compile
proves the `const` context was entered only if a *rejected* one is available to
compare against: `on_distinct_lines( 64, 65 )` is false, and the compiler
refuses the assertion at evaluation time rather than at run time. Both halves
must print for either to mean anything.

The assertion is discharged by the compiler; no code is emitted for it.

**The single division that remains** is `a / CACHE_LINE`, with `CACHE_LINE` a
power of two, which the compiler lowers to a shift. It runs only where a caller
asks the question, which in this family is inside test assertions rather than
on the write path
(→ [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md)).

### The Cost That Is Real

Memory, and it is not small in proportion:

| Type | Payload bytes | Total bytes | Overhead | Ratio |
|------|--------------:|------------:|---------:|------:|
| `CacheAligned< u8 >` | 1 | 64 | 63 | **64×** |
| `CacheAligned< u64 >` | 8 | 64 | 56 | **8×** |
| `PaddedCursor` (`CacheAligned< AtomicSeq >`) | 8 | 64 | 56 | **8×** |
| `CursorPair` (two cursors + a `Capacity`) | 24 | 192 | 168 | **8×** |

`CursorPair`'s 192 follows from its declaration
(`ring_cursor/src/lib.rs:247-252`): two 64-byte cursors plus an 8-byte
`Capacity`, rounded up to the type's 64-byte alignment. `ring_cursor`'s own
test asserts the bound `[ 2 × CACHE_LINE, 3 × CACHE_LINE ]` rather than the
exact number, so 192 is derived from the fields rather than pinned by an
assertion.

**Whether 8× is acceptable is a question about counts, not ratios.** One ring
holds one `CursorPair`, so the family's total padding overhead is 168 bytes per
ring. A design that padded per *slot* rather than per *cursor* would multiply
that by capacity, which is exactly the trade `ring_mpsc` declines in its own
gating array — its comment records the alternative as costing "64 times" more
(`ring_mpsc/src/lib.rs:331`).

### Where the Cost Buys Nothing

The overhead is a property of the type, so it is paid in every state a wrapped
value can be in — including the state where there is no neighbour to be
separated from (→ [`lifecycle/001`](../lifecycle/001_the_wrapped_values_arc.md) S2).
A `CacheAligned< u64 >` in a local variable, in a `Vec`, or as the only field
of a struct costs 56 bytes and delivers nothing, because false sharing requires
two contending values and there is one.

**Nothing detects this.** There is no lint, no test, and no plausible mechanism
for one — whether a wrapped value has a hot neighbour is a property of the
consumer's whole program. The mitigation is that the family currently wraps
exactly one thing, `AtomicSeq` inside `PaddedCursor`, and always in pairs.

### The Build Cost

One dependency edge that is declared and unused
(→ [`integration/001`](../integration/001_one_dependency_one_consumer.md)):
`ring_align → ring_types`, referenced by no `.rs` file in the crate. It costs
build ordering and nothing at runtime, which is why it is a finding rather than
a violation of this requirement.

### AL29 — Seven-Eighths of Every Wrapped Word Is Padding

```
    u64 payload 8, wrapper 64, padding 56 bytes per wrapped word
```

For the payload the family actually wraps — a cursor-sized word — 56 of every 64
bytes exist to be empty.

**Finding.** The cost is unconditional, and it buys the guarantee only when two
conditions hold that the type cannot check: that a second wrapped field sits
beside it, and that the host's line is really 64. On a 128-byte-line machine the
same 56 bytes are spent and separate nothing
(→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)); in a
struct with one wrapped field they are spent and separate nothing either. The
"costs nothing at runtime" claim above is about instructions, and this is the
claim it does not make.

**Disposition:** declined — the 56/64-byte (87.5%) overhead per wrapped word
is the accepted cost of the layout in
`data_structure/001_the_cache_aligned_wrapper.md`; the alternative of padding
per-slot rather than per-cursor multiplies it by ring capacity, which
`ring_mpsc/src/lib.rs:331` already declines as "64 times" worse, so one
`CursorPair` per ring stays the right trade at 168 bytes/ring.

---

### AL30 — Zero Branches, Measured Against a Crate One Tier Down

```
  -- branches in the crate body --
0
  -- control: branches exist in the crate one tier down --
2
```

The zero is the crate rather than the pattern: `ring_types/src/capacity.rs`
measured identically reports two. Every entry point but `into_inner` is
additionally `const fn`, and the `const` context is entered rather than merely
declared, which the probe above proves by pairing an accepted compile with a
refused one.

**Finding.** The instruction-level claim is discharged as completely as a claim
of this kind can be. What remains uncosted is memory, which AL29 measures — so
this document holds one requirement fully met and one adjacent cost that its own
title's phrasing ("costs nothing") invites a reader to assume is also zero.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | The one division, and why a power-of-two divisor makes it a shift |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | The four single-expression operations the instruction-cost claim rests on |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_cache_aligned_wrapper.md](../data_structure/001_the_cache_aligned_wrapper.md) | The size table the overhead column is computed from |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_one_consumer.md](../integration/001_one_dependency_one_consumer.md) | The unused edge, measured |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_wrapped_values_arc.md](../lifecycle/001_the_wrapped_values_arc.md) | S2 — the state where the memory cost buys nothing |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [001_the_structural_claim_is_testable.md](001_the_structural_claim_is_testable.md) | N2 — the benefit side of this cost is the part nobody has measured |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | `E0493` — the one operation that is not `const` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:247-252` | `CursorPair`'s three fields, behind the 192-byte row |
| `ring_mpsc/src/lib.rs:331` | The per-slot alternative declined, and the "64 times" figure |
| `ring_align/tests/manual/readme.md` | M2's ✅ record for the no-`unsafe` claim |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | The size assertions the overhead table is read off |
| `tests/manual/readme.md` | M2 verifies no `unsafe`; nothing verifies the instruction-count claim, which is read from four one-expression function bodies |
