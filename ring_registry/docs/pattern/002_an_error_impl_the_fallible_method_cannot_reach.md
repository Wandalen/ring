# Pattern: An Error Impl the Fallible Method Cannot Reach

### Scope

**Purpose:** Test the reason the crate gives for `impl core::error::Error for
RegistryError` — that it lets a caller write `register( .. )?` in a function
returning `Box< dyn Error >` — against a compiler, and record what the readme's
first example does with the operator that does not work.

**Responsibility:** The `Error` impl and its stated justification; the type
`register` actually returns; whether `?` converts it; where the impl is exercised;
and whether the readme's example is compiled by anything.

**In Scope:** `ring_registry/src/lib.rs:80`, `:163`;
`ring_registry/docs/type/001_registry_error.md:60`;
`ring_registry/readme.md:28`;
`ring_registry/tests/registry_test.rs:363-367`.

**Out of Scope:** Why the return type is a tuple in the first place is
[`pattern/001`](001_an_error_that_hands_the_payload_back.md). The width it costs
is [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md).

---

## The Impl, Its Reason, and What `register` Returns

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the impl, and the reason the crate gives for it --'
command grep 'impl core::error::Error' ring_registry/src/lib.rs | sed 's/^/    /'
command grep 'core::error::Error. | Reports .error. via' ring_registry/docs/type/001_registry_error.md | sed 's/^/    type\/001:/'
echo '  -- what register actually returns --'
command grep -m1 -F '  -> Result< (), ( RegistryError, Split< T > ) >' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- the readme example, now matching the module docs own .unwrap() form --'
command grep 'registry.register(.*).unwrap();' ring_registry/readme.md | sed 's/^/    readme.md:/'
echo '  -- which of this crate examples the test run compiles --'
printf '    doc-comment example fences in lib.rs: %s   include_str! of readme.md: %s\n' \
  "$( command grep -c '^//! ```$' ring_registry/src/lib.rs || true )" \
  "$( command grep -c 'include_str' ring_registry/src/lib.rs || true )"
printf '    of 33 crates, lib.rs files pulling readme.md in as doc: %s\n' \
  "$( command grep -rl 'include_str!( *"\.\./readme.md" *)' --include=lib.rs ring_*/src/ 2>/dev/null | wc -l )"
```

Live output:

```
  -- the impl, and the reason the crate gives for it --
    impl core::error::Error for RegistryError {}
    type/001:| `core::error::Error` | Reports `error` via `Box< dyn Error >` (or `?`) once a caller has destructured `Err( ( error, ring ) )` |
  -- what register actually returns --
      -> Result< (), ( RegistryError, Split< T > ) >
  -- the readme example, now matching the module docs own .unwrap() form --
    readme.md:registry.register( "events", Split::new( ring ) ).unwrap();
  -- which of this crate examples the test run compiles --
    doc-comment example fences in lib.rs: 2   include_str! of readme.md: 0
    of 33 crates, lib.rs files pulling readme.md in as doc: 0
```

## The Justification, Compiled

The exact form the derive table names, as a one-function library crate against
the real `ring_registry`, with `--emit=metadata`:

```rust
// compile/-error_via_question_mark.rs
use ring_registry::Registry;
use ring_handle::Split;
use std::error::Error;

// `RegistryError` implements Error. `register` does not return one.
pub fn f( r : &mut Registry< u32 >, s : Split< u32 > ) -> Result< (), Box< dyn Error > >
{
  r.register( "a", s )?;
  Ok( () )
}
```

```
error[E0277]: `?` couldn't convert the error: `(RegistryError, ring_handle::Split<u32>): std::error::Error` is not satisfied
 --> compile/-error_via_question_mark.rs:8:23
  |
6 | pub fn f( r : &mut Registry< u32 >, s : Split< u32 > ) -> Result< (), Box< dyn Error > >
  |                                                           ------------------------------ required `(RegistryError, ring_handle::Split<u32>): std::error::Error` because of this
7 | {
8 |   r.register( "a", s )?;
  |     ------------------^ the trait `std::error::Error` is not implemented for `(RegistryError, ring_handle::Split<u32>)`
  |     |
  |     this has type `Result<_, (RegistryError, ring_handle::Split<u32>)>`
  |
  = note: the question mark operation (`?`) implicitly performs a conversion on the error value using the `From` trait
  = note: required for `Box<dyn std::error::Error>` to implement `From<(RegistryError, ring_handle::Split<u32>)>`

error: aborting due to 1 previous error
```

## Where the Impl Is Reached Instead

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the only place the Error impl is exercised --'
command grep 'Box< dyn core::error::Error >\|Err( Box::new' ring_registry/tests/registry_test.rs | sed 's/^/    /'
echo '  -- what that test asserts --'
# matched on the assertion message, not a line number: the suite grows above this
# test, and a fixed offset then prints whichever line has slid into its place
command grep 'does not satisfy Error' ring_registry/tests/registry_test.rs | sed 's/^/    /'
echo '  -- and how often the crate uses the operator it justified the impl with --'
printf '    `?` on a register call, in src, tests and readme: %s\n' \
  "$( command grep -rc 'register(.*)?;' ring_registry/src ring_registry/tests ring_registry/readme.md 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
printf '    of those, inside a compiled file: %s\n' \
  "$( command grep -rc 'register(.*)?;' ring_registry/src ring_registry/tests 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

Live output:

```
  -- the only place the Error impl is exercised --
      fn caller() -> Result< (), Box< dyn core::error::Error > >
        Err( Box::new( RegistryError::NameTaken { name : "x".to_string() } ) )
  -- what that test asserts --
      assert!( caller().is_err(), "RegistryError does not satisfy Error" );
  -- and how often the crate uses the operator it justified the impl with --
    `?` on a register call, in src, tests and readme: 0
    of those, inside a compiled file: 0
```

---

### RG39 — The Reason Given for the `Error` Impl Does Not Compile

`type/001:60` justifies `impl core::error::Error for RegistryError` with a single
use case: "`register( .. )?` in a caller returning `Box< dyn Error >`". That form
is `error[E0277]`. `register` returns `Result< (), ( RegistryError, Split< T > )
>`, and `?` converts the error type through `From`; there is no
`From< ( RegistryError, Split< T > ) >` for `Box< dyn Error >`, because the tuple
is not an `Error` — one of its two members is a ring.

The impl is not useless. It is correct, it is what a caller wants once they have
destructured the tuple, and it is what makes `RegistryError` reportable at all.
What is wrong is the reason recorded beside it, which names the one thing the
impl cannot do — and names it in a table whose whole purpose is to justify four
derives one at a time, so a reader checking whether each derive earns its place
gets a confirmation for this one that is false.

The two facts sit fourteen source lines apart. `impl core::error::Error for
RegistryError {}` is `src/lib.rs:80`; the signature that defeats it is `:150`.
Nothing in between connects them, and the test suite does not either: the only
place the impl is exercised is `registry_test.rs:363-367`, which hand-constructs
`Err( Box::new( RegistryError::NameTaken { .. } ) )` and asserts the result is an
error. That proves the impl exists. It cannot fail if `register`'s signature
changes, and it did not notice that the signature already makes the documented
use unreachable.

**Finding.** Recorded as a wrong justification for a right decision. The repair is
one table cell: the impl is there so a caller who has destructured
`Err( ( error, ring ) )` can report `error` through `Box< dyn Error >` or `?` it
onward. Worth pairing with a test that goes through `register` rather than around
it, since the current one asserts a property of `RegistryError` in isolation and
therefore cannot see the gap at all — the gap is entirely in the relationship
between the two, which is the one thing nothing checks.

**Disposition:** applied — `docs/type/001_registry_error.md:60`'s Derives row now
reads "Reports `error` via `Box< dyn Error >` (or `?`) once a caller has
destructured `Err( ( error, ring ) )`", replacing the non-compiling
`register( .. )?` justification with the use the impl actually serves. Now
prints: `once a caller has destructured`

---

### RG40 — The First Example a Reader Meets Is the Form That Does Not Work, and Nothing Compiles It

The readme's opening example, four lines under "What it does", is the crate's
first code a reader sees. Its second line is
`registry.register( "events", Split::new( ring ) )?;` — the exact operator RG39
shows fails.

It is also not compiled. The crate has two doc-comment example fences in
`lib.rs`, and zero `include_str!` of the readme; across all thirty-three crates,
zero `lib.rs` files pull their readme in as documentation, so no readme example
in the family is a doctest. The census makes the split exact: the `?`-on-register
form appears once in the crate, and zero times in a file the compiler reads. The
module doc's own example, at `src/lib.rs:21`, does the same operation with
`.unwrap()` — so the compiled example and the uncompiled one differ in precisely
the way that keeps the compiled one passing.

The example is not otherwise sloppy; `ring` and the imports are elided the way an
illustrative snippet elides them, which is a normal readme convention and not
what this finding is about. What it is about is that the one line a reader is
most likely to copy is the one line the crate has no mechanism to check, and it
happens to be wrong in a way that costs a compile-error the reader will not
understand — E0277 on a `From` impl, pointing at a tuple they did not write.

**Finding.** Recorded as a coverage gap with a live consequence rather than a
style note. Two independent repairs, either sufficient: change the readme line to
match the module doc's `.unwrap()`, or make the readme a doctest with
`#![ doc = include_str!( "../readme.md" ) ]` and let the compiler find it. The
second is the better fix and the larger one, since it is a family-wide convention
nobody has adopted — and adopting it here first would mean this crate's readme is
the only one in thirty-three that cannot drift from its own API.

**Disposition:** applied — the readme's opening example now reads
`registry.register( "events", Split::new( ring ) ).unwrap();`, matching the
module doc's own form at `src/lib.rs:21` instead of the `?` operator that does
not compile against `register`'s actual return type; the larger doctest-conversion
repair is left as the family-wide convention it is, not a single-crate fix. Now
prints: `readme.md:registry.register`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_an_error_that_hands_the_payload_back.md) | Why the return type is a tuple, and what that tuple lacks |
| [`type/001`](../type/001_registry_error.md) | The derive table this finding tests |
| [`api/001`](../api/001_the_registry_surface.md) | The surface the operator would be used against |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | The width the same tuple costs |
| [`item/002`](../item/002_eight_declarations_and_four_must_use.md) | The other attribute-level gap in the same eight declarations |

### Sources

| Fact | Where |
|------|-------|
| The `Error` impl | `ring_registry/src/lib.rs:80` |
| The signature that defeats it | `ring_registry/src/lib.rs:163` |
| The justification given | `ring_registry/docs/type/001_registry_error.md:60` |
| E0277 on the exact documented form | Compile probe above |
| The readme's opening example | `ring_registry/readme.md:28` |
| Zero readme doctests in 33 crates | Census above |
| The only place the impl is exercised | `ring_registry/tests/registry_test.rs:363-367` |

### Tests

| Test | Covers |
|------|--------|
| `the_error_names_the_taken_name` | The `Error` impl, in isolation from `register` |
| `a_second_registration_under_a_live_name_is_refused` | The call the operator would be applied to |
| `a_refused_registration_hands_the_ring_back` | The tuple member that blocks the conversion |
