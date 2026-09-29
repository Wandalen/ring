# Lifecycle: The Constant Across a Platform Port

### Scope

- **Purpose**: Walk the one event in this crate's life that changes anything — raising `CACHE_LINE` from 64 to 128 for a 128-byte-line target — enumerating every site that must change, every check that fails usefully, and every check that survives while silently testing something weaker.
- **Responsibility**: Give the measured site list, classify each site by what happens if it is missed, and identify the sites that provide no signal.
- **In Scope**: The 64 → 128 edit across `ring_align`, `ring_cursor`, and `ring_mpsc`.
- **Out of Scope**: Why the constant is unconditional in the first place, which is [`decisions/001`](../decisions/001_the_constant_is_not_conditional.md); the consequences of *not* porting, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md).

### The Trigger

The family runs on a target whose real cache line is 128 bytes — Apple
Silicon being the case the crate's own doc comment names. Detected by
`tests/manual/readme.md` M1, which is a human running:

```sh
getconf LEVEL1_DCACHE_LINESIZE
```

Live output:

```
64
```

**Nothing automated detects this.** The trigger for the entire lifecycle below
is a person choosing to look (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) M1/M4).

### Every Site, Measured

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -n '\b64\b' ring_align/src/lib.rs ring_align/tests/align_test.rs \
  ring_cursor/src/lib.rs ring_mpsc/src/lib.rs \
  | command grep -viE 'u64|i64|f64|x86_64' \
  | sed -E 's/:[0-9]+:/: /' | LC_ALL=C sort -u
```

Live output:

```
ring_align/src/lib.rs: #[ repr( align( 64 ) ) ]
ring_align/src/lib.rs: // family has used (64, 128) and unchecked until now (-> docs/algorithm/001
ring_align/src/lib.rs: //! No `unsafe` is needed for any of it — `#[ repr( align( 64 ) ) ]` is a safe
ring_align/src/lib.rs: /// 64 on x86-64 and on AArch64's common configuration. Apple Silicon uses 128,
ring_align/src/lib.rs: /// and a value too small is the failure that matters — two cursors 64 bytes
ring_align/src/lib.rs: /// assert!( on_distinct_lines( 63, 64 ) );    // straddling the boundary
ring_align/src/lib.rs: /// assert_eq!( ring_align::CACHE_LINE, 64 );
ring_align/src/lib.rs: pub const CACHE_LINE : usize = 64;
ring_align/tests/align_test.rs:     "CACHE_LINE dropped below 64 - every assertion in this suite would still \
ring_align/tests/align_test.rs:     core::hint::black_box( CACHE_LINE ) >= 64,
ring_align/tests/align_test.rs:   // `CACHE_LINE >= 64` (both operands are literals today), which otherwise
ring_align/tests/align_test.rs:   assert!( !on_distinct_lines( 64, 127 ) );
ring_align/tests/align_test.rs:   assert!( on_distinct_lines( 0, 64 ) );
ring_align/tests/align_test.rs:   assert!( on_distinct_lines( 63, 64 ), "63 and 64 straddle the boundary" );
ring_align/tests/align_test.rs:   assert_eq!( CACHE_LINE, 64 );
ring_align/tests/align_test.rs: //! structural assertion — 64-byte size and alignment — is what a test can
ring_align/tests/align_test.rs: /// The constant is 64 — the line size on the family's stated target platforms.
ring_cursor/src/lib.rs:   /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
ring_cursor/src/lib.rs: //! Alignment alone does not separate two cursors. A 64-aligned type of size 8
ring_cursor/src/lib.rs: //! `#[ repr( align( 64 ) ) ]` happens to round the size up too, so both hold —
ring_cursor/src/lib.rs: //! about a type; two fields being 64 bytes apart is the fact the promise was
ring_cursor/src/lib.rs: //! family inherits it without a second author deciding 64 was probably fine.
ring_cursor/src/lib.rs: /// assert_eq!( core::mem::align_of::< PaddedCursor >(), 64 );
ring_cursor/src/lib.rs: /// assert_eq!( core::mem::size_of::< PaddedCursor >(), 64 );
ring_mpsc/src/lib.rs:     claim.abs_diff( consume ) >= 64
ring_mpsc/src/lib.rs:   /// assert_eq!( ring.capacity().get(), 64 );
ring_mpsc/src/lib.rs:   /// it. Unpadded on purpose: [`PaddedCursor`] would make this array 64 times
ring_mpsc/src/lib.rs:   /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 64 ).unwrap() );
```

Sites are named by their enclosing item, not by line number: the table was
written with addresses and three of them had already drifted onto unrelated code
by the time site 12 below was added to the same file.

| # | Site | Kind | If missed |
|---|------|------|-----------|
| 1 | `ring_align/src/lib.rs` — `pub const CACHE_LINE : usize = 64;` | **The change** | Nothing else matters |
| 2 | `ring_align/src/lib.rs` — `#[ repr( align( 64 ) ) ]` on `CacheAligned` | **The change, again** | Caught loudly — site 6 compares `align_of` against `CACHE_LINE` and they now disagree |
| 3 | `ring_align/src/lib.rs` — crate-level doctest `assert_eq!( CACHE_LINE, 64 )` | Pin | **Fails.** Correct: the change must be deliberate |
| 4 | `align_test.rs` — `cache_line_is_sixty_four` | Pin | **Fails.** Same, in the integration suite |
| 5 | `align_test.rs` — `distinct_lines_follows_boundaries_not_distance`, and the `on_distinct_lines` doctest in `src/lib.rs` | Pin, by example | **Fails**, and has to be *rewritten* rather than retargeted — see below |
| 6 | `align_test.rs` — `a_wrapped_value_occupies_exactly_one_line` (`size_of`/`align_of` vs `CACHE_LINE`) | Relative | Passes correctly. Written against the constant, so it ports for free |
| 7 | `ring_cursor/src/lib.rs` — `PaddedCursor` doctests asserting `size_of` and `align_of` are `64` | Pin | **Fails.** The one useful thing about a literal in a downstream doctest |
| 8 | `align_test.rs` — `an_oversized_payload_rounds_up_to_whole_lines` | **Silent** | Passes, and stops testing what it is named for |
| 9 | `ring_cursor/src/lib.rs` — `addr()` doctest asserting `% 64 == 0` | **Silent** | Passes forever. Any 128-aligned address is 64-aligned |
| 10 | `ring_mpsc/src/lib.rs` — `abs_diff( … ) >= 64` | **Silent** | Passes, stays wrong, and is the crate's live duplication (→ [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md)) |
| 11 | module prose in `ring_align/src/lib.rs`, `ring_cursor/src/lib.rs` and `align_test.rs` | Prose | No signal at all. Documentation stating 64 while the code says 128 |
| 12 | `align_test.rs` — `cache_line_must_not_shrink_below_the_current_known_minimum` | Bound, deliberately | Passes on a raise, which is the point — it is the only site that fires on a *decrease* |

**Sites 1 and 2 together are the change**, and it takes two edits inside the
owning crate because `#[ repr( align( CACHE_LINE ) ) ]` does not compile
(`E0693`, → [`type/001`](../type/001_cache_line.md)). The crate that exists to
prevent the number from being written twice writes it twice, in adjacent
declarations, for a reason the language imposes.

### The Three Silent Sites

Sites 8, 9, and 10 are the lifecycle's real content: assertions that survive
the port, report success, and no longer mean what their names say.

**Site 8** is the sharpest, because it is in this crate's own suite:

```rust
/// A payload larger than a line still gets whole lines, so two of them never
/// share one.
fn an_oversized_payload_rounds_up_to_whole_lines()
{
  let size = core::mem::size_of::< CacheAligned< [ u8; 65 ] > >();
  assert_eq!( size % CACHE_LINE, 0 );
  assert_eq!( size, 128 );
}
```

At `CACHE_LINE = 128`, a 65-byte payload rounds up to **one** line, not two.
`size` is 128, so both assertions still pass — the second by numerical
coincidence with the new line size. The test is green and the property it was
written to check, *over*-sized payloads getting whole lines, is no longer being
exercised by any input.

**Site 9** is the same failure in a downstream doctest: `addr() % 64 == 0` is
implied by `addr() % 128 == 0`, so it passes while asserting strictly less than
it did. Its message — "a 64-aligned value starts on a line boundary" — becomes
false without any assertion changing.

**Site 10** is the one that costs performance rather than meaning: `ring_mpsc`
keeps comparing against a private 64 on a machine with 128-byte lines.

**What the three have in common** is that each hardcodes the *old* value on the
correct side of a comparison that stays true when the real value grows. A pin
against a literal fails on a port, which is useful; a *bound* against a literal
passes on a port, which is not. Sites 3, 4, 5, and 7 are pins and they all
behave well. Sites 8, 9, and 10 are bounds and none of them do.

**Site 12 is a bound and behaves well, which sharpens the rule rather than
breaking it.** What makes sites 8–10 useless is not that they are bounds but
that each is a bound pointing the *wrong way* — the direction they tolerate
(the constant growing) is the direction a port actually takes, so they tolerate
exactly the change they were meant to notice. Site 12 is a bound pointing the
other way: raising `CACHE_LINE` only spends memory, while lowering it leaves
`on_distinct_lines` calibrated to the same wrong number it divides by, so every
other assertion in the suite keeps passing while the padding guarantee quietly
stops holding. A floor is the correct instrument for that, and it is the only
site in the table that fires on it. The rule is therefore about *which
direction* a literal comparison tolerates, not about pins versus bounds.

Its assertion is written `core::hint::black_box( CACHE_LINE ) >= 64` rather than
the bare comparison. Both operands are compile-time literals today, so the bare
form folds to a constant and trips `clippy::assertions_on_constants` — a lint
whose usual reading, *this assertion is dead code*, is wrong here: the
comparison is a live regression guard whose whole purpose is to survive until
one of the operands changes. `black_box` buys the runtime check the lint would
otherwise have argued away.

### The Edit Sequence

1. Change site 1 to `128`.
2. Change site 2 to `#[ repr( align( 128 ) ) ]`. Sites 1 and 2 must move
   together or site 6 fails.
3. Run `cargo test -p ring_align`. Expect sites 3, 4, and 5 to fail.
4. Update sites 3 and 4 to `128`.
5. **Rewrite site 5's examples rather than retargeting them.** The boundary
   cases are `on_distinct_lines( 63, 64 )` and `on_distinct_lines( 0, 64 )`,
   both of which become *false* at a 128-byte line and neither of which has a
   mechanical substitution — `127, 128` and `0, 128` are the new equivalents,
   and the doc comment explaining what they illustrate has to change with them.
6. **Fix site 8 by hand.** No test failure will prompt it. Change the payload to
   `[ u8; 129 ]` so the case is oversized again, or express it relative to
   `CACHE_LINE + 1`.
7. Run `cargo test -p ring_cursor`. Expect site 7 to fail; update to `128`.
8. **Fix site 9 by hand.** No test failure will prompt it.
9. **Fix site 10 by hand**, which means resolving the duplication first — it is
   a `ring_mpsc` change with its own bug record.
10. Update site 11's prose everywhere it states 64.

**Steps 6, 8, 9, and 10 have no verification.** A port that runs the suite,
sees green, and stops is a port that has left three assertions weakened and one
crate wrong.

### Frequency

Once per target platform with a different line size, which for this family's
stated targets is **zero times so far**. That is worth stating: the lifecycle
documented here has never run. Everything above is derived from the code and
the compiler rather than from a port that happened, and the site list is
therefore only as complete as the grep that produced it.

### AL33 — Eight Sites Write the Number, Two Are Compiled

```
15://! No `unsafe` is needed for any of it — `#[ repr( align( 64 ) ) ]` is a safe
28:/// 64 on x86-64 and on AArch64's common configuration. Apple Silicon uses 128,
29:/// and a value too small is the failure that matters — two cursors 64 bytes
34:/// assert_eq!( ring_align::CACHE_LINE, 64 );
36:pub const CACHE_LINE : usize = 64;
41:// family has used (64, 128) and unchecked until now (-> docs/algorithm/001
68:#[ repr( align( 64 ) ) ]
134:/// assert!( on_distinct_lines( 63, 64 ) );    // straddling the boundary
```

Of the eight, `:36` and `:68` are the port's real work. `:34` is a doctest
asserting the constant equals 64 and so fails loudly on a port, and `:134` is a
second doctest that fails the same way. `:15`, `:28`, `:29` and `:41` are prose.

**Finding.** The port has two compiled edits, two loud checks, four prose
edits — and not one silent site. `on_distinct_lines( 63, 64 )` is separated on
a 64-byte line and *not* separated on a 128-byte line, so its assertion does
not quietly weaken: it inverts, and the doctest fails. That is why the table
above rates site 5 a pin and why step 5 of the edit sequence says to rewrite
its examples rather than retarget them. Every genuinely silent site in this
lifecycle — 8, 9 and 10 — lives outside this file.

---

### AL34 — The Duplicated Literal Is Guarded, Which the Workaround Does Not Say

```
30:  assert_eq!( core::mem::size_of::< CacheAligned< u8 > >(), CACHE_LINE );
```

Raise `CACHE_LINE` to 128 and leave `#[ repr( align( 64 ) ) ]` behind, and this
assertion compares 64 against 128 and fails. The suite catches the half-finished
port.

**Finding.** [`workaround/001`](../workaround/001_the_alignment_literal_cannot_be_the_constant.md)
calls the duplicated literal "a second edit site on every platform port", which
is true, and leaves the impression the port can ship inconsistent. It cannot.
The dent is in the invariant's cleanliness rather than in the port's safety, and
the difference matters when deciding whether the workaround is worth engineering
around.

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_the_constant_is_not_conditional.md](../decisions/001_the_constant_is_not_conditional.md) | E1 — this lifecycle is that reopening condition executed |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | D1 promoted from latent to actual at step 9 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_the_wrapped_values_arc.md](001_the_wrapped_values_arc.md) | The value-level lifecycle this event reshapes in every state at once |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_owner_for_a_magic_number.md](../pattern/002_one_owner_for_a_magic_number.md) | "One edit" is the pattern's promise; sites 1, 2, and 10 are what it actually costs |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | The state this lifecycle exits |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | `E0693`, the reason sites 1 and 2 are two sites |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:27-30` | The doc comment stating the port as the intended response |
| `ring_align/tests/manual/readme.md` | M1, the only trigger detector |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | Sites 4, 5, 6, and 8 — four of the five in-crate checks, three of which port cleanly and one of which does not |
| `tests/manual/readme.md` | M1 detects the trigger; nothing verifies the sequence was completed |
