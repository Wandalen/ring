# Decision: Two Calls, Not One

### Scope

**Purpose:** Record the founding decision of the crate's shape — that reading
what is available and reporting it read are separate operations — and establish
what the separation does and does not buy.

**Responsibility:** The `available` / `commit` split: the argument for it, the
alternative it rejects, and the window it deliberately exposes.

**In Scope:** The module documentation's argument; `Consumer::available` and
`Consumer::commit`; the window between them and what guards it.

**Out of Scope:** The guard inside `commit` — that is
[`algorithm/002`](../algorithm/002_the_two_sided_guard.md). The convenience form
that collapses the two, which is
[`pitfall/001`](../pitfall/001_commit_available_does_not_call_commit.md).

---

## The Argument, As Made

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/## Why `available` and `commit` are separate calls/,/borrowed slots\./p' \
  ring_consume/src/lib.rs
```

Live output:

```
//! ## Why `available` and `commit` are separate calls
//!
//! A consumer reads a batch and then reports it. Between those, it holds slots
//! the producer must not overwrite, and the only thing preventing that is that
//! its cursor has *not* advanced yet. An `available_and_commit` that did both
//! would advance the cursor before the caller had read a byte, which frees
//! those slots for the producer while they are still being read — the exact
//! corruption the gating set exists to prevent, reintroduced above it.
//!
//! The split is what makes the dangerous window explicit: everything between
//! the two calls is a read of borrowed slots.
```

Quoted from the source:

> A consumer reads a batch and then reports it. Between those, it holds slots
> the producer must not overwrite, and the only thing preventing that is that
> its cursor has *not* advanced yet. An `available_and_commit` that did both
> would advance the cursor before the caller had read a byte, which frees
> those slots for the producer while they are still being read — the exact
> corruption the gating set exists to prevent, reintroduced above it.
>
> The split is what makes the dangerous window explicit: everything between
> the two calls is a read of borrowed slots.

### CN6 — The Rejected Alternative Is Named and Priced

The decision is recorded in the shape the family's best decisions take: the
alternative is named (`available_and_commit`), its appeal is implicit (one call
instead of two), and its cost is stated concretely — it reintroduces, above the
gating set, exactly the corruption the gating set exists to prevent.

That last clause is what makes the argument load-bearing rather than stylistic.
A one-call API would not be merely less explicit; it would be *unusable*,
because there is no correct moment for it to advance the cursor. Advancing
before the read frees slots being read. Advancing after the read requires
knowing when the read finished, which is precisely the information only the
caller has. So the two-call shape is not a tradeoff between convenience and
safety — it is the only shape that can work, and the documentation gets there
by naming what the single call would have to do.

Compare `ring_claim`'s founding decision, which rejects `fetch_add` in favour of
compare-exchange: same structure, same quality, argued at the same length. The
two crates were written to the same standard, and it shows most in their
`decisions`.

**Cost:** none. Recorded because the reasoning is the crate's primary asset and
a reader who skips the module doc will re-derive it badly.

---

### CN7 — The Window Is Explicit and Entirely Unguarded

"The split is what makes the dangerous window explicit" is true and is the
strongest thing that can be said for it. The window is explicit in the sense
that a reader who reads the module documentation knows it exists. It is not
explicit in any sense the compiler, the type system, or the test suite
participates in.

What the crate hands out between the two calls:

```rust
pub fn available( &self ) -> Available
```

`Available` is `Copy`, owns nothing, borrows nothing, and has no `Drop`. It is
two integers. Holding one grants no capability and releasing one revokes
nothing. Consequently:

| A caller who… | Result |
|---------------|--------|
| calls `available`, reads, calls `commit` | correct |
| calls `available`, calls `commit`, then reads | **corruption**, compiles clean |
| calls `available`, never commits | ring stalls; producer blocks forever |
| calls `available` twice, commits the older | silently under-commits; no error |
| calls `commit` with a value it never read | accepted if in range — the guard checks range, not provenance |

Only the first is correct. All five compile without a warning, and the second
is the one the module documentation exists to prevent.

The contrast with the write half is exact and worth stating. `ring_claim`
hands out a `Claim` carrying the family's most severe `must_use` message,
precisely because a dropped `Claim` strands a slot. `ring_consume` hands out an
`Available` with an unmessaged `must_use`, correctly, because a dropped
`Available` costs nothing. But the danger on the read side is not in dropping
the value — it is in *committing too early*, and there is no value whose
lifetime corresponds to "has finished reading." The type that would express it
would be a guard whose `Drop` performs the commit:

```rust
pub fn read( &self ) -> ReadGuard< '_ >   // commits on drop
```

That design has real costs — it forces a scope, it makes early commit
impossible rather than merely wrong, and it does not compose with a caller that
wants to commit part of a run. None of those costs are written down anywhere,
which means the decision to expose a plain `Copy` value rather than a guard was
either not considered or considered and not recorded.

The corpus cannot distinguish those two, and that is the finding. The
`available` / `commit` split is argued thoroughly against `available_and_commit`
— the alternative that collapses the window — and not at all against a guard,
the alternative that closes it.

**Cost:** reachable. The window is the crate's one genuine hazard, its
existence is documented, its shape is not defended against the one design that
would remove it, and nothing in the crate or the family detects a caller that
gets the order wrong.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F "//! The window's own alternative" ring_consume/src/lib.rs
```

Live output:

```
//! The window's own alternative — a guard whose `Drop` performed the commit,
//! `fn read( &self ) -> ReadGuard< '_ >` — would close it instead of merely
//! naming it, and has real costs of its own: it forces a scope, it makes an
//! early commit impossible rather than merely wrong, and it does not compose
//! with a caller that wants to commit part of a run. Those costs are why
//! `available` returns a plain `Copy` value instead.
```

**Disposition:** applied — the module documentation now argues against the
guard alternative by name, right after the existing argument against
`available_and_commit`, so the split's rationale covers both alternatives the
corpus named rather than one. The crate's 22 tests (1 `allocation_test.rs` +
21 `consume_test.rs`) plus 17 doctests re-verified passing (`cargo test
--all-features`, 2026-09-04). Now prints:
`The window's own alternative`

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| an `available_and_commit` | argued against in the module doc; no correct moment to advance |
| a lock around the window | the crate is single-consumer; there is nothing to exclude |
| a debug assertion that a read occurred | the crate cannot observe reads — it never sees the buffer |
| a timeout on an uncommitted window | a stalled consumer is a caller bug, not a ring state; `ring_publish` makes the same argument for its spin |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| decisions | [002](002_plain_stores_rather_than_compare_exchange.md) | the other founding decision, about the store rather than the split |
| algorithm | [002](../algorithm/002_the_two_sided_guard.md) | what `commit` checks, and what it cannot |
| pitfall | [001](../pitfall/001_commit_available_does_not_call_commit.md) | the convenience form that collapses the window safely |
| lifecycle | [001](../lifecycle/001_a_sequence_from_published_to_committed.md) | the window as a state a sequence passes through |
| invariant | [001](../invariant/001_never_reads_past_what_was_published.md) | the property the window's correct use preserves |

### Sources

| What | Where |
|------|-------|
| The argument | `ring_consume/src/lib.rs`, module doc |
| `available` | `ring_consume/src/lib.rs:336` |
| `commit` | `ring_consume/src/lib.rs:425` |
| `Available`'s `Copy` derive | `ring_consume/src/lib.rs:98` |
| `Claim`'s contrasting `must_use` | `ring_claim/src/lib.rs` |

### Tests

| Claim | Verified by |
|-------|-------------|
| The alternative is named and priced | the module doc section quoted above |
| `Available` is `Copy` with no `Drop` | the derive list at `:84`; no `impl Drop` in the crate |
| Committing early compiles clean | no type, lint or test in the crate observes read order |
| No guard type exists | the crate's 16 public items contain no `Drop` impl |
