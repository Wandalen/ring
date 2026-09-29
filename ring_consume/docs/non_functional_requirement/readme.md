# non_functional_requirement

The crate states four properties about itself and three of them were false. Not
subtly — measured, in release, with a counting allocator and a recursive grep.
Every read allocated; the dependency chain carries `std`; the chain contains a
blocking loop. The one property that held was `unsafe`-freedom, guarded by two
workspace lints, in a crate that would never have used `unsafe` anyway.

Two are still false. The allocation is gone — removed two crates away in commit
`b7e075ca`, which made every number in the first instance false in the other
direction without anything noticing — and is now the only one of the four with a
test behind it rather than a paragraph.

That is the first instance. The second counts ordering constants across the
family and finds eleven, of which exactly one is shared and four are declared
twice in crates with no dependency edge between them.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [What the Read Path Costs](001_what_the_read_path_costs.md) | CN34, CN35, CN36 — an allocation per read measured then removed, a chain carrying `std` and a blocking loop, and two enforced properties out of four |
| 002 | [Eleven Constants and the One That Is Shared](002_eleven_constants_and_the_one_that_is_shared.md) | CN37, CN38 — the ordering-constant census, and four names declared twice across unrelated crates |

### The Four Properties, Measured

| Property | Stated | Actually | Enforced by |
|----------|:------:|----------|-------------|
| no `unsafe` | ✔ | ✔ true | two workspace lints |
| no allocation | ✔ | ✔ true since `b7e075ca` — was **1 per read call** | `tests/allocation_test.rs` |
| `no_std`-clean chain | ✔ | ✘ `ring_wait` carries `std::` | nothing — 3 of 33 declare it, none in this chain |
| never blocks | ✔ | ✘ `ring_wait` has a blocking loop | nothing |

Measured then, release, counting `GlobalAlloc` — the figures the whole first
instance was written around, kept because the row above needs a before to be a
correction rather than an assertion:

```
one dependency, 300 readable:
  Consumer::position()             0
  Consumer::available()            1000
  Consumer::available_up_to(8)     1000
  Consumer::commit_available()     1000
empty barrier, frontier() is None:
  Consumer::available()            0
  Consumer::commit_available()     0
```

One allocation per call, on every read-path method that consulted the barrier.
The site was `ring_cursor::slowest` and the reason was a signature mismatch
rather than a computation
([`workaround/002`](../workaround/002_a_vector_to_change_a_slices_type.md)).
Every one of those rows now reads zero, and reads zero from a test rather than
from a scratch binary that deleted itself
([001](001_what_the_read_path_costs.md) CN34).

### Why the Zero Row Is the Interesting One

The empty-barrier case allocated nothing, because `frontier()` returns `None`
before reaching the `Vec`. That is why the false property survived so long: the
most obvious thing to measure — a consumer with no dependencies — was the one
case that reported clean.

It is still the interesting row, for the opposite reason. It read zero before the
fix and reads zero after, so it distinguishes nothing, and a suite made only of
rows like it would have reported success throughout. Both empty-barrier rows are
kept in `tests/allocation_test.rs` and labelled as unable to tell the two states
apart.

`ring_claim`'s corpus found the same allocation site by the same route and
recorded it as CL55. Two Tier 5 crates, one allocation site, discovered twice
independently because the cheap check passed in both — and closed once, by a
change neither corpus was consulted about.

### The Asymmetry With `ring_claim`

| | `ring_claim` | `ring_consume` |
|--|--------------|----------------|
| Chain length | 7 crates | 8 crates |
| `std::` in chain | 0 | 2 (`ring_wait`) |
| Blocking constructs | 0 | 2 (`ring_wait`) |
| Allocation sites | 1, at construction | 0 |

The read half's chain is one crate longer and dirtier on two of the three rows —
it was three before `b7e075ca`, and the allocation row has since reversed —
entirely because
`ring_barrier` depends on `ring_wait` for `Barrier::wait_for` — a method with no
library caller anywhere ([`integration/002`](../integration/002_eight_methods_and_the_one_that_is_called.md)).
A reader who checks `ring_claim` and generalises to "the Tier 5 primitives are
`no_std`-clean and non-blocking" is wrong about half of them.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the three negative properties, across the whole transitive chain
for c in ring_types ring_seqno ring_cursor ring_atomic ring_align ring_wait ring_barrier ring_consume; do
  printf '%-14s std %d  block %d  alloc %d  unsafe %d\n' "$c" \
    "$( grep -c 'std::' ring/$c/src/lib.rs )" \
    "$( grep -cE 'thread::(sleep|park|yield)|spin_loop|Condvar|Mutex' ring/$c/src/lib.rs )" \
    "$( grep -cE 'Vec<|vec!|\.collect\(\)|Box<|String' ring/$c/src/lib.rs )" \
    "$( grep -c 'unsafe' ring/$c/src/lib.rs )"
done

# what enforces anything
grep 'unsafe-code\|undocumented_unsafe' Cargo.toml

# the ordering constants, family-wide, with their crates
command grep -rE '^\s*(pub )?const [A-Z_]+ *: *(core::sync::atomic::)?Ordering' ring_*/src/*.rs | sed 's|ring/||'

# which names appear more than once
grep -rhoE '^\s*(pub )?const [A-Z_]+ *: *(core::sync::atomic::)?Ordering' ring_*/src/*.rs \
  | awk '{print $(NF-2)}' | sort | uniq -d
```

Live output:

```
ring_types     std 1  block 0  alloc 0  unsafe 0
ring_seqno       std 0  block 0  alloc 0  unsafe 0
ring_cursor    std 0  block 0  alloc 0  unsafe 0
ring_atomic    std 0  block 0  alloc 0  unsafe 4
ring_align     std 0  block 0  alloc 0  unsafe 4
ring_wait      std 2  block 4  alloc 0  unsafe 0
ring_barrier   std 0  block 0  alloc 0  unsafe 0
ring_consume   std 0  block 0  alloc 3  unsafe 0
unsafe-code = "deny"
undocumented_unsafe_blocks = "deny"
ring_claim/src/lib.rs:const CLAIM_SUCCESS : core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;
ring_consume/src/lib.rs:const COMMIT : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_cursor/src/lib.rs:pub const GATING : Ordering = Ordering::Acquire;
ring_debug/src/lib.rs:const OBSERVE : Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const PUBLISH : Ordering = Ordering::Release;
ring_mpsc/src/lib.rs:pub const OBSERVE : Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const COMMIT : Ordering = Ordering::Release;
ring_mpsc/src/lib.rs:pub const OWN : Ordering = Ordering::Relaxed;
ring_publish/src/lib.rs:const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_spsc/src/lib.rs:pub const OWN : Ordering = Ordering::Relaxed;
ring_spsc/src/lib.rs:pub const HANDOFF : Ordering = Ordering::Release;
COMMIT
OBSERVE
OWN
PUBLISH
```

The `alloc` column above counts source mentions, comments included, and so is
not the allocation measurement — `ring_consume`'s own three hits are all in
prose. The measurement is `tests/allocation_test.rs`, quoted in
[001](001_what_the_read_path_costs.md); the difference between the two is the
whole reason CN34 exists.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN34 | `ring_cursor` | **measured cost** | Every read of the available range allocated, measured at 1000/1000 in release, until `b7e075ca` removed the site in `ring_cursor::slowest`; now measured at 0/1000 by a test rather than a scratch binary |
| CN35 | `ring_wait` | **wrong doc** | The chain is neither `no_std`-clean nor non-blocking — `ring_wait` carries both — where `ring_claim`'s equivalent chain is both |
| CN36 | `ring_consume` | n/a — unenforced | One of four stated properties has enforcement, and it is the one nobody would break; the three that a plausible change breaks are guarded by nothing |
| CN37 | family | n/a — observation | Eleven ordering constants across seven crates, seven distinct names, and exactly one (`ring_cursor::GATING`) is actually shared |
| CN38 | family | n/a — duplication | Four names — `COMMIT`, `PUBLISH`, `OBSERVE`, `OWN` — are each declared twice, in crates with no dependency edge between them |
