# Pattern: A Proof Token Orders Two Operations

### Scope

- **Purpose**: Extract the reusable shape behind `Stopped` — making operation B unspellable without operation A having happened first — with the conditions under which it is worth its cost.
- **Responsibility**: Structure, applicability conditions, cost, and the failure mode that makes it theatre.
- **In Scope**: The technique in general.
- **Out of Scope**: This crate's own instance (→ [`type/001`](../type/001_stopped_proof_token.md)).

### Problem

Two operations, A and B, where B is only correct if A already happened. The
ordinary answer is a doc comment — *"call `close` before `drain_all`"* — and it
fails in the one case that matters: the caller who did not read it.

Worse, the failure of an out-of-order B is frequently *not* an error value. It
is a hang, a silently wrong count, or a symptom three frames away with nothing
in the stack pointing at the omitted A. A runtime check cannot help either when
the condition it would check is unobservable from inside B.

What is wanted is for the out-of-order call to be **unspellable** — a compile
error at the call site, not a comment near the definition.

### Solution

```rust
struct Token< 'a > { subject : &'a Subject }        // no state of its own

impl Subject
{
  fn a( &self ) -> Token< '_ > { /* do A */ Token { subject : self } }   // only constructor
  //   ^^^^^ mints without limit — see 002's rule 4 before copying this line
}

impl Token< '_ >
{
  fn b( &self ) { /* needs A to have happened */ }
  fn undo_a( self ) { /* consumes the token */ }
}
```

Three rules make it work, and all three are load-bearing:

1. **The token's only constructor performs A.** If it can be built another way,
   it proves nothing.
2. **B is a method on the token, not on the subject.** If B remains reachable
   on the subject, the token is decorative.
3. **Anything that undoes A consumes the token.** Otherwise the proof outlives
   the fact.

Rule 3 is the one most often skipped, and skipping it is subtle: the code
compiles, the common path is correct, and the failure only appears when a
caller re-enters the pre-A state and then calls B with a stale proof.

**The three are load-bearing and not sufficient.** They constrain how a token is
built (1), where B lives (2), and what happens to *the* token an undo is handed
(3) — and say nothing about how many tokens exist at once. Wherever an undo
exists, a fourth condition is required and is stated in
[`002`](002_a_proof_token_must_be_scarce.md): undoing A must invalidate every
outstanding token, not the one it was called on. The sketch above satisfies all
three rules and violates the fourth, which is why the `&self` receiver carries a
warning comment rather than being silently exemplary.

### Applicability

Four conditions decide whether the token earns its ergonomic cost, and one
condition decides whether it works at all.

#### When it beats a documented precondition

| Condition | Why it matters |
|---|---|
| B's failure when A did not happen is **not an error** | If B can return `Err( NotReadyYet )`, the runtime check is simpler and the token is overhead |
| B's failure is **silent or non-local** | A hang, a corrupted count, or a failure that surfaces three frames away is what a compile error is worth paying for |
| A is **cheap and idempotent** | The pattern forces A to be called; if A is expensive, callers will route around it |
| B is called **rarely**, at a boundary | Threading a token through a hot path costs ergonomics for a guarantee that path may not need |

`ring_shutdown` matches all four: draining without a prior close cannot even
return a recoverable `Err` — there is no `Stopped` to call it on (row 1);
`drain_all` against an open ring **hangs** rather than erring (row 2); `close`
is a `Release` store paired with `is_closed`'s `Acquire` load, not merely
idempotent — the pairing is what makes `Guarded::try_push`'s refusal
observable rather than racy (row 3); and teardown is a boundary (row 4).

#### When it does not

**When A is unenforceable.** This is the failure mode, and it is not obvious
from inside the pattern: the token proves *the call to A happened*, never that
A *achieved* anything.

`ring_shutdown` is itself an instance of this limit. `close` sets a flag that a
raw producer never reads, so a `Stopped` can be a perfectly valid token
attesting to a fact that does not hold (→
[`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)).
The ordering guarantee is real; the state guarantee is not, and the type is
silent about which one it is giving you.

**The generalizable check:** for each property the token appears to establish,
ask whether it follows from *A having been called* or from *A having worked*.
Only the first is what the token gives. If the interesting property is the
second, the token is theatre — well-typed theatre, which is the dangerous kind.

### Consequences

Zero at runtime — the token is one reference and has no `Drop`. The cost is
ergonomic and falls entirely on callers who want B without A, which is exactly
the population the pattern exists to stop.

The real cost is **inflexibility under refactoring**: once B is a method on the
token, moving B elsewhere means moving the token too, and a helper that wants
to take "a thing you can drain from" cannot be written generically without
naming the token type.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'what the applicability claim says: %s\n' "$( awk '/^### Regenerate/{ exit } /matches all four/{ sub( /^ */, "" ); print }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'rows in that table:            %s\n' "$( awk '/^#### When it beats/{f=1} f&&/^####/&&!/beats/{exit} f&&/^\| ./&&!/^\| Condition/&&!/^\|---/{ n++ } END{ print n+0 }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'rows the claim then cites:     %s\n' "$( awk '/^### Regenerate/{ exit } /matches all four/{f=1} f&&/row [0-9]/{ print }' docs/pattern/001_proof_token_orders_two_operations.md | command grep -oE 'row [0-9]' | sort -u | tr '\n' ' ' )"
printf 'the ordering the doc names:    %s\n' "$( command sed -n 's/.*is a `\([A-Za-z]*\)` store paired with.*/\1/p' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'the ordering close uses:       %s\n' "$( awk '/^  pub fn close\( &self \)/{f=1} f&&/closed\.store/{ sub( /.*Ordering::/, "" ); sub( / \).*/, "" ); print; exit }' src/lib.rs )"
printf 'and the one is_closed uses:    %s\n' "$( awk '/closed\.load/{ sub( /.*Ordering::/, "" ); sub( / \).*/, "" ); print; exit }' src/lib.rs )"
printf 'orderings appearing at all:    %s\n' "$( command grep -ohE 'Ordering::[A-Za-z]+' src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'rule 1 of the pattern:         %s\n' "$( awk '/^### Regenerate/{ exit } /^1\. \*\*/{ sub( /^1\. /, "" ); print }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'how the sketch spells A:       %s\n' "$( awk '/^### Regenerate/{ exit } /fn a\( /{ sub( /^ */, "" ); print }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'how the crate spells A:        %s\n' "$( command grep -o 'pub fn close.*' src/lib.rs )"
printf 'rule 2 of the pattern:         %s\n' "$( awk '/^### Regenerate/{ exit } /^2\. \*\*/{ sub( /^2\. /, "" ); print }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'what the token hands back:     %s\n' "$( command grep -o 'pub const fn shutdown( &self ) -> &.a Shutdown' src/lib.rs | head -1 )"
printf 'does the rule set admit a gap: %s\n' "$( awk '/^### Regenerate/{ exit } /and not sufficient/{ n++ } END{ print n+0 }' docs/pattern/001_proof_token_orders_two_operations.md )"
printf 'does the sketch warn about it: %s\n' "$( awk '/^### Regenerate/{ exit } /mints without limit/{ n++ } END{ print n+0 }' docs/pattern/001_proof_token_orders_two_operations.md )"
```

Live output:

```
what the applicability claim says: `ring_shutdown` matches all four: draining without a prior close cannot even
rows in that table:            4
rows the claim then cites:     row 1 row 2 row 3 row 4 
the ordering the doc names:    Release
the ordering close uses:       Release
and the one is_closed uses:    Acquire
orderings appearing at all:    Ordering::Acquire Ordering::Release 
rule 1 of the pattern:         **The token's only constructor performs A.** If it can be built another way,
how the sketch spells A:       fn a( &self ) -> Token< '_ > { /* do A */ Token { subject : self } }   // only constructor
how the crate spells A:        pub fn close( &self ) -> Stopped< '_ >
rule 2 of the pattern:         **B is a method on the token, not on the subject.** If B remains reachable
what the token hands back:     pub const fn shutdown( &self ) -> &'a Shutdown
does the rule set admit a gap: 1
does the sketch warn about it: 1
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_stopped_proof_token.md](../type/001_stopped_proof_token.md) | The instance, with the alternatives it was chosen over |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_close_is_advisory_to_an_unguarded_producer.md](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | The "A is unenforceable" failure mode, in this crate |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/pattern/001`](../../../ring_core/docs/pattern/001_uniform_surface_over_unequal_backends.md) | The family's other extracted pattern, with the same "known failure mode" section shape |

### SD37 — The Applicability Argument Names the Wrong Memory Ordering, and Cites Three of the Four Rows It Claims

The sentence that qualifies this crate as an instance reads *"`ring_shutdown`
matches all four: `drain_all` against an open ring **hangs** (row 2), `close` is
one **relaxed** store and idempotent (row 3), and teardown is a boundary (row
4)"*.

`close` is not one relaxed store. It is `self.closed.store( true, Ordering::Release )`,
paired with `is_closed`'s `Ordering::Acquire` load — the only two orderings that
appear anywhere in `src/lib.rs`. The pairing is load-bearing, not stylistic: a
producer that observes the closed flag must also observe everything the closing
thread wrote before it, which is what `Guarded::try_push` depends on when it
reads the flag and then refuses. Substituting `Relaxed` would compile, pass every
test in the suite, and remove the happens-before edge that makes the refusal mean
anything on a weakly-ordered host — which is what this workspace runs on.

The claim is also arithmetically short. The table has four rows; the
justification cites rows 2, 3 and 4 and says "all four". Row 1 — *"B's failure
when A did not happen is **not an error**"* — does hold here, so the conclusion
is right and the argument for it is missing. That is the worse shape for a
document whose stated purpose is to be a checklist for the *next* crate: a
reader applying it to their own case has a worked example for three conditions
and a bare assertion for the fourth.

Both errors are in one sentence, and the sentence is the bridge between the
general pattern and its only named instance.

**Disposition:** applied — the applicability sentence now cites all four rows
(row 1 added) and names the real ordering pair (`Release`/`Acquire`) instead
of "relaxed". The file's own `### Regenerate` block extracts this sentence
verbatim, so its extraction command and cached output were updated in the
same pass to stay consistent with the corrected text. Now prints: `Release`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
command sed -n 's/.*is a `\([A-Za-z]*\)` store paired with.*/\1/p' docs/pattern/001_proof_token_orders_two_operations.md
awk '/^### Regenerate/{ exit } /matches all four/{f=1} f&&/row [0-9]/{ print }' docs/pattern/001_proof_token_orders_two_operations.md | command grep -oE 'row [0-9]' | sort -u | tr '\n' ' '
command grep -oE 'Ordering::[A-Za-z]+' src/lib.rs | sort -u
```

Live output:

```
Release
row 1 row 2 row 3 row 4 Ordering::Acquire
Ordering::Release
```

### SD38 — The Pattern's Own Reference Sketch Contained the Defect Its Instance Exhibits

The Solution sketch spells A as `fn a( &self ) -> Token< '_ >` and comments it
*"only constructor"*, which satisfies rule 1 as written. `ring_shutdown` spells
A as `pub fn close( &self ) -> Stopped< '_ >` — the same shape. Both take a
shared borrow, so both mint without limit, and `undo_a( self )` / `reopen( self )`
consume only the receiver they were called on.

So the sketch satisfies all three rules and still permits the exact failure rule
3 was written to prevent. Rule 3's own explanation describes it precisely — *"the
failure only appears when a caller re-enters the pre-A state and then calls B
with a stale proof"* — and calls it *"the one most often skipped"*, while the
crate skips nothing and produces the failure anyway (→ [`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md),
[`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md),
[`../lifecycle/002`](../lifecycle/002_open_and_closed.md)). Rule 1 bounds *how* a
token is constructed and says nothing about *how many*; rule 3 bounds what
happens to *the* token and says nothing about the others.

Rule 2 has a mirror-image gap. It reads *"B is a method on the token, not on the
subject. If B remains reachable on the subject, the token is decorative"* — and
guards only that direction. `Stopped::shutdown( &self ) -> &'a Shutdown` runs the
other way: the token hands the subject back, from a shared borrow, so the subject
is reachable from the token and A is callable again from inside B's own scope.
The rule is stated as "B not on the subject" when what it needs is that the token
and the subject not be freely interconvertible.

A pattern document is the one artifact in a corpus whose defects are meant to be
copied. This one is well-argued, honest about its failure mode, and carried in
its own five-line sketch the bug the rest of this crate's corpus records five
times — SD15 and SD16 in `decisions/`, SD23 in `invariant/`, SD31 in
`lifecycle/`, and SD39 next door in [`002`](002_a_proof_token_must_be_scarce.md),
which states the missing rule outright.

The repair is not a fourth rule here. [`002`](002_a_proof_token_must_be_scarce.md)
already *is* rule 4, with four spellings and their costs, and renumbering this
document to four would leave two rulebooks claiming the same number — the corpus
says "three rules" in seven places and every one of them is true of what this
document contains. What was wrong was not the count but that a reader could
finish the list believing it complete, and copy a sketch whose `&self` receiver
is the defect. So the list now ends by saying so and pointing at `002`, and the
sketch's `fn a( &self )` line carries a warning comment directly under it. The
annotation *"only constructor"* is untouched, because five documents quote it
and it remains accurate — it is the true statement whose insufficiency is the
finding.

**Disposition:** applied — the rules list now closes with an explicit
incompleteness paragraph naming
[`002`](002_a_proof_token_must_be_scarce.md)'s rule 4, and the Solution sketch
carries an inline warning under the `&self` receiver so the copyable line is no
longer silently exemplary. Now prints: `does the sketch warn about it: 1`
