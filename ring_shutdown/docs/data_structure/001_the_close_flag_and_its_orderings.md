# Data Structure: The Close Flag and Its Orderings

### Scope

- **Purpose**: Record the one piece of state this crate owns — a single `AtomicBool` — its orderings, its access sites, and which type is allowed to write it.
- **Responsibility**: The field, the three operations on it, the ordering pair they form, and where the write set actually lives.
- **In Scope**: `Shutdown::closed`, `Shutdown::is_closed`, `Shutdown::close`, `Stopped::reopen`.
- **Out of Scope**: That exactly one such flag exists family-wide (→ [`../invariant/001`](../invariant/001_exactly_one_liveness_flag.md)); what the flag guarantees once read (→ [`../pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)).

### Layout

The whole of this crate's state:

```rust
pub struct Shutdown
{
  closed : AtomicBool,
}
```

One `bool`'s worth of shared mutable state, held by reference. Everything else
the crate exposes — `Stopped`, `Guarded`, `Refusal`, `Wake` — is either a
borrow of this or a value derived from reading it.

### Orderings

Three operations touch the field, forming one `Release`/`Acquire` pair:

| Operation | Ordering | What it publishes or observes |
|---|---|---|
| `Shutdown::is_closed` | `Acquire` | Everything the closing thread wrote before closing |
| `Shutdown::close` | `Release` | Everything this thread wrote before deciding to stop |
| `Stopped::reopen` | `Release` | Everything the draining thread wrote before reopening |

The pair is the point. A producer that observes `true` also observes the writes
that preceded the close, which is what lets a caller stage teardown state
before flipping the flag and rely on a reader seeing both together.

`reopen` is `Release` for the symmetric reason on the way back: a consumer that
drained the ring and then reopens it must publish the drained state before the
next producer can observe an open ring.

**The pair rests on review, and that is a choice rather than an omission.** The
field is `core::sync::atomic::AtomicBool`, not `ring_atomic`'s, so `loom` cannot
interleave it and no model in the family can reach it (→ SD9 below). Routing it
through `ring_atomic` would make it modellable and would cost a fifth runtime
edge on a crate whose
[`integration/001`](../integration/001_family_dependency_seam.md) measures its
closure at four in-house edges and no external crate. The edge is not forbidden
there — the rule that governs it is that an edge must be justified by a named
item, and `AtomicBool` is a named item — so the honest statement is that the
trade has been weighed and declined for now: one ordering pair, three call
sites, all in one file, against a dependency that exists to instrument
contention this crate does not have. What makes it reversible is that nothing
depends on the flag being `core`'s; the day this crate grows a second atomic or
a contended one, the edge is the first thing to add.

### Where the Write Set Lives

**The two writes are on two different types.** `Shutdown::close` sets the flag
true; `Stopped::reopen` sets it false, by reaching into `self.shutdown.closed`
directly. `Shutdown` has no `reopen` method — the only way to clear its own
private field is through a `Stopped` token.

That is deliberate and it is what makes the proof token work
(→ [`../type/001`](../type/001_stopped_proof_token.md)): if `Shutdown::reopen`
existed, the token would prove nothing, because the flag could be cleared
without consuming it. The cost is that the field's write set spans two `impl`
blocks, and Rust's module-level privacy is what permits it — `closed` is
private to the crate's one module, not to `Shutdown`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the whole of the state:        %s\n' "$( awk '/^pub struct Shutdown$/{f=1} f&&/^\}$/{exit} f&&/:/' ring_shutdown/src/lib.rs | sed 's/^ *//' | tr -d ',' )"
printf 'operations touching it:        %s\n' "$( command grep -cE 'closed\.(load|store)' ring_shutdown/src/lib.rs )"
printf 'the orderings they use:        %s\n' "$( command grep -ohE 'Ordering::[A-Za-z]+' ring_shutdown/src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'which impl writes it true:     %s\n' "$( awk '/^impl/{t=$0} /closed\.store\( true/{ sub(/^impl(< [^>]+ >)? /,"",t); print t }' ring_shutdown/src/lib.rs )"
printf 'which impl writes it false:    %s\n' "$( awk '/^impl/{t=$0} /closed\.store\( false/{ sub(/^impl(< [^>]+ >)? /,"",t); print t }' ring_shutdown/src/lib.rs )"
printf 'a reopen method on Shutdown:   %s\n' "$( awk '/^impl Shutdown$/{f=1} f&&/^\}$/{exit} f' ring_shutdown/src/lib.rs | command grep -c 'fn reopen' || true )"
printf 'the atomic comes from:         %s\n' "$( command grep -ohE 'use [a-z_:]+::atomic' ring_shutdown/src/lib.rs | sed 's/use //' )"
printf 'ring_atomic in this manifest:  %s\n' "$( command grep -c 'ring_atomic' ring_shutdown/Cargo.toml || true )"
printf 'family crates that do use it:  %s\n' "$( command grep -l 'ring_atomic' ring_*/Cargo.toml | sed 's|/Cargo.toml||' | tr '\n' ' ' )"
printf 'files running a loom model:    %s\n' "$( command grep -rl 'loom::model' ring_*/tests ring_*/src 2>/dev/null | wc -l )"
printf 'of those, naming Shutdown:     %s\n' "$( command grep -rl 'loom::model' ring_*/tests ring_*/src 2>/dev/null | xargs command grep -lc 'Shutdown' 2>/dev/null | wc -l )"
printf 'is the tradeoff recorded here: %s\n' "$( awk '/^### Orderings/{f=1} f&&/^### Where/{exit} f' ring_shutdown/docs/data_structure/001_the_close_flag_and_its_orderings.md | command grep -c 'ring_atomic' )"
printf 'times integration/001 names it: %s\n' "$( command grep -c 'ring_atomic' ring_shutdown/docs/integration/001_family_dependency_seam.md || true )"
```

Live output:

```
the whole of the state:        closed : AtomicBool
operations touching it:        3
the orderings they use:        Ordering::Acquire Ordering::Release 
which impl writes it true:     Shutdown
which impl writes it false:    Stopped< 'a >
a reopen method on Shutdown:   0
the atomic comes from:         core::sync::atomic
ring_atomic in this manifest:  0
family crates that do use it:  ring_atomic ring_batch ring_cursor ring_debug ring_mpsc ring_spsc ring_tls 
files running a loom model:    29
of those, naming Shutdown:     0
is the tradeoff recorded here: 2
times integration/001 names it: 0
```

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_stopped_proof_token.md`](../type/001_stopped_proof_token.md) | Why the write set is split across two types rather than gathered on one |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_exactly_one_liveness_flag.md`](../invariant/001_exactly_one_liveness_flag.md) | That this field is the family's only one, and how that is checked |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The field, the three access sites, and the two `impl` blocks that write it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it` — the only test that drives the flag through both writes |

### SD9 — The Family's One Liveness Flag Is the One Atomic Its Loom Models Cannot Reach

[`invariant/001`](../invariant/001_exactly_one_liveness_flag.md) establishes
that the family holds exactly one "is this ring closed" flag and that it is this
one. The same field is a `core::sync::atomic::AtomicBool`, taken directly from
`core` rather than from `ring_atomic`, and this crate's manifest does not name
`ring_atomic` at all.

That matters because `ring_atomic` is what the family's loom instrumentation
swaps. Seven crates depend on it; twenty-nine files in the family run a
`loom::model`, and **none of them names `Shutdown`**. So the family's one
liveness flag — the single piece of shared mutable state whose `Release`/`Acquire`
pair is this crate's only ordering decision — is structurally outside the reach
of every model the family runs.

The gap is not that a model was forgotten. It is that writing one would not
help: `loom` can only interleave atomics it controls, and this atomic is not one
of them. Making it modellable means routing the field through `ring_atomic` —
and the finding as first written said `integration/001` *"argues against"* that
edge *"on other grounds"*, which is not true. That document does not mention
`ring_atomic` anywhere; what it carries is a general rule, that an edge must be
justified by a named item rather than by a topic, and `AtomicBool` is a named
item. The rule would admit the edge, not refuse it. So the choice was real and
unrecorded, and the correction is that nothing was arguing against it except the
absence of anyone weighing it.

What is checked today is that the flag is *unique*. What is not checked, by
tests or by models, is that its two `Release` stores and one `Acquire` load are
the right orderings. That rests on review alone, for the one field in the family
where the family already built the tool for something better.

That is now stated where a reader meets the orderings rather than only here at
the end, and stated as a trade with a reversal condition — one uncontended pair
across three call sites in one file, against a fifth runtime edge on a crate
whose integration document measures its closure at four. The finding's
substance stands: the pair is unmodelled. What changed is that it is unmodelled
on the record, with the price of changing that named, instead of by default.

**Disposition:** applied — the Orderings section now records the
`core::sync::atomic` versus `ring_atomic` trade and its reversal condition, and
the miscited support for it is corrected above; the recipe measures both the
record's presence and the citation that was wrong.
Now prints: `times integration/001 names it: 0`

### SD10 — The Field Is Private to the Module, Not to Its Struct, and the Doc Says Neither

`Shutdown::close` writes the flag `true`. `Stopped::reopen` writes it `false` —
not by calling a method on `Shutdown`, which has none, but by reaching through
`self.shutdown.closed` into another struct's private field. Rust permits this
because privacy is module-scoped and both types live in the same module; it
would not compile if `Stopped` were moved to a file of its own.

The arrangement is correct and load-bearing: a `Shutdown::reopen` method would
let the flag be cleared without consuming the token, and the token's second
property is exactly that it cannot be
(→ [`../type/001`](../type/001_stopped_proof_token.md)). So the split is the
mechanism, not an accident.

What is missing is that nothing says so at the site. `Stopped::reopen`'s doc
comment explains why it takes `self` by value; it does not mention that it is
writing another type's private state, or that this is why `Shutdown` deliberately
lacks the mirror method. A maintainer tidying the crate into `shutdown.rs` and
`stopped.rs` — the ordinary next step as a module grows — gets a privacy error
with no explanation in view, and the obvious repair is to add the
`Shutdown::reopen` that dissolves the guarantee.
