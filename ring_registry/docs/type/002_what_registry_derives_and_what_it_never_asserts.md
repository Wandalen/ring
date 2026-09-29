# Type: What `Registry` Derives and What It Never Asserts

### Scope

**Purpose:** Examine the two things `Registry< T >` does at the type level that
look like boilerplate — one `Debug` derive and a hand-written `Default` — and the
one property it has, needs, and never pins.

**Responsibility:** Why `Default` is written out rather than derived, established
by compiling the derived form; whether `Registry< T >` is `Send` and `Sync`,
established the same way; and the family's practice for asserting that.

**In Scope:** `ring_registry/src/lib.rs:87-99`;
`ring_handle/src/lib.rs:69`; the six files in the family carrying a
static `Send`/`Sync` assertion.

**Out of Scope:** `RegistryError` is [`type/001`](001_registry_error.md). The
48-byte width is
[`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md).
The unbounded inherent impl is
[`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md).

---

## One Derive, One Hand-Written Impl, and a Family Practice

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- everything the struct derives, and the impl written by hand instead --'
sed -n '/^#\[ derive( Debug ) ]$/,/^}$/p;/^impl< T > Default for Registry< T >$/,/^}$/p' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- crates in the family that pin Send or Sync with a static assertion --'
command grep -rl 'assert_send\|assert_sync' --include=*.rs ring_*/ | sed 's|ring/||' | sort | sed 's/^/    /'
echo '  -- and whether this crate is one of them --'
printf '    assert_send or assert_sync anywhere in ring_registry: %s\n' \
  "$( command grep -rc 'assert_send\|assert_sync' --include=*.rs ring_registry/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
printf '    Send or Sync named in ring_registry src or readme: %s\n' \
  "$( command grep -c -e 'Send\|Sync' ring_registry/src/lib.rs ring_registry/readme.md 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- the bound Split carries, which this type does not repeat --'
command grep '^impl< T' ring_handle/src/lib.rs | head -1 | sed 's/^/    ring_handle:/'
```

Live output:

```
  -- everything the struct derives, and the impl written by hand instead --
    #[ derive( Debug ) ]
    pub struct Registry< T >
    {
      rings : HashMap< String, Split< T > >,
    }
    impl< T > Default for Registry< T >
    {
      fn default() -> Self
      {
        Self::new()
      }
    }
  -- crates in the family that pin Send or Sync with a static assertion --
    ring_flush/tests/flush_test.rs
    ring_handle/tests/handle_test.rs
    ring_mpsc/src/lib.rs
    ring_mpsc/tests/mpsc_test.rs
    ring_spsc/src/lib.rs
    ring_spsc/tests/spsc_test.rs
  -- and whether this crate is one of them --
    assert_send or assert_sync anywhere in ring_registry: 0
    Send or Sync named in ring_registry src or readme: 0
  -- the bound Split carries, which this type does not repeat --
    ring_handle:impl< T : Send > Split< T >
```

## What `#[ derive( Default ) ]` Would Have Written

The derived form, against a `T` with no `Default` of its own, compiled with
`--emit=metadata`:

```rust
// compile/-derived_default.rs
#[ derive( Debug, Default ) ]
pub struct Derived< T >
{
  rings : HashMap< String, T >,
}

// A T with no Default at all.
pub struct NoDefault;

// Does the derived bound reject it?
pub fn f() -> Derived< NoDefault > { Derived::default() }
```

```
error[E0277]: the trait bound `NoDefault: Default` is not satisfied
  --> compile/-derived_default.rs:14:38
   |
14 | pub fn f() -> Derived< NoDefault > { Derived::default() }
   |                                      ^^^^^^^ the trait `Default` is not implemented for `NoDefault`
   |
help: the trait `Default` is implemented for `Derived<T>`
note: required for `Derived<NoDefault>` to implement `Default`
   |
 4 | #[ derive( Debug, Default ) ]
   |                   ------- in this derive macro expansion
 5 | pub struct Derived< T >
   |            ^^^^^^^  - type parameter would need to implement `Default`
```

## Whether the Type Is `Send` and `Sync`

```rust
// compile/-auto_traits.rs
const fn assert_send< T : Send >() {}
const fn assert_sync< T : Sync >() {}

pub const fn f()
{
  assert_send::< Registry< u32 > >();
  assert_sync::< Registry< u32 > >();
  assert_send::< Split< u32 > >();
  assert_sync::< Split< u32 > >();
}
```

```
=== -auto_traits ===
(no diagnostics)
```

---

### RG47 — The Hand-Written `Default` Is Not Boilerplate, and Reads Exactly Like It

`impl< T > Default for Registry< T > { fn default() -> Self { Self::new() } }` is
seven lines that a reviewer skims. It is also the only correct way to write it.
`#[ derive( Default ) ]` on a generic struct adds `T : Default` to the generated
impl regardless of whether the fields need it — the field here is a `HashMap`,
whose own `Default` needs nothing from `T` — so the derived form rejects
`Registry< NoDefault >::default()` with `error[E0277]`, and the compiler names the
cause outright: "type parameter would need to implement `Default`". The hand
impl, unbounded, accepts every `T`.

The distinction matters here more than it usually would. A registry's record type
is chosen by the consumer, and the family's record types are ring payloads —
`ring_types`' identifiers, a consumer's event struct — none of which has any
reason to be `Default`. A derived `Default` would have made `Registry::default()`
unusable for most real `T` while continuing to compile for `u32`, which is what
every test and every doctest in the crate uses. It would have shipped.

Nothing records this. The crate that spends twenty lines justifying a lint
suppression gives its one non-obvious impl no comment at all, and a later
maintainer tidying "redundant" boilerplate into a derive would break consumers
without breaking a single test in this crate.

**Finding.** Recorded as an unmarked deliberate choice, the same shape as
[RG49](../workaround/001_a_key_bought_back_from_the_map.md)'s clone: a line that
looks avoidable and is not. One comment — that the derive would add a spurious
`T : Default` bound — makes it verifiable in place. Worth pairing with a
`Registry< NonDefaultRecord >::default()` line in the tests, which is the version
a compiler enforces.

---

### RG48 — The Type Is `Send` and `Sync`, the Family Pins That in Four Crates, and This One Never Mentions It

`Registry< u32 >` is both `Send` and `Sync`; so is the `Split< u32 >` it holds.
Nothing about that is accidental — a registry is built on one thread and its
rings are worked by others, and `ring_handle` states the requirement in its own
signatures, `impl< T : Send > Split< T >`.

Four crates in the family pin these properties with static assertions, in six
files: `ring_mpsc` and `ring_spsc` in both `src` and `tests`, `ring_flush` and
`ring_handle` in tests. `ring_handle` is this crate's only dependency and the
owner of the type `Registry` exists to hold, so the practice is not merely
nearby — it is one edge away, on the type in the field.

`ring_registry` has zero assertions and names `Send` or `Sync` zero times, in the
source and the readme together. The properties hold today by inference through
`HashMap< String, Split< T > >`, which means they hold until any field or any
future auxiliary type stops being `Send` — an interior-mutability cache, a
`Rc`-based name interner, an `RefCell` of statistics — none of which would fail a
single existing test, because every test builds a registry and uses it on the
thread that built it.

**Finding.** Recorded as an unpinned load-bearing property with an adopted family
precedent available. Two lines in `tests/registry_test.rs` —
`const fn assert_send< T : Send >() {}` and `assert_send::< Registry< u32 > >();`,
matching what `ring_handle` already does — turn an inference into a compile-time
guarantee. Worth noting the interaction with
[RG12](../data_structure/002_the_only_map_in_thirty_three_crates.md): adding
`T : Send` to the inherent impl, as that finding proposes, would also bound this
type's `Default`, since `default()` calls `Self::new()`. The two changes are not
independent, and doing the assertion first makes the bound question decidable on
evidence rather than on taste.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_registry_error.md) | The other type, and its four derives |
| [`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md) | The unbounded inherent impl this `Default` shares |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | What the one field costs |
| [`item/002`](../item/002_eight_declarations_and_four_must_use.md) | The eight declarations these two impls sit beside |
| [`integration/002`](../integration/002_what_ring_handle_requires_and_this_crate_does_not.md) | The `Send` bound tracked across the edge |

### Sources

| Fact | Where |
|------|-------|
| The derive and the hand-written `Default` | `ring_registry/src/lib.rs:87-99` |
| The derived form's spurious bound | Compile probe above |
| `Registry< u32 >` is `Send` and `Sync` | Compile probe above |
| Six files in four crates asserting it | Census above |
| Zero assertions, zero mentions here | Census above |
| `Split`'s own `Send` bound | `ring_handle/src/lib.rs:69` |

### Tests

| Test | Covers |
|------|--------|
| `an_empty_registry_is_empty` | The state `Default` produces |
| `two_names_hold_two_distinct_rings` | The `Split` values whose auto traits carry |
| `dropping_the_registry_drops_every_record_still_in_every_ring` | The ownership the type is built around |
