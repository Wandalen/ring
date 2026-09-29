# type

Five fields, three types, five derives. The type-level story of the record is
almost entirely about what it delegates: `Capacity` refuses a zero or a
non-power-of-two before the struct sees it, two enums have no illegal spelling,
and the record's own derive list is a subset of what those three types already
provide.

Both instances are about the boundary of that delegation. One trait every field
type derives is absent from the record and costs nothing to add; three others are
absent because a field type blocks them, and the declaration does not distinguish
the cases. Two fields skip delegation entirely and carry a bare `usize` with a
runtime clamp instead — and those two turn out to be the same two fields three
other censuses in this corpus arrive at independently.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_five_derives_and_the_one_that_is_free_and_absent.md) | Five Derives, and the One That Is Free and Absent | The derive set against its fields', `Hash`, and the two derives no one chose |
| [002](002_two_counts_that_are_usize_and_three_fields_that_are_not.md) | Two Counts That Are `usize` and Three Fields That Are Not | Two enforcement mechanisms for one job, and the line three censuses agree on |

## What the Derive Line Is Actually Made Of

Of the five derives, `Debug` is forced by `missing_debug_implementations = "warn"`
in the workspace lints table and `Clone` is forced by `Copy` needing it as a
supertrait. Three are decisions, and both of the properties those three provide
are recorded elsewhere in this corpus as costs rather than benefits: `Copy`
removes the consuming builder's use-after-move guarantee, and `PartialEq`/`Eq`
have no caller outside the test suite.

The intersection of what all three field types derive is `Clone`, `Copy`,
`Debug`, `Eq`, `Hash`, `PartialEq`. The record takes five of the six. `Hash` is
the sole free absence; `Default`, `Ord` and `PartialOrd` are absent because no
single field type offers them across the board — `Capacity` never names `Default`
and could not, since a default capacity would be zero and zero is an error.

## Two Mechanisms, One Job

`capacity`, `wait` and `overflow` carry types that make illegal values
unrepresentable. `producers` and `batch` carry `usize` and get their rules from
two expressions in two setter bodies — the only two setter bodies in the crate
that are not plain assignments.

Those same two fields are the two `ring_bench::Workload` keeps loose copies of
beside the whole record, and two of the three with no production reader anywhere
in the family. The third unread field, `wait`, is domain-typed, which is what
makes the correlation specific rather than general: the fields another crate felt
free to copy out are exactly the ones where copying loses nothing the type system
was carrying.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
derives() { command grep -B 1 "^pub \(struct\|enum\) $2\b" "$1" | sed -n 's/.*derive( *\(.*[^ ]\) *).*/\1/p' | tr -d ' ' | tr ',' '\n' | sort -u; }
echo '  -- traits every field type derives and the record does not --'
comm -12 <( derives ring_types/src/capacity.rs Capacity ) <( derives ring_types/src/policy.rs WaitKind ) \
| comm -12 - <( derives ring_types/src/policy.rs OverflowPolicy ) \
| comm -23 - <( derives ring_config/src/lib.rs RingConfig )
echo '  -- the five fields and their types --'
command grep -m1 -A4 -F '  capacity : Capacity,' ring_config/src/lib.rs
echo '  -- setter bodies that are assignments, against those that are not --'
command grep -n 'self\.[a-z_]* = ' ring_config/src/lib.rs
echo '  -- and Default mentions in the type that blocks it --'
command grep -c 'Default' ring_types/src/capacity.rs || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC45 | `ring_config` | n/a — observation | `Hash` is the only trait all three field types derive that the record does not, and it costs nothing — while `Default`, `Ord` and `PartialOrd` are absent because no field type offers them across the board, `Capacity` naming `Default` zero times and being unable to implement it since a default capacity would be zero and `Capacity::new( 0 )` is `Err( CapacityZero )` — so three absences are consequences and one is a decision, the declaration at `:41` distinguishes none of them, and no map in the family is keyed by a configuration today |
| RC46 | `ring_config` | n/a — doc gap | Two of the five derives are not choices — `Debug` is forced by `missing_debug_implementations = "warn"` at `Cargo.toml:233` and `Clone` by `Copy` requiring it — leaving `Copy`, `PartialEq` and `Eq`, whose properties this corpus already records as costs rather than benefits: `Copy` removes the consuming builder's use-after-move guarantee and the equality has no caller outside the suite, so every consequence of the three real decisions is written down in a document other than the file containing the derive, which carries no comment |
| RC47 | `ring_config` | n/a — doc gap | The record enforces the same kind of rule two ways — `Capacity` refuses zero and non-powers-of-two at construction and the two enums have no illegal spelling, while `producers` and `batch` are bare `usize` corrected by expressions at `:124` and `:143`, the only two setter bodies that are not plain assignments — and no rule anywhere says which mechanism a new field should get, though `Capacity` is the family's own demonstration one crate away and a `Producers` newtype would trade `decisions/002`'s deliberate infallibility for a structural range |
| RC48 | `ring_config` | n/a — observation | The two bare-`usize` fields are exactly the two with clamps, exactly the two `ring_bench::Workload` keeps loose copies of beside the record that already holds them, and two of the three with zero production readers — `capacity` 4, `overflow` 5, and zero for `wait`, `producers` and `batch` — so the alignment is a subset rather than a match, and the domain-typed exception `wait` is what makes it specific: the fields another crate copied out are the ones where copying loses nothing the type system was carrying, which is why a newtype would be worth more than the clamp it replaced |
