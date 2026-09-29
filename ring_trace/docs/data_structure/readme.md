# data_structure

Two structs, five fields between them, and a `Vec`. `Trace` is a `bool` and a
`Mutex< Vec< TraceEntry > >` in forty bytes; `TraceEntry` is a one-byte
discriminant, a `Seq` and a `usize` in twenty-four. Nothing here is subtle, and
that is the point — the whole crate is a flag guarding a growable array, and both
instances below are about what the measured shape of that array turns out to
cost and to carry.

The two findings pull in opposite directions. The disabled trace is better than
advertised: it never allocates, not once, however many operations pass through
it, and no line in the crate claims that. The enabled entry is worse than it
looks: seven bytes in every twenty-four are padding, and the field that causes it
is sixty-four bits wide for a quantity a ring bounds by its capacity.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | Forty Bytes, a Flag and a Vector That Never Allocated | The two layouts, the zero-allocation guarantee, and the padding |
| [002](002_three_public_fields_and_the_range_they_imply.md) | Three Public Fields and the Range They Imply | One internal construction site, seven external, and the missing dimension |

## The Guarantee Nobody Wrote Down

Ten thousand `record` calls against a disabled trace leave the vector at length
zero and capacity zero. `Vec::new` does not allocate and `record` returns before
reaching it, so a switched-off trace costs forty bytes and no heap at all, for
the life of the program.

The criterion asks for zero entries and the crate delivers zero allocations, and
the stronger property is the one that goes unstated. It is also the one a
plausible future change could lose in silence — pre-sizing the vector in
`enabled()` is a natural optimisation, and nothing today says that `disabled()`
must not do the same. The assertion that would pin it is one line and no test
makes it.

## Twenty-Four Bytes for Seventeen

The three fields sum to seventeen. The struct is twenty-four, because the
eight-aligned `Seq` and `usize` come first and the one-byte discriminant trails
seven bytes of padding. At a million recorded operations that is 23,437 KiB where
15,625 would do, the difference being a `count` narrowed from `usize` to `u32`.

Whether to take it is a range decision rather than a free win: `Capacity` is a
`usize` newtype with power-of-two validation and no declared ceiling, so
narrowing `count` asserts a bound the family currently leaves open. That is a
reasonable assertion to make and a different thing from having overlooked the
padding — which is why the alternative repair is simply to say, at the
declaration, that a third of the log is padding and why the crate accepted it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two declarations --'
command grep -m1 -A8 -F 'pub struct TraceEntry' ring_trace/src/lib.rs
command grep -m1 -A5 -F '#[ derive( Debug ) ]' ring_trace/src/lib.rs | tail -n 5
echo '  -- every entry the crate or its tests build by hand --'
command grep -rc 'TraceEntry {' --include=*.rs ring_trace | sed 's/^module\///'
echo '  -- and whether anything records time --'
printf '    time-bearing items in the crate: '
command grep -c 'Instant\|nanos\|Duration\|SystemTime' ring_trace/src/lib.rs || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR9 | `ring_trace` | n/a — doc gap | Ten thousand `record` calls against a disabled trace leave the vector at length zero *and capacity zero*, because `Vec::new` does not allocate and `record` returns before touching it, so a switched-off trace costs forty bytes and not one byte of heap for the life of the program — a stronger property than the criterion's "zero entries" and than the test header's framing of the risk avoided, and one no line in the crate claims; it is also the crate's most silently losable property, since pre-sizing the vector in `enabled()` is a natural optimisation that would have to be written so `disabled()` does not follow, with nothing today saying why, and it is directly assertable as `assert_eq!( trace.entries().capacity(), 0 )` after a loop of records, which no test does |
| TR10 | `ring_trace` | **measured cost** | `TraceEntry`'s three fields sum to seventeen bytes and the struct measures twenty-four: Rust puts the eight-aligned `Seq` and `usize` first and the one-byte discriminant trails seven bytes of padding, twenty-nine per cent of every entry, so a million recorded operations occupy 23,437 KiB where the same information needs 15,625 with a `u32` count and a sixteen-byte entry; the saving is a range decision rather than a free win, since `ring_types::Capacity` is a `usize` newtype with power-of-two validation and no declared ceiling so nothing forbids a capacity above four billion — a ring needing at least thirty-two gibibytes of slots, but representable — which means either narrowing the field with a comment naming the bound it assumes, or stating at the declaration that a third of the log is padding the crate accepted to avoid pinning a capacity ceiling |
| TR11 | `ring_trace` | n/a — doc gap | `record` builds `TraceEntry { op, seq, count }` from its three arguments unexamined, and every field is `pub`, so struct-literal syntax reaches the same place with the same freedom — which is what the crate itself does everywhere outside a `Trace`, six literals in the test file and one in the type's own doctest against a single internal construction site; the equivalence is correct because there is nothing to validate, and the type should stay constructible as the plain record it is, but the sentence that would make it self-describing is missing: that `count` is the number of consecutive sequences covered, that a claim of sixty-four is one entry rather than sixty-four, and that zero denotes an operation covering no sequences — the last of which exists only as a test assertion, "a zero-count operation covers nothing", and nowhere in the documentation |
| TR12 | `ring_trace` | n/a — doc gap | The crate's account of itself draws the line at ordering — a counter "tells you nothing about which sequences or in what order", a trace "answers 'which operations, in what order'" — and the entry carries exactly that, an operation kind, a starting sequence and a span, with zero occurrences of `Instant`, `Duration`, `SystemTime` or `nanos` anywhere in the crate, so a log establishes that publish followed claim and nothing about the gap; `ring_stats`' `wait_nanos` covers the other half only in aggregate, a sum across waiters rather than a duration attributable to any recorded event, so the pair answers how many, how long in total, and in what order, and never how long a single operation took — an absence that is defensible, since a clock read on the producers' path is exactly the cost this crate is careful about, but is invisible in a trace-against-stats section that presents ordering as what the trace adds without naming duration as outside both |
