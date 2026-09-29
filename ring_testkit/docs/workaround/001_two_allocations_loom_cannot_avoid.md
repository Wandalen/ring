# Workaround: Two Allocations Loom Cannot Avoid

### Scope

- **Purpose**: Record the constraint that forces `leak` and `leak_ends` to exist — loom has no scoped threads — what the crate absorbs on a consumer's behalf, what it costs, and the condition under which both functions can be deleted.
- **Responsibility**: The constraint, its owner, the two allocations, the cost, and the deletion condition.
- **In Scope**: `leak`, `leak_ends`, and the `'static` requirement they satisfy.
- **Out of Scope**: What the loom models do with the ends they get (→ [`../integration/002`](../integration/002_the_edge_that_only_exists_under_a_cfg.md)); the suite split the cfg forces (→ [`002`](002_a_suite_split_in_two_by_a_global_cfg.md)).

### The constraint, and whose it is

`loom::thread::spawn` requires a `'static` closure, and `std::thread::scope` has
no loom equivalent. A `ring_core::Ring` is driven through a borrowed `Ends`, so
its `Producer` and `Consumer` are bounded by a local that no spawned closure may
capture.

**The constraint is loom's, not this crate's, and not `ring_core`'s.** Neither
would change if the other were rewritten; a scoped-thread API in loom removes it
outright.

### What is absorbed

| # | Absorbed | For whom |
|---|---|---|
| W1 | The `'static` lifetime a spawned closure needs | Anyone writing a loom model over a ring |
| W2 | That **two** allocations are required, not one | The same, before they meet the compiler error |

W2 is the half worth having. The ring must be leaked to outlive its local, and
so must the `Ends` value the ring produces — a `Producer` borrowed from a
`&'static mut Ring` is still bounded by whatever local the `Ends` sat in.
`leak_ends` performs both:

```rust
let ends : &'static mut Ends< 'static, T > = Box::leak( Box::new( leak( ring ).ends() ) );
ends.split()
```

Forgetting the second produces a borrow error whose message points at the local
rather than at the missing leak — recorded as
[`tests/manual/readme.md`](../../tests/manual/readme.md) M3.

### What it costs

One ring and one `Ends` per call, never freed. Under loom that is per model
execution, against a ring the models deliberately build at two slots.

Both `leak`'s own doc comment and this definition's readme justify the cost the
same way — *"loom's own per-execution bookkeeping dwarfs it"* — and neither
states a number for either side of that comparison → TK52.

### Deletion condition

| Condition | Whose change |
|---|---|
| loom gains scoped threads | loom's |
| `ring_core` gains an owned-ends constructor | `ring_core`'s |

Either removes the need entirely. Whether a scoped wrapper should be built
*here* in the meantime is [`../decisions/readme.md`](../decisions/readme.md)
Pending 4 — the short version is that it would trade one honest line for a real
type with real unsafe in it.

### Evidence

| # | Claim | Test |
|---|---|---|
| W-E1 | `leak_ends` produces ends that survive being moved onto spawned threads | `leak_ends_produces_ends_that_can_be_moved_onto_spawned_threads` |
| W-E2 | A leaked ring and a borrowed ring give the same outcome | `a_script_runs_the_same_against_a_leaked_ring` |
| W-E3 | One leak is enough to build and split an `Ends` locally | `a_leaked_ring_gives_ends_that_outlive_their_scope` |
| W-E4 | The models get their ends from `leak_ends` and nowhere else | `tests/exhaustive_test.rs`, its `ends()` helper |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'Box::leak sites in src:          %s\n' "$( command grep -c 'Box::leak' src/lib.rs || true )"
printf 'Box::new inside leak:            %s\n' "$( awk '/^pub fn leak</{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -c 'Box::new' || true )"
printf 'Box::new inside leak_ends:       %s\n' "$( awk '/^pub fn leak_ends</{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -c 'Box::new' || true )"
printf 'the two-leak line, verbatim:     %s\n' "$( command grep -m1 -oE 'let ends : &.static mut Ends< .static, T > = Box::leak\( Box::new\( leak\( ring \).ends\(\) \) \);' src/lib.rs )"
printf 'the warning, and whose doc it is on: %s\n' "$( command grep -h '^///' src/lib.rs | sed 's|^/// \?||' | tr '\n' ' ' | command grep -oE 'Two leaks are needed, not one' )"
printf 'leak doc mentions a second leak: %s\n' "$( awk '/^pub fn leak</{exit} {print}' src/lib.rs | tail -25 | command grep -ciE 'two leaks|second leak' || true )"
printf 'leak declared at line:           %s\n' "$( command grep -n '^pub fn leak<' src/lib.rs | cut -d: -f1 )"
printf 'leak_ends declared at line:      %s\n' "$( command grep -n '^pub fn leak_ends<' src/lib.rs | cut -d: -f1 )"
printf 'the cost claim, in src:          %s\n' "$( command grep -h '^///' src/lib.rs | sed 's|^/// \?||' | tr '\n' ' ' | command grep -oE "loom's own per-execution bookkeeping dwarfs it" )"
printf 'numbers attached to it:          %s\n' "$( command grep -h '^///' src/lib.rs | sed 's|^/// \?||' | tr '\n' ' ' | command grep -coE 'bookkeeping dwarfs it[^.]*[0-9]' || true )"
printf 'the models ring capacity:        %s\n' "$( command grep -m1 -oE 'const CAPACITY : usize = [0-9]+' tests/exhaustive_test.rs )"
printf 'Pending items in the decision log: %s\n' "$( command grep -cE '^### Pending [0-9]' docs/decisions/readme.md )"
printf 'of those naming another crate:   %s\n' "$( awk '/^### Pending /{p=1} p' docs/decisions/readme.md | awk '/^### Pending /{ if ( n && hit ) c++ ; n++ ; hit=0 } /ring_[a-z]/ && !/ring_testkit/ { hit=1 } END{ if ( n && hit ) c++ ; print c+0 }' )"
```

Live output:

```
Box::leak sites in src:          2
Box::new inside leak:            1
Box::new inside leak_ends:       1
the two-leak line, verbatim:     let ends : &'static mut Ends< 'static, T > = Box::leak( Box::new( leak( ring ).ends() ) );
the warning, and whose doc it is on: Two leaks are needed, not one
leak doc mentions a second leak: 0
leak declared at line:           531
leak_ends declared at line:      565
the cost claim, in src:          loom's own per-execution bookkeeping dwarfs it
numbers attached to it:          0
the models ring capacity:        const CAPACITY : usize = 2
Pending items in the decision log: 4
of those naming another crate:   3
```

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | What the leaked ends are handed to |

### Workarounds

| File | Relationship |
|------|--------------|
| [002_a_suite_split_in_two_by_a_global_cfg.md](002_a_suite_split_in_two_by_a_global_cfg.md) | The other thing the same cfg forces |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | Where the two leak helpers sit in the public surface |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `leak`, `leak_ends`, and both doc comments |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | W-E1 through W-E3 |
| `tests/exhaustive_test.rs` | W-E4 |

### TK51 — three of four open questions are resolved in a crate that has not been told

The deletion condition above names two events, and one of them —
`ring_core` gaining an owned-ends constructor — is a change to a different crate.
That is not an isolated case. Of the four Pending items in this crate's decision
log, **three** name another `ring_*` crate as where the answer lives: Pending 2
in `ring_shutdown`, Pending 3 in `ring_core`, Pending 4 in `ring_spsc`. Only
Pending 1, the record type's `u32`, resolves here.

`ring_testkit` is the crate best positioned to notice all three. It is the only
one that holds a `Shutdown`, a `Ring` and a `TlsBuffer` in one function, so it
meets each seam from the outside, which is exactly where a design cost becomes
visible. Filing what it notices in its own log is the right first move.

There is no second move. [`../pitfall/002`](../pitfall/002_reopening_closes_first.md)
TK44 measures the specific case — `ring_shutdown`'s own decision log has never
heard the question, and one file in that whole crate names `ring_testkit`, a test
comment. This is the general shape: a consumer's decision log is a private
notebook, and nothing in the corpus carries an entry from the crate that observed
a cost to the crate that could remove it. Three questions are parked where their
answer cannot be given.

### TK52 — the cost is dismissed by comparison, and neither side of the comparison is measured

`leak`'s doc comment and this definition's readme both retire the leak's cost the
same way: the ring is *"deliberately tiny"* and *"loom's own per-execution
bookkeeping dwarfs it"*. The models do build at `CAPACITY = 2`, so the first half
is true and checkable.

The second half is a comparison with no number on either side. Nothing states how
much loom allocates per execution, how many executions the three models run, or
what the total leaked bytes come to for one `--cfg loom` invocation. The claim is
almost certainly right — loom's per-execution state is large and a two-slot ring
is a handful of bytes — and it is still an assertion the reader is asked to take
on trust in the one place the document is accounting for a resource it never
frees.

It matters more than it looks because the leak is unbounded in the dimension
nobody has counted. The per-execution cost is fixed and small; the number of
executions is loom's to choose and is not pinned anywhere in this crate. A model
whose interleaving count grows leaks linearly in that count, and the sentence
that would warn a reader is the one currently doing the dismissing.
