# Workaround: A Third Dependency Two Documents Still Name

### Scope

**Purpose:** Record that both of this crate's prose documents name a third
dependency the manifest does not declare and the source does not use, establish
that the manifest is the only one of the three artefacts that recorded the
correction, and locate the decomposition assumption that made the edge look
necessary.

**Responsibility:** The `Depends on` line in `readme.md` and in task 115, the
manifest's two entries, the family-wide `ring_seqno` edge census, and where the
`Seq` type this crate's tests use actually lives.

**In Scope:** `ring_event/readme.md:5`;
`ring_event/task/unverified/115_implement_ring_event.md:19`;
`ring_event/Cargo.toml`; `ring_event/tests/event_test.rs:23`;
`ring_types/src/id.rs:25`.

**Out of Scope:** The task file's stage and its unwritten sections are
[`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md).
The dev-dependency that *is* declared is
[`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md).

---

## Three Artefacts, Two Answers

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the two documents claim --'
# The narrowed forms, for the reason recorded under EV51 below: a whole-file
# literal match on the pre-fix sentence goes silently empty the moment the fix
# this instance called for is applied.
command grep -m1 -A1 '^Depends on' ring_event/readme.md
command grep -m1 '^\*\*Depends on:\*\*' ring_event/task/unverified/115_implement_ring_event.md
echo '  -- what the manifest declares --'
sed -n '/^\[dependencies\]/,/^$/p' ring_event/Cargo.toml | command grep '^ring_'
echo '  -- ring_seqno edges across the family: readme / manifest / src --'
printf '  %-16s %-8s %-9s %s\n' crate readme manifest src
for c in ring_*/; do
  n=$( basename "$c" )
  r=$( command grep -c '^Depends on.*ring_seqno' "$c"readme.md 2>/dev/null || true )
  m=$( command grep -c '^ring_seqno *=' "$c"Cargo.toml 2>/dev/null || true )
  s=$( command grep -rc 'ring_seqno::' --include=*.rs "$c"src 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )
  if [ "$r" != 0 ] || [ "$m" != 0 ] || [ "$s" != 0 ]; then printf '  %-16s %-8s %-9s %s\n' "$n" "$r" "$m" "$s"; fi
done
echo '  -- and where the type this crate actually uses lives --'
command grep -r '^pub struct Seq' --include=*.rs ring_types/src ring_seqno/src
command grep 'use ring_' ring_event/tests/event_test.rs
```

Live output:

```
  -- what the two documents claim --
Depends on [`ring_types`](../ring_types/readme.md) and
[`ring_slot`](../ring_slot/readme.md).
**Depends on:** `ring_types`, `ring_slot`
  -- what the manifest declares --
ring_types = { path = "../ring_types" }
ring_slot = { path = "../ring_slot" }
  -- ring_seqno edges across the family: readme / manifest / src --
  crate            readme   manifest  src
  ring_batch       1        1         1
  ring_consume     1        1         1
  ring_cursor      1        1         3
  ring_gating      1        1         1
  ring_seqno         0        0         5
  ring_spsc        0        0         1
  -- and where the type this crate actually uses lives --
ring_types/src/id.rs:pub struct Seq( pub u64 );
use ring_store::Buffer;
use ring_event::{ drain_from, publish_into, recycle, Fill, Peek };
use ring_slot::{ BytesSlot, Slot, TypedSlot };
use ring_types::{ Capacity, RingError, Seq };
```

---

### EV51 — The Manifest Is the Only Artefact That Recorded the Correction

Three documents state this crate's dependencies and two of them agree with each
other rather than with the code. `readme.md:5` names `ring_types`, `ring_slot`
and `ring_seqno`, with a working markdown link to the third. Task 115's
`**Depends on:**` line names the same three. `Cargo.toml` declares two, and the
source imports from `ring_seqno` zero times.

This is not a typo in one place. Both prose documents descend from the same
early decomposition, which task 115 traces explicitly, and both preserve the
edge that decomposition assigned. Somewhere between the
design and the implementation the edge was found unnecessary, the manifest was
written without it, and neither document was revisited.

The family census shows the same seam in both directions. `ring_trace` carries
the identical pair and is equally finished — readme and task file naming
`ring_seqno` against a manifest that does not declare it and a source that imports
it zero times — so the error is reproduced rather than isolated. `ring_atomic`
was the mirror image: readme, manifest and design agreed, and the source used it
zero times, a declared dependency that nothing imported — since corrected in
both readme and manifest, so the census above no longer lists it at all. Of the
six crates with a `ring_seqno` edge in any artefact, three have it in every one and
three do not.

**Finding.** The manifest is the artefact a build enforces, so it is the one
that gets corrected; the two that only a reader enforces stay as designed. That
is the general mechanism, and it is worth naming because the corpus's own
[`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md)
finds the same asymmetry in the same task file — a state marker reading
`❓ Unverified` over a crate whose tests pass. Both fixes are one line each and
they are in the same two files, so they should land together: dropping the third
linked crate from the `Depends on` sentence at `readme.md:5`, and `, ring_seqno`
from task 115's line 19.

```sh
cd "$(git rev-parse --show-toplevel)"
# The claim is about two sentences, so the recipe reads those two sentences.
# A whole-file `grep -c` was the earlier instrument and it is the wrong one:
# see the paragraph below this block.
echo '  -- the readme dependency sentence --'
command grep -m1 -A1 '^Depends on' ring_event/readme.md
echo '  -- task 115 Depends-on line --'
command grep -m1 '^\*\*Depends on:\*\*' ring_event/task/unverified/115_implement_ring_event.md
echo '  -- ring_seqno named in each of those two, and in the manifest a build enforces --'
printf '    readme dependency sentence : %s\n' "$( command grep -m1 -A1 '^Depends on' ring_event/readme.md | command grep -c 'ring_seqno' )"
printf '    task 115 Depends-on line   : %s\n' "$( command grep -m1 '^\*\*Depends on:\*\*' ring_event/task/unverified/115_implement_ring_event.md | command grep -c 'ring_seqno' )"
printf '    Cargo.toml                 : %s\n' "$( command grep -c 'ring_seqno' ring_event/Cargo.toml )"
```

Live output:

```
  -- the readme dependency sentence --
Depends on [`ring_types`](../ring_types/readme.md) and
[`ring_slot`](../ring_slot/readme.md).
  -- task 115 Depends-on line --
**Depends on:** `ring_types`, `ring_slot`
  -- ring_seqno named in each of those two, and in the manifest a build enforces --
    readme dependency sentence : 0
    task 115 Depends-on line   : 0
    Cargo.toml                 : 0
```

**The recipe had to be narrowed, and the reason is this document.** It used to
count `ring_seqno` across the whole of both files and assert both counts were
zero. Task 115 then gained an `## Implementation Record` — a section written to
record that the `Depends-on` line had been corrected — and the sentence "The
`Depends-on` line once named a third (`ring_seqno`)" pushed the whole-file count
from 0 to 1. Nothing regressed: line 19 still reads
`**Depends on:** ring_types, ring_slot`. The measurement broke because it was
measuring the file when the claim was about one line of it, so a paper trail
describing the fix registered identically to the defect returning. The narrowed
form is immune: the Implementation Record may say whatever it likes about
`ring_seqno`, and only the `**Depends on:**` line itself is read.

**Disposition:** applied — both one-line corrections this instance names are
made: `ring_event/readme.md`'s dependency sentence no longer names
`ring_seqno`, and
`ring_event/task/unverified/115_implement_ring_event.md`'s
`**Depends on:**` line does the same. Both now measure 0, against a manifest
that measures 0 as well. Now prints: `    task 115 Depends-on line   : 0`

---

### EV52 — The Edge Was Wrong Because the Type and the Arithmetic Live in Different Crates

The interesting question is not that the documents are stale but why the edge
looked right when it was assigned. This crate's tests do use sequences —
`land_and_read< S, P >( buffer : &mut Buffer< S >, seq : Seq, payload : P )` is
one of the two generic helpers, and four of the seventeen tests address
storage by one. A decomposition that lists a `ring_seqno` crate and a crate that
handles sequences will connect them.

But `Seq` is not in `ring_seqno`. `event_test.rs:23` imports it from
`ring_types`, and the census confirms `pub struct Seq` is declared at
`ring_types/src/id.rs:25` and nowhere in `ring_seqno/src` at all.
`ring_seqno` holds *arithmetic over* sequences — spans, folds, the readings that
answer how far apart two positions are — and this crate performs none of it. It
takes a `Seq`, hands it to `Buffer::at_mut`, and never does arithmetic on one.

So the naming carries an implication the layout does not honour: the type is in
the types crate, the operations on it are in the crate named after it, and a
consumer needs `ring_types` for the former, `ring_seqno` for the latter, or both.
Three of the family's crates need only the type and got the edge right; this one
needed only the type and was documented as needing both.

**Finding.** This is a decomposition seam rather than a defect in either crate,
and it will keep producing the same error as long as `Seq` and the operations on
`Seq` sit in differently-named crates. It costs one sentence in `ring_seqno`'s own
readme to defuse — that `Seq` itself is declared in `ring_types` and a crate
that only holds or passes one needs no edge here — which would have made this
crate's readme wrong at a glance rather than plausible. Recorded here rather
than filed against `ring_seqno` because this crate is the instance that proves the
implication misleads: an implementation finished, correct, and shipping two
documents that name a dependency it never had.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md) | The same task file, stale in a second way |
| [`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md) | The edges this crate does have |
| [`workaround/001`](001_a_slicing_expression_at_every_generic_call_site.md) | The other thing callers carry that nothing documents |
| [`data_structure/001`](../data_structure/001_a_crate_that_declares_no_data.md) | Why this crate needs so little |

### Sources

| Fact | Where |
|------|-------|
| The readme's three-dependency claim | `ring_event/readme.md:5` |
| The task file's identical claim | `ring_event/task/unverified/115_implement_ring_event.md:19` |
| The manifest's two entries | `ring_event/Cargo.toml` |
| The family-wide `ring_seqno` edge census | Census above |
| `Seq` declared in `ring_types`, not `ring_seqno` | `ring_types/src/id.rs:25`; census above |
| The test suite importing it from `ring_types` | `ring_event/tests/event_test.rs:23` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | The `Seq` use that made the edge look necessary |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | Another, addressing storage by sequence |
| `a_typed_payload_survives_storage_byte_identically` | A third, with no arithmetic on one |
