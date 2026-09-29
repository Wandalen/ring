# API: Four Declarations, Three of Them `const`

### Scope

**Purpose:** Record the crate's entire public surface as declarations — one
type, one associated const, and four functions — with every attribute that
appears on them and the reason the one unmarked function is unmarked. The
title's four are the four functions; `Resolution::ALL` carries no attribute at
all.

**Responsibility:** The `const` split, the three `#[ must_use ]` marks, the derive
list, and what each is doing.

**In Scope:** `ring_overflow/src/lib.rs:55-56`, `:94`, `:110-111`,
`:137-138`, `:192`, `:228-229`.

**Out of Scope:** Why `resolve` cannot be `const` is
[`workaround/001`](../workaround/001_the_recorder_forecloses_const.md). What the
derive list enables is [`type/001`](../type/001_six_derives_on_a_fieldless_enum.md).

---

## The Whole Surface, With Its Attributes

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
# the third alternative is `pub const ` and not `pub const fn ` on purpose:
# `pub const ALL` is a declaration too, and the narrower form cannot see it
echo '  -- every declaration, with the attribute above it --'
command grep -B1 'pub enum \|pub fn \|pub const ' ring_overflow/src/lib.rs | command grep -v '^--$'
echo '  -- how many must_use marks in total --'
command grep -c 'must_use' ring_overflow/src/lib.rs
```

Live output:

```
  -- every declaration, with the attribute above it --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
pub enum Resolution
  /// ```
  pub const ALL : [ Self; 3 ] = [ Self::DroppedIncoming, Self::EvictedOldest, Self::Refused ];
  #[ must_use ]
  pub const fn lost_an_item( self ) -> bool
  #[ must_use ]
  pub const fn accepted_incoming( self ) -> bool
/// [`would_resolve`] to decide and `resolve` once to record.
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
#[ must_use ]
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
  -- how many must_use marks in total --
3
```

The doc-comment line above `pub const ALL` is `-B1` context, not an attribute.
`ALL` is the one declaration here that carries none, which is why the
`must_use` count stays at three while the declaration count is six. It is also
the newest — it was added to close OV10
([`data_structure/001`](../data_structure/001_one_byte_and_two_hundred_fifty_three_spare_niches.md)),
after this document's first census, and until the recipe's third alternative
widened from `pub const fn ` to `pub const ` this document could not see it at
all: a public-surface census whose own command could not match the declaration
it was missing.

---

### OV5 — The One Function That Is Neither `const` Nor `must_use` Is the One That Does Something

Four functions. `lost_an_item`, `accepted_incoming` and `would_resolve` are all
`pub const fn` and all carry `#[ must_use ]`. `resolve` is neither.

Both absences have the same cause and it is not an oversight. `resolve` calls
`stats.record_drop`, which performs an atomic `fetch_add` — not callable in a
const context — so `const` is foreclosed by the language rather than declined.
And `resolve` returns `Result< Resolution, RingError >`, which is already
`#[ must_use ]` in `core`, so an explicit mark would be redundant.

**Finding.** The attribute set is therefore exactly right and reads as if it were
inconsistent. Three of four functions carry a mark that the fourth does not need,
and three of four are `const` where the fourth cannot be — and the source states
neither reason at the declaration. A reader comparing the two free functions sees
`pub const fn would_resolve` above `#[ must_use ]` and `pub fn resolve` above
nothing, and the difference that matters — one touches an atomic — is visible
only two lines into the body.

This is the inverse of the pattern this family usually produces, where an
attribute is applied by position rather than by consequence
([`ring_stats` § ST6](../../../ring_stats/docs/api/001_fourteen_methods_and_no_exclusive_borrow.md)).
Here every mark is load-bearing and every absence is correct; what is missing is
the sentence saying so.

---

### OV6 — Four Functions and No Way to Ask the Question Without Naming a Policy

Every entry point takes an `OverflowPolicy` or a `Resolution`. There is no
constructor, no default, and no function from `Resolution` back to the policy
that produced it — even though the mapping is injective and the suite proves it
([`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md)).

One item that used to be on that list no longer is. `Resolution::ALL` — "Every
variant, for exhaustive iteration", `:80` — supplies the iteration surface, and
it was added to close OV10 rather than to answer this finding. The inverse is
what remains missing, and adding a roster does not supply one: `ALL` enumerates
resolutions, it does not say which policy produced any of them.

For a caller holding a `Resolution` and wanting to report what policy the ring was
configured with, the surface offers nothing; the caller must have kept the policy.
`ring_core` does exactly that — it stores `self.overflow` and re-derives the
resolution at each full-ring event rather than carrying one.

**Finding.** The absent inverse is a real gap only if someone wants it, and the
one consumer does not: it has the policy in hand at every site that could ask.
Recording it here because the injectivity test exists, is named
`distinct_policies_give_distinct_resolutions`, and establishes precisely the
property an inverse would need — so the crate proves the inverse is well-defined
and does not provide it, which is a decision no document makes.

The cost of providing it is one `const fn` and three arms. The cost of not
providing it is that a diagnostic holding a `Resolution` cannot name the
configuration that produced it, which is the same shape as
[`ring_stats` § ST7](../../../ring_stats/docs/api/002_seven_readers_and_no_way_to_read_the_set.md):
a value that answers less than the data behind it could.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_two_predicates_and_no_caller.md) | The two readings on this surface, and who calls them |
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | Why `resolve` cannot join the other three |
| [`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) | The two free functions one at a time |
| [`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md) | The injectivity an inverse would rest on |

### Sources

| Fact | Where |
|------|-------|
| The derive list | `ring_overflow/src/lib.rs:55` |
| The roster's stated purpose | `ring_overflow/src/lib.rs:80` |
| `ALL`, the one declaration with no attribute | `ring_overflow/src/lib.rs:94` |
| Both predicates `const` and `must_use` | `ring_overflow/src/lib.rs:110-111`, `:137-138` |
| `resolve` with neither | `ring_overflow/src/lib.rs:192` |
| `would_resolve` with both | `ring_overflow/src/lib.rs:228-229` |
| The atomic that forecloses `const` | `ring_overflow/src/lib.rs:199` |

### Tests

| Test | Covers |
|------|--------|
| `distinct_policies_give_distinct_resolutions` | The injectivity an inverse would need |
| `resolution_has_exactly_three_variants_and_no_overwrite` | `ALL`'s length, membership and distinctness — what makes the roster load-bearing rather than decorative |
| `a_resolution_is_a_plain_comparable_value` | What the derive list buys a caller |
| `would_resolve_touches_no_counters` | The property that makes `would_resolve` `const`-able |
