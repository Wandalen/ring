# Lifecycle: Teardown and Reuse

### Scope

- **Purpose**: Describe the close→drain→reopen path end to end, and state precisely what "the ring afterwards is fit for the next run" does and does not promise.
- **Responsibility**: The path, what `reset` composes, the promise, and how the promise is asserted rather than claimed.
- **In Scope**: `close`, `drain_all`, `discard_all`, `reopen`, `reset`.
- **Out of Scope**: Construction and backend selection (→ [`ring_core/docs/lifecycle/001`](../../../ring_core/docs/lifecycle/001_construction_and_backend_selection.md)); handle lifetimes (→ [`ring_core/docs/lifecycle/002`](../../../ring_core/docs/lifecycle/002_ends_split_and_handle_lifetimes.md)).

### Lifecycle Phases

```
      ┌─ close() ──────────► Stopped
      │                         │
      │                    drain_all( &mut consumer, &mut out )   records recovered
      │                         │   or
      │                    discard_all( &mut consumer )           records dropped
      │                         │
      └─────────────────── reopen()  ──────► ring is Open and empty
```

`reset` is that path with `discard_all` in the middle, in one call:

```rust
pub fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '_, T > ) -> usize
{
  let stopped = shutdown.close();
  let discarded = stopped.discard_all( consumer );
  stopped.reopen();
  discarded
}
```

**Why `reset` discards rather than drains.** A reset whose records had to go
somewhere would need the caller to supply a sink at teardown, which is the
moment they least want one. The count comes back so a caller who cares can
notice a non-zero teardown — and a caller who wants the records calls the three
steps explicitly.

### Phase Transitions

Four edges, and what each one costs to take:

| From | Edge | To | Repeatable? |
|---|---|---|---|
| Open or Closed | `close()` | Closed, holding a `Stopped` | **yes** — a second close returns a fresh, usable token |
| Closed | `drain_all` / `discard_all` | Closed, ring empty | yes — a second call returns `0` |
| Closed | `reopen()` | Open | **no** — takes `self`, so the proof is consumed |
| Open or Closed | `reset()` | Open, ring empty | yes — its contract is the end state, not the transition |

**The middle phase is the only optional one.** A caller may close and reopen
without draining, which leaves the records in place — that is a legitimate pause,
not a broken teardown, and nothing in the crate forbids it. What the type system
does forbid is the reverse: draining without having closed first
(→ [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)).

**`reopen` consuming the token is what makes the path one-way per close.** After
it, a further drain needs a further close, so no code can drain a ring that has
already been handed back to producers.

### Dependencies

The path needs three things from outside this crate, and notably not a config or
a constructor — it operates on a ring somebody else built:

| Needed | For which phase | Why it cannot be avoided |
|---|---|---|
| `ring_core::Consumer< '_, T >` | drain / discard | The records come out through the ring's own consumer; this crate owns no storage |
| `ring_types::RingError` | the refusals along the way | One family vocabulary rather than a second |
| Every producer being `Guarded` | the termination argument | A raw producer publishing across the close makes the drain phase unbounded (→ [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)) |

The third row is a dependency on the *caller's* discipline rather than on a
crate, which is exactly why it is listed here — it is the one prerequisite
`Cargo.toml` cannot express and the compiler cannot check.

### Cleanup Requirements

The reason this path exists at all is that state must not leak between runs:
rings carry state between cases, and a leftover from one case makes failures
appear in whichever test happens to run after the one that actually broke.

So the promise is **behavioural indistinguishability, not bitwise identity**:

| Promised | Not promised |
|---|---|
| Accepts a full capacity again | Cursors return to zero |
| Delivers in the same order a fresh ring does | Sequence numbers restart |
| Reports the same occupancy readings | The same memory contents in unoccupied slots |
| Reuses the same allocation — no realloc | That any backend's internal counters reset |

The right-hand column is not a weakness. The in-house backends are
sequence-stamped, and a sequence number is only ever compared relatively — a
ring whose cursors sit at 4 behaves exactly as one whose cursors sit at 0. A
reset that insisted on the left-hand column *and* the right would have to
rebuild the ring, which loses the allocation reuse that is the feature's other
half.

#### How the promise is asserted

Behavioural indistinguishability is a claim about two things being the same, so
the test uses two things. `the_three_operations_hand_back_a_ring_fit_for_the_next_run`
builds a **reference ring** alongside the reset one, drives both through the
same script, and compares outputs:

```rust
let mut used  = ring( 4 );   // dirtied, closed, drained, reopened
let mut fresh = ring( 4 );   // never touched

for ( ring, out ) in [ ( &mut used, &mut from_used ), ( &mut fresh, &mut from_fresh ) ] { … }

assert_eq!( from_used, from_fresh );
```

This is deliberately not an assertion against a remembered constant. A constant
encodes what the author expected a fresh ring to do; the reference ring encodes
what one *does*. If the fresh ring's behaviour ever changes, the test compares
the new behaviour against itself and keeps testing the property that matters.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'what reset composes:           %s\n' "$( awk '/^pub fn reset</{f=1} f&&/^\}$/{exit} f&&/stopped\.|shutdown\./{ sub( /^ */, "" ); printf "%s ", $0 }' ring_shutdown/src/lib.rs )"
printf 'the named reached-test:        %s\n' "$( command grep -o 'fn the_three_operations[a-z_]*' ring_shutdown/tests/shutdown_test.rs )"
printf 'reset calls inside it:         %s\n' "$( awk '/^fn the_three_operations/{f=1} f&&/^\}$/{exit} f&&/reset\(/{ n++ } END{ print n+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'what it composes by hand:      %s\n' "$( awk '/^fn the_three_operations/{f=1} f&&/^\}$/{exit} f' ring_shutdown/tests/shutdown_test.rs | command grep -ohE 'close\(\)|drain_all|discard_all|reopen\(\)' | tr '\n' ' ' )"
printf 'reference rings it builds:     %s\n' "$( awk '/^fn the_three_operations/{f=1} f&&/^\}$/{exit} f&&/= ring\( /{ n++ } END{ print n+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'the test that does call reset: %s\n' "$( command grep -o 'fn reset_discards[a-z_]*' ring_shutdown/tests/shutdown_test.rs )"
printf 'reference rings it builds:     %s\n' "$( awk '/^fn reset_discards/{f=1} f&&/^\}$/{exit} f&&/= ring\( /{ n++ } END{ print n+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'must_use attributes in src:    %s\n' "$( command grep -c '#\[ must_use' ring_shutdown/src/lib.rs )"
printf 'the fns they sit above:        %s\n' "$( awk '/#\[ must_use/{ mu=1; next } /pub (const )?fn /{ if(mu){ sub( /.*fn /, "" ); sub( /\(.*/, "" ); printf "%s ", $0 } mu=0 }' ring_shutdown/src/lib.rs )"
printf 'is close among them:           %s\n' "$( awk '/#\[ must_use/{ mu=1; next } /pub (const )?fn /{ if(mu){ sub( /.*fn /, "" ); sub( /\(.*/, "" ); print } mu=0 }' ring_shutdown/src/lib.rs | command grep -cx 'close' || true )"
printf 'family sites discarding it:    %s\n' "$( command grep -rn 'shutdown\.close();' ring_*/src ring_*/tests --include='*.rs' | command grep -cvE '=[^;]*shutdown\.close\(\);' || true )"
printf 'family sites stating it:       %s\n' "$( command grep -rc 'let _ = shutdown\.close();' ring_*/src ring_*/tests --include='*.rs' | command grep -v ':0$' | tr '\n' ' ' )"
```

Live output:

```
what reset composes:           let stopped = shutdown.close(); let discarded = stopped.discard_all( consumer ); stopped.reopen(); 
the named reached-test:        fn the_three_operations_hand_back_a_ring_fit_for_the_next_run
reset calls inside it:         0
what it composes by hand:      close() drain_all reopen() 
reference rings it builds:     2
the test that does call reset: fn reset_discards_and_leaves_the_ring_open
reference rings it builds:     1
must_use attributes in src:    12
the fns they sit above:        new is_closed close shutdown into_record is_closed reason free_capacity is_blocked shutdown is_ready 
is close among them:           1
family sites discarding it:    0
family sites stating it:       ring_shutdown/src/lib.rs:5 ring_testkit/src/lib.rs:1 ring_shutdown/tests/shutdown_test.rs:7 
```

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/002_open_and_closed.md](../lifecycle/002_open_and_closed.md) | The same path as states and edges, with the idempotence table |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | Guarantee 3 — `reset` ends open however it started |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_drain_to_empty.md](../algorithm/001_drain_to_empty.md) | The middle step, and why one batch is not enough |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `reset`'s composition of `close`, `discard_all`, and `reopen` into the one-call teardown |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `the_three_operations_hand_back_a_ring_fit_for_the_next_run` — the reached-test, with the reference ring |
| `tests/shutdown_test.rs` | `reset_discards_and_leaves_the_ring_open` — `reset`'s own contract, from both start states |

### SD29 — The Reached-Test for the Feature Never Calls the Function the Feature Names

`the_three_operations_hand_back_a_ring_fit_for_the_next_run` is cited above as
*"the reached-test, with the reference ring"*, and it is the only test in the
crate that builds two rings and compares them — the whole apparatus this
document's "How the promise is asserted" section describes. It makes **zero**
calls to `reset`.

What it composes by hand is `close()`, `drain_all`, `reopen()`. What `reset`
composes is `close()`, **`discard_all`**, `reopen()`. The middle step differs,
and the difference is not incidental: this document's own "Why `reset` discards
rather than drains" paragraph is an argument that these are *different
operations for different callers*. The reference-ring machinery is spent on the
composition a caller writes out; the composition the crate ships is validated by
`reset_discards_and_leaves_the_ring_open`, which builds **one** ring, checks the
discard count and the flag, and compares nothing.

So "behavioural indistinguishability" — the promise this document exists to
state, the one the Cleanup Requirements table splits into promised and
not-promised columns — is asserted for a path that is not `reset`. A
regression in `discard_all` that left the ring subtly
unfit would pass both tests: the reference-ring test does not call it, and the
`reset` test does not compare.

The fix is one line — call `reset` in the dirty phase of the reference-ring
test instead of spelling out the three steps — and the reason it was not is
visible in the test's own name: it was written to prove *the three operations*,
which is the API surface, rather than *the operation*, which is the feature.

### SD30 — The Proof Token Was Not `must_use` While Nine Accessors Were

`src/lib.rs` carried nine `#[ must_use ]` attributes, on `Shutdown::new`,
`Shutdown::is_closed`, `Stopped::shutdown`, `Refusal::is_closed`,
`Refusal::reason`, `Guarded::free_capacity`, `Guarded::is_blocked`,
`Guarded::shutdown`, and `Wake::is_ready`. Every one is an accessor or a
predicate whose discarded return value costs nothing. `Shutdown::close` — which
mints the `Stopped` token — carried none.

The consequence was that `shutdown.close();` compiled clean under
`RUSTFLAGS="-D warnings"`, and the test suite wrote it **seven** times. That was
not itself wrong: all seven of those tests want the flag set and nothing else.
But the same statement is what a caller writes when they meant to close *and
drain*, and the compiler had nothing to say about it.

The asymmetry is what made it a finding. [`002`](002_open_and_closed.md)'s
second Behavioral Invariant calls the token structural — *"a drain in the Open
state is not an error a caller could hit, it is an expression that does not
compile"* — and [`../type/001`](../type/001_stopped_proof_token.md) exists to
explain why the type carries the proof. A type whose entire purpose is to be
carried to a second call site could be dropped at the first with no diagnostic,
while `Refusal::reason` returning a `Copy` enum could not.

The prediction held for the crate the census could see, and only for that one.
Adding the attribute turned all seven suite sites into warnings under
`-D warnings`, and each turned out to be the benign case rather than a bug —
every one really did want the flag and nothing else. But the census was
crate-scoped, and the first thing the attribute did was break the build of a
crate this finding never looked at: `ring_testkit`'s `Step::Close` arm was an
eighth bare `shutdown.close();`, equally benign and equally invisible to a
recipe that only ever grepped `ring_shutdown/`. The compiler found it in two
seconds; the finding had not found it at all.

That outcome is the point rather than a disappointment. Eight benign sites are
eight places where the family's own code already writes the shape a caller
writes by mistake, and the count is now a standing *family-wide* measurement:
the attribute's message names the deliberate form (`bind \`_\``), so a *new*
bare `shutdown.close();` anywhere under `ring_*` is a compile-time warning
rather than an indistinguishable ninth.

**Disposition:** applied —
`#[ must_use = "a Stopped is the only route to a drain; bind it, or bind \`_\` to close and nothing else" ]`
now sits on `Shutdown::close`; the seven suite sites, five doctest lines and the
one `ring_testkit` site the crate-scoped census had missed were all rewritten to
`let _ = shutdown.close();`, which states the discard instead of hiding it, and
the recipe now counts across the whole family rather than this crate alone.
Now prints: `family sites discarding it:    0`
