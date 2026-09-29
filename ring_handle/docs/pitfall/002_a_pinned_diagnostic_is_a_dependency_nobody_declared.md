# Pitfall: A Pinned Diagnostic Is a Dependency Nobody Declared

### Scope

- **Purpose**: Warn the maintainer who breaks this crate's test suite from a crate that does not depend on it, and give them the shortest route to the cause.
- **Responsibility**: State the trap, who walks into it, what it looks like when they do, and what to do instead of the obvious thing.
- **In Scope**: `tests/ui/*.stderr` as an undeclared reverse dependency; the `TRYBUILD=overwrite` reflex.
- **Out of Scope**: Why the suite exists (→ [`workaround/001`](../workaround/001_asserting_an_absence_needs_a_second_compiler_run.md)); the full cost accounting (→ [`workaround/002`](../workaround/002_the_expected_output_names_crates_this_one_never_sees.md)).

### The Trap

**A `.stderr` fixture is a contract on text the compiler happens to print, and
compiler diagnostics quote whatever types are involved — including private ones,
from crates the fixture's own crate has never heard of.**

`ring_handle` declares one dependency: `ring_core`. One of its seven pinned
diagnostics quotes `ring_spsc::Producer`, `ring_slot::TypedSlot`, and
`ring_core::ProducerInner` — a private enum. All three arrive because the
compiler walks the composition chain when it explains a `Sync` violation.

So the crate has three reverse-dependency edges that exist only in a fixture:

| Whose edit breaks it | Where they would look | Where the breakage is |
|----------------------|-----------------------|-----------------------|
| Someone renaming `ring_slot::TypedSlot` | `ring_slot`'s own consumers | `ring_handle/tests/ui/producer_shared_across_threads.stderr` |
| Someone renaming `ring_spsc::Producer` | `ring_spsc`'s consumers | Same file |
| Someone renaming `ring_core::ProducerInner` | Nowhere — it is private | Same file |

The third row is the sharp one. **A private type has no consumers by
definition**, so the refactor is one nobody would think to check.

### What It Looks Like

`cargo test -p ring_handle` fails one test, `ui`, with a `trybuild` diff
against `producer_shared_across_threads.stderr`. The diff shows a type name
changing in a `note:` line. Nothing about the failure mentions ownership,
handles, cloning, or any property this crate is about — because the property
under test did not change.

### The Reflex to Resist

`TRYBUILD=overwrite cargo test -p ring_handle` makes it go away, and **that is
correct in this case and catastrophic in the neighbouring one.** The same
command, run against a diff caused by `Producer` genuinely becoming `Sync`,
accepts the regression permanently and silently.

### What to Do Instead

1. Read the diff. If every changed line is a `note:` naming a type, and the
   `error[E....]` line and the `help:` line are unchanged, the property still
   holds and regeneration is safe.
2. If the `error` code or the `help` text changed, stop. The property may have
   changed.
3. Either way, run `both_handles_are_send` afterwards — it is the positive
   assertion the compile-fail case is the boundary of, and it does not depend on
   any pinned text.

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_a_convenience_method_undoes_the_crate.md](001_a_convenience_method_undoes_the_crate.md) | The other way this crate's guarantee gets lost — by addition rather than by acceptance |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_asserting_an_absence_needs_a_second_compiler_run.md](../workaround/001_asserting_an_absence_needs_a_second_compiler_run.md) | Why a fixture is the mechanism at all |
| [../workaround/002_the_expected_output_names_crates_this_one_never_sees.md](../workaround/002_the_expected_output_names_crates_this_one_never_sees.md) | The reach measured |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | Reads the same pinned chain as an explanatory asset |

### Sources

| File | Relationship |
|------|--------------|
| [`tests/ui/producer_shared_across_threads.stderr`](../../tests/ui/producer_shared_across_threads.stderr) | The fixture with the reach |

### Tests

| Test | Relationship |
|------|--------------|
| `both_handles_are_send` | The pinned-text-free positive check to fall back on |

### HD37 — The One Coupled Fixture Documents Its Coupling and Not Its Cost

Six pinned files are inert; the seventh is coupled, and it says so:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- foreign crate names per fixture --'
for f in ring_handle/tests/ui/*.stderr; do
  printf '  %-38s %s\n' "$( basename "$f" )" \
    "$( command grep -hoE '\bring_[a-z_]+::' "$f" \
         | command grep -vE '^ring_handle::' | sort -u | tr -d ':' | tr '\n' ' ' )"
done
echo '  -- what the coupled case says about it --'
command grep 'ring_spsc\|inherit\|two crates' \
  ring_handle/tests/ui/producer_shared_across_threads.rs
echo '  -- and whether it warns about regenerating --'
printf '  mentions of TRYBUILD/overwrite/rename in that file: %s\n' \
  "$( command grep -ciE 'trybuild|overwrite|renam' \
       ring_handle/tests/ui/producer_shared_across_threads.rs )"
```

Live output:

```
  -- foreign crate names per fixture --
  consumer_clones.stderr                 
  consumer_publishes.stderr              
  producer_clones.stderr                 
  producer_drains.stderr                 
  producer_shared_across_threads.stderr  ring_core ring_slot ring_spsc 
  producer_try_clones.stderr             
  ring_used_after_split.stderr           
  -- what the coupled case says about it --
//! Not from this crate. `ring_spsc`'s handles carry a
//! and then `ring_handle` inherit it structurally.
//! **That inheritance is the reason this case is worth writing rather than
//! assuming.** The property holds today for a reason two crates away, and
  -- and whether it warns about regenerating --
  mentions of TRYBUILD/overwrite/rename in that file: 0
```

`producer_shared_across_threads` is the only fixture naming anything outside
`ring_handle`, and its `.rs` file carries eighteen lines of module doc that
explain exactly why: the `!Sync` property is inherited from `ring_spsc` through
`ring_core`, and that inheritance is the reason the case is worth writing.

**It documents the property's provenance and never the fixture's fragility.**
Those are different facts. The comment tells a reader why `&Producer` is not
sendable and which crate owns the reason — genuinely useful, and more than most
fixtures get. What it does not say is that the *expected output file beside it*
now pins `ring_slot::TypedSlot` and a private `ring_core::ProducerInner`, so a
rename in either crate turns this test red for a reason unrelated to the
property the comment is about.

The gap is narrow and consequential: the person reading that comment is
reasoning about `Sync`, and the person who will break the test is reasoning
about a rename.

### HD38 — The Positive Test and the Compile-Fail Test Cover the Same Property and Only One Is Fragile

Two tests assert the `Send`/`Sync` boundary from opposite sides:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the positive assertion --'
sed -n '/^fn both_handles_are_send/,/^}/p' ring_handle/tests/handle_test.rs \
  | command grep -vE '^\s*//'
echo '  -- and what the negative one depends on --'
printf '  lines of pinned text: %s\n' \
  "$( wc -l < ring_handle/tests/ui/producer_shared_across_threads.stderr )"
```

Live output:

```
  -- the positive assertion --
fn both_handles_are_send()
{
  fn assert_send< T : Send >() {}

  assert_send::< ring_handle::Split< u32 > >();
  assert_send::< ring_handle::Producer< '_, u32 > >();
  assert_send::< ring_handle::Consumer< '_, u32 > >();
}
  -- and what the negative one depends on --
  lines of pinned text: 40
```

`both_handles_are_send` is a handful of lines with no external text dependency.
The compile-fail case that asserts the complementary `!Sync` property depends on
every line of a 30-plus-line pinned diagnostic.

**That asymmetry is not avoidable and it is worth knowing which side to trust
under churn.** The positive test fails only when the property fails. The
negative test fails when the property fails *or* when the compiler rephrases
*or* when a private type two crates away is renamed. Three causes, one signal,
and the two spurious ones are far more frequent than the real one.

When both are red, read the positive one first.
