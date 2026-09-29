# Decisions Doc Definition

### Scope

- **Purpose**: Record the questions about this crate that are open — each with its options, the measurement that would settle it, what it would cost to act, and who owns it.
- **Responsibility**: The question, the alternatives with their real costs, the threshold or evidence that decides, and the honest state of any deferral.
- **In Scope**: Both open questions about `PARKING_CRATES` — whether it should be generated, and which set it should name.
- **Out of Scope**: Settled choices, which are documented where they take effect (→ [`../api/`](../api/readme.md), [`../invariant/`](../invariant/readme.md)); the measurements that raised the second question (→ [`../api/002`](../api/002_the_roster_as_a_public_constant.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Should `PARKING_CRATES` Be Generated](001_should_the_roster_be_generated.md) | A tripwire that stops being a tripwire once it is always correct | ❓ |
| 002 | [Reach Or Declaration](002_reach_or_declaration.md) | The constant documents one set and contains another; one of them has to move | ❓ |

**Both about one constant, and not the same question.** `001` asks how the roster
should be *maintained* — by hand or by tooling — and takes its membership rule
for granted. `002` asks what that membership rule should be, and takes the
maintenance method for granted. Answering either changes what the other costs,
which is PL15's subject and the reason they are filed separately rather than as
one question with four options.

They also differ in what they are waiting for. `001` is blocked on evidence that
does not exist — how often the family's dependency graph moves. `002` is blocked
on nothing; every number it needs is in its own Regenerate block.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/decisions
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each declares a status:     %s\n' "$( command grep -lc '^\*\*Status:\*\* open' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each names its options:     %s\n' "$( command grep -licE 'option|should .* be|or written' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'owned:                      %s\n' "$( command grep -lE '^\*\*Status:\*\* open\. Owned' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'unowned:                    %s\n' "$( command grep -lE '^\*\*Status:\*\* open\. Unowned' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'settled decisions here:     %s\n' "$( command grep -lE '^\*\*Status:\*\* (accepted|closed)' [0-9][0-9][0-9]_*.md 2>/dev/null | wc -l )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
each declares a status:     2
each names its options:     2
owned:                      1
unowned:                    1
settled decisions here:     0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL13 | a threshold priced against the narrower of two counts | **misleading doc** | `001` proposes generating the roster at six entries and measures the current size with a manifest scan that returns three, presenting the question as comfortably deferred — the reachability count the roster's own doc comment claims to use returns five, one below that threshold, so which reading is taken decides whether this decision is half-way to acting or one crate away from it, and the document never takes the second measurement. |
| PL14 | a deferral whose unblocking event silently did not happen | n/a — drift | `001` defers to `ring_bench` on the grounds that it will need build-time metadata anyway and the work folds in cheaply; `ring_bench` is now 1098 source lines, the family's second largest, and has no build script, and neither does any other `ring_*` crate — the family has zero `build.rs` files and zero call sites naming `cargo metadata` — so the question is waiting on work nobody has scheduled while reading as though it were waiting on work already planned. |
| PL15 | two decisions about one constant, each priced without the other | n/a — inconsistency | `001`'s generate-it threshold is six entries and `002`'s option B would take the array from three to five in a single edit, while `002`'s `cargo tree` closure is most of the machinery generation would need — so each decision materially changes the other's cost and neither document names the other's number, leaving both settleable in isolation on figures the other invalidates. |
| PL16 | no vocabulary separates a blocked question from an undecided one | n/a — doc gap | `001` is blocked on evidence that does not exist and `002` is blocked only on somebody choosing, with every number it needs already in its own recipe, yet both render as `❓` and both say *"Status: open"* — the schema has no marker for *decidable, undecided*, which is the state that quietly never resolves because no scheduled event will force it. |
