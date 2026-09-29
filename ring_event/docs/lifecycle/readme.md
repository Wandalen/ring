# lifecycle

Two lifecycles run through this crate and neither is written down. The first is
the slot's: three operations that move a slot between states, described one
function at a time and never as a sequence. The second is the crate's own: a
governing task file that still reads `❓ Unverified` above a finished library
with a green suite.

The pairing is deliberate. Both instances ask the same question — what does the
record say, and what is actually the case — at two very different scales, and
both find the record coarser than the thing it describes. A byte slot occupies
four positions and reports two; a crate has shipped and its work item says the
scope is not yet worked out.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_four_positions_three_representations_two_observable_states.md) | Four Positions, Three Representations, Two Observable States | The slot's lifecycle, and where observation stops following it |
| [002](002_a_finished_crate_in_the_unverified_stage.md) | A Finished Crate in the Unverified Stage | The crate's own lifecycle position, against what it has shipped |

## Two Transitions and an Observation

`publish_into` and `recycle` take `&mut S` and move the slot. `drain_from` takes
`&S`, calls `peek`, and leaves it exactly where it was. So a ring that publishes
and drains has moved its slot once and is still holding a payload; emptying it is
a third call with no return value and no error, which is the one easiest to omit.
`recycle`'s own doc calls all three "the operations a ring performs on a slot",
which flattens the distinction that matters most.

`TypedSlot`'s state machine is faithful — `Option< T >` has two inhabitants, both
observers read the same field, every position has a distinct answer. `BytesSlot`'s
is lossy in two directions at once: never-written and written-with-zero-bytes are
byte-identical, and recycled is distinct in memory but identical under both
`is_empty` and `peek`. Four positions, three representations, two observable
states — and nothing records that the shapes differ in this respect at all.

## A Marker That Cannot Distinguish Anything

Task 115 reads `❓ Unverified`, its Scope section says "Not yet worked out" and
its Verification section says "Not yet defined". The crate underneath is 234
lines with seventeen integration tests and five doctests, all passing. Every
one of the family's 33 task files sits in that same stage, and each crate
carries twelve stage directories holding one file between them.

So the marker separates nothing: a skeleton and a finished implementation wear
it identically. The harder gap is the Verification section, because something did
verify this crate — twenty-two assertions named after the properties they check —
and no document connects that work to the task that asked for it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three operations, whole --'
awk '/^pub fn publish_into< S, P >\( slot : &mut S, payload : P \) -> Result< \(\), RingError >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^\/\/\/ let slot = BytesSlot::< 4 >::empty\(\);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 8 { print } /^pub fn recycle< S >\( slot : &mut S \)$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 5 { print }' ring_event/src/lib.rs
echo '  -- the entire state vocabulary the Slot trait offers --'
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs
echo '  -- and what clear writes, per shape --'
awk '/^    self\.0\.is_none\(\)$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 6 { print } /^    Self::is_empty\( self \)$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print }' ring_slot/src/lib.rs
echo '  -- what the governing task says about this crate --'
awk '/^# Task 115: Implement `ring_event`$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^\*\*Depends on:\*\*/{ n2 = NR } n2 && NR >= n2 { print }' ring_event/task/unverified/115_implement_ring_event.md
echo '  -- which lifecycle stage every ring task sits in --'
for f in ring_*/task/*/*.md; do echo "$f" | cut -d/ -f4; done | sort | uniq -c
echo '  -- and what the crate has actually shipped --'
wc -l < ring_event/src/lib.rs
command grep -c '^#\[ test \]' ring_event/tests/event_test.rs || true
command grep -c '^/// ```' ring_event/src/lib.rs || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV29 | `ring_event` | n/a — doc gap | Of the three operations the crate defines, two are transitions and one is not: `publish_into` and `recycle` take `&mut S` and move the slot, `drain_from` takes `&S` and returns a borrow into a slot in exactly the state it was already in, so a ring that publishes and drains has moved its slot once and is still holding the payload — emptying it needs a third call with no return value and no error, the easiest of the three to omit, and omitting it holds a typed payload alive for the rest of the lap and leaves a byte slot readable; `recycle`'s doc groups all three as "the operations a ring performs on a slot" and no doc comment draws the sequence, so the reader must assemble it from three separate signatures, which is one reason `ring_core` reaches for `ring_slot::take` instead, where read-and-empty is a single call |
| EV30 | `ring_event` | n/a — doc gap | `Slot` offers two words and `Peek` one observation, which is the entire state vocabulary a generic caller has, and `TypedSlot` fits it exactly — `Option< T >` has two inhabitants, `clear` writes `None`, both observers read the one field — while `BytesSlot` occupies four positions that collapse twice: never-written and written-with-zero-bytes are byte-identical down to the `Debug` rendering, and recycled differs in memory (`[ 97, 98, 0, 0 ]` with `len` at `0`) while answering identically under both `is_empty` and `peek`, giving four positions, three representations, two observable states; each collapse is individually defensible and separately documented, but that the two shapes differ in faithfulness at all is recorded nowhere, so a reader who has understood the typed lifecycle has not understood the byte one |
| EV31 | `ring_event` | **wrong doc** | Task 115 carries `**State:** ❓ Unverified` — the lifecycle's pre-claim stage — with a Scope section reading "Not yet worked out" and explaining that a readiness gate requires concrete deliverables to be named there first, above a crate that is a 204-line library with two traits, three free functions, five doctests and a sixteen-test suite that all pass; every one of the family's 33 task files sits in that same stage and each crate carries twelve stage directories with one file among them, so the marker cannot distinguish a skeleton from a finished implementation and tells a reader the opposite of the truth about this one, and the deeper absence is that the file records nothing about what was actually built, leaving the crate itself as the only account of its own deliverables |
| EV32 | `ring_event` | n/a — doc gap | The task file's Verification section — the artifact that would define what "done" means here — reads "Not yet defined", while sixteen integration tests and five doctests already exercise both slot shapes through one generic body, the refusal path, both recycling paths, the zero-length reading and the emptiness invariant, and all twenty-one pass; nothing connects the two, the tests never reference the task and the task never references the tests, and the same file's `**Depends on:**` line still names `ring_seqno` beside the two dependencies the manifest actually declares, so it describes a shape the crate does not have and has not been revisited since — the crate is verified and its verification is unrecorded, which looks identical to unverified from the task file and is a different problem entirely |
