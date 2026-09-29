# Pattern: Register Before First Append

### Scope

- **Purpose**: State the ordering rule that closes the window in which appends succeed and are silently discarded, and explain why the rule cannot be enforced from inside the append path.
- **Responsibility**: Give the problem its precise shape, the solution's two forms, when each applies, and what each costs.
- **In Scope**: The ordering between a thread's registration and its first append; the structural options for enforcing it.
- **Out of Scope**: What registration *is* mechanically, which is unresolved (→ [Registration State](../lifecycle/004_registration_state.md)); the thread's later teardown, a separate ordering problem (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)).

### Problem

A thread-local initializes on first access. The natural implementation of a
per-thread append log therefore allocates its region during the first
`append` call — which means that append lands in a buffer no consolidator
knows about.

The failure has three properties that together make it the worst shape a bug
can take:

1. **It succeeds.** The append writes the record correctly into a valid
   region. Nothing returns an error, nothing panics, nothing logs.
2. **It is invisible downstream.** A buffer absent from the registry
   consolidates to nothing, and a buffer with no records also consolidates to
   nothing. The consolidator cannot tell them apart
   (→ [Registration State](../lifecycle/004_registration_state.md)'s
   Behavioral Invariant 4).
3. **It is timing-dependent in the direction that hides it.** On a thread
   that lives a long time and appends steadily, the lost window is a handful
   of records at startup out of millions — easily dismissed as noise if
   noticed at all. On a short-lived worker thread it can be *everything the
   thread ever wrote*, which is when it finally becomes visible, in
   production, under load.

The append path cannot check for this. A branch verifying registration on
every append would put a check on the hot path whose only purpose is catching
a setup error that is the same on every call — paying per-record for a
per-thread mistake, in the one code path the entire crate exists to keep
free of overhead
(→ [Writer Append Surface](../api/001_writer_append_surface.md)).

### Solution

**Register the thread's buffer, then append. Never rely on lazy
initialization to do both.** Two forms, differing in whether the rule is
enforced or merely documented:

**Form A — explicit registration call, documented rule.**

```rust
// once per thread, before any append
let writer = tls_log.register();
// … thereafter
writer.append( tag, payload );
```

Registration returns the writer. There is no way to obtain a writer without
registering, so the ordering is enforced by the fact that `append` is a
method on a value only `register` produces. The rule becomes structural
rather than documentary, at the cost of the consumer having to thread the
writer through its own call graph.

**Form B — implicit thread-local accessor.**

```rust
// no setup; the accessor registers on first use
tls_log::append( tag, payload );
```

Ergonomic, and the shape a consumer will reach for. It requires the accessor
itself to register-then-append atomically on the first call, which is
possible — but it reintroduces the per-call branch the append path was trying
to avoid, unless the branch is the thread-local's own initialization check,
which it already pays.

**Form A is the one this pattern recommends**, because Form B's convenience
is exactly the property
[Implicit Thread-Locals Are Hidden State](../pitfall/001_implicit_thread_locals_are_hidden_state.md)
identifies as this crate's characteristic trap: a call with no visible
receiver hides that there is per-thread state at all, which is how the
registration question gets skipped in the first place.

### Applicability

| Situation | Applies? |
|-----------|----------|
| Long-lived worker threads created by the consumer | Yes — cheap, and the registration point is obvious |
| Short-lived threads that append and exit | **Critically** — this is the case where violation loses everything |
| Threads from a third-party pool the consumer does not create | Yes, and hardest: there may be no natural registration point, which is an argument for Form B despite its costs |
| A single-threaded consumer | Technically yes, and trivially satisfied; the pattern costs nothing to follow |
| `async` tasks rather than OS threads | **Unclear, and this is a real gap.** A task migrating between executor threads mid-append would violate the single-writer invariant outright, not merely the registration ordering. Nothing read here establishes whether the consumers are `async`; if any is, this crate's whole model needs re-examination rather than this pattern needing an extra row |

**The last row is worth flagging rather than burying.** Thread-local
buffering assumes a stable thread identity per unit of work. `async` tasks do
not have one. This is not a limitation of the pattern; it is a limitation of
the crate, surfaced here because the applicability question is where it
naturally appears.

### Consequences

- **The rule closes the loss window entirely** when followed. There is no
  residual risk, no race, no partial mitigation — registration before first
  append is sufficient.
- **Form A makes the rule unenforceable-to-violate at the cost of
  ergonomics.** The consumer carries a writer value. In a deep call graph
  that means threading it through, or storing it in the consumer's own
  thread-local — which is Form B's problem relocated rather than solved.
- **Neither form helps a thread that never registers because nobody thought
  to.** If registration is the consumer's responsibility and the consumer
  does not know it exists, the pattern is documentation nobody read. That is
  an argument for Form A being the *only* form exposed: an API with no
  unregistered path cannot be used wrong.
- **This pattern's rule and its teardown counterpart are the same shape.**
  Register before first append, deregister after last consolidation. Both are
  unenforceable orderings around the buffer's life, both lose data silently
  when violated. A consumer following one and not the other has fixed half
  the problem (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)'s
  Cleanup Requirement 1).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | Its third error row is this problem; Form A is the structural remedy it names and does not choose |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | The applicability table's `async` row is a threat to this invariant, not merely to this pattern |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | Its N1 is the transition this pattern orders correctly; its N3 is the mirror-image rule |

### Patterns

| File | Relationship |
|------|--------------|
| [002_staging_then_merge.md](002_staging_then_merge.md) | The composition this rule is a precondition of — an unregistered stage contributes nothing to the merge |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | Why Form B's ergonomics are the trap rather than the reward |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_registration_state.md](../lifecycle/004_registration_state.md) | Its Unregistered state is this problem; its Behavioral Invariant 1 is this rule |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — neither form declared yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registration_pattern_test.rs` (to create) | Under Form A, no code path reaches `append` without a prior `register` — asserted as a `compile_fail` doc test rather than a runtime check |

### TL43 — The Ordering Rule Protects a Window the Built Crate Does Not Have

The pattern is sound for the specified design, where a thread could append
into a buffer no consolidator knew about. Here `with_capacity` both creates and
makes usable, so there is no unregistered-but-appendable state
(→ [`../lifecycle/004`](../lifecycle/004_registration_state.md)).

It is filed as misleading rather than wrong because a reader who follows it
loses nothing — there is simply no `register` to call.

**Disposition:** declined — the register-then-append ordering rule this
pattern specifies (Form A/Form B, the loss-window problem) protects a window
the built crate does not have, since `with_capacity` both creates and makes
usable with no unregistered-but-appendable state. One of the nineteen
pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs; whether to rewrite it or relocate it toward a prospective
consumer's own registration need is a future pass's call.
