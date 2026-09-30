# Lifecycle: No Initialization and No Teardown

### Scope

**Purpose:** Record that the crate has no lifecycle of its own — no `impl` block,
no constructor, no `Default`, no `Drop` — and record what that absence buys: a
test suite that never builds a ring, and a function callable before any ring
exists and after every ring is gone.

**Responsibility:** The crate's own construction and destruction story, which is
that it has none.

**In Scope:** `ring_index/src/lib.rs`;
`ring_index/tests/index_test.rs`; `ring_index/Cargo.toml`.

**Out of Scope:** the cycle the crate participates in without having one is
[`lifecycle/001`](001_the_lap_is_the_only_cycle.md). The crate shape this is a
consequence of is
[`pattern/002`](../pattern/002_stateless_arithmetic_over_borrowed_types.md).

---

## The Census

```sh
cd "$(git rev-parse --show-toplevel)"
echo "  impl blocks / new / Default / Drop in src : $( command grep -cE '^\s*impl |fn new|Default|Drop' ring_index/src/lib.rs )"
echo '  -- what the test file imports --'
command grep '^use ' ring_index/tests/index_test.rs
echo '  -- dev-dependencies --'
sed -n '/\[dev-dependencies\]/,/^\[/p' ring_index/Cargo.toml || true
echo "  rings, buffers or threads built in the tests : $( command grep -cE 'Buffer|Ring::|thread' ring_index/tests/index_test.rs )"
```

Live output:

```
  impl blocks / new / Default / Drop in src : 0
  -- what the test file imports --
use ring_index::{aliases, of, run};
use ring_types::{Capacity, Seq, SlotIndex};
  -- dev-dependencies --
  rings, buffers or threads built in the tests : 0
```

---

### IX55 — Three Functions and Nothing to Construct

**Finding.** The crate has zero `impl` blocks. There is no `new`, no `Default`,
no `Drop`, no builder, no handle, no guard — nothing that has to be created
before the crate can be used, and nothing that has to be released after.

Two properties follow, and both are stronger than they sound.

**`of` is callable at any point in a process's life.** Before a ring exists,
after every ring has been dropped, from a `const` context
([`api/002`](../api/002_the_three_signatures_and_the_const_they_are_not.md) IX7),
from inside a `Drop` impl belonging to something else. The function's only
requirement is a `Capacity`, and a `Capacity` is a `Copy` value that cannot be
invalid. There is no ordering constraint anywhere in this crate's contract.

**Nothing here can leak, double-free, or be used after free.** That is not a
claim about care taken; it is a claim about there being no resource. The one
allocation the crate can produce is `run`'s `Vec`, which is owned by the caller
the moment it returns
([`data_structure/002`](../data_structure/002_the_one_collection_the_crate_builds.md)).

The contrast that gives this weight is `ring_mpsc`, six tiers up: an
`UnsafeCell< Buffer< S > >`, a `Box< [ AtomicSeq ] >`, a `GatingSet`, a
constructor that threads one capacity into three places, and an entire safety
argument written in its module comment. Every one of those exists because that
crate holds something. `ring_index` holds nothing and therefore argues nothing.

---

### IX56 — Ten Tests, Zero Rings

**Finding.** The test file imports two crates — this one and `ring_types` — and
the manifest declares no `[dev-dependencies]` at all. Nothing in the suite
constructs a `Buffer`, a `Ring`, a cursor, or a thread.

Every test follows the same shape: build a `Capacity` with the local `cap`
helper, call one of the three functions, assert on the returned value. The
largest of them,
`mask_equals_modulo_over_four_laps_of_every_capacity`, runs 8,184 assertions
without a single allocation or a single piece of shared state.

That is what "no lifecycle" is worth in practice. The concurrency crates in this
family need a producer, a consumer, a barrier, and often a `loom` model to say
anything at all about their behaviour; this crate's central invariant is
established by two nested loops. The identity it asserts holds for every input in
the tested range regardless of what else is running, because there is no "else"
that can interact with it.

The cost sits on the other side of the same coin. Because nothing here has to be
set up, nothing here is exercised in situ either: the suite proves the arithmetic
and proves nothing about whether the arithmetic is reached. That the fold is
actually used — rather than reimplemented — is asserted in `ring_batch`'s test
suite, not this one
([`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md)
IX48), and for `ring_mpsc` it is asserted nowhere
([`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md)).

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_the_lap_is_the_only_cycle.md) | The cycle the crate sets the period of without joining |
| [`pattern/002`](../pattern/002_stateless_arithmetic_over_borrowed_types.md) | The crate shape this absence is the lifecycle view of |
| [`api/002`](../api/002_the_three_signatures_and_the_const_they_are_not.md) | `of` const-evaluating, which is the extreme case of "callable anywhere" |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | Where "is the fold reached" is asserted, since it is not asserted here |

### Sources

| Fact | Where |
|------|-------|
| Zero `impl`/`new`/`Default`/`Drop` | Census above |
| Two imports, no dev-dependencies | `ring_index/tests/index_test.rs:13-14`; `ring_index/Cargo.toml` |
| The 8,184-assertion loop | `ring_index/tests/index_test.rs:21-41` |
| What a crate that does hold state looks like | `ring_mpsc/src/lib.rs:308-338, 369-382` |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | The invariant, established without constructing anything |
| `non_power_of_two_capacity_is_rejected_upstream` | The only precondition, checked without a ring |
| `capacity_one_maps_everything_to_slot_zero` | The degenerate capacity, also without a ring |
| *(to create)* | Nothing — there is no setup or teardown path to cover |
