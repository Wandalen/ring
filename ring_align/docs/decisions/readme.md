# Decisions Doc Definition

### Scope

- **Purpose**: Record the two choices in this crate that had a defensible alternative, so that a later reader changing them knows what was traded rather than rediscovering it.
- **Responsibility**: For each, state the alternative, why it lost, and the specific condition that would reopen the question.
- **In Scope**: Whether `CACHE_LINE` varies by target; whether `on_distinct_lines` takes integers or references.
- **Out of Scope**: The consequences of the value being wrong, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md); where the constant lives, which is [`integration/002`](../integration/002_why_the_constant_lives_here.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Constant Is Not Conditional](001_the_constant_is_not_conditional.md) | One value for every target, rather than `cfg`-per-architecture — accepting a known-wrong value on Apple Silicon in exchange for one place to change | 🔄 |
| 002 | [The Predicate Takes Integers](002_the_predicate_takes_integers.md) | `usize` addresses rather than references, and what that costs in caller ceremony | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the constant, declared once and unconditionally --'
command grep 'pub const CACHE_LINE' ring_align/src/lib.rs
echo '  -- cfg-gated alternatives in this crate --'
command grep -c 'cfg( target' ring_align/src/lib.rs
echo '  -- control: the family does gate on cfg elsewhere --'
command grep -rl 'cfg( ' ring_*/src/*.rs | head -3
echo '  -- the signature the predicate settled on --'
command grep 'pub const fn on_distinct_lines' ring_align/src/lib.rs
```

Live output:

```
  -- the constant, declared once and unconditionally --
pub const CACHE_LINE : usize = 64;
  -- cfg-gated alternatives in this crate --
0
  -- control: the family does gate on cfg elsewhere --
ring_atomic/src/lib.rs
ring_bench/src/lib.rs
ring_core/src/lib.rs
  -- the signature the predicate settled on --
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
```

**The zero carries a control** for the reason this whole definition exists: a
zero from a grep whose pattern went stale looks exactly like a decision held to.
The three crates beneath it prove the family uses `cfg` where it wants to, so
the absence here is a choice rather than an unavailable tool.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL13 | `CACHE_LINE` | n/a — observation | The value is unconditional across every target: zero `cfg( target… )` in the crate, against three crates in the family that do gate on `cfg`. What was bought is one edit site for a port; what was sold is correctness on Apple Silicon, where the real line is 128 and this constant is knowingly wrong |
| AL14 | `CACHE_LINE` | **latent hazard** | The direction of the error is the whole risk and is not visible at the declaration: a value too *large* wastes memory and still separates, a value too *small* silently stops separating anything. 64 on a 128-byte-line machine is the second case, and every assertion in the suite still passes there |
| AL15 | `on_distinct_lines` | n/a — observation | Taking `usize` rather than `&T` costs every caller an explicit `from_ref( … ) as usize`, and buys the elimination of a doc example over two stack locals — a trade made because that example was written once and was wrong, not because the ceremony was judged cheap |
| AL16 | Both decisions | n/a — observation | Each is reopened by a condition about the world rather than about this crate: a target whose line is not 64 for the first, and a signature form that cannot express the stack-locals mistake for the second. Neither has a trigger anything in the repository watches for |
