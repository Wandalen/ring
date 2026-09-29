# Type: Five Derives, and the One That Is Free and Absent

### Scope

**Purpose:** Record which of the record's five derives are decisions, which are
forced, and which absent derive is available for nothing.

**Responsibility:** The derive set against the derive sets of all three field
types, the one trait in the intersection that the record drops, the three that
are genuinely unavailable, and the two derives no one chose.

**In Scope:** `ring_config/src/lib.rs:41`;
`ring_types/src/capacity.rs:22`; `ring_types/src/policy.rs:21`,
`:104`; `Cargo.toml:233`; `ring_registry/src/lib.rs:90`.

**Out of Scope:** What the field types are and why is
[`type/002`](002_two_counts_that_are_usize_and_three_fields_that_are_not.md). What
`Copy` costs the setters is
[`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md).

---

## The Record's Derives Against Its Fields'

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
derives() { command grep -B 1 "^pub \(struct\|enum\) $2\b" "$1" | sed -n 's/.*derive( *\(.*[^ ]\) *).*/\1/p' | tr -d ' ' | tr ',' '\n' | sort -u; }
echo '  -- what the record derives, and what each of its field types derives --'
printf '%-16s %s\n' RingConfig "$( derives ring_config/src/lib.rs RingConfig | paste -sd' ' )"
printf '%-16s %s\n' Capacity "$( derives ring_types/src/capacity.rs Capacity | paste -sd' ' )"
printf '%-16s %s\n' WaitKind "$( derives ring_types/src/policy.rs WaitKind | paste -sd' ' )"
printf '%-16s %s\n' OverflowPolicy "$( derives ring_types/src/policy.rs OverflowPolicy | paste -sd' ' )"
echo '  -- traits every field type derives and the record does not --'
comm -12 <( derives ring_types/src/capacity.rs Capacity ) <( derives ring_types/src/policy.rs WaitKind ) \
| comm -12 - <( derives ring_types/src/policy.rs OverflowPolicy ) \
| comm -23 - <( derives ring_config/src/lib.rs RingConfig )
echo '  -- traits only some field type derives, which the record therefore cannot --'
cat <( derives ring_types/src/capacity.rs Capacity ) <( derives ring_types/src/policy.rs WaitKind ) <( derives ring_types/src/policy.rs OverflowPolicy ) | sort -u \
| comm -23 - <( derives ring_config/src/lib.rs RingConfig ) | command grep -v '^Hash$'
echo '  -- Default mentions in the type that blocks it --'
command grep -c 'Default' ring_types/src/capacity.rs || true
echo '  -- the two derives that are not choices --'
command grep 'missing_debug_implementations' Cargo.toml   # unanchored: the members list above it grows
echo '  -- and what the family keys a registry by --'
command grep -m1 -F '  rings : HashMap< String, Split< T > >,' ring_registry/src/lib.rs
```

Live output:

```
  -- what the record derives, and what each of its field types derives --
RingConfig       Clone Copy Debug Eq PartialEq
Capacity         Clone Copy Debug Eq Hash Ord PartialEq PartialOrd
WaitKind         Clone Copy Debug Default Eq Hash PartialEq
OverflowPolicy   Clone Copy Debug Default Eq Hash PartialEq
  -- traits every field type derives and the record does not --
Hash
  -- traits only some field type derives, which the record therefore cannot --
Default
Ord
PartialOrd
  -- Default mentions in the type that blocks it --
0
  -- the two derives that are not choices --
missing_debug_implementations = "warn"
  -- and what the family keys a registry by --
  rings : HashMap< String, Split< T > >,
```

---

### RC45 — Exactly One Trait Is Free and Absent, and It Looks Identical to Three That Are Impossible

A derive over a struct needs every field type to support it, so the set the
record could derive is the intersection of its three field types' sets. That
intersection is `Clone`, `Copy`, `Debug`, `Eq`, `Hash`, `PartialEq`. The record
derives all of them but one.

`Hash` is the only trait every field type derives and the record does not, and it
costs nothing to add. The other absences from the union — `Default`, `Ord`,
`PartialOrd` — are unavailable rather than declined. `Default` is blocked by
`Capacity`, which mentions the trait zero times and cannot sensibly implement it:
a default capacity would be zero and `Capacity::new( 0 )` is
`Err( RingError::CapacityZero )`. `Ord` and `PartialOrd` are on `Capacity` alone,
and neither policy enum has them.

**Finding.** Three absences are consequences and one is a decision, and the
declaration at `:41` distinguishes none of them — a reader sees a derive list and
cannot tell which of the missing traits were considered.

Nothing needs `Hash` today. `ring_registry` keys its `HashMap` by `String`, which
is the right key for a registry of named rings, and no other map anywhere in the
family is keyed by a configuration. So this is worth recording as a property of
the type rather than as a gap: `RingConfig` is one derive away from being usable
as a map key, the one derive is free, and the reason it is absent is that nobody
asked — which is a different fact from the reason `Default` is absent, and reads
the same at the declaration.

---

### RC46 — Two of the Five Derives Are Not Choices, and the Two Genuine Ones Are Both Recorded Elsewhere as Costs

`Debug` is not a decision: `missing_debug_implementations = "warn"` sits in the
workspace lints table at `Cargo.toml:233` and the crate inherits it, so a record
without `Debug` warns. `Clone` is not a decision either — `Copy` requires it as a
supertrait, so it appears in every `Copy` type's derive list by obligation.

That leaves three: `Copy`, `PartialEq` and `Eq`. Both of the properties they
provide are already recorded in this corpus with a cost attached rather than a
benefit. `Copy` is what removes the consuming builder's use-after-move guarantee,
leaving four hand-written `#[ must_use ]` attributes to stand in for a compile
error
([`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md)).
`PartialEq` and `Eq` have no production caller anywhere in the family — the
equality exists so `setters_commute` can state its property
([`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md)).

**Finding.** Read together, the derive line is two obligations and three
decisions, and every one of the three decisions has a consequence written down in
a document other than the one containing the derive. The declaration carries no
comment at all.

That is not an argument for changing anything — `Copy` is right for a
thirty-two-byte record read four times per ring, and an equality a test needs is
an equality worth having. It is an argument that the line most worth annotating
in the file is the one with no annotation, since a maintainer trimming an unused
derive would find `PartialEq`, `Eq` and `Hash` all equally unreferenced by
production code and only one of them actually absent.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_two_counts_that_are_usize_and_three_fields_that_are_not.md) | The field types these derives are the intersection of |
| [`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md) | What `Copy` costs the four setters |
| [`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md) | The equality with no production caller |
| [`data_structure/001`](../data_structure/001_twenty_six_bytes_of_fields_in_thirty_two_of_struct.md) | What `Copy` costs per hand-off |

### Sources

| Fact | Where |
|------|-------|
| The record's five derives | `ring_config/src/lib.rs:41` |
| `Capacity`'s eight, including `Hash` and `Ord` | `ring_types/src/capacity.rs:22` |
| Both policy enums' seven, including `Hash` and `Default` | `ring_types/src/policy.rs:21`, `:104` |
| `Hash` as the sole free absence | Census above, set difference |
| `Capacity` never naming `Default` | Census above, zero occurrences |
| `missing_debug_implementations = "warn"` | `Cargo.toml:233` |
| A registry keyed by name rather than configuration | `ring_registry/src/lib.rs:90` |

### Tests

| Test | Covers |
|------|--------|
| `the_record_is_copy_and_compares_by_value` | Three of the five derives, together |
| `defaults_are_the_documented_ones` | The values a `Default` would have to produce, written by hand instead |
| `setters_commute` | The only consumer of the derived equality |
