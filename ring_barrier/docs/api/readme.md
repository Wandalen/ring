# api

One type, nine methods, every one of them `&self`. The surface is a slice
reference with a fold, a subtraction and a wait hung off it, and its two
interesting properties are both about lifetimes: four methods are `const`
because they never touch a cursor, and one returns a reference the barrier does
not own.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Nine Methods Over One Borrowed Slice](001_nine_methods_over_one_borrowed_slice.md) | The whole surface, the three tiers, the four `const fn`, and the eight `#[ must_use ]` |
| 002 | [The Borrow Is the Whole Type](002_the_borrow_is_the_whole_type.md) | Sixteen bytes, `Copy`/`Send`/`Sync`, and the `&'a` that lets a cursor outlive its barrier |

### The Surface at a Glance

| Tier | Methods | Returns |
|------|---------|---------|
| Shape | `over`, `dependencies`, `len`, `is_empty`, `cursor` | `Self`, a slice, two sizes, one borrow |
| Reading | `frontier`, `available`, `admits` | A position, a count, a verdict |
| Waiting | `wait_for` | A position, or a refusal |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE '^\s*pub (const )?fn ' ring_barrier/src/lib.rs   # 9
grep -c '#\[ must_use \]'       ring_barrier/src/lib.rs   # 8
grep -cE '^\s*pub const fn '    ring_barrier/src/lib.rs   # 4
grep -c '&mut self'             ring_barrier/src/lib.rs || true   # 0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR1 | `ring_barrier` | n/a — coverage | `Barrier::over` has 63 call sites and **none** in any `src/` — 33 in this crate's tests, 20 in `ring_consume`'s, 10 in `ring_publish`'s |
| BR7 | `ring_barrier` | n/a — observation | `size_of::< Barrier >() == 16 == size_of::< &[ PaddedCursor ] >()`, `align_of == 8`, and `Send + Sync + Copy` all hold — probe-verified, stated nowhere in the source |
| BR8 | `ring_barrier` | n/a — observation | `Barrier::cursor` returns `Option< &'a PaddedCursor >`, so a handed-out reference outlives its barrier; the same shape against `GatingSet::cursor` is `error[E0597]` |
| BR9 | `ring_barrier` | n/a — observation | 4 of 9 methods are `const fn` here against 1 of 11 in `ring_gating`; the boundary is `<[T]>::get` not being stable-`const` |
| BR25 | `ring_barrier` | n/a — observation | Eight of nine methods carry an explicit `must_use` and the ninth is the only one returning a `Result`, which `std` already marks — the correct allocation, and the opposite of what `ring_wait` does under the same ruling |
| BR26 | `ring_barrier` | n/a — observation | `cursor` and `dependencies` both return `'a` rather than a borrow of `&self`, so both outlive the barrier that produced them; nine methods take `&self` and none returns a reference tied to it |
