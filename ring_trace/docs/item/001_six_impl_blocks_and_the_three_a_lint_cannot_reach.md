# Item: Six Impl Blocks and the Three a Lint Cannot Reach

### Scope

**Purpose:** Record what `#![ deny( missing_docs ) ]` buys this crate at the
declaration level, establish the three impl blocks it cannot reach into, and
locate the one thing left undocumented inside them.

**Responsibility:** The lint attribute, the six impl blocks and their split into
inherent and trait, which trait-impl methods carry documentation, and how the
same lint is set across the family.

**In Scope:** `ring_trace/src/lib.rs:53`, `:84`, `:120`, `:160`, `:175`,
`:179`, `:209`, `:369`; the workspace lints table in the root `Cargo.toml`;
`#![ deny( missing_docs ) ]` across all 33 `ring_*` crates.

**Out of Scope:** The public surface those blocks expose is
[`api/001`](../api/001_three_types_nine_attributes_and_one_private_item.md). The
`TraceOp::ALL` constant and its tripwire are
[`item/002`](002_traceop_all_and_the_tripwire_that_is_not_one.md). How the
undocumented format reads is
[`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md).

---

## What the Lint Reaches

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the lint the crate sets, and where --'
command grep 'deny( missing_docs )' ring_trace/src/lib.rs | sed 's/^/    /'
echo '  -- every impl block, and which are trait impls the lint cannot reach into --'
command grep '^impl' ring_trace/src/lib.rs | sed 's/^/    /'
t=$( command grep -c '^impl .* for ' ring_trace/src/lib.rs || true )
printf '    trait impls: %s   inherent impls: %s\n' "$t" \
  "$(( $( command grep -c '^impl' ring_trace/src/lib.rs || true ) - t ))"
echo '  -- how many of those trait-impl methods carry a doc comment --'
d=$( awk '
  /^impl .* for / { window = 2; hit = 0; next }
  window > 0 { if ( $0 ~ /\/\/\// ) hit = 1; window--; if ( window == 0 && hit ) doc++ }
  END { print doc + 0 }
' ring_trace/src/lib.rs )
printf '    documented: %s of %s\n' "$d" "$t"
echo '  -- the one output format the crate offers, and its doc comment --'
command grep -m1 -F '    write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )' ring_trace/src/lib.rs
printf '    doc lines above it: %s\n' \
  "$( command grep -m1 -A3 -F 'impl fmt::Display for TraceEntry' ring_trace/src/lib.rs | command grep -c '///' || true )"
echo '  -- and what the family does with the same lint --'
h=0
for c in ring_*/; do
  if command grep -q 'deny( missing_docs )' "$c"src/lib.rs 2>/dev/null; then h=$(( h + 1 )); fi
done
printf '    ring_* crates escalating it in source: %s of 33\n' "$h"
printf '    what the workspace lints table says: '
command grep 'missing_docs' Cargo.toml | sed 's/^ *//'
```

Live output:

```
  -- the lint the crate sets, and where --
    #![ deny( missing_docs ) ]
  -- every impl block, and which are trait impls the lint cannot reach into --
    impl TraceOp
    impl fmt::Display for TraceOp
    impl TraceEntry
    impl fmt::Display for TraceEntry
    impl Trace
    impl Default for Trace
    trait impls: 3   inherent impls: 3
  -- how many of those trait-impl methods carry a doc comment --
    documented: 1 of 3
  -- the one output format the crate offers, and its doc comment --
    write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )
    doc lines above it: 0
  -- and what the family does with the same lint --
    ring_* crates escalating it in source: 33 of 33
    what the workspace lints table says: missing_docs = "warn"
```

---

### TR25 — The Lint Covers Everything It Can See, and the Crate's Only Output Format Is Outside It

`#![ deny( missing_docs ) ]` sits directly after the module doc comment, before
the crate's `use` declarations, and every public item in the three
inherent impl blocks carries documentation — the census in
[`api/001`](../api/001_three_types_nine_attributes_and_one_private_item.md) found
no exception. That is what the lint is for and it worked.

`missing_docs` does not apply to items inside a trait impl, because the trait's
own declaration is where the contract lives. Three of this crate's six impl
blocks are trait impls, and one of the three documents its method anyway —
`Default::default` carries "Disabled — the state a ring that was never asked to
trace must be in.", which is a real statement and not a restatement of the trait.
The other two are the `Display` impls, and they carry nothing.

That matters more here than it usually would. `TraceEntry`'s `Display` is
`write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )` — the crate's only
rendering of a log entry, the format anything printing a trace will produce, and
the shape a human reading a dump has to parse. Nowhere in the crate is that
format written down: not at the impl, not at `TraceEntry`, not in the module doc.
A reader learns it by reading the `write!`.

**Finding.** The lint's blind spot is standard and correct — what is specific to
this crate is that the blind spot contains its output format. `Display` on a
diagnostic type is not an incidental trait impl; it is the interface between the
log and the person reading it, and it is exactly the kind of thing a caller needs
specified before they parse or grep it. Two lines on the impl — the format
string, and that `end` is exclusive so an entry with `count` 0 prints as
`claim 7..7` — cost nothing and are unreachable by any lint the crate can set.

---

### TR26 — Thirty-Three Crates Hand-Write the Same One-Line Escalation

The workspace lints table sets `missing_docs = "warn"`, and `ring_trace`'s
manifest opts into it with `[lints] workspace = true`. The crate then escalates
the same lint to `deny` with an inner attribute in source, which overrides the
level the manifest passed on the command line.

Every one of the 33 `ring_*` crates does the same thing. Not most — all of them,
with the identical line in the identical position. So the family is unanimous that
`deny` is the level it wants, and expresses that unanimity 33 times in source
rather than once in the table that already carries the lint.

**Finding.** Recorded because the fix is a single word: `missing_docs = "deny"` in
the workspace lints table, and 33 inner attributes become deletable. The current
arrangement is not wrong — it works, and it has the small merit that the level is
visible in the file it governs — but it costs a line per crate to say something
the table was built to say once, and it means a 34th crate joining the family
inherits `warn` unless someone remembers the ritual. The unanimity is the
argument: a rule 33 out of 33 crates adopt is a workspace default that has not
been promoted.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](../api/001_three_types_nine_attributes_and_one_private_item.md) | The public surface these blocks expose |
| [`item/002`](002_traceop_all_and_the_tripwire_that_is_not_one.md) | The constant inside the first inherent block |
| [`pitfall/001`](../pitfall/001_a_range_that_reads_backwards.md) | How the undocumented format reads |
| [`data_structure/002`](../data_structure/002_three_public_fields_and_the_range_they_imply.md) | The fields the format prints |

### Sources

| Fact | Where |
|------|-------|
| The lint attribute | `ring_trace/src/lib.rs`, directly after the module doc comment |
| Six impl blocks, three of them trait impls | Census above |
| One of three trait-impl methods documented | Census above |
| The undocumented format string | `ring_trace/src/lib.rs:179` |
| 33 of 33 crates escalating the lint | Census above |
| The workspace level | Root `Cargo.toml`, `[workspace.lints.rust]` |

### Tests

| Test | Covers |
|------|--------|
| `entries_compare_by_value_and_print_readably` | The `Display` output the impl produces |
| `an_operation_prints_as_its_name` | The other `Display` impl |
| `an_entry_reports_the_range_it_covers` | The `end()` the format calls |
