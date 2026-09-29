# API Doc Definition

### Scope

- **Purpose**: Define this crate's two callable surfaces — the checks a caller invokes, and the sentences those checks render when they fail.
- **Responsibility**: Signatures, guarantees, preconditions, and the coverage each guarantee actually has.
- **In Scope**: `check`, `Watch::new`, `Watch::observe`, `Watch::last`, `check_ends`; both `Display` impls.
- **Out of Scope**: The invariants being checked (→ [`invariant/`](../invariant/readme.md)); the reported value's shape (→ [`type/`](../type/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Check Surface](001_the_check_surface.md) | Five entry points in three groups, split by what the caller must supply | 🔄 |
| 002 | [The Message Is a Second API](002_the_message_is_a_second_api.md) | The rendered form as a surface in its own right, with its own coverage | 🔄 |

**The two are separated because they are pinned by different things and to very
different degrees.** The check surface's guarantees are asserted by equality
against typed values; the rendered surface's are asserted by substring containment
against digits. Both are public, both are what a caller meets, and only one of them
would notice being rewritten.

A crate whose whole subject is that a reading can be confidently wrong ought to
document the sentence it prints as carefully as the value it returns. That is the
reason `002` exists at all rather than being three paragraphs inside `001`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/api
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB25 | `Watch::last` | n/a — inconsistency | The crate defines an enum whose only job is naming which end a cursor belongs to, then returns both ends as an unlabelled pair whose order lives in a doc comment. |
| DB26 | guarantee A4 | n/a — coverage | The precedence of D3 over D1/D2 is pinned by one fixture whose numbers were chosen for a different purpose, so tidying the fixture would silently delete the guarantee's only coverage. |
| DB27 | `ConsumerAheadOfProducer` message | **latent hazard** | The crate's most load-bearing sentence is a claim about a non-dependency's arithmetic held in a string literal, and nothing compared the two until a test built the D1 pair, asked `ring_core` what it reports, and asserted the words against the answer. |
| DB28 | `ProducerLappedConsumer` rendering | **latent hazard** | The one number the crate derives outside a check was computed with saturating arithmetic in a publicly constructible variant, and its only fixture picked a consumer of zero, which made the subtraction indistinguishable from the field beside it; `checked_sub` and a nonzero fixture make both observable. |
