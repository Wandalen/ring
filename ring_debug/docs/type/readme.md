# Type Doc Definition

### Scope

- **Purpose**: Define the crate's three public types and the trait surface they carry, separating the values a check reports from the value a check runs on.
- **Responsibility**: Each type's shape, what it validates on construction, what each derive is for, and which of them has a use.
- **In Scope**: `Violation` and its four variants; `Cursor`; `Watch` and its baseline; the derive sets; the `Error` impl.
- **Out of Scope**: Which entry point produces which variant (→ [`api/`](../api/readme.md)); what a variant means about the ring (→ [`invariant/`](../invariant/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Violation](001_violation.md) | The reported value and its discriminant — why every variant carries numbers rather than a message | 🔄 |
| 002 | [Watch, and the Traits Nothing Asks For](002_watch_and_the_traits_nothing_asks_for.md) | The stateful type, and what the shared derive line costs on it | 🔄 |

**The split is inert value against instrument, and the trait surface is why it
matters.** `Violation` and `Cursor` are reports: constructible from impossible
numbers by design, `Copy` because a diagnostic that borrowed would be awkward,
compared by value in every test. `Watch` is none of those things — it refuses to be
built over a broken pair, it advances through `&mut self`, and it carries the same
derive line anyway.

Keeping them together would let the second read as a footnote to the first. It is
closer to the opposite: the arguments `001` makes for its two types are sound, and
each one inverts on the third.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/type
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB41 | the value-carrying argument | **misleading doc** | The consequence offered as evidence for the design — that every test asserts on a whole `Violation` — holds for five of the suite's twenty-six, and the precise claim it should have made is stronger than the loose one. |
| DB42 | the `Display` fixture | n/a — coverage | The guard against a rendering that throws the numbers away cannot detect a rendering that keeps every number and exchanges two of them; all four cases survive an operand swap, including the one whose entire diagnostic value is direction. |
| DB43 | `Copy` on `Watch` | **latent hazard** | The only type holding a baseline derived `Copy`, so a by-value use duplicated it silently instead of moving it and the caller stopped checking the one invariant a watch exists to add; the derive is gone and a by-value use is a move again. |
| DB44 | the shared derive line | n/a — observation | One trait surface is declared across an inert label, an inert report and a stateful instrument; measured per type it is used on the reports, unused on the instrument, and on the instrument the one derive with a consequence is the one with no call site. |
