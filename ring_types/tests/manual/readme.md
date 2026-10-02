# Manual testing for ring_types

What a person checks by hand for the ring family's shared vocabulary, and what
was observed the last time they did.

`ring_types` is pure data, with no threads, no IO and no timing, so the automated
suite can decide almost everything about it. The manual plan covers the two
things a test cannot assert about itself: that the *compiler* rejects what the
design says it should, and that the documented examples say what a reader needs.

## Plan

### M1. The two position types do not mix

`Seq` and `SlotIndex` exist as separate types so a folded position is never
compared against an unfolded one. A test cannot assert this, because the code
that would prove it does not compile.

```bash
cd ring_types
cat > /tmp/-ring_types_m1.rs <<'EOF'
fn main()
{
  let _ = ring_types::Seq( 3 ) == ring_types::SlotIndex( 3 );
}
EOF
```

Expected: adding that expression to a test file and building fails with
`mismatched types`, naming `Seq` and `SlotIndex`. A build that succeeds means
the two types have collapsed into one.

### M2. `Capacity` cannot be constructed around its own validation

There is exactly one constructor and the field is private, so no caller can
hold a `Capacity` whose `mask()` is wrong.

```bash
grep -n "pub struct Capacity" ring_types/src/capacity.rs
grep -rn "Self( slots )\|Capacity(" ring_types/src/capacity.rs | grep -v '///'
```

Expected: the struct's field is unnamed and **not** `pub`. The line reads
`pub struct Capacity( usize )`, not `pub struct Capacity( pub usize )`. The
only construction outside a doc comment is the `Self( slots )` inside `new`,
after both validation branches have returned.

### M3. Overflow has no overwrite variant

The claim to check is a negative. The automated test asserts the
set has three members; a reader should confirm by eye that none of the three
overwrites unread data.

```bash
sed -n '/pub enum OverflowPolicy/,/^}/p' ring_types/src/policy.rs
```

Expected: exactly `DropNewest`, `DropOldest`, `Fail`. `DropOldest` evicts an
item the consumer has *not yet been handed*, which is a drop, not an overwrite
of one in flight. That distinction is the one to check.

### M4. Documented examples compile and are worth reading

```bash
cargo test -p ring_types --doc
```

Expected: every doc example runs. Then read the rendered docs and check the
examples show the type's *point*, not its syntax:

```bash
cargo doc -p ring_types --no-deps --open
```

### M5. The 584-year reachability figure holds up under direct arithmetic

`Seq::next` is protected against wrapping only because the wrap point sits far
outside any reachable workload, and it has no saturating mode. `advanced_by`
has none either, and its caller owns the bound on `n`.
`seq_does_not_wrap_within_any_reachable_workload`
asserts the quotient exceeds 500 without ever calling `next` or `advanced_by`,
so a person needs to check the figure itself by hand, beyond whether some test
passes.

```bash
awk 'BEGIN { printf "%.1f\n", (2^64 - 1) / 1e9 / 86400 / 365.25 }'
```

Expected: approximately `584` years, matching the figure quoted in
`src/id.rs`'s doc comments on `Seq` and `Seq::next`. A materially different
result means either `u64`'s range changed or the 10⁹/s publish-rate assumption
no longer holds, and both citations need re-deriving from whatever the true
figure becomes.

## Run Record

| Date | By | Result | Notes |
|------|-----|--------|-------|
| 2026-08-28 | dev | M1 pass, M2 pass, M3 pass, M4 pass | M1: `Seq(3) == SlotIndex(3)` fails with `mismatched types`, as required. M2: field is private, `Capacity(` appears only in `new`. M3: three variants, none overwrites in-flight data. M4: 20 doc examples pass. |
| 2026-09-06 | dev | M5 pass | M5: `awk` computed `584.5` years, matching every "584 years" citation across `src/id.rs`, `docs/pitfall/001`, `docs/item/struct/002_seq.md`, `docs/data_structure/001`, `docs/api/001`. Closes `docs/pitfall/001`'s Mitigation 3 (previously open: no manual-plan entry existed for this figure). |
