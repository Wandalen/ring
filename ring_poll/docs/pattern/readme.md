# Pattern Doc Definition

### Scope

- **Purpose**: Techniques extracted from this crate in a form reusable elsewhere, each with its failure mode.
- **Responsibility**: Two — asserting a reachability rule against the dependency graph, and splitting an operation from the bookkeeping over it.
- **In Scope**: Each pattern's parts, when it applies, and the specific way it degrades in this crate's own instance of it.
- **Out of Scope**: The proof-token technique (→ [`ring_shutdown/docs/pattern/001`](../../../ring_shutdown/docs/pattern/001_proof_token_orders_two_operations.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Enforcement by Dependency Graph](001_enforcement_by_dependency_graph.md) | How to hold a rule the language cannot express, and why part three is the one people skip | 🔄 |
| 002 | [Accounting Wrapper Over Free Functions](002_accounting_wrapper_over_free_functions.md) | How to make bookkeeping optional without duplicating the operation, and what the seam costs | 🔄 |

**One pattern enforces a rule; the other shapes a surface.** `001` is about a
constraint that spans crates and has no expression in the language, held by an
assertion against the dependency graph. `002` is about a single crate's own
layering: four stateless operations, and a small `Copy` type that calls them and
counts.

They are separate because they generalize to different problems and go stale on
different edits — `001` on any family manifest changing, `002` on a signature or
a `Tick` method changing — and because a reader looking for one has no reason to
read the other.

The four findings divide the same way, and land on the same underlying shape in
both. `001`'s two are about a check that is narrower than the claim it is
credited with enforcing: the pattern's own load-bearing question, asked of its own
instance, answers *nothing* for the reachability half (PL37), and the placement
that makes the check cheap is exactly what makes it a proxy (PL38). `002`'s two
are about a contract that is prose: nothing tests that the wrapper delegates as
its doc says, and one method already does not (PL39), while the wrapper's counter
can be bypassed through the type's own public accessor with no warning and no
test covering the seam (PL40).

Both patterns are worth keeping and both are correctly applied in the part that
matters. What the findings record is that in each case the document claims a
slightly wider guarantee than the mechanism delivers.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/pattern
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'roster names / really reach:%s\n' "$( cd ../../.. && printf ' %s / %s' "$( command grep '^pub const PARKING_CRATES' ring_poll/src/lib.rs | command grep -oE '"ring_[a-z_]+"' | wc -l )" "$( for c in $( ls -d ring_*/ | sed 's|/||' ); do cargo tree -p "$c" -e normal 2>/dev/null | command grep -q 'ring_wait' && echo x; done | wc -l )" )"
printf 'free ops / Tick wrappers:   %s\n' "$( printf '%s / %s' "$( command grep -cE '^pub fn [a-z_]+' ../../src/lib.rs || true )" "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' ../../src/lib.rs | command grep -cE '^  pub fn ' || true )" )"
printf 'wrappers passing the budget:%s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' ../../src/lib.rs | command grep -c 'self.budget )' || true )"
printf 'tests asserting layer parity:%s\n' "$( awk '/^fn /{n=$0;t=0;g=0} /Tick::|tick\./{t=1} /push_within\(|push_batch_within\(|recv_within\(|drain_up_to\(/{g=1} /^\}$/{if(n!=""){if(t&&g)b++;n=""}} END{printf "%d", b+0}' ../../tests/poll_test.rs )"
printf 'cargo_metadata in family:   %s\n' "$( cd ../../.. && command grep -l 'cargo_metadata' ring_*/Cargo.toml 2>/dev/null | wc -l )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
roster names / really reach: 3 / 5
free ops / Tick wrappers:   4 / 4
wrappers passing the budget:3
tests asserting layer parity:1
cargo_metadata in family:   0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL37 | the pattern's own load-bearing question, asked of its own instance, answers "nothing" | **misleading doc** | The Consequences section defines the check that decides whether the pattern was applied — *"if the roster were wrong, what would fail?"* — and for the naming claim the answer is good, but for the reachability claim its doc comment actually makes the answer is nothing: five crates reach `ring_wait` through `[dependencies]` and a sixth through `[dev-dependencies]` against a roster of three, and the scan and the roster are wrong in the same direction; the document cited P2 as establishing the family had no such path when P2 measures `cargo tree -p ring_poll`, which is `0` and is this crate's own guarantee, not the family's. |
| PL38 | the placement that makes the check affordable is what makes it a proxy | n/a — observation | The *Where else this fits* table marks two of four family rules as already enforced by `gate/g6_unsafe.sh` and `gate/g5_export_surface.sh` and reads the difference as scope — *"narrower than a gate deserves and wider than one crate's suite can see"* — but a gate script can run `cargo tree` and get reachability directly while a `#[ test ]` inside the crate can only read manifest text, since no crate in the family depends on `cargo_metadata`; nineteen gate scripts already run over this tree, so the proxy is what the placement leaves available rather than a shortcut anyone chose. |
| PL39 | the delegation contract is a sentence, with one exception already in force | n/a — unenforced | `Tick`'s doc says *"Every method delegates to the free function of the same shape and adds only the accounting"*, which is what lets the retry rules be got right once — yet no test calls a wrapper and its free function against the same ring and compares, so a divergent budget or an extra attempt would compile and pass all twenty-three tests; and *every* is already too strong, since three methods pass `self.budget` and `drain` passes `max` alone for a reason given only at the method rather than carried in the type-level claim. |
| PL40 | the accumulator is bypassable, and the type publishes the bypass | **latent hazard** | `Tick::budget()` is a public accessor and the free functions are public, so `push_within( &mut producer, record, tick.budget() )` spends the tick's own budget, publishes the record, and leaves `moved` at zero — the four incrementing sites are all inside the four methods, `moved` is private and has no setter, so the count cannot be repaired — and a scheduler backing off on `Progress::None`, the pattern the enum is shaped for, then backs off a subsystem that did work; the suite has three `Tick`-only tests and fourteen free-function-only tests and none that uses both, so the seam is untested in exactly the arrangement that hides it. |
