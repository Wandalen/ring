# Algorithm: The Slowest Fold

### Scope

- **Purpose**: State what `slowest` computes, why the empty case answers `None`, and record that the fold used to build a heap-allocated `Vec` to reuse a function whose entire body is `.min()`.
- **Responsibility**: Give the computation, its two callers, the exact cost the delegation used to carry, and the test that now holds it at zero.
- **In Scope**: `slowest` in `src/lib.rs`; `ring_seqno::slowest`; the `None`-not-`ZERO` decision; the allocation that was.
- **Out of Scope**: What each caller *means* by the answer, which is [`pattern/002`](../pattern/002_one_fold_two_questions.md); the ordering the loads use, which is [`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md).

### What It Computes

```rust
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
```

It was three lines until commit `b7e075ca`, and the middle one is what the rest
of this document is about:

```rust
// as it stood before commit b7e075ca
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

Read every cursor in the slice at [`GATING`](../type/001_gating.md), take the
minimum, answer `None` when the slice is empty. In the older form the middle step
was not performed here at all:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F 'pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >' ring_seqno/src/lib.rs
```

Live output:

```
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

```rust
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

**The delegated function was one method call.** `ring_seqno::slowest` is
`.min()` with a name — no lap arithmetic, no capacity, no wraparound handling.
The whole of the shared logic this crate reaches for is `Iterator::min`.

### The Reads Are Not Atomic Together

The loop loads each cursor separately. Nothing prevents a consumer from
advancing between the first load and the last, so the returned minimum may be a
position no single instant of the system ever showed:

| Time | Cursor A | Cursor B | Loaded |
|------|---------:|---------:|--------|
| t0 | 5 | 9 | A → 5 |
| t1 | 12 | 9 | — |
| t2 | 12 | 9 | B → 9 |

The answer is `5`, and by the time it is returned no cursor is at 5.

**This is correct, and it is correct for a reason worth stating.** The value is
used as a *lower bound* — "no consumer is behind this" — and a stale lower bound
is conservative in the only direction that matters. A producer that believes the
slowest consumer is further behind than it really is claims fewer slots than it
could have; it never claims one it should not. The error is always toward
caution because cursors only advance.

A caller wanting a coherent snapshot would need every cursor read under one
lock, which is the thing the whole crate exists to avoid.

### `None` Rather Than `Seq::ZERO`

The empty slice could plausibly answer `Seq::ZERO` — the position of a consumer
that has read nothing. It answers `None` instead, and the reason is that the two
callers resolve *no cursors* to opposite values:

| Caller | Empty set means | Resolves to |
|--------|-----------------|-------------|
| `ring_gating::GatingSet::headroom` | nobody is reading, so nothing can be lost | the **full capacity** — `self.slowest().map_or( self.capacity.get(), … )` |
| `ring_barrier::Barrier::available` | nothing has been declared readable | **zero** |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn headroom( &self, producer : Seq ) -> usize' ring_gating/src/lib.rs
```

Live output:

```
  pub fn headroom( &self, producer : Seq ) -> usize
  {
    self.slowest().map_or( self.capacity.get(), | slowest |
    {
      ring_seqno::free_slots( producer, slowest, self.capacity )
    } )
```

`Seq::ZERO` would have been silently wrong for `ring_gating`: a gating set with
no consumers would report a slowest consumer at zero, and a producer one lap
ahead would be told it is full — a ring nobody reads, blocking forever. Handing
back `None` forces each caller to write down its own answer, where it can be
read and argued.

**The general shape:** a fold shared by callers that disagree about the identity
element must not pick one. `Option` is how it declines.

### The Allocation There Was

`ring_seqno::slowest` takes `&[ Seq ]`. The cursors are `&[ PaddedCursor ]`. A
slice cannot be produced from a mapped iterator without materialising it, so
`collect()` allocated.

| Property | Before `b7e075ca` | Now |
|----------|-------------------|-----|
| Allocations per call | 1 (plus the matching free) | 0 |
| Bytes | `cursors.len() * size_of::< Seq >()` = `n * 8` | 0 |
| Body | `collect()` into a `Vec< Seq >`, then `ring_seqno::slowest( &positions )` | `cursors.iter().map( \| c \| c.load( GATING ) ).min()` |
| Behavioural difference | — | none — `Iterator::min` answers `None` on an empty iterator, exactly as `ring_seqno::slowest` does |

The replacement is *shorter* than what it replaced and drops a dependency edge
from this function. What the `Vec` bought was that the word `min` was written in
`ring_seqno` rather than here — the delegation discipline
[`algorithm/002`](002_three_readings_of_two_cursors.md) describes, applied to a
fold that carries none of the arithmetic the discipline exists to protect. That
is the trade the change reversed, and the loss is real: `ring_seqno::slowest` has
had no caller outside its own tests since.

### Where the Cost Landed

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- production call sites --'
grep -r 'ring_cursor::slowest' --include=*.rs */src/ \
  | command grep -vE ':[[:space:]]*(///|//!)' | sed -E 's/:/: /' | LC_ALL=C sort
echo '  -- and the test files that now measure it --'
grep -rl 'ring_cursor::slowest' --include=*.rs */tests/ | LC_ALL=C sort
```

Live output:

```
  -- production call sites --
ring_barrier/src/lib.rs:         ring_cursor::slowest(self.dependencies)
ring_gating/src/lib.rs:         ring_cursor::slowest(&self.cursors)
  -- and the test files that now measure it --
ring_barrier/tests/allocation_test.rs
ring_claim/tests/allocation_test.rs
ring_cursor/tests/allocation_test.rs
```

Two call sites, and the first of them is on the producer's admission path:

| Chain | Frequency |
|-------|-----------|
| `GatingSet::may_claim` → `headroom` → `slowest` → **here** | once per claim attempt on a gated ring |
| `Barrier::frontier` → **here** | once per readability question |
| `Barrier::available` → `frontier` → **here** | once per consumer batch |

**A heap allocation per claim attempt was the shape of cost this family is built
to avoid.** The crate pads a cursor to 64 bytes so two cores stop invalidating
one line; a producer that mallocked before every claim had taken on a global
allocator's synchronisation to save writing `.min()`. Three of the four crates on
those chains now carry a counting allocator in `tests/` that would catch it
coming back — `ring_cursor` at the fold, `ring_barrier` at the frontier read,
`ring_claim` at the claim.

### The Question That Was Never Answered, and the One That Was

For the whole life of the `Vec` the open question was whether it survived
optimisation. `collect()` calls the global allocator through an opaque call that
LLVM does not generally remove, but Rust does elide some allocations and this one
had a short, entirely-local lifetime — the case most likely to be optimised.
Nothing measured it, and the reason nothing did is recorded in
[`non_functional_requirement/002`](../non_functional_requirement/002_the_gating_read_allocates_nothing.md)
§ CU35: a policy claim that turned out not to cover the file such a test would
live in.

By the time anyone looked, the answer had stopped being recoverable — the
allocation was removed on the argument rather than on a number, so there is no
before-state left to disassemble. What was measured, from `ring_barrier`, is that
it *had* charged one 8-byte allocation per call at every arity above zero, which
settles the elision question retrospectively and settles nothing about cost.

The two structural facts that made elision unlikely are still checkable, and the
second of them is unchanged:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- is the disassembly instrument available? --'
command -v cargo-asm > /dev/null && echo '    installed' || echo '    not installed'
printf '    Vec< Seq > left in the fold: %s\n' \
  "$( command grep -c 'Vec< Seq >' ring_cursor/src/lib.rs )"
printf '    calls left from this crate into ring_seqno: %s\n' \
  "$( command grep -c 'ring_seqno::' ring_cursor/src/lib.rs )"
echo '  -- and whether the release profile enables cross-crate inlining --'
command grep -c 'lto' Cargo.toml | sed 's|^|    lto mentions in the root manifest: |'
```

Live output:

```
  -- is the disassembly instrument available? --
    not installed
    Vec< Seq > left in the fold: 0
    calls left from this crate into ring_seqno: 3
  -- and whether the release profile enables cross-crate inlining --
    lto mentions in the root manifest: 0
```

**Nothing configures LTO, and the fold no longer needs it to.** Elision would
have had to survive a cross-crate call the profile does not ask the linker to
inline; the rewrite removed the call rather than the uncertainty about it.

The direct instrument — a counting global allocator in a test binary, installing
a `GlobalAlloc` that increments an `AtomicUsize` and asserting the counter does
not move across a call — is `tests/allocation_test.rs`, and it exists now. It was
described here as the thing to build long before it was built.

### What Is Not Tested Here

`slowest` has no test in `tests/cursor_test.rs`, and exactly one elsewhere in
this crate:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every test file here, and how often it names the fold --'
command grep -c 'slowest' ring_cursor/tests/*.rs | sed 's|^ring_cursor/tests/|    |'
echo '  -- what the one that names it asserts --'
# joined first: two of these four assertions are wrapped across five source
# lines each, and a line-oriented match would silently find only two
tr '\n' ' ' < ring_cursor/tests/allocation_test.rs | tr -s ' ' \
  | command grep -oE '\( 0, 0 \), "[^"]+"' | sed 's/( 0, 0 ), /    0 allocations : /'
echo '  -- and the ordering it reads at, which still nothing here checks --'
command grep -c 'GATING' ring_cursor/tests/cursor_test.rs \
  | sed 's/^/    occurrences of GATING in cursor_test.rs: /'
```

Live output:

```
  -- every test file here, and how often it names the fold --
    allocation_test.rs:10
    cursor_test.rs:0
  -- what the one that names it asserts --
    0 allocations : "slowest over an empty slice"
    0 allocations : "slowest over one cursor"
    0 allocations : "slowest over three cursors — the arity the removed Vec was sized by"
    0 allocations : "a thousand gate reads — the shape the removed Vec charged a thousand times"
  -- and the ordering it reads at, which still nothing here checks --
    occurrences of GATING in cursor_test.rs: 0
```

Its interpretive coverage is its own doctest (which exercises the empty case,
the minimum, and a minimum that moves) plus the consumers' suites —
`ring_gating/tests/gating_test.rs` and `ring_barrier/tests/barrier_test.rs`.
That is real coverage, and it is coverage of the *callers'* interpretations
rather than of the fold. `tests/allocation_test.rs` is the one test here that
checks the fold itself, and it checks one property: that it allocates nothing.
Nothing in this crate's own suite would still notice if the fold started reading
`Relaxed`.

### CU1 — The Allocation Was One Line From Its Call, and Nothing Was Watching Either

```
// as it stood before commit b7e075ca
120:  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
121:  ring_seqno::slowest( &positions )
lto mentions in the root manifest: 0
```

The optimistic reading was that the `Vec` never reached the heap — LLVM sees the
allocation, the consumption and the drop in one function and elides it. That
reading needed the callee inlined, and the callee was in another crate. Nothing
in the root manifest asks for cross-crate inlining: the profile sets no `lto`,
and that is still true.

The optimistic reading was wrong. When the fold was finally measured, from
`ring_barrier`, it charged one 8-byte allocation per call at every arity above
zero — so the elision LLVM was being credited with never happened.

**Finding, and what closed it.** The elision was plausible and unmeasured, and
this section's own evidence for why nobody would notice —
`tests/cursor_test.rs` containing zero occurrences of `alloc` — was accurate for
the whole life of the allocation. The fold was rewritten as
`cursors.iter().map( | c | c.load( GATING ) ).min()`, which does not allocate
whether or not anything is inlined, and `tests/allocation_test.rs` now asserts
that at three arities and at a thousand calls, with a control arm that fails if
the counter is not installed. The requirement
[`non_functional_requirement/002`](../non_functional_requirement/002_the_gating_read_allocates_nothing.md)
recorded as unmeasurable under gate G6 is measured now, in this crate, by that
file.

---

### CU2 — The Crate That Owns the Allocation Is Not the Crate That Pays for It

```
ring_barrier/src/lib.rs:193:    ring_cursor::slowest( self.dependencies )
ring_gating/src/lib.rs:199:    ring_cursor::slowest( &self.cursors )
```

`slowest` has no caller inside `ring_cursor`. Both production callers are one
crate away, and both are on paths that run per claim rather than per ring: a
barrier resolving its frontier and a gating set finding its slowest reader.

**Finding.** The cost is written here and incurred there. A reader auditing
`ring_barrier` for allocations on the hot path sees a function call; the `Vec` is
only visible by opening this crate's source, and neither caller's documentation
mentions it.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | `slowest` is the free function on the deciding half of the surface — it fixes the ordering rather than taking one |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_three_readings_of_two_cursors.md](002_three_readings_of_two_cursors.md) | The same delegation discipline, applied where the delegated arithmetic is genuinely non-trivial |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | Why the loads inside the fold take no ordering argument |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_gating_read_allocates_nothing.md](../non_functional_requirement/002_the_gating_read_allocates_nothing.md) | The allocation as a cost claim, what would have had to be true for it to matter, and the policy argument that kept it unmeasured |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_fold_two_questions.md](../pattern/002_one_fold_two_questions.md) | Why one function serves two crates that mean opposite things by its answer |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_gating.md](../type/001_gating.md) | The ordering every load in the fold uses |

### Sources

| File | Relationship |
|------|--------------|
| `slowest` in `ring_cursor/src/lib.rs` | The fold |
| `ring_seqno/src/lib.rs:133-136` | The delegated `.min()` |
| `ring_gating/src/lib.rs:197-228` | The first caller, and its empty-set resolution |
| `ring_barrier/src/lib.rs:191-194` | The second caller |

### Tests

| File | Relationship |
|------|--------------|
| `slowest`'s doctest in `src/lib.rs` | The empty case, the minimum, and a minimum that moves |
| `ring_cursor/tests/allocation_test.rs` | Four call shapes at zero allocations, behind a control arm that must allocate |
| `ring_gating/tests/gating_test.rs:126-136` | The slowest consumer sets the bound regardless of its index |
| `ring_gating/tests/gating_test.rs:190` | The empty set answers `None` |
| `ring_barrier/tests/barrier_test.rs:410` | `frontier()` and `set.slowest()` agree — the two callers on one fold |
