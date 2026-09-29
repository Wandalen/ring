# Item: Seventeen Verbs That Never Touch the Producer

### Scope

- **Purpose**: Catalogue every method this crate declares, against what it forwards to and what it adds — and record the one dependency that no public method can reach.
- **Responsibility**: The seventeen inherent `pub fn`, the four private functions, what each borrows and what each invents.
- **In Scope**: Method bodies in `src/lib.rs`; which container each call lands in; which signature takes `self` by value.
- **Out of Scope**: The types (→ [`001`](001_seven_nouns_thirteen_variants.md)); the driver surface as a caller sees it (→ [`api/002`](../api/002_the_driver_surface.md)); the seal-drain-reset ordering inside `run` (→ [`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)).

### The Seventeen

| Verb | Receiver | Body | Adds |
|------|----------|------|------|
| `FlushEntry::count` | `&self` | Matches the stored outcome | Derives a count so log and outcome cannot disagree |
| `FlushLog::new` | — | `Vec::new` | — |
| `FlushLog::entries` | `&self` | `&self.entries` | Hands out a slice, never the `Vec` |
| `FlushLog::len` | `&self` | `Vec::len` | — |
| `FlushLog::is_empty` | `&self` | `Vec::is_empty` | The assertion that discharges "and at no other point" |
| `FlushLog::clear` | `&mut self` | `Vec::clear` | Keeps the allocation |
| `Flusher::new` | — | Validation, then a struct literal | Refuses two batch sizes that would degrade into other policies |
| `Flusher::with_log` | `self` | Sets one field | The opt-in that replaced a compilation boundary |
| `Flusher::policy` | `&self` | Copies a field | — |
| `Flusher::staged` | `&self` | `TlsBuffer::len` | Advisory only |
| `Flusher::buffer_capacity` | `&self` | `TlsBuffer::capacity` | With `staged`, the headroom an `OnBarrier` caller needs |
| `Flusher::log` | `&self` | `Option::as_ref` | — |
| `Flusher::clear_log` | `&mut self` | `FlushLog::clear`, if present | — |
| `Flusher::append` | `&mut self` | `TlsBuffer::push` | **Nothing.** That is the whole commitment |
| `Flusher::drive` | `&mut self` | `trigger( false )` then `run` | Policy evaluation |
| `Flusher::drive_at_barrier` | `&mut self` | `trigger( true )` then `run` | The only route to `OnBarrier` |
| `Flusher::drain_final` | `&mut self` | `run( Shutdown )` | Ignores the policy |

Seven of the seventeen are a single unchanged call into a container this crate
does not own. **They land in two different crates**: four in the standard
library's `Vec`, three in `ring_tls`'s `TlsBuffer`.

### The Four That Are Not Public

| Function | Called from | Why private |
|----------|-------------|-------------|
| `ConfigError::fmt` | The `Display` impl | A trait method, not a declaration |
| `Flusher::trigger` | `drive`, `drive_at_barrier` | Reads the policy by value; touches no atomic. Exposing it would let a caller ask *whether* a flush would fire without firing it, which is a second surface for the same decision |
| `Flusher::run` | `drive`, `drive_at_barrier`, `drain_final` | The cold path. Public would be an unconditional `flush_now()`, which is the exact capability this crate exists to remove |
| `Flusher::record` | `run` | Writes the entry inside the call that produced the outcome, so an entry disagreeing with its outcome is unrepresentable |

**`run` being private is the crate's design in one line.** Every public route to
it carries a condition — a policy, a barrier announcement, or a named teardown
phase — and there is no route that carries none.

### Algorithms

| File | Relationship |
|------|-----------------|
| [`../algorithm/001_evaluating_a_policy_at_an_append.md`](../algorithm/001_evaluating_a_policy_at_an_append.md) | `append` and `trigger`, as a procedure |
| [`../algorithm/002_sequencing_seal_drain_reset.md`](../algorithm/002_sequencing_seal_drain_reset.md) | What `run` does, and why the capacity check precedes the drain |

### APIs

| File | Relationship |
|------|-----------------|
| [`../api/002_the_driver_surface.md`](../api/002_the_driver_surface.md) | The same verbs as a caller meets them, with the obligations attached |

### Items

| File | Relationship |
|------|-----------------|
| [`001_seven_nouns_thirteen_variants.md`](001_seven_nouns_thirteen_variants.md) | The other half of the catalogue |

### Patterns

| File | Relationship |
|------|-----------------|
| [`../pattern/002_driven_not_self_firing.md`](../pattern/002_driven_not_self_firing.md) | Why `append` adds nothing and `run` is unreachable without a condition |

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | Every method catalogued here |

### Tests

| Test | Relationship |
|------|--------------|
| `appending_never_publishes` | `append`'s one-line body, asserted behaviourally |
| `a_second_final_drain_is_an_empty_trigger` | Calls `drain_final` twice deliberately — the cardinality the signature permits |
| `a_refused_final_drain_keeps_the_records` | Calls it twice incidentally, because retrying a rejection *is* a second call |
| `a_driver_still_works_after_a_final_drain` | The driver survives the phase its name calls final |

### FL27 — Seven Borrowed Verbs, Two Containers, and a Producer No Verb Reaches

The forwarding half of the surface, and where it lands:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- calls into a container this crate does not own --'
command grep -E 'self\.(buffer|entries)\.[a-z_]+\(|Vec::new' ring_flush/src/lib.rs
echo '  -- every line that names the producer --'
command grep 'self\.producer' ring_flush/src/lib.rs
echo '  -- and the functions those lines sit in --'
awk '/^  (pub )?(const )?fn [a-z_]+/{ f = $0 } /self\.producer/{ print "    " NR ":" f }' \
  ring_flush/src/lib.rs
```

Live output:

```
  -- calls into a container this crate does not own --
    Self { entries : Vec::new() }
    self.entries.len()
    self.entries.is_empty()
    self.entries.clear();
    self.buffer.len()
    self.buffer.capacity()
    self.buffer.push( record )
      FlushPolicy::OnFull => if self.buffer.is_full() { Some( FlushCause::Full ) } else { None },
      FlushPolicy::OnBatch( n ) => if self.buffer.len() >= n { Some( FlushCause::Batch ) } else { None },
    let staged = self.buffer.len();
    let count = self.producer.try_push_batch( &mut self.buffer.drain() );
  -- every line that names the producer --
    if self.producer.free_capacity() < staged
    let count = self.producer.try_push_batch( &mut self.buffer.drain() );
  -- and the functions those lines sit in --
    629:  fn run( &mut self, cause : FlushCause ) -> FlushOutcome
    637:  fn run( &mut self, cause : FlushCause ) -> FlushOutcome
```

`ring_core::Producer` is the crate's publish-side dependency and **every use of
it in the whole crate is two lines inside one private function**. The field is
populated by `new`'s struct literal — which names the constructor's parameter,
not `self.producer`, so it does not appear above — and after that `run` is the
only reader. There is no public route to `run` that does not first pass a policy
check.

**That is the strongest structural statement the crate makes and it is nowhere
stated as one.** The design intent — publication happens at a moment somebody
chose — is documented in prose, argued from naming, and defended in three
instances. It is *enforced* by the fact that reaching a producer requires going
through `run`, and `run` is private. A future `pub fn producer( &mut self ) ->
&mut Producer` would undo every one of those arguments without touching any of
them.

Nothing detects that. `ring_handle` faces the same shape and named it: a
convenience accessor that hands out the wrapped value undoes the wrapper, and
no compile-fail case can be written for "any route", because a route has no
name to pin.

### FL28 — The Verb Documented as Once-Only Has a Signature That Permits Any Number

`drain_final` is described everywhere as a named teardown phase called once:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the signature --'
command grep 'fn drain_final' ring_flush/src/lib.rs
echo '  -- how the crate describes its cardinality --'
command grep -r 'once-only\|called once' ring_flush/src ring_flush/docs/lifecycle
echo '  -- tests that call it more than once --'
awk '/^fn [a-z_]+/{ f = $0; n = 0 } /\.drain_final\(/{ n += 1; if ( n == 2 ) print "    " f }' \
  ring_flush/tests/flush_test.rs
```

Live output:

```
  -- the signature --
  pub fn drain_final( &mut self ) -> FlushOutcome
  -- how the crate describes its cardinality --
ring_flush/src/lib.rs:  /// A named lifecycle phase — called once, at teardown, by the owner — and not
ring_flush/docs/lifecycle/002_from_configuration_to_the_final_drain.md:`Drained` type — would enforce the once-only property and is not obviously
  -- tests that call it more than once --
    fn a_refused_final_drain_keeps_the_records()
    fn a_second_final_drain_is_an_empty_trigger()
```

The signature is `&mut self`. The once-only property is held by the name, by
`lifecycle/002`'s table — which records it as enforced by "nothing but its name
and this document" — and by nothing else.

**The suite does not merely permit a second call; it pins one, in two tests
written for unrelated purposes.** `a_second_final_drain_is_an_empty_trigger`
does it deliberately. `a_refused_final_drain_keeps_the_records` does it
incidentally — its subject is the retry path, and retrying *is* a second call.
Those tests are right: leaving the behaviour unspecified was the one answer
definitely wrong, because callers would discover it empirically. But they also
mean **the reusable behaviour is now the tested contract**, which is a harder
thing to walk back than an unspecified one — and one of the two would not have
been recognised as a cardinality test by anyone reading its name.

That is exactly the trade the crate's own Pending 4 describes — a consuming
`fn drain_final( self ) -> Result< Drained, ( Self, FlushOutcome ) >` would make
the once-only property structural — and it now has a cost the pending decision
was written before: two tests would have to change, not just a signature.
Recorded here so the catalogue and the pending decision agree about what is
actually holding the line.

**Disposition:** declined — the finding's own text identifies the structural
fix (a consuming `fn drain_final( self ) -> Result< Drained, ( Self,
FlushOutcome ) >`) as the crate's own open Pending 4, and states its cost
explicitly: two existing tests (`a_second_final_drain_is_an_empty_trigger`,
`a_refused_final_drain_keeps_the_records`) would have to change because they
currently pin the reusable, `&mut self` behavior as the tested contract.
That is a breaking public-API change gated on a design ruling this finding
correctly declines to make unilaterally, not a documentation gap this pass
can close — `drain_final`'s own doc comment already calls the once-only
property "a convention that can erode" (`ring_flush/src/lib.rs`), and
`docs/lifecycle/002`'s state table separately records it as enforced by
"Nothing but its name and this document," so neither the source nor the
corpus is silent about the weakness. No in-scope fix exists short of
resolving Pending 4 itself.
