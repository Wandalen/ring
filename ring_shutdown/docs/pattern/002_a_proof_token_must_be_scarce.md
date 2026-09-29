# Pattern: A Proof Token Must Be Scarce

### Scope

- **Purpose**: State the condition [`001`](001_proof_token_orders_two_operations.md)'s three rules do not cover — that a proof token's supply must be bounded, not just its construction and its consumption — and give the four spellings that bound it.
- **Responsibility**: The missing condition, why the existing three rules do not imply it, the four ways to satisfy it, and what each costs.
- **In Scope**: Token scarcity as a general property of the proof-token technique.
- **Out of Scope**: The ordering guarantee the token exists to give (→ [`001`](001_proof_token_orders_two_operations.md)); whether *this* crate should adopt one of the four (→ [`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)).

### Problem

[`001`](001_proof_token_orders_two_operations.md) gives three rules: the token's
only constructor performs A, B is a method on the token, and anything that
undoes A consumes the token. A design can satisfy all three and still let a
caller call B after A has been undone.

The gap is one word. Rule 1 constrains **how** a token is made. Rule 3
constrains what happens to **the** token — the one passed to the undo. Neither
constrains **how many** exist. If the constructor takes `&self`, the supply is
unbounded; undoing A consumes one and leaves the rest, each still a well-typed
proof of a fact that stopped being true.

The failure is exactly the one rule 3 describes and warns is *"the one most
often skipped"* — a caller re-enters the pre-A state and calls B with a stale
proof — reached by a route rule 3 does not close.

### Solution

**Rule 4: undoing A must invalidate every outstanding token, not the one it was
called on.**

There are four ways to satisfy it, and they are not equivalent.

| Spelling | How it bounds the supply | Cost |
|---|---|---|
| **`fn a( &mut self ) -> Token< '_ >`** | The borrow checker: one token at a time, and the subject is unusable while it lives | The subject cannot be shared; every other method needs the token or a reborrow |
| **`fn a( self ) -> Token`** | Linearity: the subject is consumed, so there is exactly one token ever | The subject cannot be recovered without an explicit `undo_a( self ) -> Subject` |
| **Generation counter inside the token** | A runtime check: `undo_a` bumps the subject's generation, `b` compares and refuses | Restores the runtime error the pattern existed to remove — but a *loud* one, not a hang |
| **No undo at all** | Vacuously: nothing re-enters the pre-A state, so no proof can go stale | The subject is single-use; a reset or reopen is out of the question |

The first two are the only ones that keep the guarantee at compile time. The
third trades the pattern's whole premise for a diagnostic that at least fires.
The fourth is what most correct instances of this pattern do by accident: they
have no undo, so rule 4 never comes up and rule 3 looks sufficient.

**That last row is why the gap survives review.** A pattern extracted from
instances that have no undo will not contain rule 4, and the first instance
*with* an undo inherits three rules that were complete for a different problem.

### Applicability

Rule 4 is only load-bearing when an undo exists. The check is two questions:

| Question | If yes |
|---|---|
| Is there an operation that returns the subject to its pre-A state? | Rule 4 applies; pick a spelling above |
| Can A be called more than once between two undos? | The supply is unbounded; rule 3 alone is insufficient |

Both are answerable by reading two signatures — A's receiver and undo's — which
is cheaper than any of the four fixes and is the check that was not performed.

### When it does not apply

**When the token carries state that makes staleness observable.** A token
holding a snapshot the subject can invalidate — a generation, an epoch, a
version — is the third spelling above, and it detects the stale case rather than
preventing it. That is a different pattern with a different failure mode: it
cannot hang, and it can refuse a call that would have been fine.

**When A is idempotent and undo is unreachable in practice.** This is a real
argument and a fragile one: it makes the guarantee a property of the current
call graph rather than of the types, and the next caller to add an undo path
inherits the hazard silently. It belongs in a decision record, not in the type.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'rules 001 states:              %s\n' "$( awk '/^### Regenerate/{ exit } /^[0-9]\. \*\*/{ n++ } END{ print n+0 }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'how the instance spells A:     %s\n' "$( command grep -o 'pub fn close.*' src/lib.rs )"
printf 'how it spells the undo:        %s\n' "$( command grep -o 'pub fn reopen.*' src/lib.rs )"
printf 'A takes the subject by:        %s\n' "$( command grep -o 'pub fn close( &self )' src/lib.rs | command grep -o '&self' )"
printf 'so tokens per undo:            %s\n' "$( awk '/^fn close_is_idempotent/{f=1} f&&/^\}/{exit} f&&/= shutdown\.close\(\)/{ n++ } END{ print n+0 }' tests/shutdown_test.rs )"
printf 'undos in that same test:       %s\n' "$( awk '/^fn close_is_idempotent/{f=1} f&&/^\}/{exit} f&&/\.reopen\(\)/{ n++ } END{ print n+0 }' tests/shutdown_test.rs )"
printf 'what close doc claims it does: %s\n' "$( awk '/^ *\/\/\//{ b = b " " $0; next } /^ *#\[/{ next } /^  pub fn close/{ print b; exit } { b = "" }' src/lib.rs | sed 's|///||g; s/  */ /g' | command grep -o 'returns the same token' )"
printf 'what it constructs each call:  %s\n' "$( awk '/^  pub fn close\( &self \)/{f=1} f&&/Stopped \{/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'spellings this doc lists:      %s\n' "$( awk '/^### Solution/{f=1} f&&/^### Applicability/{exit} f&&/^\| \*\*/{ n++ } END{ print n+0 }' docs/pattern/002_a_proof_token_must_be_scarce.md )"
printf 'of those, compile-time ones:   %s\n' "$( awk '/^### Solution/{f=1} f&&/^### Applicability/{exit} f&&/^\| \*\*.fn a\(/{ n++ } END{ print n+0 }' docs/pattern/002_a_proof_token_must_be_scarce.md )"
printf 'docs quoting that signature:   %s\n' "$( cd docs; command grep -rl 'close( &self )' --include='*.md' . | sort -u | wc -l )"
printf 'findings filed against it:     %s\n' "$( cd docs; command grep -rhoE '^### SD[0-9]+' --include='*.md' . | sort -u | wc -l )"
```

Live output:

```
rules 001 states:              3
how the instance spells A:     pub fn close( &self ) -> Stopped< '_ >
how it spells the undo:        pub fn reopen( self )
A takes the subject by:        &self
so tokens per undo:            2
undos in that same test:       1
what close doc claims it does: 
what it constructs each call:  Stopped { shutdown : self }
spellings this doc lists:      4
of those, compile-time ones:   2
docs quoting that signature:   15
findings filed against it:     52
```

### Patterns

| File | Relationship |
|------|--------------|
| [`001_proof_token_orders_two_operations.md`](001_proof_token_orders_two_operations.md) | The three rules this one extends, and the sketch that shares the gap |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_stopped_proof_token.md`](../type/001_stopped_proof_token.md) | The instance, whose `close( &self )` is the unbounded-supply spelling |

### Decisions

| File | Relationship |
|------|--------------|
| [`../decisions/002_should_a_stopped_token_be_unique.md`](../decisions/002_should_a_stopped_token_be_unique.md) | Whether this crate should adopt one of the four spellings, and at what cost to its callers |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_drain_terminates_because_close_preceded_it.md`](../invariant/002_drain_terminates_because_close_preceded_it.md) | The guarantee that rests on rule 4 and is stated as though rule 3 supplied it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `close`, `reopen`, and the doc comment claiming a second close *"returns the same token"* |

### SD39 — The Instance's Source Doc Says a Second Close Returns the Same Token, and It Constructs a New One

`Shutdown::close`'s doc comment reads: *"Idempotent — closing an already-closed
shutdown is not an error and **returns the same token**."* The body is
`self.closed.store( true, Ordering::Release ); Stopped { shutdown : self }` — it
constructs a fresh `Stopped` on every call, and `close_is_idempotent_and_admit_reports_it`
binds two of them at once.

The sentence is defensible as loose English — the two tokens are equal in every
observable way, since `Stopped` holds one reference and no state of its own. It
is indefensible in this crate, because token *identity* is the whole mechanism.
[`001`](001_proof_token_orders_two_operations.md)'s rule 3 turns on which token
the undo consumed; [`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)'s
enforcement argument turns on there being nothing left after a `reopen`. A
reader who takes "the same token" literally concludes that `reopen` on either
one ends both, which is precisely the guarantee the crate does not have.

This is the fifth statement of the same fact in this crate and the first to
imply the missing rule 4 is already in force. The other four are a doc instance
each: an open question (→ [`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)),
an asserted impossibility (→ [`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)),
an intended feature (→ [`../lifecycle/002`](../lifecycle/002_open_and_closed.md)),
and a reachability hazard (→ [`../api/002`](../api/002_the_surface_the_table_does_not_grade.md)).
Five statements, one line of code, no two agreeing — and the only one a caller
reads in their editor is this one, which is the one that is wrong.

**Disposition:** applied — `Shutdown::close`'s doc comment no longer says
"the same token"; it now says "an equal but distinct token, constructed
fresh on every call rather than cached from the first close," matching
`close_is_idempotent_and_admit_reports_it`'s two-token binding. Now prints:
`an equal but distinct token, constructed fresh on every call`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
# Anchored on the sentence rather than on lines 87-89: the address moves on any
# edit above it in a file this crate's own findings keep rewriting, and what is
# being shown is which words the doc comment uses, never where they sit.
command grep -m1 -A2 -F 'Idempotent — closing an already-closed shutdown' src/lib.rs
command grep -c 'returns the same token' src/lib.rs
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
  /// Idempotent — closing an already-closed shutdown is not an error and
  /// returns an equal but distinct token, constructed fresh on every call
  /// rather than cached from the first close. That matters because teardown
0
```

### SD40 — The Cheapest Fix Is Excluded Because the Pattern Never Asked the Question

Of the four spellings above, `fn a( &mut self ) -> Token< '_ >` is nearly free:
one character in one signature, and the borrow checker supplies rule 4 with no
runtime cost, no new type, and no change to any caller who was using the pattern
correctly. It is not adopted, and the reason is not that it was weighed and
rejected.

`Shutdown::close` takes `&self` because `Shutdown` is shared across threads —
that is the correct and necessary reason, and `&mut self` is genuinely
unavailable here. But no document says so. [`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)
lists four options and this is not among them;
[`001`](001_proof_token_orders_two_operations.md)'s sketch writes `&self`
without comment; the source doc comment asserts the opposite property
altogether (→ SD39).

The general form is worth naming, because it is what turned a one-character
question into five inconsistent documents: **a pattern that does not state a
condition cannot record why an instance fails it.** Rule 4's absence from
[`001`](001_proof_token_orders_two_operations.md) is not just a missing rule —
it removed the place where `close( &self )`'s justification would have been
written down, so the constraint that makes the fix unavailable lives only in
the reader's head, and every later document had to rediscover the gap on its
own terms.
