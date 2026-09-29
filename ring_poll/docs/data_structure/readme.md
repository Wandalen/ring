# Data Structure Doc Definition

### Scope

- **Purpose**: Record the shape of every value `ring_poll` declares — how each is laid out, which fields a caller can reach, which traits each carries, and what those choices permit or prevent.
- **Responsibility**: Declarations rather than meanings: field visibility, construction routes, derived and hand-written trait impls, and the arithmetic the fields are subject to.
- **In Scope**: `Budget`, `Progress`, `Tick`, and the `PARKING_CRATES` array's type.
- **Out of Scope**: What the values mean to a caller (→ [`../type/`](../type/readme.md)); the operations that read and write them (→ [`../api/`](../api/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Three Values, Three Shapes](001_three_values_three_shapes.md) | Newtype, enum and struct side by side, and the trait coverage they do not share | 🔄 |
| 002 | [A Copy Accumulator](002_a_copy_accumulator.md) | What `Copy` means for a struct holding a running total, and what guards the total does not have | 🔄 |

**Why these two and not one.** `001` compares the three declarations against each
other; `002` looks inside one of them. The split is the difference between a
question about the set — do these types agree? — and a question about a single
field's behaviour, which no comparison would surface. `001` finds two
inconsistencies across types; `002` finds a hazard that exists entirely within
`Tick` and would read as a footnote in a comparison table.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/data_structure
printf 'instances:                 %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:   %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:   %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'types this definition covers: %s\n' "$( command grep -oE '^pub (struct|enum) [A-Za-z]+' ../../src/lib.rs | sed 's/^pub [a-z]* //' | tr '\n' ' ' )"
printf 'total src lines declaring them: %s\n' "$( command grep -cE '^pub (struct|enum) |^#\[ derive|^impl Default' ../../src/lib.rs || true )"
printf 'evidence rows, both files: %s\n' "$( command grep -hcE '^\| (C|D)[0-9] \| ' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
```

Live output:

```
instances:                 2
finding headings inside:   4
rows in the table below:   4
each instance has a recipe: 2
types this definition covers: Budget Progress Tick 
total src lines declaring them: 8
evidence rows, both files: 8
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL9 | the type with the most obvious default has none | n/a — inconsistency | `Budget` and `Tick` each carry a hand-written `Default` chosen from among alternatives, while `Progress` — whose `None` variant is `then`'s identity element and therefore the one default the type forces rather than invites — has none, so a caller aggregating per-subsystem progress into a `#[ derive( Default ) ]` struct cannot, and that caller is the one this crate is written for. |
| PL10 | one default policy expressed twice, pinned by value | n/a — duplication | `Tick::default` returns `Self::new( Budget::once() )` rather than `Self::new( Budget::default() )`, and both covering tests assert against `Budget::once()` too — `a_default_tick_is_a_single_attempt` even carries the doc comment *"matching the default budget"* above an assertion that names the literal instead, so changing `Budget::default` would leave the tick's default silently unmatched and every test still green. |
| PL11 | an accumulator that duplicates silently | **latent hazard** | `Tick` derives `Copy` while holding a `moved` counter that four `&mut self` methods increment, so `fn run_systems( tick : Tick )` compiles, moves records correctly, and reports `Progress::None` because the increments landed in a copy — a scheduler reading that value backs off exactly when it should not, and nothing in the crate mentions the shape: `Tick`'s doc covers the accounting it adds, and zero tests name `Copy`. |
| PL12 | the guarded field and the unguarded one share a struct | n/a — unenforced | `Budget` spends an entire newtype declaration making one `usize` unable to hold a nonsense value, and `moved` — the other `usize`, in the same struct — is incremented by bare `+=` at four sites with no checked, saturating or wrapping form in the file and `overflow-checks` set nowhere in the workspace, so the reasoning that makes it safe today is about the object's lifetime rather than its type and is recorded nowhere. |
