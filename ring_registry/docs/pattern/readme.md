# pattern

Seven public signatures in this family return the caller's own value on failure
rather than destroying it. They agree on the contract completely — nothing is
consumed by a refusal — and disagree on how to say it three ways: a bare payload
at five sites, a named enum once in `ring_shutdown`, and an error-payload tuple
once, here. The split tracks how many failure reasons each method has, which is
the right axis; what is unexplained is that the two methods with more than one
thing to say picked different shapes, and only one of them looked at the family
first.

The tuple's cost is concrete and the crate does not name it. It can carry no
methods, because `( RegistryError, Split< T > )` admits no inherent impl here, so
a caller cannot inspect the error and keep the ring. And the `Error` impl the
crate does write is justified by a use that does not compile — a `?` through
`register`, which is also the first line of code the readme shows a reader.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_an_error_that_hands_the_payload_back.md) | An Error That Hands the Payload Back | The seven signatures, the three spellings, and what the named form carries |
| [002](002_an_error_impl_the_fallible_method_cannot_reach.md) | An `Error` Impl the Fallible Method Cannot Reach | The `?` that does not compile, and the readme line nothing checks |

## Three Spellings of One Contract

The bare `Result< (), T >` fits a method with exactly one way to fail — the error
*is* the value, so no reason needs carrying. `ring_shutdown` has two ways to fail
and wraps the record in a named `Refusal< T >`, writing its derivation out: the
enum exists "because the record must come back intact in both — `ring_core`'s own
refusal contract, extended by one case." This crate has two things to say as
well, and reaches for a tuple; its source and readme name `Refusal` and
`ring_shutdown` zero times between them.

`Refusal< T >` is what the tuple would be if it were finished — `into_record`,
`is_closed`, a conversion to the family error, and five derives. The tuple has
none, unavoidably. One of the four gaps is not cosmetic: matching
`Err( ( error, ring ) )` moves both out, and borrowing gives a reference from
which the ring cannot be taken, so a caller that wants to log the name and then
retry under a different one must destructure and rebuild the message from the
pieces.

## A Justification and an Example, Neither Compiled

`impl core::error::Error for RegistryError {}` at `src/lib.rs:80` is justified by
"`register( .. )?` in a caller returning `Box< dyn Error >`", and that form is
`error[E0277]`: `register`'s error type is a tuple, one of whose members is a
ring, so no `From` impl exists to convert it. The impl is right and the reason
recorded beside it names the one thing it cannot do. The two facts sit fourteen
source lines apart and nothing connects them.

The same form is `readme.md:28` — the crate's opening example, the first code a
reader meets. No `lib.rs` in the family includes its readme as documentation, so
no readme example anywhere is a doctest. The module doc's own example does the
same operation with `.unwrap()`, which is precisely the difference that keeps the
compiled one passing.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order; -e is required because
# the pattern begins with a hyphen
echo '  -- the three spellings of one contract --'
printf '    bare payload: %s sites   named enum: %s   error-payload tuple: %s\n' \
  "$( command grep -rc -e '-> *Result< (), T >' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc -e '-> *Result< (), Refusal' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc -e '-> *Result< (), ( ' --include=lib.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- what the named form carries that the tuple cannot --'
command grep -n -e 'pub enum Refusal' -e 'pub fn into_record' -e 'pub const fn is_closed' \
  ring_shutdown/src/lib.rs | sed 's/^/    /'
echo '  -- the Error impl, and the signature that defeats its documented use --'
command grep -n 'impl core::error::Error' ring_registry/src/lib.rs | sed 's/^/    /'
command grep -m1 -F '  -> Result< (), ( RegistryError, Split< T > ) >' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- the readme example a reader meets first, and what compiles it --'
command grep -n 'register(' ring_registry/readme.md | cut -c1-88 | sed 's/^/    /'
printf '    lib.rs files in the family including their readme as documentation: %s\n' \
  "$( command grep -rc 'include_str!' --include=lib.rs ring_*/src/ | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG37 | `ring_registry` | n/a — inconsistency | **Seven public signatures in the family return the caller's own value on failure rather than destroying it**, agreeing on the contract completely and disagreeing on how to say it three ways — bare `Result< (), T >` at five sites in four crates, where the error *is* the value and there is exactly one way to fail; a named `Result< (), Refusal< T > >` once, in `ring_shutdown`, where there are two; and the error-payload tuple once, here — a split that tracks how many failure reasons each method has, which is exactly the right axis, while what is arbitrary is that the two methods with more than one thing to say picked different shapes and only one looked at the family first: `ring_shutdown:325` writes its derivation out, its enum existing "because the record must come back intact in both — `ring_core`'s own refusal contract, extended by one case", and this crate's source and readme name `Refusal` and `ring_shutdown` **zero** times between them; the tuple is defensible since `RegistryError` has one variant today and an enum wrapping a single case would be ceremony — what is missing is the sentence saying so, in a crate that writes twenty lines about the same signature's width |
| RG38 | `ring_registry` | n/a — observation | `Refusal< T >` is what the tuple would be if it were finished: `into_record( self ) -> T` so a caller gets the payload out by name, `is_closed( &self ) -> bool` marked `#[ must_use ]` and `const` so the arms are distinguishable without matching, a conversion to the family's `RingError` for callers that report rather than retry, and `Debug, Clone, Copy, PartialEq, Eq` on the type itself — and the tuple has none, the census finding zero impls on it, unavoidably, since `( RegistryError, Split< T > )` admits no inherent impl here, so recovery is positional; one of the four gaps is not cosmetic, because matching `Err( ( error, ring ) )` moves both out while borrowing the `Err` gives `&( RegistryError, Split< T > )` from which the ring cannot be taken, so a caller that wants to log the name and then retry under a different name must destructure first and reconstruct the message from the pieces, or clone the error — and `RegistryError` does derive `Clone` justified in the derive table because "an error a caller wants to keep alongside a log line should not have to be re-created", which is this exact situation described without the tuple being named as its cause; the minimal repair is an inherent `into_ring` on a named wrapper, the cheaper one a sentence in `register`'s doc saying inspecting one costs the other |
| RG39 | `ring_registry` | **wrong doc** | `type/001:60` justifies `impl core::error::Error for RegistryError` with a single use case — "`register( .. )?` in a caller returning `Box< dyn Error >`" — and that form is `error[E0277]`, because `register` returns `Result< (), ( RegistryError, Split< T > ) >`, `?` converts through `From`, and there is no `From< ( RegistryError, Split< T > ) >` for `Box< dyn Error >` since the tuple is not an `Error`, one of its members being a ring; the impl is not useless — it is correct, it is what a caller wants once they have destructured the tuple, and it is what makes `RegistryError` reportable at all — what is wrong is the reason recorded beside it, in a table whose whole purpose is to justify four derives one at a time, so a reader checking whether each earns its place gets a false confirmation for this one; the two facts sit fourteen source lines apart, `:80` and `:150`, with nothing between them connecting them and no test either, since the only exercise of the impl hand-constructs `Err( Box::new( RegistryError::NameTaken { .. } ) )` at `registry_test.rs:355-359` and therefore cannot fail if `register`'s signature changes — the repair is one table cell, best paired with a test that goes through `register` rather than around it |
| RG40 | `ring_registry` | **wrong doc** | `readme.md:28` is the crate's opening example, four lines under "What it does", and its second line is `registry.register( "events", Split::new( ring ) )?;` — the exact operator RG39 shows fails, and the first code a reader sees; it is also not compiled, since the crate has two doc-comment example fences in `lib.rs` and zero `include_str!` of the readme, and across all thirty-three crates **zero `lib.rs` files pull their readme in as documentation**, so no readme example in the family is a doctest — the `?`-on-register form appears once in the crate and zero times in a file the compiler reads, while the module doc's own example at `src/lib.rs:21` does the same operation with `.unwrap()`, differing precisely in the way that keeps the compiled one passing; the elided imports are a normal readme convention and not the finding, which is that the one line a reader is most likely to copy is the one line the crate has no mechanism to check, wrong in a way that costs an `E0277` on a `From` impl pointing at a tuple the reader did not write — two independent repairs suffice, matching the module doc's `.unwrap()`, or `#![ doc = include_str!( "../readme.md" ) ]`, the second being the better and larger fix since it is a family-wide convention nobody has adopted |
