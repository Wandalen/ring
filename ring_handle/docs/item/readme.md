# item

Seventeen declarations: five nouns and twelve verbs. The catalogue splits on
that line, and both halves report the same measurement from opposite ends —
four of the five nouns are byte-identical to the `ring_core` type they wrap, and
eleven of the twelve verbs are one-line forwards to it.

What is left over after subtracting the forwarding is the entire crate: one
withheld method, one added method, and one struct with state of its own.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Five Nouns, Four of Them the Same Width as What They Wrap](001_five_nouns_four_of_them_the_same_width.md) | HD25, HD26 — the wrapper costs zero bytes, and the one struct that is not a wrapper |
| 002 | [Twelve Verbs, Eight of Them a Bare Forward](002_twelve_verbs_eight_bare_forwards.md) | HD27, HD28 — what the forwarding subtracts and what it adds |

### The Seventeen, by Kind

The seventeen are the five `pub struct` and the twelve inherent `pub fn`. The
`Iterator` impl's `next`/`size_hint` are reachable public behaviour but are not
declarations of this crate's own, so they close the table as one row rather than
counting toward the total.

| Kind | Declaration | Wraps | Notes |
|------|-------------|-------|-------|
| noun | `pub struct Split< T >` | `ring_core::Ring< T >` | 320 bytes, same as the ring |
| noun | `pub struct Ends< 'a, T >` | `ring_core::Ends< 'a, T >` | 256 bytes, same |
| noun | `pub struct Producer< 'a, T >` | `ring_core::Producer< 'a, T >` | 24 bytes, same |
| noun | `pub struct Consumer< 'a, T >` | `ring_core::Consumer< 'a, T >` | 16 bytes, same |
| noun | `pub struct Drain< 'c, 'a, T >` | — | The only struct with a field of its own: `remaining : usize` |
| verb | `Split::new` | — | Takes the ring by value; nothing in `ring_core` returns a `Split` |
| verb | `Split::ends` | `Ring::ends` | Forward |
| verb | `Ends::split` | `Ends::split` | Forward, rewrapping both halves |
| verb | `Producer::try_push` | same | Forward |
| verb | `Producer::try_push_batch` | same | Forward |
| verb | `Producer::free_capacity` | same | Forward |
| verb | `Producer::is_full` | same | Forward |
| verb | `Consumer::try_recv` | same | Forward |
| verb | `Consumer::try_recv_batch` | same | Forward |
| verb | `Consumer::len` | same | Forward |
| verb | `Consumer::is_empty` | same | Forward |
| verb | `Consumer::drain` | — | **Added.** Snapshots `len()` and hands it to `Drain` |
| verb | `Drain::next` / `size_hint` | — | `impl Iterator`, the crate's only branch |

**`ring_core::Producer::try_clone` is the seventeenth declaration that is not
here**, and it is the reason the other sixteen exist
(→ [`pattern/001`](../pattern/001_enforce_by_withholding.md)).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the five nouns --'
command grep -nE '^pub struct ' ring_handle/src/lib.rs
echo '  -- the twelve verbs --'
command grep -cE '^  pub (const )?fn ' ring_handle/src/lib.rs
echo '  -- the impl blocks --'
command grep -nE '^impl' ring_handle/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD25 | the five nouns | n/a — coverage | The zero-cost claim is asserted for `Producer` and `Consumer`, the two structs that cannot acquire state without the assertion failing, and not for `Drain`, the one that carries a field of its own |
| HD26 | `Drain` | n/a — observation | `Drain` is the only one of the five nouns with a field and the only one with no counterpart in `ring_core` — the crate's single piece of original structure is also its single unmirrored one |
| HD27 | the twelve verbs | n/a — observation | Of twelve public methods, eight forward a single `ring_core` call unchanged, two rewrap a returned value, one is new, and the twelfth is the withheld `try_clone` that is the crate's actual contribution |
| HD28 | `drain` | n/a — observation | `drain` is the only verb whose body is not a forward or a rewrap, which makes it the only one that can be wrong in a way `ring_core`'s own tests would not catch |
