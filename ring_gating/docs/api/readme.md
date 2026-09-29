# api

One type, eleven methods, every one of them `&self`. The surface is small enough
to list in full, and its two interesting properties are both absences: nothing
takes `&mut self`, and only one method is `const`.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Eleven Methods Over One Owned `Vec`](001_eleven_methods_over_one_owned_vec.md) | The whole surface, the `&mut self` that does not exist, and why mutation still happens |
| 002 | [The Reading That Returns a Position](002_the_reading_that_returns_a_position.md) | `limit` and `slowest` returning `Option< Seq >` where `headroom` returns a plain `usize` |

### The Surface at a Glance

| Group | Methods | Returns |
|-------|---------|---------|
| Construction | `new` | `Self` |
| Structure | `len`, `is_empty`, `cursor`, `cursors`, `capacity` | Sizes, borrows, the capacity |
| Gating | `slowest`, `headroom`, `admits`, `check`, `limit` | A position, a count, a verdict, a reason, a position |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^\s*pub (const )?fn ' ring_gating/src/lib.rs || true   # 11
command grep -c '#\[ must_use \]'       ring_gating/src/lib.rs || true   # 10
command grep -cE '^\s*pub const fn '    ring_gating/src/lib.rs || true   # 1
command grep -c '&mut self'             ring_gating/src/lib.rs || true   # 0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT6 | Interior mutability through `&self` | **latent hazard** | No method takes `&mut self`, so the set's membership is fixed at construction; the type is nonetheless not immutable, because `cursor()` hands out interior mutability through a shared borrow |
| GT7 | `capacity()` | n/a — observation | The crate's only `const fn`, and the other ten cannot be, for three different reasons — `Vec` indexing, atomic loads, and delegation into non-`const` callees |
| GT8 | `check` | n/a — observation | The one method of eleven without `#[ must_use ]`, and the one method that does not need it — `Result` carries the attribute already, so the convention is complete without being uniform and a reader counting attributes finds an apparent gap that is not one |
| GT9 | `limit` | n/a — inconsistency | Returns `None` for an ungated ring and, by construction, for any set with no consumers, so "no limit" and "no consumers" are the same value. A caller diagnosing a stall cannot separate an unbounded producer from a misconfigured set without also calling `is_empty`, and the doc comment describes only the first reading |
| GT10 | The one cast | n/a — observation | `limit` holds the crate's only `as` conversion, `self.capacity.get() as u64`, forced by `Capacity::get` returning `usize` while `Seq::advanced_by` takes `u64`. The method that returns a position is also the single point where this crate crosses a width boundary |
