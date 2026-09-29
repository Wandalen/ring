# Type: `CACHE_LINE`

### Scope

- **Purpose**: Document the family's single answer to "how big is a cache line", and why a `usize` constant is a design decision rather than a magic number that happened to get a name.
- **Responsibility**: State the value, its justification, the platform where it is wrong, and what owning it in one place buys.
- **In Scope**: The constant's value, type, and ownership.
- **Out of Scope**: The failure a wrong value produces, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md); how a port would change it, which is [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md).

### Representation

```rust
pub const CACHE_LINE : usize = 64;
```

`usize` rather than `u32` or `u16`, because every use is arithmetic against a
size or an address: `size_of` returns `usize`, `align_of` returns `usize`, and
[`on_distinct_lines`](../item/002_on_distinct_lines.md) divides addresses that
are `usize`. A narrower type would put a cast at every call site, and a cast is
where a truncation hides.

### Why 64

64 is the line size on x86-64 and on AArch64's common configuration — the two
platforms this family targets, and the two the crate's own test host reports:

```sh
getconf LEVEL1_DCACHE_LINESIZE   # 64 on this host, checked 2026-08-28
```

Live output:

```
64
```

**Apple Silicon uses 128, and that is not a hypothetical.** The constant is
wrong there, and wrong in the direction that fails silently: two cursors 64
bytes apart still share a 128-byte line, so the padding buys nothing while
every structural assertion in the test suite still passes
(→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)). The
manual plan's M1 exists to check this against the real host rather than trust
the constant, and records the reading rather than adjusting the test.

### Why It Is One Constant, Not a Conditional

The obvious alternative is `#[ cfg( target_arch ) ]` selecting 64 or 128. The
crate does not do this, and the reasoning is recorded as a decision
(→ [`decisions/001`](../decisions/001_the_constant_is_not_conditional.md)): a
conditional constant makes every layout assertion in the family conditional
too, and the family's tests assert exact numbers. The stated plan is that a
future port **raises** this value rather than making it vary — one number, one
place, and a pessimistic one if the targets diverge.

### Why It Is a Crate

A 64 could be written at each use site, and this crate would not exist. What
the crate buys is that **the family cannot acquire two independent answers to
the same question** — the property [`ring_cursor`'s own readme](../../../ring_cursor/readme.md)
names as the reason the decision is not held there:

> The padding *decision* is not here — it is `ring_align`'s single
> `CACHE_LINE` — and keeping it there is what stops the family from acquiring
> two independent answers to the same question.

That is [`pattern/002`](../pattern/002_one_owner_for_a_magic_number.md), and it
is the whole argument for a crate this small.

### Kind

A `const` item — inlined at every use, no storage, usable in `const fn` bodies
and in `#[ repr( align( … ) ) ]`… **with one exception that matters.**

**The constant cannot be used in the attribute it exists to justify.**
`#[ repr( align( 64 ) ) ]` requires a literal; `#[ repr( align( CACHE_LINE ) ) ]`
is rejected by the compiler with `E0693` — *"incorrect `repr(align)` attribute
format: `align` expects a literal integer as argument"*. Check it directly:

```sh
printf 'pub const CACHE_LINE : usize = 64;\n#[ repr( align( CACHE_LINE ) ) ]\npub struct Probe( u64 );\nfn main() {}\n' > ./-probe.rs
rustc --crate-name probe --edition 2021 -o /dev/null ./-probe.rs   # E0693
rm -f ./-probe.rs
```

Live output:

```
error[E0693]: incorrect `repr(align)` attribute format: `align` expects a literal integer as argument
 --> ./-probe.rs:2:17
  |
2 | #[ repr( align( CACHE_LINE ) ) ]
  |                 ^^^^^^^^^^

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0693`.
```

So [`CacheAligned`](002_cache_aligned.md) carries a hardcoded
`64` in its attribute and this constant is the *documented* form of the same
number — two occurrences of one value, which is exactly what this crate exists
to prevent, in the one place the language does not permit preventing it.

The mitigation is a test rather than a mechanism:

```rust
assert_eq!( core::mem::align_of::< CacheAligned< u64 > >(), CACHE_LINE );
assert_eq!( core::mem::size_of::< CacheAligned< u64 > >(), CACHE_LINE );
```

If the attribute and the constant ever disagree, those assertions fail. That is
a genuine bind rather than an oversight, and it is worth stating in the type's
own documentation because a reader who notices the duplication should find the
reason here rather than concluding the crate is careless.

### Usage

| Crate | Uses it for |
|-------|-------------|
| `ring_align` | Its own layout assertions and `on_distinct_lines`'s divisor |
| [`ring_cursor`](../../../ring_cursor/readme.md) | Asserting `PaddedCursor` is exactly one line, and that a `CursorPair` is 2–3 lines rather than more |

Regenerate the second column:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "CACHE_LINE" --include=*.rs . | grep -v '^ring_align/'
```

Live output:

```
ring_cursor/tests/cursor_test.rs:use ring_align::CACHE_LINE;
ring_cursor/tests/cursor_test.rs:    CACHE_LINE,
ring_cursor/tests/cursor_test.rs:    "the 64 above is ring_align::CACHE_LINE, not a coincidence"
ring_cursor/tests/cursor_test.rs:    gap >= CACHE_LINE,
ring_cursor/tests/cursor_test.rs:    "clause 3: producer and consumer are {gap} bytes apart, need >= {CACHE_LINE}"
ring_cursor/tests/cursor_test.rs:    assert_eq!( addr % CACHE_LINE, 0, "{name} starts mid-line at {addr}" );
ring_cursor/tests/cursor_test.rs:    assert_eq!( gap, CACHE_LINE, "consecutive cursors are one line apart, not {gap}" );
ring_cursor/tests/cursor_test.rs:  // size stops being CACHE_LINE and the first test catches it — but this one
ring_cursor/tests/cursor_test.rs:  assert_eq!( core::mem::size_of::< PaddedCursor >(), CACHE_LINE );
ring_cursor/tests/cursor_test.rs:  assert!( size >= 2 * CACHE_LINE, "two cursors at minimum, got {size}" );
ring_cursor/tests/cursor_test.rs:  assert!( size <= 3 * CACHE_LINE, "capacity may cost up to one additional cache line, got {size}" );
ring_cursor/tests/cursor_test.rs:  assert_eq!( core::mem::align_of::< CursorPair >(), CACHE_LINE );
ring_cursor/src/lib.rs:/// `size_of` and `align_of` are both [`ring_align::CACHE_LINE`], which is what
```

**One consumer, and that is the expected number** — the constant reaches the
rest of the family through `ring_cursor`'s types rather than directly
(→ [`integration/001`](../integration/001_one_dependency_one_consumer.md)).

### AL45 — The One Number Whose Misuse Is Silent Is the One Left as a Primitive

The family newtypes `Capacity`, `Budget` and `SlotIndex` — three plain counts.
It leaves `CACHE_LINE` a bare `usize`.

**Finding.** That is what lets `claim.abs_diff( consume ) >= 64` typecheck at
`ring_mpsc/src/lib.rs:865`: a distance and a line size are both `usize`, so the
compiler has nothing to object to. A `LineSize( usize )` would not have
prevented the literal, but it would have made the comparison between a *distance*
and a *line size* a type error, which is the specific mistake
[`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md)
exists to describe.

---

### AL47 — The Crate's Entire Published Vocabulary Fits in One Grep

```
35:pub const CACHE_LINE : usize = 64;
53:pub struct CacheAligned< T >( T );
122:pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
```

Three names, no module tree, no re-exports.

**Finding.** This is the property that makes `ring_align` a tier-1 crate rather
than a utility drawer, and it is checkable rather than aspirational — the block
above prints a fourth name the moment one appears, before anybody notices it is
undocumented. Two of the three are Domain Types documented here; the third is a
predicate documented in [`item/002`](../item/002_on_distinct_lines.md), which is
why those two definitions do not overlap.

---

### AL48 — Wrapping the Constant Would Cost Nothing at the One Site That Cannot Use It

The usual objection to newtyping a constant is that it becomes unusable in
positions requiring a primitive. Here the position that matters —
`#[ repr( align( … ) ) ]` — already refuses the constant in *any* form, newtype
or not (→ [`workaround/001`](../workaround/001_the_alignment_literal_cannot_be_the_constant.md)).

**Finding.** So the trade AL45 describes is unusually one-sided: call-site type
safety on one hand, and on the other a cost that the language has already
imposed regardless. The primitive form was chosen for reasons that read as
obvious and turn out, when the one hard site is examined, not to apply.

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_the_constant_is_not_conditional.md](../decisions/001_the_constant_is_not_conditional.md) | Why one number rather than a `cfg` per architecture |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | The restriction this constant's existence imposes on the other 32 crates |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_cache_aligned_and_its_associated_functions.md](../item/001_cache_aligned_and_its_associated_functions.md) | The declaration-site catalog, including the literal `64` this constant cannot replace |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_constant_across_a_platform_port.md](../lifecycle/002_the_constant_across_a_platform_port.md) | The one event that changes this value, and how it is detected |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_owner_for_a_magic_number.md](../pattern/002_one_owner_for_a_magic_number.md) | The general form of why this constant is a crate |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | What a wrong value does, and why the wrongness is silent |

### Types

| File | Relationship |
|------|--------------|
| [002_cache_aligned.md](002_cache_aligned.md) | The wrapper whose attribute restates this number as a literal |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The invariant requiring two wrapped fields to land on different cache lines — this constant is the size of that line |
| [`bench_harness/gate/declared/ring/unsafe_allowlist.txt`](../../../bench_harness/gate/declared/ring/unsafe_allowlist.txt) | Removed this crate's unused `unsafe` allowlist entry, on the ground that a permission nobody exercises is a bound looser than the code actually is |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `cache_line_is_sixty_four` asserts the value; every layout test asserts the attribute agrees with it, which is the only available guard on the literal/constant duplication |
| `tests/manual/readme.md` | M1 checks the constant against the real host via `getconf`, because no automated test can |
