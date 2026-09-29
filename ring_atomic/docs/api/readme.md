# api

The public surface is one trait, two structs that implement it, one plain data
struct, and six inherent methods. Everything a caller of this crate can reach is
in ten declarations, and eight of the ten are constructors or accessors. The
interesting question is not what the surface offers but what it *requires* — of
its implementors, and of the callers who hold its return values.

On both counts the answer is: nothing. `SeqCell` declares no supertrait, so an
implementor need not be `Sync` and a trait object of it cannot cross a thread
boundary. None of its methods is `#[ must_use ]`, so the one return value that
represents an irreversible claim can be dropped in silence. Both gaps close with
one attribute or one word on the declaration, and both would propagate to every
present and future implementor without touching either of the two that exist.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_return_value_that_is_a_claim.md) | The Return Value That Is a Claim | The three returning methods, the five `must_use` spent elsewhere, and the proof the attribute works here |
| [002](002_a_shared_cell_that_is_not_sync.md) | A Shared Cell That Is Not `Sync` | The empty supertrait list, the three bare bounds it produces, and what each admits |

## Ten Declarations, Two Missing Words

The surface is small enough to read in one sitting, which is what makes the two
omissions legible. `AtomicSeq::new` and `CountingSeq::new` are `#[ must_use ]`;
`SeqCell::fetch_add`, whose return value is the range of sequences the caller now
owns and cannot give back, is not. `AtomicU64` is `Sync`; `dyn SeqCell` is not.

Neither is an argument the crate lost. There is no prose anywhere in the crate,
the family, or the design corpus weighing either choice — the attributes are
simply on the items where they were easy to think of, and absent from the one
place where they cross a crate boundary.

## The Attribute Habit and Its Blind Spot

`#[ must_use ]` appears 253 times across the 33 crates and never once on a trait
method. Every instance is inherent, applying to exactly the item it is written on.
That is a strong, consistent habit with one shape missing from it, and the missing
shape is the only one that reaches implementors the author will never see.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the whole public surface --'
command grep -nE '^\s*pub (fn|const fn|struct|trait|enum|type)' ring_atomic/src/lib.rs
echo '  -- what the trait requires of an implementor --'
command grep -m1 -F 'pub trait SeqCell' ring_atomic/src/lib.rs
echo '  -- and every must_use in the crate --'
command grep -c 'must_use' ring_atomic/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT5 | `ring_atomic` | **latent hazard** | `fetch_add`'s own doc calls the return value "the first sequence the caller now owns", the advance cannot be undone, and dropping the value compiles clean — while all five `must_use` the crate spends sit on items where dropping costs nothing |
| AT6 | `ring_atomic` | n/a — unenforced | `#[ must_use ]` on the trait declaration propagates to every implementor, proved against a patched copy; across all 33 crates the attribute appears 253 times and never on a trait method |
| AT7 | `ring_atomic` | **latent hazard** | `SeqCell` declares no supertrait, so `&dyn SeqCell` is neither `Send` nor `Sync` and cannot be shared between the producer and consumer the trait exists to serve — the one object-safety test runs on a single thread |
| AT8 | `ring_atomic` | n/a — unenforced | The three generic bounds in the family are bare `C : SeqCell`, which a `Cell< u64 >`-backed implementor satisfies — `ring_batch::claim` hands it a claim for eight sequences with no atomicity anywhere in the call |
