# Pitfall: Close Is Advisory to an Unguarded Producer

### Scope

- **Purpose**: State this crate's central limitation as a limitation — `close` stops nothing on its own, and a caller holding a raw `ring_core::Producer` publishes through it without complaint.
- **Responsibility**: The trap's shape, why it cannot be closed at this layer, what actually mitigates it, and what the mitigation does not reach.
- **In Scope**: `Shutdown::close`, `Shutdown::admit`, `Guarded`, `Guarded::into_inner`.
- **Out of Scope**: The overflow-policy trap, which is a different failure with a different mitigation (→ [`pitfall/002`](002_ok_does_not_mean_kept_under_drop_newest.md)).

### Trap

`close` sets an `AtomicBool`. That is all it does. It does not reach into the
ring, does not revoke a producer, and cannot — `ring_core::Ring` has no flag to
set and deliberately never will (→
[`invariant/001`](../invariant/001_exactly_one_liveness_flag.md)).

So this compiles, runs, and publishes into a ring the caller has just closed:

```rust
let shutdown = Shutdown::new();
shutdown.close();

producer.try_push( record ).unwrap();   // a raw ring_core::Producer. Ok( () ) — not necessarily accepted (→ pitfall/002).
```

**The close is real; the enforcement is not.** Every guarantee this crate makes
about publication stopping is a guarantee about `Guarded`, and `Guarded` is
something a caller opts into.

The sharp version is not the obvious one. Nobody writes the snippet above
deliberately. What happens is that a `Guarded` is unwrapped somewhere — for a
call that needs the raw type, for a helper with the wrong signature, by
`into_inner` — and the raw producer outlives the reason it was taken. From that
point the close is decoration.

### Failure

| Carried assumption | Failure | Visibility |
|---|---|---|
| `close()` stops publication | A raw producer keeps publishing; teardown never sees an empty ring | **Load-dependent** — a slow producer stops on its own before anyone notices |
| `drain_all` terminates | It does not, if a raw producer outpaces it: `try_recv_batch` never returns 0 | **Hang**, at teardown, with no error |
| An unwrapped producer is a local matter | It is not — unwrapping once disables the guarantee for the whole ring | Invisible at the unwrap site; the failure is elsewhere |

Row 2 is the one that costs a debugging session. `drain_all`'s termination
argument (→ [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md))
is *"publication has stopped, so the remainder is bounded"* — and publication
stopping is precisely what this trap invalidates. The loop is correct; its
premise is not.

### Mitigation

1. **Hold a `Guarded`, not a `Producer`.** Its only push consults the flag, and
   there is no unchecked path on it. This is the whole mitigation, and it works
   by removing the operation rather than by adding a check.

2. **Treat `into_inner` as a scope exit, not a conversion.** If a raw producer
   is genuinely needed, take it, use it, and drop it in the same scope — never
   store it. Whether the method should exist at all is genuinely open (→
   [`decisions/001`](../decisions/001_should_into_inner_exist.md)).

3. **`admit` for a call site that cannot be wrapped.** A caller holding a raw
   producer for an unavoidable reason can perform the check explicitly. This is
   strictly worse than mitigation 1 — it is a rule again, not a guarantee — but
   it is better than nothing and it is one line.

**What does not mitigate it: making `Guarded` the only way to get a producer.**
This crate does not construct producers; `ring_core::Ends::split` does, one
layer down and without knowing this crate exists. Closing the hole entirely
would mean moving the flag into `ring_core`, which is the failure
[`invariant/001`](../invariant/001_exactly_one_liveness_flag.md) exists to
prevent. The hole is the price of the invariant, and it is the right trade —
but it is a price, and this document is where it is recorded rather than
implied.

**What does not mitigate it: a test.** `an_unguarded_producer_publishes_straight_through_a_close`
asserts the trap rather than guarding against it. That is deliberate: a test
that failed when a raw producer published would be asserting a guarantee the
crate does not make, and it would go red the moment someone read the code
correctly.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/pitfall/001_close_is_advisory_to_an_unguarded_producer.md
printf 'what the trap snippet claims:  %s\n' "$( awk '/^### Regenerate/{ exit } /\.unwrap\(\);/{ sub( /^.*\/\/ /, "" ); print }' $D )"
printf 'what 002 says of that return:  %s\n' "$( command grep -oh 'Ok( () ),  and the record is gone' docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md )"
printf 'how this doc classifies 002:   %s\n' "$( awk '/^### Regenerate/{ exit } /Out of Scope/{ sub( /.*Out of Scope\*\*: /, "" ); print }' $D )"
printf 'rows in the failure table:     %s\n' "$( awk '/^### Failure/{f=1} f&&/^### Mitigation/{exit} f&&/^\|---/{ h=1; next } f&&h&&/^\| /{ n++ } END{ print n+0 }' $D )"
printf 'the row it calls expensive:    %s\n' "$( awk '/^### Regenerate/{ exit } /costs a debugging session/{ print }' $D | sed 's/\..*//' )"
printf 'mitigations it lists:          %s\n' "$( awk '/^### Mitigation/{f=1} f&&/^### Regenerate/{exit} f&&/^[0-9]\. \*\*/{ n++ } END{ print n+0 }' $D )"
printf 'of those, bounding the drain:  %s\n' "$( awk '/^### Mitigation/{f=1} f&&/^### Regenerate/{exit} f&&/^[0-9]\. \*\*/ && /drain/{ n++ } END{ print n+0 }' $D )"
printf 'unbounded drain loops in src:  %s\n' "$( command grep -c 'while taken > 0\|while let Some( record )' src/lib.rs )"
printf 'waits taking a spin budget:    %s\n' "$( command grep -c 'spins : usize' src/lib.rs )"
printf 'drains taking a budget:        %s\n' "$( command grep -c 'budget : usize' src/lib.rs || true )"
printf 'what the bounded drain returns: %s\n' "$( awk '/pub fn drain_all_bounded/{f=1} f&&/-> Result/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'tests pinning both its arms:   %s\n' "$( command grep -c 'drain_all_bounded' tests/shutdown_test.rs || true )"
printf 'real into_inner call sites:    %s\n' "$( cd ..; command grep -rn 'into_inner()' ring_*/src ring_*/tests --include='*.rs' | command grep -c 'guarded' || true )"
printf 'crates outside naming Guarded: %s\n' "$( cd ..; command grep -rl 'Guarded' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | command grep -vc ring_shutdown || true )"
printf 'crates outside holding a guard: %s\n' "$( cd ..; command grep -rl '\.guard( ' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | command grep -v ring_shutdown | tr '\n' ' ' )"
```

Live output:

```
what the trap snippet claims:  a raw ring_core::Producer. Ok( () ) — not necessarily accepted (→ pitfall/002).
what 002 says of that return:  Ok( () ),  and the record is gone
how this doc classifies 002:   The overflow-policy trap, which is a different failure with a different mitigation (→ [`pitfall/002`](002_ok_does_not_mean_kept_under_drop_newest.md)).
rows in the failure table:     3
the row it calls expensive:    Row 2 is the one that costs a debugging session
mitigations it lists:          3
of those, bounding the drain:  0
unbounded drain loops in src:  2
waits taking a spin budget:    2
drains taking a budget:        1
what the bounded drain returns: -> Result< usize, RingError >
tests pinning both its arms:   3
real into_inner call sites:    1
crates outside naming Guarded: 0
crates outside holding a guard: ring_testkit 
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | The two rows in its guarantee column that read **convention** |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_liveness_flag.md](../invariant/001_exactly_one_liveness_flag.md) | The invariant this trap is the cost of |
| [../invariant/002_drain_terminates_because_close_preceded_it.md](../invariant/002_drain_terminates_because_close_preceded_it.md) | The termination argument this trap invalidates |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_should_into_inner_exist.md](../decisions/001_should_into_inner_exist.md) | Whether mitigation 2's escape hatch should be removed outright |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `an_unguarded_producer_publishes_straight_through_a_close` — the trap, asserted as fact |
| `tests/shutdown_test.rs` | `a_guarded_producer_refuses_a_closed_ring_and_returns_the_record` — mitigation 1 |

### SD41 — The Trap's Own Snippet Annotates a Return Value the Next Document Exists to Refute

The five lines that demonstrate this trap end with:

```rust
producer.try_push( record ).unwrap();   // a raw ring_core::Producer. Accepted.
```

The comment is the payload of the whole document — *the close did not stop
this* — and it says one word too many. [`002`](002_ok_does_not_mean_kept_under_drop_newest.md)
exists to establish that this exact call, under `RingConfig`'s default
`OverflowPolicy::DropNewest`, returns `Ok( () )` for a record that was
discarded. On a full ring the snippet's `unwrap()` succeeds and nothing was
accepted.

That does not weaken the trap — a raw producer publishing into a closed ring is
the point, and on a ring with room it does publish. It makes the snippet
demonstrate **two** traps while annotating one, and the annotation asserts the
proposition its sibling document was written to remove.

The two documents are filed as disjoint. This one's Out of Scope bullet calls
the other *"a different failure with a different mitigation"*, and read as
mitigations they are: hold a `Guarded` here, choose `OverflowPolicy::Fail`
there. Read as *failures* they compose, and this snippet is where they compose:
an unguarded producer under the default policy has two independent ways to
report a publication that did not happen, and a reader who applies only this
document's mitigation still has the other one.

The general shape is that a pitfall directory partitions by *cause* and a caller
meets failures by *symptom*. Both documents are correct in isolation; neither
owns the composition, and the composition is the state a default-configured
crate is actually in.

**Disposition:** applied — the trap snippet's comment no longer claims
unconditional acceptance; it now reads "Ok( () ) — not necessarily accepted
(→ pitfall/002)", which states only what `unwrap()` succeeding actually
proves and points at the document that shows the same `Ok` can mean
discarded. The finding's own quotation of the original comment (above,
under "The five lines that demonstrate this trap end with:") is left
unchanged as the historical record of what was found. Now prints: `not
necessarily accepted (→ pitfall/002)`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
# Content-addressed rather than line-addressed, and -m1 rather than a census:
# this document carries the same snippet line three times -- the live trap, the
# historical record of the comment it used to have, and this block's own quoted
# output -- so an absolute address moves whenever the document is edited, and an
# unbounded match would append its own previous answer on every sweep. The trap
# snippet opens the document, so the first match is always the live one.
command grep -m1 -F 'producer.try_push( record ).unwrap();' \
  docs/pitfall/001_close_is_advisory_to_an_unguarded_producer.md
```

Live output:

```
producer.try_push( record ).unwrap();   // a raw ring_core::Producer. Ok( () ) — not necessarily accepted (→ pitfall/002).
```

### SD42 — The Failure This Document Calls Expensive Was the Only One With No Mitigation, and the Only Loop With No Budget

The Failure table has three rows, and the prose immediately under it names row
2 — `drain_all` failing to terminate — as *"the one that costs a debugging
session"*. Three mitigations follow. All three prevent a raw producer from
existing or from publishing; none bounds the loop. A caller who already has the
raw producer, which is the situation every mitigation is written to have
avoided, gets nothing.

That would be an unavoidable gap if the crate had no way to bound a loop. It
has one, twice. `wait_for_close` and `for_space_or_close` each take
`spins : usize`, hand it to `ring_wait`, and return `Err( RingError::Empty )` or
`Err( RingError::Full )` when the budget runs out rather than spinning forever
— the crate's stated position on unbounded waiting, applied to the two
operations that were never the hazard. `drain_all` and `discard_all` take no
budget and return `usize`, so there is no value in which exhaustion could be
reported even if it were detected.

The asymmetry is worth stating precisely because it inverts the risk. The two
budgeted functions wait on a **flag another thread sets**, and a caller who gets
`Empty` back learns something actionable. The two unbudgeted ones loop on a
**ring another thread fills**, which is the case this document says hangs, and
they are the ones with no exit. `ring_wait` is already a dependency; the
machinery is imported, used, and pointed at the wrong two functions.

What this costs is not the hang itself — the hang is the honest consequence of
the invariant (→ [`invariant/001`](../invariant/001_exactly_one_liveness_flag.md))
and the document says so. It was that the hang was indistinguishable from a slow
drain, forever, with no diagnostic. A `drain_all_bounded( consumer, out, budget )`
returning `Err( Empty )` turns the crate's worst documented outcome into a
value, and its absence was not recorded as a decision anywhere — the two
decisions on file are about `into_inner` and about token uniqueness.

So it was added rather than deferred, because the argument for deferring is
usually "we do not know the shape yet" and here the shape was already written
twice in the same file. `drain_all_bounded` spends its budget the way
`ring_wait::wait_until` spends `spins`, including reading zero as one attempt
rather than none, so a caller computing the number cannot accidentally ask for
no work at all. Records already moved stay in `out` when the budget runs out, so
`Err` costs the caller the verdict and not the data.

`drain_all` is deliberately still here and still unbounded. It is the right call
against a `Stopped` ring with no live producer — the case the reached-test
exercises — where a budget would be a number the caller has to invent for a loop
that provably terminates. What changed is that the *unbounded* form is now a
choice with an alternative rather than the only thing on offer.

**Disposition:** applied — `Stopped::drain_all_bounded( consumer, out, budget )`
returns `Ok( n )` once the ring is observed empty and `Err( RingError::Empty )`
when the budget is exhausted first; `a_bounded_drain_separates_finishing_from_running_out`
pins both arms and the zero-budget reading against one ring.
Now prints: `drains taking a budget:        1`
