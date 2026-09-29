# pitfall

Both pitfalls here have the same shape: a statement that compiles, passes every
lint the project enables, and does not do what its spelling says. Neither is a
defect — the implementations are correct and one of them is documented in its own
first line — and neither is caught by anything.

The first is a naming collision the type cannot escape: `Buffer::is_empty` asks
whether the buffer has zero slots, which is never, so it answers `false` for a
buffer holding nothing. The question a caller means is `all_empty`, ninety lines
away in a different `impl` block and called by nothing in the family. The second
is an attribute asymmetry: `get` and `at` are `#[ must_use ]`, `get_mut` and
`at_mut` are not, so calling either as a bare statement is a warning-free no-op
that borrows the buffer exclusively and drops the result.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Two Questions Named `is_empty`](001_the_two_questions_named_is_empty.md) | BF38, BF39 — a predicate that is always false, and seventeen more like it |
| 002 | [The Exclusive Accessor You Can Throw Away](002_the_exclusive_accessor_you_can_throw_away.md) | BF40, BF41 — two accessors that compile into nothing, and the lint that is not on |

### What Each Predicate Actually Asks

| Call | Asks | On a fresh buffer |
|------|------|-------------------|
| `Buffer::is_empty()` | Does this buffer have zero slots? | `false` — and always |
| `Buffer::all_empty()` | Is every slot empty of payload? | `true` |
| `TlsBuffer::is_empty()` | Are zero items staged? | `true` |

Two types named "buffer", both with `is_empty`, answering opposite questions.
`ring_tls`'s test file has both in scope and resolves it by binding the
`ring_store::Buffer` to a variable called `ring` — a convention with no comment
attached and nothing enforcing it.

### The Accessor Protection Map

| Accessor | `#[ must_use ]` | Protected? |
|----------|----------------|------------|
| `get` | yes | yes |
| `at` | yes | yes |
| `iter` | no | yes — `slice::Iter` carries its own |
| `iter_mut` | no | yes — `slice::IterMut` carries its own |
| `get_mut` | no | **no** |
| `at_mut` | no | **no** |

Four of six protected, and the two that are not are the two whose purpose is to
be written through. `unused_results` catches both and is allow-by-default;
the workspace's lint table, which carries a maintained explanatory comment about
`unexpected_cfgs`, says nothing about it either way.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the predicate and its doc's first line
awk '/^  \/\/\/ Always false — a `Capacity` cannot be zero, so a buffer always has slots\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^  \/\/\/ assert!\( !buffer\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_store/src/lib.rs

# every is_empty in the family — sorted: grep -r has no stable multi-file order
grep -rn 'pub \(const \)\?fn is_empty' ring_*/src/*.rs | sort

# the naming inversion where both types are in scope
grep -n 'let mut buffer = TlsBuffer\|let mut ring : Buffer\|buffer\.is_empty()' ring_tls/tests/tls_test.rs | head -8

# which functions carry the attribute
grep -nE -B1 'pub (const )?fn ' ring_store/src/lib.rs | grep -E 'must_use|pub (const )?fn |^--'

# the lint table that does not mention unused_results
sed -n '/^\[workspace.lints.rust\]/,/^\[workspace.lints.clippy\]/p' Cargo.toml
```

Both measured behaviours — `is_empty` false while `all_empty` is true, and the
two discards compiling under `RUSTFLAGS="-D warnings"` — come from a release
probe quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF38 | `ring_store` | **latent hazard** | `Buffer::is_empty()` returns `false` for a buffer holding nothing, so `if buffer.is_empty()` is a branch that can never be taken under a predicate that compiles, is `const` and is `#[ must_use ]`; the question a caller means is `all_empty` |
| BF39 | family | n/a — observation | Seventeen `is_empty` methods family-wide, and where the two buffer types meet the collision is resolved by binding the `ring_store::Buffer` to a variable named `ring` — a convention with no comment and nothing enforcing it |
| BF40 | `ring_store` | **latent hazard** | `get_mut` and `at_mut` carry no `#[ must_use ]` while `get` and `at` do, so calling either as a statement compiles clean under `-D warnings` and does nothing |
| BF41 | `ring_store` | n/a — unenforced | `unused_results` rejects both discards and is absent from a workspace lint table that is otherwise deliberate and commented, so neither closure — the attribute or the lint — was recorded as considered |
