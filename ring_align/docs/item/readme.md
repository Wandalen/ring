# Item Doc Definition

### Scope

- **Purpose**: Give each exported item its own declaration-level reference — the signature as written, the attributes on it, its callers, and the checks that touch it — so a change to one item has a single place to be checked against.
- **Responsibility**: Per item: exact declaration, every attribute and derive, the measured caller tree, and the test that would fail.
- **In Scope**: `CacheAligned<T>` with its four associated functions; `on_distinct_lines`.
- **Out of Scope**: Design reasoning, which is [`type/`](../type/readme.md) and [`decisions/`](../decisions/readme.md); the surface read as a whole, which is [`api/`](../api/readme.md).

**On the overlap with `type/`.** These two definitions are close, and
[`ring_flush`'s own `docs/readme.md`](../../../ring_flush/docs/readme.md) argues
against carrying both. The split held here is: `type/` answers *why this type
exists and what contract it makes*; `item/` answers *what exactly is declared,
where it is used, and what breaks if it changes*. `CACHE_LINE` has no `item/`
entry because it is a constant with no declaration detail beyond its value —
[`type/001`](../type/001_cache_line.md) covers it completely, and a second file
would restate it.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`CacheAligned` and Its Associated Functions](001_cache_aligned_and_its_associated_functions.md) | The declaration, six derives, four associated functions, and which of them are `const` | 🔄 |
| 002 | [`on_distinct_lines`](002_on_distinct_lines.md) | One `const fn`, its `#[ must_use ]`, and a caller tree measured across all 33 crates | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- CacheAligned as declared, with every attribute --'
command grep -E '^#\[ derive|^#\[ repr|^pub struct CacheAligned' ring_align/src/lib.rs
echo '  -- on_distinct_lines as declared --'
command grep -B1 '^pub const fn on_distinct_lines' ring_align/src/lib.rs
echo '  -- crates naming each exported item, comments stripped --'
for n in CACHE_LINE CacheAligned on_distinct_lines; do
  printf '    %-18s' "$n"
  for c in ring_*/; do
    cat "$c"/src/*.rs "$c"/tests/*.rs 2>/dev/null | sed 's://.*::' | command grep -q "\b$n\b" && printf ' %s' "$( basename "$c" )"
  done
  echo
done
```

Live output:

```
  -- CacheAligned as declared, with every attribute --
#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
  -- on_distinct_lines as declared --
#[ must_use ]
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
  -- crates naming each exported item, comments stripped --
    CACHE_LINE         ring_align ring_cursor
    CacheAligned       ring_align ring_cursor
    on_distinct_lines  ring_align ring_cursor ring_mpsc ring_spsc
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL25 | `CacheAligned< T >` | n/a — observation | Six derives and one `repr`, and the `repr` is the only one that does anything the type could not live without — `Debug`, `Clone`, `Copy`, `Default`, `PartialEq` and `Eq` all forward to the payload, so every trait the type has is the payload's and the only thing it contributes is layout |
| AL26 | `Copy` | n/a — observation | Deriving `Copy` on a 64-byte type means a move copies a full cache line for an eight-byte payload; it is the right choice for a cursor read that must not borrow, and it is the one derive with a cost rather than a forwarding |
| AL27 | The caller tree | n/a — observation | Both `CACHE_LINE` and `CacheAligned` are named by exactly two crates — this one and `ring_cursor` — so the entire family-facing use of this crate passes through a single consumer, and the third name's four-crate spread is the spelling collision recorded at [`integration`](../integration/readme.md) AL24 rather than four users |
| AL28 | `#[ must_use ]` | n/a — observation | Present on the one item where ignoring the result is merely useless, absent on `into_inner` where ignoring it destroys the value — the annotation is on the predicate because a predicate is the obvious case, not because the cases were compared |
