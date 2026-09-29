# Data Structure: The Policy Enum

### Scope

- **Purpose**: Examine the policy value as a structure — its layout, its size, and the cost of consulting it on a path that runs once per appended record.
- **Responsibility**: Fix the representation, the operations, and the properties the hot path depends on.
- **In Scope**: Layout and size; consultation cost; why it is not boxed, not a trait object, not a function pointer.
- **Out of Scope**: The variants' meanings and validation (→ [Flush Policy](../type/001_flush_policy.md)); the evaluation procedure (→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)).

### Abstract

The same value [`type/001`](../type/001_flush_policy.md) defines semantically,
examined for what it costs. **Its structural properties are load-bearing
because it is read on the append path**, where `ring_tls` permits no atomic
and no allocation — so a policy that were a trait object, a boxed closure, or a
registry lookup would break another crate's acceptance criterion by existing.

### Structure

```text
FlushPolicy                     size: 16 bytes ( 8 discriminant + 8 payload )
├─ OnFull                       no payload
├─ OnBarrier                    no payload
└─ OnBatch( usize )             8-byte payload
```

| Property | Value | Why it matters |
|----------|-------|----------------|
| Size | 16 bytes (`usize` payload plus discriminant) | Fits in two registers; passed by value with no spill in the common case |
| Alignment | 8 | Natural; no padding beyond the discriminant |
| Indirection | **None** | No pointer to chase on the hot path |
| Heap | **None** | `ring_tls` forbids allocation on the append path |
| Interior mutability | **None** | `Sync` falls out; no synchronisation on read |
| Discriminant | Compiler-chosen | Three variants; the branch is exhaustive and predictable |

**Verify the size claim rather than trusting it:**

```rust
// in tests/flush_test.rs
assert_eq!
(
  core::mem::size_of::< FlushPolicy >(),
  2 * core::mem::size_of::< usize >(),
  "a discriminant plus one usize — a payload was added"
);
```

**A size assertion in a test is not pedantry here.** The structure's whole
contribution is that consulting it is free; a variant added later carrying a
`String`, a `Duration`, or a boxed handler would change the size, move the value
out of registers, and possibly introduce a `Drop` — none of which any existing
test would notice. The assertion is what makes that a red test rather than a
gradual regression.

### Operations

| Operation | Cost | Where |
|-----------|------|-------|
| Copy | Register move | Every consultation |
| Match on variant | One predicted branch | [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md) step 2 |
| Read `OnBatch`'s `n` | Register read | `OnBatch` only |
| Equality | Two comparisons | Tests, and the flush log's cause field |
| Construct | Trivial; validation is at binding, not construction | [`type/001`](../type/001_flush_policy.md)'s N2 |
| Drop | **None — no `Drop` impl** | Required; a `Drop` on the append path is a hidden cost |

**The branch is predicted because the policy is fixed for a buffer's
lifetime** ([`type/001`](../type/001_flush_policy.md)'s N3). The first
consultation mispredicts; every subsequent one on that thread does not. This is
why N3 is a correctness rule with a performance consequence rather than merely
a tidiness preference.

### Shapes refused

| Shape | Why refused |
|-------|-------------|
| `Box< dyn FlushStrategy >` | Heap allocation, a vtable indirection per append, and an open variant set that defeats exhaustive testing (→ [`pattern/001`](../pattern/001_policy_as_a_value.md)'s applicability) |
| `fn( &Buffer ) -> bool` | No allocation, but an indirect call per append and a value that cannot be compared, printed, or asserted on |
| A registry index | Turns a register read into a lookup, and reintroduces a shared structure the thread-local design exists to avoid |
| `Option< NonZeroUsize >` collapsing the three cases | Smaller, and unreadable — `None` would have to mean two different things |
| Storing the counter inside `OnBatch` | Makes the policy stateful and therefore no longer `Copy`. **And there is no counter to store** — the count is the buffer's own occupancy (→ [`lifecycle/004`](../lifecycle/004_policy_arming_and_firing.md)) |

**The last row is the one worth dwelling on, and its conclusion moved one step
further than it expected.** Putting the count inside `OnBatch( n, count )` is
the tidy-looking design, and it changes the value from a configuration into a
mutable object — no longer `Copy`, no longer comparable as "the policy in
effect," and no longer safe to hand out.

This row's remedy was to keep the counter *beside* the policy rather than inside
it. That preserves every property in the table above and it is not what was
built: **there is no counter in either place.** Records staged since the last
flush is exactly the staging buffer's occupancy, so `OnBatch( n )` reads
`buffer.len()` at consultation and stores nothing at all
(→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)'s
deleted Step 3).

**Beside-not-inside was the right instinct and did not go far enough.** The
question "where should this state live" has a third answer the table does not
contain — *nowhere, because it is already somewhere else* — and it is only
visible once you ask whether the state is genuinely new rather than only where
to put it.

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_flush_log.md](002_the_flush_log.md) | The other structure this crate defines, with the opposite cost profile |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | The consultation these properties exist to make free |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) | The requirement the size assertion partially discharges |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) | Why an enum rather than the trait object refused above |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_policy_arming_and_firing.md](../lifecycle/004_policy_arming_and_firing.md) | Where the counter lives, given it is not inside the value |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_flush_policy.md](../type/001_flush_policy.md) | The same value, defined semantically |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/readme.md`](../../../ring_tls/readme.md) | The zero-atomic, zero-allocation criterion every refused shape would break |
| [`../type/001_flush_policy.md`](../type/001_flush_policy.md) | The three variants this layout represents |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | `the_policy_is_a_value` — the `size_of` assertion, plus `Copy` and `!needs_drop`. Written as an equality against `2 × size_of::< usize >()` rather than a literal, so it guards the shape rather than re-pinning a measurement. The literal was checked separately and is 16 (`tests/manual/readme.md`'s F2) |

### FL9 — Eight Instances Cross-Reference a Definition the Corpus Does Not Have

The section heading and the directory it links into do not agree:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- instances carrying a "### State Machines" section --'
command grep -rl '^### State Machines' ring_flush/docs --include='[0-9][0-9][0-9]_*.md' \
  | sed 's|ring_flush/docs/|    |' | sort
echo '  -- what those sections actually link to ( prose only; the findings quote it ) --'
for f in ring_flush/docs/*/[0-9][0-9][0-9]_*.md
do
  awk '/^### FL/{ exit } { print }' "$f"
done | command grep -ohE '\.\./(state_machine|lifecycle)/[0-9]{3}' | sort | uniq -c | sed 's/^/    /'
echo '  -- and which of the two directories exists, family-wide --'
printf '    ring_* crates with docs/state_machine/: %s\n' "$( ls -d ring_*/docs/state_machine 2>/dev/null | wc -l )"
printf '    ring_* crates WITHOUT docs/lifecycle/:  %s\n' "$( for d in ring_*/ ; do [ -d "$d/docs/lifecycle" ] || echo x ; done | wc -l )"
```

Live output:

```
  -- instances carrying a "### State Machines" section --
    algorithm/001_evaluating_a_policy_at_an_append.md
    algorithm/002_sequencing_seal_drain_reset.md
    data_structure/001_the_policy_enum.md
    integration/001_two_dependencies_and_the_barrier_it_cannot_see.md
    lifecycle/001_the_consolidation_cycle.md
    lifecycle/003_buffer_state_through_a_flush.md
    lifecycle/004_policy_arming_and_firing.md
    type/001_flush_policy.md
  -- what those sections actually link to ( prose only; the findings quote it ) --
          7 ../lifecycle/001
         17 ../lifecycle/002
          6 ../lifecycle/003
          8 ../lifecycle/004
  -- and which of the two directories exists, family-wide --
    ring_* crates with docs/state_machine/: 0
    ring_* crates WITHOUT docs/lifecycle/:  0
```

Eight instances in this crate head a cross-reference section **State Machines**.
Zero crates in the family have a `state_machine/` directory and zero lack a
`lifecycle/` one, and every link under those eight headings resolves into
`lifecycle/`. The heading names a definition that exists nowhere; the links
point at one that exists everywhere.

**The links are right and the headings are stale**, which is the combination that
survives longest. A broken link is caught by the citation checker on the next
run. A correct link under a heading naming a directory that does not exist is
invisible to every mechanical check the corpus has: the path resolves, the file
is there, the anchor renders. Only a reader who knows the corpus's definition
list would notice, and a reader who knows the list is the reader least likely to
be confused by the heading.

**Three of the eight are `lifecycle/` instances cross-referencing "State
Machines"** — 001, 003 and 004, pointing at each other. A directory referring to
its own siblings under the name it used to have is the clearest evidence
available that the rename was applied to the filesystem and not to the prose: the
move was mechanical, the headings were hand-written, and nothing connected the
two.

The general shape: **a rename lands on paths and skips headings, because paths
are checkable and headings are not.** The corpus gains a definition name that
appears eight times and exists zero times, and the only cost is to the reader.

### FL10 — The Snippet Under "Verify Rather Than Trust" Is Not the Assertion That Shipped

Two spellings of one assertion, and the instance shows the weaker one:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this instance displays as the verification --'
awk '/^### FL/{ exit } /size_of/ { printf "%d: %s\n", NR, $0 }' \
  ring_flush/docs/data_structure/001_the_policy_enum.md | sed -E 's/^(.{0,110}).*/\1/'
echo '  -- what tests/flush_test.rs actually asserts --'
command grep -m1 -A10 -F '  let taken = policy;' ring_flush/tests/flush_test.rs | command grep . | sed 's/^/    /'
echo '  -- and the width the claim is anchored to on this host --'
printf '    target: %s\n' "$( rustc -vV | command grep '^host' | sed 's/host: //' )"
```

Live output:

```
  -- what this instance displays as the verification --
42:   core::mem::size_of::< FlushPolicy >(),
43:   2 * core::mem::size_of::< usize >(),
149: | `tests/flush_test.rs` | `the_policy_is_a_value` — the `size_of` assertion, plus `Copy` and `!needs_drop
  -- what tests/flush_test.rs actually asserts --
      let taken = policy;
      assert_eq!( policy, taken, "the original was moved rather than copied" );
      assert_eq!
      (
        core::mem::size_of::< FlushPolicy >(),
        2 * core::mem::size_of::< usize >(),
        "a discriminant plus one usize — a payload was added"
      );
      // C4 of `docs/non_functional_requirement/002` — nothing runs when a policy
  -- and the width the claim is anchored to on this host --
    target: aarch64-unknown-linux-gnu
```

The instance prints `assert_eq!( size_of::< FlushPolicy >(), 16 );`. The shipped
test asserts `2 * size_of::< usize >()`. On this host the two agree, which is
why nothing has ever noticed.

**They stop agreeing on any 32-bit target**, where `FlushPolicy` is eight bytes
and the literal is wrong — and the instance's own Tests row already knows this.
It describes the test as "written as an equality against `2 × size_of::<usize>()`"
rather than a literal, which is the portability point stated correctly, in a
table, a hundred and four lines below a code block stating it incorrectly.

**The section heading is what makes this worth recording.** It reads "Verify the
size claim rather than trusting it," and the block beneath it is the one artifact
in the instance a reader would copy. A documentation snippet that is *nearly* the
shipped code is worse than one that is obviously schematic: `// in
tests/flush_test.rs` reads as a location, so the block reads as a quotation, and
a reader has no signal that the file says something different.

The reusable shape: **an illustrative snippet inherits the authority of the code
it resembles and none of its verification.** Nothing compiles this block, nothing
diffs it against line 945, and the more accurate it looks the less likely anyone
is to check.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m2 'a discriminant plus one usize' ring_flush/docs/data_structure/001_the_policy_enum.md
```

Live output:

```
  "a discriminant plus one usize — a payload was added"
        "a discriminant plus one usize — a payload was added"
```

**Disposition:** applied — the "Verify Rather Than Trust" snippet now shows the
same `2 * size_of::< usize >()` portable form the shipped test uses, with the
same explanatory string literal, instead of the narrower `16` literal that only
this host's target width happens to agree with. Now prints: `a discriminant plus one usize`
