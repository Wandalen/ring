# Type Doc Definition

### Scope

- **Purpose**: The two types that carry this crate's arguments rather than its data — a proof token and a two-armed refusal.
- **Responsibility**: What each type makes unspellable, and what it costs.
- **In Scope**: `Stopped`, `Refusal`, `Wake`.
- **Out of Scope**: `RingError`, which is `ring_types`' (→ [`ring_types`](../../../ring_types/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The `Stopped` Proof Token](001_stopped_proof_token.md) | A value whose only purpose is that it cannot be obtained without closing first | 🔄 |
| 002 | [`Refusal` Carries the Record, Not an Error](002_refusal_carries_the_record.md) | Two arms rather than a `RingError`, because the record must survive both | 🔄 |

**The split is what the type is spending itself on.** `001`'s `Stopped` buys an
*ordering* — it is a pointer with no state whose entire job is to make one
sequence of calls unspellable. `002`'s `Refusal` buys *information* — two arms
and a payload, so a refused push tells the caller both what to do next and hands
back the record to do it with. One removes an expression from the language; the
other adds facts to a return value.

They are separate documents because their failure modes are opposite. A proof
token fails by proving too much: it is a valid token for a fact that stopped
being true, and nothing in the type says so. An information-carrying enum fails
by being ignored: every arm is right, and the caller never looked. Merging them
would blur the one thing a reader needs to take away, which is that the first
type protects you whether you cooperate or not and the second protects you only
if you read it.

The four findings land on that split exactly. **`001`'s** are about the boundary
of what an ordering proof covers and who says so: the `Option< Stopped >`
alternative sits in its Alternatives Rejected table as settled and in
[`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md) as
`❓ open`, with the decision citing the type document twice and the type document
citing the decision not once (SD45), and its headline property is stated without
the qualifier its own manual probe writes down — a caller can drain an open ring
any time they like through `ring_core::Consumer`, and the token only forecloses
reaching *this crate's* drain (SD46). **`002`'s** are about information that is
wrong and information that is discardable: the reachability table names two of
the family's three overflow policies and gets `Refusal::Full`'s trigger wrong
for the third, which on the default build is `Fail` wearing another name (SD47),
and `Wake`, the enum introduced so a producer cannot mistake *closed* for
*ready*, carries no `#[ must_use ]`, so a single `?;` discards it under
`-D warnings` (SD48).

Read together the two halves rhyme: an ordering proof is worth exactly the
scope its verification records, and an informational type is worth exactly the
attribute that makes ignoring it a warning. Neither is worth its doc comment.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/type
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each shows its definition:    %s\n' "$( command grep -lc '^### Definition' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each states its validation:   %s\n' "$( command grep -lc '^### Validation' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'public types in the crate:    %s\n' "$( cd ../..; command grep -cE '^pub (struct|enum) ' src/lib.rs )"
printf 'their names:                  %s\n' "$( cd ../..; command grep -ohE '^pub (struct|enum) [A-Za-z]+' src/lib.rs | awk '{ printf "%s ", $3 }' )"
printf 'of those, documented here:    %s\n' "$( cd ../..; for n in $( command grep -ohE '^pub (struct|enum) [A-Za-z]+' src/lib.rs | awk '{ print $3 }' ); do command grep -lq "pub \\(struct\\|enum\\) $n" docs/type/[0-9][0-9][0-9]_*.md && echo x; done | wc -l )"
printf 'must_use attributes in src:   %s\n' "$( cd ../..; command grep -c '#\[ must_use' src/lib.rs )"
printf 'of those, on a type:          %s\n' "$( cd ../..; command grep -A2 '#\[ must_use' src/lib.rs | command grep -cE '^src/lib.rs.pub (struct|enum)|^pub (struct|enum)' || true )"
printf 'derives on the two enums:     %s\n' "$( cd ../..; command grep -B1 '^pub enum ' src/lib.rs | command grep -o 'derive( [^)]*)' | tr '\n' ' ' )"
printf 'derives on the token:         %s\n' "$( cd ../..; command grep -B1 '^pub struct Stopped' src/lib.rs | command grep -o 'derive( [^)]*)' )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
each shows its definition:    2
each states its validation:   2
public types in the crate:    5
their names:                  Shutdown Stopped Refusal Guarded Wake 
of those, documented here:    3
must_use attributes in src:   12
of those, on a type:          1
derives on the two enums:     derive( Debug, Clone, Copy, PartialEq, Eq ) derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) 
derives on the token:         derive( Debug )
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD45 | the same alternative is filed as rejected here and as open four directories away | n/a — inconsistency | [`001`](001_stopped_proof_token.md)'s Alternatives Rejected table dismisses *"`close` returning `Option< Stopped >`, `None` on an already-closed flag"* because it *"Breaks idempotence, which teardown needs: an unwind path and a normal path both close, and neither should have to know which is first"*, while option 3 of [`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md) is `close( &self ) -> Option< Stopped >` under a document whose Status is `❓ open`, costed in different words with the same argument — *"Kills the idempotence property `close`'s doc names as the reason it exists"*; one alternative, one objection, two files, two statuses, and a reader of either concludes the opposite thing about whether the design space is closed, with the corpus unable to notice because nothing compares two documents; the citation graph runs one way only — the decision cites the type document twice, the type document's body cites the decision zero times — so the newer open record knew about the older closed one and the reader arriving here first never learns it exists, and the general rule is that an *Alternatives Rejected* table asserts a status invisible to the decision system that owns statuses. |
| SD46 | the headline property is stated without the qualifier its own verification probe writes down | **misleading doc** | [`001`](001_stopped_proof_token.md)'s first validation property ends *"There is no expression that reaches a drain without closing first"* — unqualified, the crate's strongest claim, and the premise [`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md) builds termination on — while `tests/manual/readme.md` D1, the probe this document names as its verification, records both compile errors (`E0599`, `E0382`) and then adds a paragraph headed *"What this stage does not establish"*: *"That a caller **cannot** drain an open ring. They can: `split()` hands out a `Consumer` and `try_recv_batch` is public on it."*; both `try_recv` and `try_recv_batch` are public on `ring_core::Consumer` and `drain_all` is a four-line loop over the second, so a hand-written loop on an open ring reproduces the hang the token forecloses with no `Stopped` in the program, and what property 1 actually says is *there is no expression that reaches* **this crate's** *drain without closing first* — the qualifier exists, is precise, and lives in the artifact that verifies the claim rather than the one that makes it, the same shape as SD23 reached from the other side. |
| SD47 | the reachability table knows two overflow policies and the family declares three | **wrong doc** | [`002`](002_refusal_carries_the_record.md) names `DropNewest` and `Fail` while `OverflowPolicy` declares `DropNewest` (the `#[ default ]`), `DropOldest` and `Fail`, and the omission falsifies the one claim the table adds beyond the signature: it says `Refusal::Full` needs *"the ring was full **and** the policy is `OverflowPolicy::Fail`"*, but `ring_core::Producer::try_push` ends `Resolution::DroppedIncoming => Ok( () ), _ => Err( record )` and `would_resolve( DropOldest ) = Resolution::EvictedOldest`, so `DropOldest` also produces `Refusal::Full` and a caller reading this table matches that arm as dead; worse, `EvictedOldest` is documented *"The oldest unread item was discarded to make room; the incoming item was accepted"* and neither happened — the only evicting code is `if self.overflow == OverflowPolicy::DropOldest { let _evicted = queue.force_push( record ); return Ok( () ); }` inside the arm gated by `feature = "crossbeam"`, and `ring_core` declares `default = []` — so on the default build `DropOldest` **is** `Fail` under another name, producing a transient refusal whose documented response is *"Retry, with back-pressure"*, which is what the caller picked that policy to avoid; the column heading is where it breaks, since *"Reachable under the default config?"* answers for `RingConfig`'s default on two rows and turns on `ring_core`'s **feature** default on the third. |
| SD48 | the enum that exists to stop a publish was one `?` away from being discarded, and now carries the attribute | **latent hazard** | `Wake` exists because *"a producer that read 'the wait succeeded' and went on to publish would publish into a closed ring"*, and it is returned as `Result< Wake, RingError >` — so `for_space_or_close( … );` warns on `Result`'s own `#[ must_use ]` while `for_space_or_close( … )?;` does not, the `?` satisfying that attribute and leaving a `Wake` in expression-statement position where nothing objects, because neither `Wake` nor `Refusal` carried `#[ must_use ]`; all nine in the file sat on accessors and predicates (`new`, two `is_closed`, `reason`, `free_capacity`, `is_blocked`, two `shutdown`, `is_ready`), so the crate marks *reading* the answer and not *having* it, and one line written by reflex inside any `Result`-returning function restores the exact merge the type was introduced to prevent, clean under `-D warnings`; `#[ must_use = "a Wake::Closed means stop, not publish" ]` on the enum makes `expr?;` warn and touches no caller who reads the value, and the moment to add it was then: it sits on the enum now, so `for_space_or_close( .. )?;` in statement position warns under `-D warnings` while `let wake = ..?;` stays silent, and zero call sites needed editing, as predicted — unlike the same omission on `close` (→ SD30), where the discarding form was already written seven times and every one had to be respelled. |
