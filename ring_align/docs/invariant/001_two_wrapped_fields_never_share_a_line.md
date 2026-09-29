# Invariant: Two Wrapped Fields Never Share a Line

### Scope

- **Purpose**: State the property this crate exists to deliver, as a restriction on layout rather than as a fact about a type — and mark exactly where the crate's guarantee ends and the caller's begins.
- **Responsibility**: State the restriction, name what enforces it and how strongly, and record what a violation costs.
- **In Scope**: The separation of two `CacheAligned` fields within one struct.
- **Out of Scope**: Whether the line size is right, which is [`invariant/002`](002_one_constant_for_the_whole_family.md) and [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md); what the separated values are used for, which is [`ring_cursor`](../../../ring_cursor/readme.md)'s.

### Invariant Statement

**Two [`CacheAligned`](../type/002_cache_aligned.md) fields declared in one
struct occupy different cache lines, for every payload and every field order.**

Note what the statement is *not*. It is not "a wrapped value occupies one cache
line" — false for payloads over 64 bytes
(→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md)).
It is not "wrapping a value separates it from everything" — the wrapper knows
nothing about what else is in the struct. It is a claim about **two of them,
together**, which is the smallest unit at which the property the family needs
is even expressible.

This invariant exists because losing it has a direct consequence:

> The producer and consumer cursors land on one line and every write by either
> invalidates the other's cached copy. Throughput falls off a cliff as core
> count rises, which is the opposite of what adding cores is supposed to do.

### Enforcement Mechanism

| # | Mechanism | Strength |
|---|-----------|----------|
| P1 | `#[ repr( align( 64 ) ) ]` sets alignment to 64 | Real — the compiler places each field at a line boundary |
| P2 | Whole-line **sizing** (size rounds up to a multiple of the alignment) | Real, and it is the half that makes P1 compose — alignment alone would still let a third value share a padded field's trailing bytes |
| P3 | The payload field is private | **Weak, and weaker than it reads.** `get`, `get_mut`, and `into_inner` all reach the payload, so privacy restricts no caller — it buys this crate freedom to change the representation, and nothing about the layout (→ [`pattern/001`](../pattern/001_the_newtype_as_layout_carrier.md)) |
| P4 | `tests/align_test.rs` asserts it on real addresses, with a negative control | Real evidence, on one machine, for the payloads tested |
| P5 | `on_distinct_lines` lets a caller assert it about its own struct | Real, and the reason it is exported rather than a test helper |
| P6 | A check that a caller wrapped *both* of two hot fields rather than one | **Does not exist** — and cannot, from here |

**P1 and P2 together are the whole guarantee, and P2 is the one a reader
overlooks.** A 64-byte-aligned type of size 8 would put both fields on line
boundaries and leave 56 bytes of each line available for the compiler to pack
something else into — the separation would evaporate for reasons no assertion
about a single type would catch. Rounding the size up is what forecloses it.

**P6 is worth naming even though it cannot exist here.** It is the obligation
the invariant hands to the caller, and it is discharged by construction in
`ring_cursor`: `CursorPair` holds both cursors, so there is no arrangement in
which one is padded and the other is not. That is the correct shape — the
guarantee is completed by a type that owns both halves, not by a check.

### Violation Consequences

| # | Violation | What is lost |
|---|-----------|--------------|
| C1 | A caller wraps one hot field and not its partner | The unwrapped field lands in the padded one's line. Every assertion about the padded type still passes |
| C2 | Two padded fields are separated by the caller into different allocations, each sharing a line with something else hot | Separation from *each other* is preserved and irrelevant; the contention moved rather than went away |
| C3 | `CACHE_LINE` is smaller than the host's real line | Both fields are separated by 64 on a 128-byte-line machine — the invariant holds as stated and buys nothing (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)) |
| C4 | A future `repr` or compiler change alters field packing | P4 catches it on the machine the suite runs on, at the payloads tested — which is why the test reads addresses rather than trusting `size_of` |
| C5 | A caller reaches the payload by unwrapping and stores the result beside another hot value | The padding is discarded with the wrapper (→ [`lifecycle/001`](../lifecycle/001_the_wrapped_values_arc.md)); nothing here can observe it |

**C1 is the realistic one and it does not look like a violation while it is
being written.** A developer padding "the cursor that gets written most" is
optimising thoughtfully and has produced exactly nothing: false sharing is
symmetric, and one padded field beside one unpadded field still contends.

**C3 is the one that defeats the invariant while satisfying it.** The statement
above is true on Apple Silicon — the fields are on different 64-byte units —
and the property it was written to secure is absent. That is why the invariant
is stated in terms of *cache lines* rather than *64 bytes*, and why the gap
between the two is a separate instance.

### AL20 — The Invariant Is Stated Over a Pair Because That Is the Only True Form

A single `CacheAligned` has no line to be separate from. The property comes into
existence when a second one is placed beside it, and it is a property of the
struct holding both rather than of either wrapper.

**Finding.** Stating it over the wrapper — "a `CacheAligned` occupies its own
line" — is the formulation a reader reaches for and the one that fails on the
zero-sized payload
(→ [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md)
AL9). The pair formulation survives that case, and it is why the API cannot
enforce this invariant at all: no signature over one wrapper can mention the
second.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | P1 and P2 — the rule that establishes this, and why the sizing half matters |
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | P5 — how a caller asks whether the invariant holds for its own fields |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | W1–W3 — the same caller obligations, stated from the surface's side |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_a_padded_pair_in_one_struct.md](../data_structure/002_a_padded_pair_in_one_struct.md) | The arrangement this invariant is about, and how `CursorPair` discharges P6 |

### Invariants

| File | Relationship |
|------|--------------|
| [002_one_constant_for_the_whole_family.md](002_one_constant_for_the_whole_family.md) | The companion restriction — this one can hold while that one is broken, which is C3 |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | Why P4 reads addresses and carries a negative control |
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | C3 in full |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:68-69` | The `#[ repr( align( 64 ) ) ]` wrapper this invariant is stated about |
| `ring_align/tests/align_test.rs:75` | `two_wrapped_fields_land_on_different_lines` — the test this invariant's claim is checked by |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `two_wrapped_fields_land_on_different_lines` is P4; `two_unwrapped_fields_share_a_line` is the control that makes it evidence |
| `tests/manual/readme.md` | M3 runs both and names the control as the load-bearing one |
