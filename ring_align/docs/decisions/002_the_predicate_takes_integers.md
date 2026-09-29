# Decision: The Predicate Takes Integers

### Scope

- **Purpose**: Record why `on_distinct_lines` accepts two `usize` addresses rather than two references, and why the convenience alternative is not merely unchosen but unavailable in the form it would want.
- **Responsibility**: State the alternatives, the compiler constraint that decides between them, the ceremony the choice costs every caller, and what would reopen it.
- **In Scope**: The signature of `on_distinct_lines`.
- **Out of Scope**: What the function computes, which is [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md); who calls it, which is [`api/001`](../api/001_the_reading_surface.md).

### The Decision

```rust
#[ must_use ]
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
```

The crate's doc comment gives the reason as an argument about *meaning*
(`ring_align/src/lib.rs:126-128`):

> Takes addresses as plain integers rather than references, because the
> question is about where two *fields* sit — the caller has the addresses
> already, and two stack locals in a doc example would say nothing.

That argument is real and it is not the binding one. The binding one is that
the reference-taking form **cannot be `const fn`**, and the reason it cannot is
not a temporary gap in the standard library.

### Alternatives

| # | Alternative | Why it lost |
|---|-------------|-------------|
| F1 | `fn on_distinct_lines< A, B >( a : &A, b : &B ) -> bool` | Cannot be `const fn` — see below. Also cannot express an address the caller obtained any other way: from a raw pointer, from an offset, from an `AtomicUsize` load |
| F2 | A method on `CacheAligned`: `a.on_distinct_line_from( &b )` | Reads best of all at the call site, and constrains the check to *this crate's own wrapper* — which is backwards. The predicate's job is to let a caller check an arrangement the crate never sees, including one built without `CacheAligned` at all (which is what the negative-control test needs, → [`pitfall/002`](../pitfall/002_size_of_proves_nothing_about_addresses.md)) |
| F3 | A macro taking two field expressions | Removes the ceremony and adds a macro to a crate whose entire surface is one constant, one struct, and one function. The cost is not worth the saving, and macros do not compose into the `assert!` the callers actually write |
| F4 | **Two `usize` addresses** | Chosen |

### Why F1 Cannot Be `const`

Both routes from a reference to an address are rejected in a `const fn`.
Verified on `rustc 1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cat > ./-probe.rs <<'EOF'
pub const CACHE_LINE : usize = 64;
pub const fn on_distinct_lines_ref< A, B >( a : &A, b : &B ) -> bool
{
  let pa = core::ptr::from_ref( a ).addr();
  let pb = core::ptr::from_ref( b ).addr();
  pa / CACHE_LINE != pb / CACHE_LINE
}
fn main() {}
EOF
rustc --crate-name probe --edition 2021 -o /dev/null ./-probe.rs
# error[E0015]: cannot call non-const method `<impl *const A>::addr` in constant functions
rm -f ./-probe.rs
```

Live output:

```
error[E0015]: cannot call non-const method `std::ptr::const_ptr::<impl *const A>::addr` in constant functions
 --> ./-probe.rs:4:37
  |
4 |   let pa = core::ptr::from_ref( a ).addr();
  |                                     ^^^^^^
  |
  = note: calls in constant functions are limited to constant functions, tuple structs and tuple variants

error[E0015]: cannot call non-const method `std::ptr::const_ptr::<impl *const B>::addr` in constant functions
 --> ./-probe.rs:5:37
  |
5 |   let pb = core::ptr::from_ref( b ).addr();
  |                                     ^^^^^^
  |
  = note: calls in constant functions are limited to constant functions, tuple structs and tuple variants

error: aborting due to 2 previous errors

For more information about this error, try `rustc --explain E0015`.
```

Substituting the older `a as *const A as usize` cast does not help, and its
error states the reason the restriction is permanent rather than pending:

```
error: pointers cannot be cast to integers during const eval
  = note: at compile-time, pointers do not have an integer value
```

**A reference has no address until run time, so the question F1 asks is not
answerable in the context `const fn` promises to work in.** `addr()` becoming
const-stable would not change this; the const-eval restriction is about
pointers not *having* integer values during evaluation, not about which method
name is blessed. F1 is unavailable in principle, not by omission.

The integer form sidesteps it entirely: by the time a caller has a `usize`, the
compile-time/run-time question is already settled on the caller's side, and the
function is left doing arithmetic that is const-evaluable for any input.

### What the Decision Costs

The caller writes the address extraction itself
(`ring_align/tests/align_test.rs:65-68`):

```rust
let a = core::ptr::from_ref( &pair.producer ) as usize;
let b = core::ptr::from_ref( &pair.consumer ) as usize;
assert!( on_distinct_lines( a, b ) );
```

Three lines where F2 would have given one — but measured across the workspace
the ceremony appears in exactly **two** places, not in every caller:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'from_ref' --include=*.rs ring_align/ ring_cursor/
```

Live output:

```
ring_align/tests/align_test.rs:  let a = core::ptr::from_ref( &pair.producer ) as usize;
ring_align/tests/align_test.rs:  let b = core::ptr::from_ref( &pair.consumer ) as usize;
ring_align/tests/align_test.rs:  let a = core::ptr::from_ref( &pair.producer ) as usize;
ring_align/tests/align_test.rs:  let b = core::ptr::from_ref( &pair.consumer ) as usize;
ring_cursor/src/lib.rs:    core::ptr::from_ref( self ) as usize
```

Both `ring_align`'s own two tests and one accessor,
`PaddedCursor::addr` (`ring_cursor/src/lib.rs:186-189`), which is a
single `core::ptr::from_ref( self ) as usize` written once. `ring_spsc` and
`ring_mpsc` never write it: they call methods that already have it.

**That absorption is why F3's macro was a defensible loss rather than a wrong
one** — the repetition the macro would have removed never accumulated, because
the natural place to put the cast turned out to be an accessor on the type that
has the address, not a macro at the assertion site. `ring_cursor`'s own doc
comment reaches the same conclusion from the consumer's side, independently:

> Returned as a plain integer because the answer is arithmetic on line numbers,
> not anything a caller should dereference.

There is a second cost, and it is the one to watch: `usize` has no type-level
meaning, so nothing stops a caller passing a length, an index, or an offset.
The function will happily answer. `#[ must_use ]` catches the result being
discarded; nothing catches the arguments being wrong, and no test can, because
any two integers are a valid input.

### Reopening Conditions

| # | Condition | Response |
|---|-----------|----------|
| G1 | `const_fn` gains a way to reason about addresses | It will not — the const-eval note above is a statement about semantics. Treat any claim that it has as requiring the probe above to be re-run |
| G2 | A caller passes a non-address `usize` and gets a meaningless pass | Reopens the newtype question: an `Addr( usize )` wrapper constructible only from a pointer. Costs a type in a crate deliberately holding three items; buys the only enforcement available |
| G3 | The cast appears in a third and fourth place, un-absorbed by any accessor | Reopens F3. The threshold is repetition that resists encapsulation, not principle — and the two-site measurement above says that threshold has not been approached |

### AL15 — The Ceremony Was Bought With a Mistake, Not With a Cost Estimate

`( usize, usize )` rather than `( &A, &B )` costs every caller an explicit
`core::ptr::from_ref( … ) as usize`, and buys the elimination of a doc example
written over two stack locals — an example that asserts something false about
the machine while looking entirely reasonable.

**Finding.** The trade was made because that example was written once and was
wrong, not because the per-call ceremony was judged cheap. That is a stronger
justification than a cost estimate, and it is one a later reader cannot
reconstruct from the signature: what they will see is a predicate that made
itself harder to call for no visible reason.

---

### AL16 — Both Decisions Reopen on Conditions Nothing Watches

The first reopens on a target whose line is not 64. The second reopens on a
signature form that can express the stack-locals mistake — that is, on the
language gaining something it does not have.

**Finding.** Each trigger is about the world rather than about this crate, and
no gate, test, or task in the repository watches for either. A recorded decision
whose reopening condition nobody monitors is a decision that will be found again
by whoever hits the condition, which is the same position as having recorded
nothing — except that the reasoning is here when they look. The same shape
appears in [`workaround/readme.md`](../workaround/readme.md) AL49.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | The arithmetic the signature makes const-evaluable, and why the parameter type is the interesting half of the design |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_reading_surface.md](../api/001_the_reading_surface.md) | The four crates carrying the name, and the two places that actually pay the ceremony |

### Decisions

| File | Relationship |
|------|--------------|
| [001_the_constant_is_not_conditional.md](001_the_constant_is_not_conditional.md) | The other choice with a live alternative; both are decided by what the compiler will accept rather than by taste |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | The negative control F2 would have made inexpressible |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:126-128` | The doc comment's stated reason — correct, and not the binding constraint |

### Tests

| File | Relationship |
|------|--------------|
| `src/lib.rs` doctest | Three integer literal cases, which is the form the signature makes possible — `on_distinct_lines( 63, 64 )` needs no allocation, no struct, and no live values |
| `tests/align_test.rs` | The ceremony in its real form, over real fields |
