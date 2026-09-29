# type

`ring_index` declares no type. Its entire vocabulary is one `use` line importing
three names from `ring_types`, and its entire contribution to the type system is
being the single function that converts between two of them. This definition
records what those three types are, which of them actually protects anything, and
what the protection is made of in each case.

The organizing observation is that the three split unevenly. Two are transparent
newtypes with public fields, carrying no invariant beyond their own name; one is
opaque with a private field and a fallible constructor, carrying the invariant
this whole crate rests on. Every property `ring_index` claims for itself —
totality, correctness of the fold, the range of its output — is downstream of
which side of that line a given type sits on.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_three_types_borrowed_none_owned.md) | Three Types Borrowed, None Owned | The vocabulary, the transparent/opaque split, and what `Capacity`'s privacy buys |
| [002](002_the_sentence_the_public_field_contradicts.md) | The Sentence the Public Field Contradicts | `SlotIndex`'s "never constructed by counting", and the 36 constructions that count |

## Two Kinds of Enforcement, Documented in One Register

`Capacity`'s invariant is enforced by privacy: there is no way to build one
except `Capacity::new`, so a violation is a compile error and `mask()`'s
unchecked `self.0 - 1` is safe.

`SlotIndex`'s invariant — "always in `0..capacity`", stated in the first line of
its doc — is enforced by convention: the field is `pub`, anyone may forge one,
and what actually holds the line is that exactly one line of non-test code in the
family constructs one. Both invariants are stated in the same declarative
register, and only one of them is a fact about the program.

## The Transfer, Not the Division

The module comment says validation "lives upstream in `ring_types::Capacity`, not
here." Read against the declarations, that is a transfer rather than a division
of labour: the check happens once, at one constructor, in a crate `ring_index`
depends on, and every function downstream inherits totality as a consequence.
`ring_index` is the first beneficiary and contributes nothing to the guarantee it
relies on most.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo "  types declared here: $( command grep -cE '^\s*(pub )?(struct|enum) ' ring_index/src/lib.rs )"
command grep -n '^use ' ring_index/src/lib.rs
echo "  SlotIndex constructions family-wide, excluding doctests and ring_types/src: $( command grep -rn 'SlotIndex( ' --include=*.rs /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep -v 'ring_types/src' | command grep -vc ':[0-9]*: *///' )"
echo "  of which in a src/ file: $( command grep -rn 'SlotIndex( ' --include=*.rs /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep -v 'ring_types/src' | command grep -v ':[0-9]*: *///' | command grep -c '/src/' )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX37 | `ring_types` | n/a — observation | Three borrowed types split two-transparent/one-opaque along the invariant boundary; `Capacity` alone withholds its field and alone declines `Default` |
| IX38 | `ring_types` | n/a — observation | `Capacity::mask` is an unchecked `self.0 - 1`; privacy is the only guard, and it is sufficient — zero constructions outside `new` family-wide |
| IX39 | `ring_types` | n/a — unenforced | `SlotIndex`'s "never constructed by counting" is contradicted by its own doctest and its `pub` field, yet 35 of 36 constructions are tests and the 36th is `of` |
| IX40 | `ring_index` | n/a — observation | One line converts between sequence space and slot space by unwrapping one public field and wrapping another; the newtypes' protection comes from that line being unique, not from their fields |
