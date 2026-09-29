# workaround

External constraints absorbed on someone's behalf, each with the cost it imposes
and the condition under which it can be deleted. There are two, and they point in
opposite directions: one is a constraint `ring_index` absorbs from the crate
above it, and one is a constraint `ring_index` imposes on the crate below.

Both are deletable by a signature change, both deletions are currently free
because the affected functions have no external callers, and neither is recorded
anywhere in the source. That combination — cheap to remove, invisible from the
code — is what makes them worth writing down rather than fixing in passing.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_mask_that_lives_one_crate_up.md) | The Mask That Lives One Crate Up | Why the crate exists at all: `ring_types` publishes the bitmask instead of the fold |
| [002](002_the_iterator_ring_batch_built_instead.md) | The Iterator `ring_batch` Built Instead | `drain_order` as `run` with the allocation removed, and the test it carries |

## Absorbed From Above

`Capacity` has three methods: `new`, `get`, and `mask`. The fold's natural owner
is the type carrying the precondition it depends on, and that type declined to
own it — so `ring_index` exists to fill the gap, and `mask()` must stay public
for it to reach across the crate boundary.

That public `mask()` has three call sites family-wide. One is `of`'s body, one is
the test asserting `of` is a mask and not a division, and one is
`ring_mpsc/src/lib.rs:543` — the violation the boundary made possible. Moving
`of` onto `Capacity` as `pub const fn slot_of` and making `mask` private deletes
122 source lines, 188 test lines, one manifest, one workspace member, and stops
line 543 compiling.

## Imposed Below

`run` returns `Vec< SlotIndex >`. `ring_batch` needed exactly that computation,
had a lazy sequence iterator in hand, and would have had to collect it and zip it
back — so it wrote `drain_order`, which is the same fold as a `Map`, yielding
`( Seq, SlotIndex )` pairs and allocating nothing.

It did so carefully: the workaround carries
`drain_order_folds_through_ring_index_and_not_a_second_implementation`, the only
place in the 33 crates where the one-owner rule is asserted mechanically rather
than stated in prose.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- Capacity, and every caller of its third method --'
command grep -nE '^\s*pub (const )?fn ' ring_types/src/capacity.rs
command grep -rn '&[^&]*\.mask()' --include=*.rs /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep -v '^ring_types/'
echo '  -- every import from ring_index, family-wide --'
command grep -rn 'use ring_index' --include=*.rs /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep -v '^ring_index/'
echo "  ring_index size: $( wc -l < ring_index/src/lib.rs ) src + $( wc -l < ring_index/tests/index_test.rs ) test lines"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX45 | `ring_types` | n/a — observation | The whole crate exists because `Capacity` publishes `mask()` rather than the fold; every item `of`'s body touches is re-exported from `ring_types` two lines apart |
| IX46 | `ring_types` | **latent hazard** | Keeping `mask()` public across a crate boundary is the door `ring_mpsc:543` walked through; making the fold an inherent method would make that line a compile error |
| IX47 | `ring_batch` | n/a — duplication | `drain_order` is `run` with `.collect()` removed and the `Seq` paired in; the arithmetic is identical and nothing in `ring_index` records that its version was declined |
| IX48 | `ring_batch` | n/a — coverage | The workaround carries the only mechanical assertion of the one-owner rule in the family, and neither it nor `run` has a caller outside its own crate |
