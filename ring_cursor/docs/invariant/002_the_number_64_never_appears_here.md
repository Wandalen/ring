# Invariant: The Number 64 Never Appears Here

### Scope

- **Purpose**: State the restriction that this crate's source contains no cache-line literal, show that it currently holds, and record that nothing automatic enforces it.
- **Responsibility**: Give the check with its output, explain why a literal here would be a family-level defect rather than a local one, and name the violation most likely to arrive.
- **In Scope**: Every non-doc `64` in `ring_cursor/src/lib.rs`; the deliberate literals in `tests/`.
- **Out of Scope**: What the alignment guarantees, which is [`invariant/001`](001_one_cursor_one_line.md); the constant's own placement argument, which is [`ring_align`'s `integration/002`](../../../ring_align/docs/integration/002_why_the_constant_lives_here.md).

### The Restriction

No literal `64` may appear in this crate's source outside documentation. The
alignment comes from `ring_align::CacheAligned`'s own attribute and is never
restated.

A bare pass here is an empty result, so it is paired with a control — the same
check against a file that genuinely does restate the number, so that an empty
first arm is evidence rather than a broken pattern:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- this crate (expect 0 -- invariant holds) --'
command grep -E "\b64\b" ring_cursor/src/lib.rs \
  | command grep -vE "^[[:space:]]*(///|//!)" \
  | sed 's:^:    :'
printf '    matches: %s\n' "$( command grep -E "\b64\b" ring_cursor/src/lib.rs | command grep -vE "^[[:space:]]*(///|//!)" | wc -l )"
echo '  -- control: the same check where the literal is genuinely written --'
command grep -E "\b64\b" ring_align/src/lib.rs \
  | command grep -vE "^[[:space:]]*(///|//!)" \
  | sed 's:^:    :'
```

Live output:

```
  -- this crate (expect 0 -- invariant holds) --
    matches: 0
  -- control: the same check where the literal is genuinely written --
    pub const CACHE_LINE : usize = 64;
    // family has used (64, 128) and unchecked until now (-> docs/algorithm/001
    #[ repr( align( 64 ) ) ]
```

**Expected: nothing under the first heading, something under the second.** Every
`64` in this crate's file is in prose or in a doc example, where a literal is the
assertion rather than a definition; `ring_align`'s two are the declaration and
the attribute the crate exists to own.

### Why a Literal Here Would Be a Family Defect

The number is not this crate's to know. `ring_align::CACHE_LINE` exists so the
padding decision is made in one place, for one reason, and every cursor in the
family inherits it without a second author deciding 64 was probably fine.

A literal here forks that:

| | With inheritance | With a literal |
|---|---|---|
| Statements of the cache-line size in the family | 1 | 2 |
| Edits needed to port to a 128-byte line | `ring_align` only | `ring_align` **and** here |
| What happens if only one is edited | — | Cursors align to 64 on a machine with 128-byte lines. Every test still passes |

The last cell is the whole point. `align_of == CACHE_LINE` would still hold —
against the *new* `CACHE_LINE` of 128 — only if the type inherited it. A literal
`64` makes `align_of` 64 while `CACHE_LINE` is 128, and the third assertion in
`a_padded_cursor_occupies_exactly_one_cache_line` is the one that catches it:

```rust
assert_eq!(
  core::mem::size_of::< PaddedCursor >(),
  CACHE_LINE,
  "the 64 above is ring_align::CACHE_LINE, not a coincidence"
);
```

**That assertion is the invariant's only automatic defence, and it is partial.**
It catches a literal in the *size*. It would not catch a crate that computed the
right size by an independent route, and it does not run until someone changes
`CACHE_LINE`.

### The Tests Use Literals Deliberately

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E "\b64\b" ring_cursor/tests/cursor_test.rs | head
```

Live output:

```
//! as three clauses: `align_of::<PaddedCursor>() == 64`,
//! `size_of::<PaddedCursor>() == 64`, and two `PaddedCursor` values in one
//! struct sitting at least 64 bytes apart.
//! A type of size 8 with `align_of == 64` satisfies the first clause and packs
//! start, not how much room it occupies. A type of size 64 with `align_of == 8`
//! the cursors together despite each still measuring 64 bytes on its own.
    64,
    64,
    "the 64 above is ring_align::CACHE_LINE, not a coincidence"
  let large = CursorPair::new( cap( 64 ) );
```

`tests/cursor_test.rs` asserts against `64` *and* against `CACHE_LINE`, in the
same test. The asymmetry is intentional:

| Location | `64` means |
|----------|------------|
| `src/` | a second definition — forbidden |
| `tests/` | an independent statement of the expected value — required |

A test that only asserted `size_of == CACHE_LINE` would pass if both were wrong
together. Asserting the literal pins the value; asserting the constant pins the
provenance. Both, in one test, is what makes the pair meaningful.

### The Violation Most Likely to Arrive

From `tests/manual/readme.md`'s own closing note:

> M1 is the check worth keeping for the crate rather than for the method. … writing
> `#[ repr( align( 64 ) ) ]` directly on `PaddedCursor` is the obvious
> implementation — shorter, one fewer dependency, and it passes all three clauses
> of the reached-test. It is also how the family acquires a second, independent
> cache-line constant. The test suite would notice nothing.

**"Shorter, one fewer dependency" is the dangerous part.** It is not a careless
change; it is a change a reviewer would approve. The `ring_align` dependency
exists solely to supply one attribute, and removing a dependency that supplies
one attribute reads like simplification.

And the family has already done it once. `ring_mpsc/src/lib.rs:865` carries a
second copy of the cache-line constant under a method name shared with
`ring_align`'s function — recorded in
[`ring_align`'s `invariant/002`](../../../ring_align/docs/invariant/002_one_constant_for_the_whole_family.md).
So the failure mode is not hypothetical; it is one instance old.

### What Enforces It

| # | Check | Status |
|---|-------|--------|
| U1 | `tests/manual/readme.md` M1 | **A human running a grep.** Recorded ✅ on 2026-08-28, and re-run for this document with the same result |
| U2 | The third assertion in `a_padded_cursor_occupies_exactly_one_cache_line` | Automatic, and partial — see above |
| U3 | The compiler | Never. `#[ repr( align( 64 ) ) ]` is valid Rust |
| U4 | `cargo +nightly udeps` | Would report `ring_align` as unused *after* the violation, at verification level 4 |

**U4 is worth noting as a second-order detector.** Replacing the wrapper with a
direct attribute makes `ring_align` an unused dependency, which `udeps` reports —
but only if the author leaves the dependency declared, and only at a verification
level ordinary work does not reach. `ring_align` itself has an unused-dependency
finding of exactly this shape, recorded in
[its `integration/001`](../../../ring_align/docs/integration/001_one_dependency_one_consumer.md).

### CU23 — The Check Is Blind to the Three Literals That Execute

```
140:/// assert_eq!( core::mem::size_of::< PaddedCursor >(), 64 );
141:/// assert_eq!( core::mem::align_of::< PaddedCursor >(), 64 );
184:  /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
```

The restriction's own grep excludes lines beginning `///` or `//!`, so it reports
zero while three `64` literals sit in the file.

**Finding.** Those three are doctests: `cargo test --doc` compiles and runs them.
They are the one place in this file where a literal both executes today and would
keep executing after the fork this invariant forbids — and they are precisely the
lines the check cannot see. The exclusion is right for prose and wrong for the
examples inside it.

---

### CU24 — The Only Instrument That Sees This Invariant Is a Human

| Instrument | Sees the restriction | Runs |
|------------|---------------------|------|
| `tests/manual/readme.md` M1 | yes | when a human runs it |
| the recipe in this definition's readme | yes | when the corpus gate re-quotes it |
| `cargo nextest` | no | every change |
| `cargo +nightly udeps` | second-order only | level 4 |

**Finding.** The published recipe is the same grep as M1, and the corpus gate
checks that it still prints what it says — not that what it prints is zero. So
the invariant has a documented check, a manual check, and no automatic one; a
literal `64` added to this file tomorrow would make the recipe *stale* rather than
*failing*, and staleness is repaired by re-quoting.

---

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_four_dependencies_all_used.md](../integration/001_four_dependencies_all_used.md) | The `ring_align` edge this invariant keeps load-bearing, and M6 which checks it |

### Invariants

| File | Relationship |
|------|--------------|
| [001_one_cursor_one_line.md](001_one_cursor_one_line.md) | The restriction that would still pass after this one is broken |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | The shape that makes inheriting the attribute possible without restating it |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_obvious_implementation_forks_the_constant.md](../pitfall/001_the_obvious_implementation_forks_the_constant.md) | The violation, as a failure mode rather than as a restriction |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The type whose alignment is inherited |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:13-21` | "the padding decision is made in one place, for one reason" |
| `ring_align/src/lib.rs` | `CACHE_LINE` and `CacheAligned` |
| `ring_mpsc/src/lib.rs:865` | The family's existing second copy |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` M1 | U1 — the only check that sees this invariant directly |
| `tests/manual/readme.md` § closing note | The argument for keeping M1 |
| `tests/cursor_test.rs:67-71` | U2 — `size_of` pinned to `CACHE_LINE`, not to a literal |
| `tests/cursor_test.rs:41-45` | The test file imports `CACHE_LINE` rather than hard-coding it in its helpers |
