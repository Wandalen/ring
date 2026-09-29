# Pattern Doc Definition

### Scope

- **Purpose**: The reusable shape this crate is an instance of: ordering two operations by making the second unspellable without the first.
- **Responsibility**: Structure, applicability conditions, cost, and the failure mode.
- **In Scope**: The proof-token pattern as a general technique, and the scarcity condition its three rules omit.
- **Out of Scope**: This crate's own use of it (→ [`type/001`](../type/001_stopped_proof_token.md)); whether this crate should change to satisfy rule 4 (→ [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Proof Token Orders Two Operations](001_proof_token_orders_two_operations.md) | When a token beats a documented precondition, and when it is only theatre | 🔄 |
| 002 | [A Proof Token Must Be Scarce](002_a_proof_token_must_be_scarce.md) | The fourth rule: undoing A must invalidate every token, not the one it was called on | 🔄 |

**The split is the pattern and its missing condition.** `001` extracts the shape
— three rules, four applicability conditions, and an honest section on when the
whole technique is theatre. `002` states the condition those three rules do not
imply: that the token's *supply* must be bounded, not only its construction and
its consumption.

They are separate documents rather than a fourth rule appended to `001` because
they are answerable at different times. `001`'s three rules are checkable while
writing the token type. `002`'s question — can A be called twice between two
undos — is checkable only once an undo exists, which is often a later release.
Folding it in would also lose what makes it worth writing: `001` is a correct
pattern for the problem it was extracted from, and `002` is the record of the
problem it was not.

The four findings run along that seam. `001`'s are about **an extraction that is
slightly wrong about its own only instance**: the applicability argument names
`relaxed` where the code has `Release`/`Acquire` and claims four rows while
citing three (SD37), and the reference sketch spells A as `fn a( &self )` — the
same unbounded-supply shape — so it satisfies all three rules and still permits
the failure rule 3 exists to prevent (SD38). `002`'s are about **what that
omission cost downstream**: the source doc comment claims a second close
*"returns the same token"* when it constructs a new one, the fifth and most
misleading statement of one fact (SD39), and the cheapest available fix is
absent from every options table because the pattern never posed the question
that would have required justifying its absence (SD40).

SD40 is the general lesson and the reason `002` exists at all: a pattern that
does not state a condition removes the place where an instance's failure to meet
it would have been written down.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/pattern
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rules stated in 001:          %s\n' "$( awk '/^### Regenerate/{ exit } /^[0-9]\. \*\*/{ n++ } END{ print n+0 }' 001_proof_token_orders_two_operations.md )"
printf 'rules stated in 002:          %s\n' "$( command grep -c '^\*\*Rule 4' 002_a_proof_token_must_be_scarce.md )"
printf 'the ordering 001 names:       %s\n' "$( awk '/^### Regenerate/{ exit } /is one .* store and idempotent/{ sub( /.*is one /, "" ); sub( / store.*/, "" ); print }' 001_proof_token_orders_two_operations.md )"
printf 'the orderings the code uses:  %s\n' "$( cd ../..; command grep -ohE 'Ordering::[A-Za-z]+' src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'how the sketch spells A:      %s\n' "$( awk '/^### Regenerate/{ exit } /fn a\( /{ sub( /^ */, "" ); sub( / ->.*/, "" ); print }' 001_proof_token_orders_two_operations.md )"
printf 'how the crate spells A:       %s\n' "$( cd ../..; command grep -o 'pub fn close( &self )' src/lib.rs )"
printf 'crates outside adopting it:   %s\n' "$( cd ../../..; command grep -rl 'proof token\|proof-token' ring_*/docs --include='*.md' | cut -d/ -f1 | sort -u | command grep -vc ring_shutdown || true )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
rules stated in 001:          3
rules stated in 002:          1
the ordering 001 names:       
the orderings the code uses:  Ordering::Acquire Ordering::Release 
how the sketch spells A:      fn a( &self )
how the crate spells A:       pub fn close( &self )
crates outside adopting it:   1
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD37 | the applicability argument names the wrong memory ordering and cites three of four rows | **wrong doc** | [`001`](001_proof_token_orders_two_operations.md)'s bridge sentence — the one linking the general pattern to its only named instance — reads *"`ring_shutdown` matches all four: `drain_all` against an open ring **hangs** (row 2), `close` is one **relaxed** store and idempotent (row 3), and teardown is a boundary (row 4)"*, and `close` is `self.closed.store( true, Ordering::Release )` paired with `is_closed`'s `Ordering::Acquire`, the only two orderings anywhere in `src/lib.rs`; the pairing is load-bearing rather than stylistic, since a producer observing the closed flag must also observe what the closing thread wrote before it — which is what `Guarded::try_push` relies on when it reads and refuses — and substituting `Relaxed` would compile, pass the whole suite, and delete the happens-before edge on the weakly-ordered host this workspace runs; the same sentence also says "all four" while arguing rows 2, 3 and 4, and row 1 does hold, so a document written to be a checklist for the next crate offers a worked example for three conditions and a bare assertion for the fourth. |
| SD38 | the pattern's own reference sketch contained the defect its instance exhibits, and now warns about it | **latent hazard** | [`001`](001_proof_token_orders_two_operations.md)'s Solution spells A as `fn a( &self ) -> Token< '_ >` annotated *"only constructor"*, the same shape as `pub fn close( &self ) -> Stopped< '_ >`, so both mint without limit while `undo_a( self )` / `reopen( self )` consume only their own receiver — the sketch satisfies all three rules and still permits precisely the failure rule 3 was written to prevent, which rule 3 itself describes as *"a caller re-enters the pre-A state and then calls B with a stale proof"* and calls *"the one most often skipped"*; rule 1 bounds how a token is constructed and not how many, rule 3 bounds what happens to *the* token and not the others, and rule 2 has the mirror gap — it forbids B on the subject while `Stopped::shutdown( &self ) -> &'a Shutdown` hands the subject back from a shared borrow, making A callable again from inside B's own scope, so what the rule needs is that token and subject not be freely interconvertible — a gap now stated outright: the rules list closes with an explicit incompleteness paragraph naming `pattern/002`'s rule 4, and the sketch carries an inline warning under the `&self` receiver, so the copyable line is no longer silently exemplary. |
| SD39 | the source doc comment says a second close returns the same token, and it constructs a new one | **wrong doc** | `Shutdown::close`'s doc comment reads *"Idempotent — closing an already-closed shutdown is not an error and **returns the same token**"* while the body is `self.closed.store( true, Ordering::Release ); Stopped { shutdown : self }`, constructing a fresh value every call, and `close_is_idempotent_and_admit_reports_it` binds two at once; the sentence is defensible as loose English — the two are observably equal, since `Stopped` holds one reference and no state — and indefensible in this crate, where token *identity* is the mechanism: [`001`](001_proof_token_orders_two_operations.md)'s rule 3 turns on which token the undo consumed and [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)'s enforcement argument turns on nothing being left after a `reopen`, so a reader taking "the same token" literally concludes a reopen ends both — exactly the guarantee the crate does not have — and this is the fifth statement of one fact across five documents, the only one a caller sees in their editor, and the one that is wrong. |
| SD40 | the cheapest fix is absent from every options table because the pattern never posed the question | n/a — doc gap | Of the four spellings [`002`](002_a_proof_token_must_be_scarce.md) enumerates, `fn a( &mut self ) -> Token< '_ >` is nearly free — one character, the borrow checker supplies the missing rule, no runtime cost, no new type, no change to any correct caller — and it is unavailable here for a good reason (`Shutdown` is shared across threads, so `&mut self` cannot be taken) that **no document states**: [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md) lists four options and this is not among them, [`001`](001_proof_token_orders_two_operations.md)'s sketch writes `&self` without comment, and the source doc comment asserts the opposite property (→ SD39); the general form is what turned a one-character question into five inconsistent statements — a pattern that does not state a condition cannot record why an instance fails it, so rule 4's absence removed the one place `close( &self )`'s justification would have been written down, leaving every later document to rediscover the gap on its own terms. |
