# item

A trait and three structs, six constructors, four trait methods and two beside
them — and the crate's documentation of them splits cleanly in half. Every reason
behind a *constructor* is written down, including one subtle enough that a reader
would otherwise find it six months later through a broken `--cfg loom` build. The
whole file carries exactly one contract clause — a `# Errors` on `compare_exchange`
— and it describes a return value rather than an obligation. What a cell may not be
moved to, what a count means while traffic is running, and what putting the
instrument off the trait forecloses are all unwritten.

The crate documents how to build things thoroughly and what they promise almost not
at all, and the two findings here are the two ends of that. One records a decision
so well explained that its only defect is a missing number — nothing in the family
uses the `const` the split exists for. The other records two contracts, eight words
and a sentence, that both describe an instant neither call delivers.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_six_constructors_for_two_types.md) | Six Constructors for Two Types | The `loom` split, the hand-written `Default`, and the `const` nothing uses |
| [002](002_counts_the_method_that_is_not_a_snapshot.md) | `counts`, the Method That Is Not a Snapshot | The two off-trait methods, their contracts, and the code they can never reach |

## Where the Trait Boundary Falls

`SeqCell` carries the four operations a production cursor performs and nothing
else. `counts` and `reset_counts` sit beside it on `CountingSeq`, which is right —
an `AtomicSeq` has nothing to report — and which decides, silently, that only
generic code can be measured.

The family has three generic `SeqCell` parameters and four concrete `PaddedCursor`
fields. The three generic ones live in the two crates that assert operation counts.
The four concrete ones include `ring_claim::Claimer`, whose claim is an unbounded
CAS retry loop — the one path in the family whose operation count cannot be derived
by reading it, and the one that admits no substitution.

## Constructors Are the Best-Documented Surface Here

Six constructors, two duplications, and a reason on the line above each: `loom`'s
atomics have no `const` constructor, and a derived `Default` would pin the crate to
whichever `AtomicU64` happens to be in scope. Both are correct and both are the
kind of thing normally rediscovered by a build failure.

They are also, with the trait-versus-struct paragraph, the only explained decisions
in the crate — against nine undocumented ordering literals, an undocumented packed
cache line, and two absent declaration-level attributes.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every public item in the crate --'
command grep -nE '^  pub (const )?fn |^pub (trait|struct|fn) ' ring_atomic/src/lib.rs
echo '  -- constructors among them, then every contract clause in the file --'
command grep -cE 'pub (const )?fn new|fn default' ring_atomic/src/lib.rs
command grep -nE '# Panics|# Safety|# Errors' ring_atomic/src/lib.rs
echo '  -- the two substitution shapes --'
command grep -rcE '\b[A-Z] *: *SeqCell' --include=lib.rs */src/ | command grep -v ':0' | sed -E 's|/src/lib.rs||'
command grep -rcE '^ +[a-z_]+ *: *PaddedCursor,' --include=lib.rs */src/ | command grep -v ':0' | sed -E 's|/src/lib.rs||'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT25 | `ring_atomic` | n/a — observation | Both duplications among the six constructors carry a written reason, including the subtle one about not depending on `loom`'s own trait impls — making constructors the best-documented surface in a crate that documents no contract at all |
| AT26 | `ring_atomic` | n/a — observation | Zero `const` or `static` items in all 33 crates hold a cell, so the capability the `cfg( loom )` split exists for is unused — while those four declarations are exactly where the crate's compile-time coupling to the root manifest lives |
| AT27 | `ring_atomic` | n/a — doc gap | `counts`/`reset_counts` are off the trait, so only generic code can be measured — the family's three generic parameters sit in the two crates that count, while `ring_claim::Claimer`'s unbounded CAS retry loop, the one path whose count is not derivable by reading, owns its cursor concretely and admits no substitution |
| AT28 | `ring_atomic` | **misleading doc** | Both contracts describe an instant neither call delivers — `counts` is four reads, `reset_counts` four stores — and following `reset_counts`' documented recipe costs one of the counts being measured, an effect the crate's own test asserts deliberately and its doctest is ordered to dodge |
