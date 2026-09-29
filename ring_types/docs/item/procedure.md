# Item Catalog Procedure

- **Actor**: whoever changes `ring_types/src/`.
- **Trigger**: any item added to, removed from, renamed in, or moved between the crate's four modules; also any change to a signature, a visibility, or an attribute that this catalog quotes.
- **Emits**: nothing to a journal — this crate has none. The observable output is the diff to `docs/item/` in the same change as the `src/` diff.

The catalog is exhaustive by construction (`item_des.rulebook.md § Item Eligibility`),
which means it is wrong the moment `src/` moves without it. These two procedures
are what keep that from happening quietly.

### Procedure — Add an Item

1. **Classify the kind** against `item_des.rulebook.md § Item Kind Taxonomy`. Write down the number now — `## Kind` must cite it, and picking it later invites a free-text label, which is forbidden.
2. **Pick the subdirectory.** It is the kind's snake_case name: `struct/`, `enum/`, `implementation/`, `associated_function/`, `associated_constant/`, `module/`, `use_declaration/`. A kind with no subdirectory yet needs one, plus its own `readme.md` with an Overview Table, plus a row in [`readme.md`](readme.md)'s kind table — and it should also come out of that file's **What the crate does not declare** list, which currently names eleven absent kinds and would otherwise name one that is present.
3. **Take the next free `NNN`** in that subdirectory. Numbers are per-subdirectory, not crate-wide, and are never reused after a deletion — a gap is cheaper than a stale inbound link resolving to the wrong item.
4. **Write the five required sections in order**: `## Representation`, `## Kind`, `## Definition`, `## File Usage`, `## Crate Usage`. If the kind is Function (#4) or Associated Function/Method (A#1), append `## Caller Tree` then `## Callee Tree` — in that order, and on no other kind.
5. **Generate the two usage tables**; do not write them from memory. The recipes are in [`readme.md`](readme.md) § Where the counts come from. Separate doc-comment references from production references — a raw count conflates them and this crate's ratio is roughly fifty to one.
6. **Add the row** to the subdirectory's `readme.md` Overview Table.
7. **Verify** with `python3 <scratchpad>/check_docs.py ring_types/docs`, ignoring the H2 findings it reports against `item/` — `##` headings are correct here and are the one place this Local Extension departs from `doc_des.rulebook.md`'s H1/H3/H4 rule.

### Procedure — Update an Item

1. **Re-run both recipes** before editing prose. Most updates are a changed count, not a changed argument, and reading the new numbers first stops a stale sentence being preserved around a fresh table.
2. **Re-check `## Definition` against the current source.** It quotes a signature; a signature that has drifted is the failure mode this catalog exists to catch.
3. **Follow the inbound links.** The other eleven doc definitions cite these files by path — `pattern/002` cites `associated_function/001`, `type/001` cites `struct/001`, and so on. A rename here breaks them silently, because nothing in this repository validates a relative Markdown link.
4. **If the item was deleted**, delete the file, strike its Overview Table row, and grep the whole `docs/` tree for its filename before considering the change complete:

```bash
grep -rn 'NNN_name\.md' ring_types/docs/
```

**Step 4's grep is the one step that is not optional.** Twenty-two instances
across eleven definitions link into this subtree, and a dangling link in a
generated catalog is worse than no catalog — it reads as a claim that something
exists.

### What this procedure deliberately does not do

**It does not require a `docs/item/` change for a body-only edit.** If a function's
implementation changes but its signature, visibility, attributes, callers and
callees do not, nothing in the five required sections is affected, and forcing a
touch here would train the actor to make empty ones. The Caller and Callee trees
are the exception — they are derived from bodies, so a body edit that adds or
removes a call *is* in scope for step 2 of the update procedure.

**It does not gate on anything mechanical.** No script asserts that the number of
files under `item/` equals the number of items in `src/`, so the exhaustiveness
this definition claims rests on this procedure being followed rather than on a
check that would catch it not being. That is a real gap, and it is the same shape
as the one [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)
records for the empty dependency table: a stated property with no mechanism
watching it. The count is small enough (40) that a gate comparing
`ls item/*/[0-9]*.md | wc -l` against a grep over `src/` would be a few lines —
whether it is worth a seventh gate script is not this crate's call
(→ [`../decisions/readme.md`](../decisions/readme.md), P5).
