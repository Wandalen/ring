# Invariant: Uniform Delivery Across Every Backend

### Scope

- **Purpose**: State exactly what "the same program on every backend" asserts, and — equally — what it deliberately does not, so that the reached-test's `sort_unstable` is read as a decision rather than a weakening.
- **Responsibility**: The invariant, the three behaviours excluded from it and why each exclusion is forced, and how the whole thing is checked.
- **In Scope**: Delivery guarantees a caller may rely on regardless of which backend is underneath.
- **Out of Scope**: Per-backend ordering internals (→ each backend's own `invariant/`); `free_capacity`'s two contracts, which have their own instance.

### Invariant Statement

**For every backend the build offers, a program written against this surface
observes: every accepted record delivered exactly once, no record delivered
that was not accepted, and no record delivered twice.** Exactly-once delivery
of the accepted set, over an unbounded number of laps.

That is the whole of it. It is deliberately narrower than "the backends behave
identically", and the gap is the subject of the next section.

#### Three behaviours explicitly excluded

An invariant that claimed more than this would be false, and the reached-test
would have to be weakened to keep passing — so the exclusions are stated here,
where they can be argued, rather than discovered in the test.

| Excluded | SPSC | MPSC | crossbeam | Why it cannot be uniform |
|---|---|---|---|---|
| **Drain order** | publication order | per-producer order, interleaved | unspecified | With N producers there is no total order to preserve; only each producer's own sequence survives, and `ArrayQueue` does not promise even that across producers |
| **`free_capacity` exactness** | binding | advisory | advisory | Binding requires that nothing else can take the room, which is exactly what a second producer contradicts (→ [`pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md)) |
| **`OverflowPolicy::DropOldest`** | rejected at construction | rejected | supported | Eviction contradicts exactly-once, which the two in-house rings guarantee. `ArrayQueue::force_push` provides it, so refusing it everywhere would discard the one backend that has it |

**Occupancy readings are a separate, narrower claim than the table above.**
`len`, `is_empty`, and `free_capacity` describe what the ring reports about
itself mid-run, not what it delivers, so they sit outside the three
delivery exclusions above. `free_capacity`'s own binding/advisory split is
already in the table; `len` and `is_empty` carry the analogous advisory-only
caveat at every backend, including SPSC, detailed in
[`../api/002`](../api/002_consumer_surface.md) (CO7).

**The reached-test sorts before asserting, and that is these exclusions in
code**, not a concession. A test that asserted drain order would pass at SPSC,
fail at MPSC, and be meaningless at crossbeam — so it would have to be three
tests, which is precisely the "rewrite rather than a build flag" outcome this
crate's uniform-surface requirement exists to avoid.

**The sort has a cost worth naming.** `ring_mpsc`'s own suite has a concurrent
doc example that sorts before asserting and therefore cannot fail on any
ordering defect — a real weakness, recorded in that crate's manual plan. The
difference here is that the sort is paired with a stated exclusion and with an
exactly-once assertion that the sort does not weaken: sorting cannot hide a lost
record, a duplicated one, or an invented one. Ordering defects are caught by
each backend's own suite, where a total order exists to check against.

#### What holds because of the exclusions rather than despite them

Three properties *are* uniform, and each is uniform only because the surface
declined to promise something adjacent:

- **A refused record comes back.** `try_push` returns `Result< (), T >`, and
  the `Err` carries the record. Uniform at all three — but it required claiming
  a slot before consuming the record at MPSC, because
  `ring_mpsc::Producer::push` takes the value and cannot hand it back
  (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)).
- **`len` and `is_empty` never disagree.** Two computations of one fact — a
  subtraction at the in-house rings, a load at crossbeam — which is exactly the
  pair that can drift.
- **An undrained record is dropped exactly once when the ring is dropped.** Not
  leaked, not double-freed, at every backend and for a heap payload.

### Enforcement Mechanism

The reached-test is one program, parameterized over backends rather than
written three times:

```
tests/core_test.rs::the_same_program_behaves_identically_on_every_backend
```

It pushes 200 records through an 8-slot ring, draining when refused — so the
ring wraps roughly 25 times — then asserts the sorted result is exactly
`0..200`.

**That shape has its own failure mode, and it is guarded separately.** A
parameterized test that silently runs against one backend proves nothing about
the other two while still reporting green. `every_backend_the_build_offers_is_actually_exercised`
pins the count to the build's feature set: three with `crossbeam`, two without,
all distinct. Without it, a mistake in the backend list would make the reached
test vacuous in exactly the way it is designed to prevent.

The drain loop also asserts that a refusal is followed by a successful drain,
rather than assuming it — if the ring ever stops releasing slots, the test fails
with a message instead of spinning forever.

### Violation Consequences

A break in this invariant is a break in the reason the crate exists: the whole
value of a uniform surface is that a program can be written once and the backend
chosen later.

| Broken clause | What a caller observes | Why it is worse than a plain bug |
|---|---|---|
| A record delivered that was not accepted | An invented record downstream | The write path's own accounting is then unusable — a consumer cannot distinguish it from its own defect |
| A record delivered twice | Duplicate work, double side effects | The sort in the reached-test does **not** hide this; a duplicate survives sorting, which is what makes the sorted assertion still meaningful |
| An accepted record never delivered | Silent loss under load only | Indistinguishable at the call site from a full ring correctly refusing, so it presents as a capacity problem and gets "fixed" by growing the ring |
| Uniformity itself | A program that works on one backend and not another | The build flag stops being a build flag and becomes a rewrite — precisely the outcome this crate's uniform-surface requirement exists to avoid |

The last row is the one with no test that could catch it directly, which is why
`every_backend_the_build_offers_is_actually_exercised` guards the *coverage*
rather than the behaviour: a vacuous parameterization would report green while
this clause was already broken.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_backend_dispatch_and_the_refusal_seam.md](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) | How the refusal is made uniform when one backend consumes the record |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The publish half, and which of its promises are per-backend |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The drain half, including why `try_recv_batch` appends |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_free_capacity_carries_two_contracts.md](../pitfall/001_free_capacity_carries_two_contracts.md) | Exclusion 2, worked out as a caller-facing trap |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_occupancy_across_backends.md](../lifecycle/003_occupancy_across_backends.md) | The occupancy readings that must agree for `len`/`is_empty` to be uniform |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_mpsc/tests/manual/readme.md`](../../../ring_mpsc/tests/manual/readme.md) | M8 — the sorted concurrent example that cannot fail, and why this one's sort is not the same case |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `the_same_program_behaves_identically_on_every_backend` — the reached-test |
| `tests/core_test.rs` | `every_backend_the_build_offers_is_actually_exercised` — the guard against the reached-test going vacuous |
| `tests/core_test.rs` | `records_left_in_a_dropped_ring_are_released_exactly_once`, `len_and_is_empty_agree_at_every_point_of_a_lap`, `a_refused_record_comes_back_on_every_backend` — the three uniform properties |
| `tests/manual/readme.md` | C1 — the mutation establishing that the suite can fail at all, and that the doc tests cannot |

### CO25 — One Test Asserts Identical Behaviour and Runs Per Backend

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'test attrs:   '; grep -c '#\[ test \]' tests/core_test.rs
printf 'fn defs:      '; grep -cE '^fn [a-z_]+' tests/core_test.rs
echo '--- the uniformity tests ---'
grep -oE '^fn [a-z_]+' tests/core_test.rs | sed 's/^fn //' \
  | grep 'every_backend\|identically\|uniform'
```

Live output:

```
test attrs:   24
fn defs:      27
--- the uniformity tests ---
every_backend
every_backend
the_same_program_behaves_identically_on_every_backend
every_backend_the_build_offers_is_actually_exercised
every_backend_reports_the_capacity_it_was_configured_with
a_refused_record_comes_back_on_every_backend
a_capacity_of_one_cycles_correctly_on_every_backend
```

Twenty-four `#[ test ]` attributes over twenty-seven function definitions — the
three extras are helpers, `every_backend` among them.

**Correction (2026-09-28):** this paragraph read "Twenty-two ... over twenty-five
function definitions". The live block above was already corrected to 24/27 when
this crate's docs were restructured, but that pass updated only the two count
lines and missed both this sentence and the uniformity-tests list two lines
above, which was still missing `a_capacity_of_one_cycles_correctly_on_every_backend`
— now added. `tests/core_test.rs` itself has not changed; only the documentation
had drifted behind it. The three-extras arithmetic was never affected either
way (27-24 = 25-22 = 3).

**The invariant's strength is therefore the helper's coverage.** If
`every_backend` iterates only the backends compiled into the current build, then
a default-feature test run asserts uniformity across two backends and says
nothing about the third — which is the same gap
`pitfall/002` records for coverage instrumentation, arriving here through
assertions instead.

### CO26 — Uniform Delivery Does Not Mean Uniform Occupancy

The invariant as stated covers what comes out: every backend delivers every
accepted record, in order, exactly once. That holds.

What does not hold uniformly is what the ring *reports about itself* mid-run.
`free_capacity` is binding at SPSC and advisory elsewhere
(→ [`../pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md)),
and `len`/`is_empty` carry the mirror asymmetry
(→ [`../api/002`](../api/002_consumer_surface.md), CO7).

**"Uniform delivery across backends" is true and reads as broader than it is.**
A caller who takes it as "the ring behaves identically on every backend" — which
is close to how the module documentation phrases the crate's purpose — will
expect `free_capacity` to mean one thing. The narrowing is documented on the
method and not next to the invariant.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F 'Occupancy readings are a separate' ring_core/docs/invariant/002_uniform_delivery_across_backends.md
```

Live output:

```
**Occupancy readings are a separate, narrower claim than the table above.**
`len`, `is_empty`, and `free_capacity` describe what the ring reports about
itself mid-run, not what it delivers, so they sit outside the three
```

**Disposition:** applied — a new paragraph now sits directly under the
"Three behaviours explicitly excluded" table, next to the invariant
statement rather than only on the method, cross-referencing `len`/`is_empty`'s
advisory-only caveat alongside `free_capacity`'s. Now prints: `Occupancy
readings are a separate, narrower claim than the table above.`
