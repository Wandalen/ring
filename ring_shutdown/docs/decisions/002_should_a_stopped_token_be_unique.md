# Decision: Should a `Stopped` Token Be Unique

- **Status:** ❓ open
- **Owner:** Deferred alongside [`001`](001_should_into_inner_exist.md) — both change what a holder of a `Guarded` can reach, and picking one without the other decides the wrong question

### Scope

- **Purpose**: Record an open question — whether `Shutdown::close` should be able to mint an unlimited number of `Stopped` tokens — with the options and the evidence that would choose between them, rather than treating the current behaviour as settled because a test asserts it.
- **Responsibility**: The question, why the signature is `&self` today, the four options with what each breaks, and the measurement that would decide.
- **In Scope**: `Shutdown::close`, `Stopped::reopen`, `Stopped::shutdown`.
- **Out of Scope**: The pattern in general and its three rules (→ [`pattern/001`](../pattern/001_proof_token_orders_two_operations.md)); the token's own design rationale (→ [`type/001`](../type/001_stopped_proof_token.md)); what a stale drain costs (→ [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)).

### Question

`Shutdown::close` takes `&self` and returns a `Stopped< '_ >`. Calling it twice
yields two tokens; the crate's own test does exactly that. `Stopped::reopen`
takes `self` by value and consumes one of them.

Should the second token be possible?

The type's doc comment says it should not: *"`Stopped::reopen` takes `self` by
value: once the ring is open again the token is gone, and a drain written
against it no longer compiles."* With two tokens outstanding, spending one
leaves the other alive on a reopened ring.

### Why the Signature Is `&self` Today

Not by oversight. `&self` is what the rest of the crate needs:

- `Shutdown::guard` stores `&'a Shutdown` inside every `Guarded`, so the flag is
  shared with each producer for the producer's whole life.
- `wait_for_close` and `for_space_or_close` both take `&Shutdown` and are meant
  to be callable from a thread that is not the closer.
- `close`'s own doc makes idempotence a feature: *"teardown is often reached from
  more than one path … and neither path should have to know whether it is
  first."*

A `&mut self` close would take exclusive access to a value the design
deliberately shares. So the unlimited supply of tokens is a consequence of a
decision that was made for good reasons somewhere else, which is the shape of
problem worth filing rather than patching.

### Options

| Option | Effect | Cost |
|---|---|---|
| **Keep as is** | Any number of tokens; `reopen` spends one | The type's own doc comment is false whenever more than one exists, and a token held across a `reopen` proves a fact that stopped holding |
| **`close( &mut self )`** | Uniqueness by borrow checker | Breaks `guard`, `wait_for_close` and `for_space_or_close`, which all hold `&Shutdown` — this is the whole concurrency model, not a signature tweak |
| **`close( &self ) -> Option< Stopped >`** | `Some` on the open→closed transition, `None` afterwards | Kills the idempotence property `close`'s doc names as the reason it exists; a second teardown path gets `None` and cannot drain, which is the case idempotence was for |
| **Generation-stamp the token** | `Shutdown` carries a counter; `reopen` bumps it; drains check it | Turns a compile-time proof into a runtime check for the stale case, adds a word of state and a load per drain, and needs a decision about what a stale drain *returns* |

### What Would Settle It

**Whether two tokens are ever legitimately outstanding at once.** Today they are
not. Every `Stopped` bound anywhere in the family is bound in a scope that binds
exactly one — including `ring_testkit`'s, the only such binding outside this
crate — except the idempotence test, which binds two on purpose to assert that
closing twice is not an error.

The measurement to make once a second caller exists. It looks for
*bindings* rather than calls, because a `shutdown.close();` whose token goes
nowhere discards it immediately and is not a caller this question is about.
`let _ = ` is filtered for the same reason and not a different one: since
[`../lifecycle/001`](../lifecycle/001_teardown_and_reuse.md)'s SD30 put
`#[ must_use ]` on `close`, that is the spelling a deliberate discard takes, so
it is a binding by syntax and a discard by intent. Line numbers are omitted —
the question is which crates and which scopes, and a position moves every time
the file above it does:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '= .*\.close()' ring_*/src ring_*/tests --include='*.rs' | command grep -v 'let _ =' | sed 's/:/: /' | sed 's/  */ /g' | sort -u
```

Live output:

```
ring_shutdown/src/lib.rs: /// let stopped = shutdown.close();
ring_shutdown/src/lib.rs: let stopped = shutdown.close();
ring_shutdown/tests/shutdown_test.rs: let first = shutdown.close();
ring_shutdown/tests/shutdown_test.rs: let second = shutdown.close();
ring_shutdown/tests/shutdown_test.rs: let _stopped = guarded.shutdown().close();
ring_shutdown/tests/shutdown_test.rs: let stopped = shutdown.close();
ring_testkit/src/lib.rs: let stopped = shutdown.close();
```

- **Every close is followed by its own `reopen` or drop in the same scope** →
  `Option< Stopped >` costs nothing real; take uniqueness.
- **Two live tokens in one scope, both used** → generation-stamping is the only
  option that keeps both callers working.
- **Closes from separate threads** → neither borrow-based option is available;
  the question becomes what a stale drain should return, not how to prevent one.

### Why This Is Filed Rather Than Decided

Three of the four options change a public signature, and two of them change the
concurrency model. The current behaviour is not obviously wrong — it is
obviously *undocumented*, which is a different defect with a much cheaper fix
(→ SD15). Deciding now would trade a false sentence in a doc comment for a
guess about callers that do not exist yet.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'how a token is minted:         %s\n' "$( awk '/^  pub fn close/{ sub( /^ */, "" ); print }' src/lib.rs )"
printf 'how a token is spent:          %s\n' "$( awk '/^  pub fn reopen/{ sub( /^ */, "" ); print }' src/lib.rs )"
printf 'what Stopped derives:          %s\n' "$( awk '/^pub struct Stopped/{ print d; exit } /^#\[ derive/{ d=$0 }' src/lib.rs )"
printf 'what its doc comment claims:   %s\n' "$( awk '/^pub struct Stopped/{ print b; exit } /^\/\/\//{ b = b " " $0; next } /^#\[/{ next } { b="" }' src/lib.rs | sed 's|///||g; s/  */ /g' | command grep -o 'once the ring is open again[^.]*\.' )"
printf 'the test that mints two:       %s\n' "$( command grep -o 'fn close_is_idempotent_and_admit_reports_it' tests/shutdown_test.rs )"
printf 'tokens it binds:               %s\n' "$( awk '/fn close_is_idempotent_and_admit_reports_it/{f=1} f&&/^\}$/{exit} f&&/= shutdown.close\(\)/{ n++ } END{ print n+0 }' tests/shutdown_test.rs )"
printf 'tokens it spends:              %s\n' "$( awk '/fn close_is_idempotent_and_admit_reports_it/{f=1} f&&/^\}$/{exit} f&&/\.reopen\(\)/{ n++ } END{ print n+0 }' tests/shutdown_test.rs )"
printf 'a token can reach the flag by: %s\n' "$( awk '/^  pub const fn shutdown/{ if ( ++n == 1 ) { sub( /^ */, "" ); print } }' src/lib.rs )"
printf 'signatures sharing the flag:   %s\n' "$( command grep -cE '&.a? ?Shutdown' src/lib.rs )"
printf 'rule 3 of the extracted pattern: %s\n' "$( command grep -o 'Anything that undoes A consumes the token' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'what the pattern says about it: %s\n' "$( command grep -o 'Rule 3 is the one most often skipped' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'the missing rule, where it lives: %s\n' "$( command grep -o 'undoing A must invalidate every outstanding token' docs/pattern/002_a_proof_token_must_be_scarce.md )"
printf 'does 001 now point at it:      %s\n' "$( awk '/^### Regenerate/{ exit } /rule 4/{ n++ } END{ print n+0 }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'what a drain on an open ring does: %s\n' "$( command grep -ohE 'the loop never sees an empty batch' docs/invariant/002_drain_terminates_because_close_preceded_it.md )"
```

Live output:

```
how a token is minted:         pub fn close( &self ) -> Stopped< '_ >
how a token is spent:          pub fn reopen( self )
what Stopped derives:          #[ derive( Debug ) ]
what its doc comment claims:   
the test that mints two:       fn close_is_idempotent_and_admit_reports_it
tokens it binds:               2
tokens it spends:              1
a token can reach the flag by: pub const fn shutdown( &self ) -> &'a Shutdown
signatures sharing the flag:   4
rule 3 of the extracted pattern: Anything that undoes A consumes the token
what the pattern says about it: Rule 3 is the one most often skipped
the missing rule, where it lives: undoing A must invalidate every outstanding token
does 001 now point at it:      1
what a drain on an open ring does: the loop never sees an empty batch
```

### Decisions

| File | Relationship |
|------|--------------|
| [001_should_into_inner_exist.md](001_should_into_inner_exist.md) | The other open question about what a `Guarded` holder can reach, and the one this must be decided with |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_stopped_proof_token.md](../type/001_stopped_proof_token.md) | The token's design rationale and the alternatives it was chosen over |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_proof_token_orders_two_operations.md](../pattern/001_proof_token_orders_two_operations.md) | Rule 3, and the failure it names as the reason rule 3 exists |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_drain_terminates_because_close_preceded_it.md](../invariant/002_drain_terminates_because_close_preceded_it.md) | What a drain against an open ring actually does, which is the cost of a stale token |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_the_table_does_not_grade.md](../api/002_the_surface_the_table_does_not_grade.md) | `Guarded::shutdown`, the accessor that puts `close` in reach of a producer |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `close`, `reopen`, `Stopped` and its doc comment |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it` — mints two tokens, spends one, and ends with the other still live on a reopened ring |

### SD15 — The Token's Doc Comment Is Falsified by the Crate's Own Test

`Stopped`'s doc comment states the guarantee the type exists to provide:

> *"`Stopped::reopen` takes `self` by value: once the ring is open again the
> token is gone, and a drain written against it no longer compiles."*

The first clause is true — `reopen( self )` consumes its receiver. The second
does not follow from it, because `close( &self )` can mint another token at any
time, and `reopen` consumes only the one it is called on.

The falsifying case is not hypothetical, and it is not in some future caller. It
is in `tests/shutdown_test.rs`, in `close_is_idempotent_and_admit_reports_it`:

```rust
let first  = shutdown.close();
// …
let second = shutdown.close();
// …
second.reopen();
assert!( !shutdown.is_closed() );
```

After the last line the ring is open and `first` is still in scope, still
usable, and still spelled `Stopped` — a value the type system will accept
anywhere a proof-of-closure is required. `first.drain_all( &consumer, &mut out )`
compiles. The test does not write that line, so nothing fails; the test exists to
assert idempotence and it does, and the token it leaves behind is incidental to
its subject.

That drain is the expensive failure, not a wrong answer.
[`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)
is explicit about a drain on a ring with a live producer: *"the loop never sees
an empty batch."* So the shortest path to the crate's own worst documented
outcome is: close twice, reopen once, drain on the surviving token — three lines,
all of them compiling, two of them already written down in the suite.

The cheap fix is a sentence, not a signature. The doc comment should say what is
true: `reopen` consumes *the token it is called on*, and a `Stopped` obtained
before a `reopen` outlives it. Whether the underlying behaviour should change is
the question this document files.

**Disposition:** applied — `Stopped`'s doc comment in `src/lib.rs` now says
`reopen` consumes *the token it is called on*, and that a `Stopped` obtained
from an earlier `close` call is a distinct value that outlives this one's
reopen — matching what `close_is_idempotent_and_admit_reports_it` actually
demonstrates. Whether minting should be restricted stays open, as this
document's own subject. Now prints: `it consumes *the token it is called on*`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
awk '/^pub struct Stopped/{ print b; exit } /^\/\/\//{ b = b " " $0; next } /^#\[/{ next } { b="" }' src/lib.rs | sed 's|///||g; s/  */ /g'
```

Live output:

```
 Proof that a ring is closed, and the only route to a drain. Held by reference to the [`Shutdown`] it came from, so it cannot outlive it. [`Stopped::reopen`] takes `self` by value: it consumes *the token it is called on*, and a drain written against that specific token no longer compiles afterward. A `Stopped` obtained from an earlier [`Shutdown::close`] call is a distinct value and outlives this one's reopen.
```

### SD16 — The Type Withholds `Clone` and an Accessor on the Same Type Reconstitutes It

`Stopped` derives `Debug` and nothing else. In a crate where `Wake` derives six
traits and `Refusal` four, a token that derives one is a deliberate statement:
this value is not to be duplicated.

`Stopped::shutdown( &self ) -> &'a Shutdown` undoes it. Given any `Stopped`, a
caller writes `stopped.shutdown().close()` and holds a second one — from a
*shared* borrow of the first, which therefore stays alive too. The absent
`Clone` costs a caller one method call and the knowledge that `close` takes
`&self`.

This matters beyond ergonomics because the crate extracted the pattern and
wrote down the rule it breaks. [`pattern/001`](../pattern/001_proof_token_orders_two_operations.md)
lists three load-bearing rules and says of the third — *"Anything that undoes A
consumes the token"* — that it is *"the one most often skipped, and skipping it
is subtle: the code compiles, the common path is correct, and the failure only
appears when a caller re-enters the pre-A state and then calls B with a stale
proof."*

`ring_shutdown` satisfies rule 3 to the letter: `reopen` does consume the token.
And it produces rule 3's failure anyway, in precisely the words used to describe
it, because the rule assumes what rule 1 was supposed to supply — that tokens
are scarce. Rule 1 says *"the token's only constructor performs A"*, which is
true here and turns out not to be enough: the constructor performs A every time,
and there is no limit on how often it may be called.

The pattern document is the natural place to fix this, by adding the missing
condition — a token proves a *current* fact only if the operation that undoes A
invalidates every outstanding token, not merely the one passed to it. That
generalizes beyond this crate, which is what a pattern instance is for.

That is where it went. [`pattern/002`](../pattern/002_a_proof_token_must_be_scarce.md)
states the condition as rule 4 and prices the four ways to satisfy it, and
[`pattern/001`](../pattern/001_proof_token_orders_two_operations.md) now ends its
three rules by saying they are not sufficient and pointing there — so a reader
of the pattern no longer leaves with a rule set that this crate satisfies and
still defeats (→ that document's SD38).

**The code is unchanged, and that is this document's whole subject.** Two of
rule 4's four spellings would fix `Stopped` — `close( &mut self )` or
`close( self )` — and both cost the shared `Shutdown` that `Guarded` and both
waits are built on, which is the trade this decision exists to weigh and has not
settled. Recording the rule does not settle it; it makes the open question
precise, which is the most a Pending decision should claim.

**Disposition:** applied — the missing scarcity condition is recorded as rule 4
in [`pattern/002`](../pattern/002_a_proof_token_must_be_scarce.md) and
[`pattern/001`](../pattern/001_proof_token_orders_two_operations.md) now defers
to it; the receiver change stays open as this decision's own subject rather than
being made under cover of a finding. Now prints: `does 001 now point at it:      1`
