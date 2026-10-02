# ring_seqno manual testing plan

Automated tests (`tests/seq_test.rs`) assert what the functions return. This
plan covers what a *reader* of the crate has to be able to conclude, which no
assertion reaches: that the crate's reason for existing is legible, and that its
one dangerous boundary is documented where someone will hit it.

Every check below is a command with an expected observation. Run them from the
workspace root.

## M1. The never-fold property is stated, not just implemented

The whole reason `ring_seqno` is separate from `ring_index` is that a sequence is
never folded into a slot index, so two positions many laps apart stay
comparable. If that is only true of the code and not said anywhere, the next
person to "optimise" it by storing a folded `usize` breaks every gate silently.

```bash
grep -rn "lap" ring_seqno/src/lib.rs | head -20
```

**Expected:** the module doc explains comparison across laps, and at least one
function's own doc names the lap boundary. The word `wrap` does not describe the
*arithmetic*, since the crate compares and does not wrap (decision 121 § 6).

## M2. The exclusive claim boundary is documented where it is used

`may_claim` refuses at exactly one full lap. Off-by-one here is the classic ring
bug: a producer one lap ahead overwrites the slot the consumer is reading.

```bash
grep -n -B 4 -A 12 "fn may_claim" ring_seqno/src/lib.rs
```

**Expected:** the doc states the boundary is exclusive at one lap, and the doc
example shows both the last accepted and the first refused position, not just
one side.

## M3. Reversed arguments produce an obviously-wrong answer, not a plausible one

`laps_between( later, earlier )` and `pending( consumer, producer )` are easy to
call backwards. The design choice is to return zero rather than a huge number,
so a caller sees "nothing to do" instead of a plausible-looking figure.

```bash
cd ring_seqno && cargo test --test seq_test laps_backward_read_zero -- --nocapture
```

**Expected:** passes. Then read the assertion. The point is that `100` and `4`
swapped yields `0`, which a human reading a log recognises as wrong, where
`18446744073709551612` looks like a real measurement.

## M4. The doc examples are the API's first reader

```bash
cd ring_seqno && cargo test --doc
```

**Expected:** every example compiles and passes. Then read them. Each should be
readable as an explanation on its own, without the surrounding prose.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | Module doc names comparison across laps; `laps_between` and `may_claim` both state the lap boundary. `wrap` does occur twice, both times *denying* it ("the sequence does not wrap, the slot index does", plus the decision-121 rename note); the rest are `Capacity::new(..).unwrap()`. Nothing describes the arithmetic as wrapping. |
| 2026-08-28 | M2 | ✅ | `may_claim`'s doc states the boundary is exclusive; example shows `Seq(3)` accepted and `Seq(4)` refused at capacity 4. |
| 2026-08-28 | M3 | ✅ | Passes. Backward pair reads 0. |
| 2026-08-28 | M4 | ✅ | 5 doc tests pass. |
