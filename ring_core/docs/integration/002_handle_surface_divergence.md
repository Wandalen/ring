# Integration: Divergence From `ring_handle`'s Specified Surface

### Scope

- **Purpose**: Record — as an obligation with a named owner, not a note — every point where this crate's surface differs from the one [`ring_handle`](../../../ring_handle/docs/api/001_producer_surface.md) already specifies, since the two are meant to be the same surface at two layers.
- **Responsibility**: The divergence table, the measured cause of each entry, which entries are defects and which are settled decisions, and what `ring_handle` must do about them.
- **In Scope**: Signature-level differences between `ring_core`'s handles and `ring_handle`'s specification.
- **Out of Scope**: The contracts themselves (→ [`api/001`](../api/001_producer_surface.md), [`api/002`](../api/002_consumer_surface.md)); `ring_handle`'s own implementation, which does not exist yet.

### System Description

`ring_handle` specifies the handle surface for the whole family.
This crate implements a handle surface **without depending on it** — the edge
would invert the layering (→ [`integration/001`](001_family_dependency_seam.md)).

So there is nothing that can detect a divergence: no shared trait, no compile
error, no test. The two surfaces are held in agreement by whoever reads both.
That is exactly the condition under which a divergence survives, which is why it
is written down with an owner rather than left to be noticed.

### Integration Points

| Point | `ring_handle` api/001 | here | Verdict |
|---|---|---|---|
| `try_push` receiver | `&self` | `&mut self` | **open** |
| `try_push_batch` receiver | `&self` | `&mut self` | **open** |
| `try_recv` receiver | `&self` | `&mut self` | **open** |
| `try_recv_batch` receiver | `&self` | `&mut self` | **open** |
| `free_capacity` receiver | `&self` | `&self` | agrees |
| `len` / `is_empty` receiver | `&self` | `&self` | agrees |
| `free_capacity` semantics | advisory at MPSC, binding at SPSC | identical | agrees |
| `len` semantics | lower bound that only grows | identical | agrees |
| `try_recv` return | `Option< T >`, emptiness not an error | identical | agrees |
| Refusal type | `Full< T >` | `T` | **open**, cheap |
| `is_closed` | present | absent | **settled** — see below |

**The split is exactly mutating versus reading**, and that was not obvious until
C6 measured it. The stage was written expecting `free_capacity` to diverge too;
it does not. The narrowed fact is the useful one, because it names the cause:
the four that differ reach a backend handle, the ones that agree only read.

**Every *semantic* row agrees.** The two specifications were written
independently, months apart, and land on the same contract for `free_capacity`'s
split bindingness, `len`'s growth direction, and `Option`-not-`Result` for
emptiness. That is worth recording as evidence rather than assumed: it means the
open rows are about Rust receivers and error naming, not about what the surface
promises.

#### The receiver divergence reduces to one crate

Measured, not assumed:

| Backend handle | Usable behind `&self`? | Evidence |
|---|---|---|
| `ring_mpsc::Producer` | yes | `impl Copy` at `ring_mpsc/src/lib.rs:738` |
| crossbeam arm | yes | it is a `&ArrayQueue`; every operation takes `&self` |
| `ring_spsc::Producer` | **no** | neither `Copy` nor `Clone` |

So `&self` is already achievable at two of three backends — `try_clone`'s MPSC
arm writes `*producer` today — and `ring_spsc` is the single blocker. The
consumer side is tighter still: `ring_mpsc`'s own documentation states its
`Consumer` is neither `Copy` nor `Sync`, deliberately, because single-consumer
is that ring's invariant.

**That points at the likely resolution, and `ring_handle` should evaluate it
rather than inherit this table:** `&self` may be the wrong specification for the consumer
half regardless of what the producer half does. Exclusive access is not an
implementation detail there; it is the invariant. Changing `ring_handle` is as
legitimate an outcome as changing this crate.

#### `is_closed` is settled, not deferred

`ring_handle` specifies it; this crate does not have it, and will not.

Liveness belongs to `ring_shutdown`. A handle-local copy of a
closed flag is precisely the failure that crate exists to prevent, and holding
one here would also mean an atomic on the path — which
[`invariant/001`](../invariant/001_no_atomic_of_its_own.md) forbids for a reason
that shows up two crates away, in `ring_spsc`'s zero-RMW assertion.

**`ring_handle`'s specification already agrees on the substance.** Both its
`api/001` and `api/002` state that `is_closed` *reads* `ring_shutdown`'s flag
and "does not carry a copy of it". So the two documents differ only on whether
the reading method belongs on this layer's handle — not on where the flag lives.

A caller who needs liveness composes `ring_shutdown` around this crate. This row
needs no reconciliation, only the note that a layer with no `ring_shutdown` edge
has nothing to read.

### Error Handling

**The divergences are surface-shape disagreements, not error-shape ones** — which
is what keeps them reconcilable later rather than breaking callers now:

| Aspect | `ring_handle`'s specification | This crate | Same? |
|---|---|---|---|
| Refusal of a push | The record is handed back | `Err( record )` | ✅ identical |
| Empty drain | Not an error | `None` / `0` | ✅ identical |
| Construction refusal | `RingError` | `RingError`, from `ring_types` | ✅ identical — one vocabulary, not two |
| `is_closed` | Present | Absent | ❌ divergent, and settled — see above |

So a caller written against either surface handles failures the same way; what
differs is receiver mutability and one absent predicate. That is why the
obligation below is a specification question for `ring_handle` rather than a
defect to fix here.

### Compatibility Requirements

**Owner:** `ring_handle`. On implementing it, one of three outcomes
must be recorded — not left implicit:

1. `ring_spsc::Producer`/`Consumer` gain a form usable behind `&self`, and this
   crate follows; or
2. `ring_handle`'s specification moves to `&mut self` for mutating operations,
   and this crate is already correct; or
3. The two layers deliberately differ, with the reason written down here and
   there.

Whichever it is, `Full< T >` versus `T` is decided at the same time — it is the
cheapest entry in the table and there is no argument for carrying it further.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The producer contract as this crate actually ships it |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The consumer contract |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_atomic_of_its_own.md](../invariant/001_no_atomic_of_its_own.md) | Why `is_closed` is settled rather than open |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_handle/docs/api/001_producer_surface.md`](../../../ring_handle/docs/api/001_producer_surface.md) | The specification this table diverges from |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | C6 — the divergence, checked by running a command rather than by review; it is where the mutating/reading split was found |

### CO21 — Every Dependent Imports `Ring`; None Imports `Backend`

```sh
cd "$(git rev-parse --show-toplevel)"
DEP="ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit"
for c in $DEP; do
  printf '%-14s ' "$c"
  grep -hoE 'ring_core::\{?[A-Za-z_, ]+' $c/src/*.rs | sed 's/ring_core:://' \
    | tr -d '{}' | tr ',' '\n' | tr -d ' ' | sort -u | tr '\n' ' '
  echo
done
```

Live output:

```
ring_bench     Producer Ring 
ring_debug     Consumer Producer Ring 
ring_factory   Ring 
ring_flush     Producer Ring 
ring_handle    Consumer Ends Producer Ring 
ring_poll      Consumer Producer Ring 
ring_shutdown  Consumer Producer Ring 
ring_testkit   Consumer Ends Producer Ring 
```

Five public types, and the distribution is a gradient: the ring itself
everywhere, the write end almost everywhere, the read end in five, the split
handle in the two crates that re-expose a split, and the backend discriminant
nowhere.

**That ordering is the shape of the seam.** A change to `Ring`'s signature
touches eight crates; a change to `Backend`'s touches none.

### CO22 — `ring_handle` Diverges by Subtraction, Not by Addition

`ring_handle/src/lib.rs:19` states the divergence as a table row: `ring_core`'s
producer can be duplicated on an MPSC backend, and a `ring_handle::Producer`
cannot be duplicated at all. `tests/ui/producer_try_clones.rs` is a compile-fail
case proving the narrowing holds.

A wrapper that only subtracts is the easy case to reason about: every guarantee
`ring_core` makes still holds, and the wrapper's own guarantee is strictly
stronger. Recorded so that a future divergence which *adds* is visibly a
different kind of change.
