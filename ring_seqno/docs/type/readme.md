# type

The promises a caller may rely on. This crate declares no type of its own, so
both instances are about the types it *returns*.

### Overview Table

| ID | Name | Subject |
|----|------|---------|
| 001 | [What a Span Is Measured In](001_what_a_span_is_measured_in.md) | `u64`, `usize` and `bool` across four readings that compute the same thing |
| 002 | [The `Option` That `slowest` Returns](002_the_option_that_slowest_returns.md) | The one return type in the crate that carries a design decision |

### Why a Type Definition for a Crate With No Types

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^pub (struct|enum|trait|type)' ring_seqno/src/lib.rs || true   # 0
# control — the identical expression over a crate that does declare types
command grep -cE '^pub (struct|enum|trait|type)' ring_types/src/capacity.rs
```

Live output:

```
0
1
```

A caller of `ring_seqno` names `ring_types::Seq` and `ring_types::Capacity`
directly; nothing here stands between them and it. What this crate does own is
the **return** side of five signatures, and that is where its promises live —
three different types for four readings of one subtraction, and one `Option` that
exists to refuse an answer.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ47 | Two widths for one unit | n/a — inconsistency | `free_slots` returns `usize` and `pending` returns `u64` for quantities in the same units, which is what makes the narrowing cast look deliberate |
| SQ48 | Count and index | n/a — observation | A free-slot count and a slot index are both `usize` in `0..=capacity`, so the compiler cannot separate them |
| SQ49 | `Option< Seq >` | n/a — observation | `Option< Seq >` is 16 bytes — `Seq` has no niche — and is never stored anywhere in the family |
| SQ50 | The single failure path | n/a — observation | The test suite's one `expect` is in the `cap` helper, so every assertion in ten tests rests on a capacity constructor from another crate and the suite has no way to fail other than by asserting |
