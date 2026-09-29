# Workaround: The `loom` Constructor Cannot Be `const`

### Scope

- **Purpose**: Record why two constructors exist for each of this crate's types, what the duplication costs, and what would notice if the copies diverged.
- **Responsibility**: State the constraint, show that no narrower construction exists in stable Rust, count the duplication family-wide, and name the check that does not exist.
- **In Scope**: `PaddedCursor::new` and `CursorPair::new` under `#[ cfg( loom ) ]`; the workspace `check-cfg` declaration.
- **Out of Scope**: What `loom` verifies and how the seam is argued, which is `ring_atomic`'s module documentation.

### The Constraint

Under `RUSTFLAGS="--cfg loom"`, `ring_atomic`'s atomics come from `loom` rather
than `core`. Loom's atomics carry per-execution model state, so **their
constructors are not `const`**. That property propagates: anything built from one
cannot be `const` either.

`PaddedCursor::new` calls `AtomicSeq::new`, so it inherits the restriction, and
its own doc comment says so:

> `const` in an ordinary build, and not under `--cfg loom` — inherited from
> [`ring_atomic::AtomicSeq::new`], where the seam and its cost are argued.

### The Workaround

Two items with the same name, selected by `cfg`, with **byte-identical bodies**:

```rust
#[ cfg( not( loom ) ) ]
#[ must_use ]
pub const fn new( value : Seq ) -> Self
{
  Self( CacheAligned::new( AtomicSeq::new( value ) ) )
}

/// A cursor at `value` — the `--cfg loom` build, where it is not `const`.
#[ cfg( loom ) ]
#[ must_use ]
pub fn new( value : Seq ) -> Self
{
  Self( CacheAligned::new( AtomicSeq::new( value ) ) )
}
```

Only the `const` keyword and the attribute differ.

### Why Nothing Narrower Works

`const`ness is part of a function's *signature*, not its body, and stable Rust
has no way to make it conditional. Every alternative was unavailable:

| Alternative | Status |
|-------------|--------|
| `#[ cfg_attr( not( loom ), const ) ]` | Not valid — `const` is not an attribute |
| A `const_new` / `new` pair | Two names in the ordinary build for one operation, and callers would have to pick |
| Always non-`const` | Loses `const fn` in the build that ships. `CursorPair::new` is `const`, so a static ring becomes impossible |
| A macro generating both | Hides two public items behind an expansion, for four lines of body |

The duplication is the minimum. It is worth stating explicitly, because "why is
this written twice" is the first question a reader has and the source answers it
only by reference.

### The Cost, Counted

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'cfg( *loom *)' --include=*.rs \
  {module,ring}/*/src/ {module,ring}/*/tests/ \
  | sed -E 's/:/: /' | LC_ALL=C sort
```

Live output:

```
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ws_verify/src/probe.rs: /// `--all-features` is not optional: two silent gates (`cfg(loom)` and the
ring_atomic/src/lib.rs:   #[ cfg( loom ) ]
ring_atomic/src/lib.rs:   #[ cfg( loom ) ]
ring_atomic/src/lib.rs: #[ cfg( loom ) ]
ring_cursor/src/lib.rs:   #[ cfg( loom ) ]
ring_cursor/src/lib.rs:   #[ cfg( loom ) ]
ring_mpsc/tests/mpsc_test.rs: #[ cfg( loom ) ]
ring_publish/tests/handshake_test.rs: #[ cfg( loom ) ]
ring_publish/tests/publish_test.rs: // `#![ cfg( loom ) ]` its `exhaustive` module carries. `--cfg loom` swaps
ring_spsc/tests/spsc_test.rs: #[ cfg( loom ) ]
ring_spsc/tests/spsc_test.rs: //! simply narrow enough that sampling it 100 000 times does not open it. The `#[ cfg( loom ) ] mod exhaustive` at the bottom
ring_testkit/src/lib.rs: //! and it is already a `cfg(loom)` dev-dependency of `ring_atomic`,
ring_testkit/tests/exhaustive_test.rs: #![ cfg( loom ) ]
ring_testkit/tests/exhaustive_test.rs: //! **The whole file is `cfg( loom )`.** Under `--cfg loom`, `ring_atomic`
ring_testkit/tests/testkit_test.rs: // The inverse of `exhaustive_test.rs`'s `#![ cfg( loom ) ]`. `--cfg loom` swaps
```

| Crate | Duplicated constructors | Which |
|-------|:-----------------------:|-------|
| `ring_atomic` | 2 | `AtomicSeq::new`, and the counting cell's |
| `ring_cursor` | **2** | `PaddedCursor::new`, `CursorPair::new` |

Four duplicated bodies in the family, two of them here. `CursorPair::new`'s is
the larger — a five-line struct literal, written out twice at `src/lib.rs:268-291`.

### What Would Notice a Divergence

**Nothing, in an ordinary build.**

| # | Check | Sees a divergence? |
|---|-------|:------------------:|
| W1 | `cargo test`, `cargo clippy`, `cargo build` | **No** — the `cfg( loom )` copy is not compiled at all |
| W2 | `RUSTFLAGS="--cfg loom" cargo test …` | Yes — but only for whichever crate is named, and only if a loom test constructs the type |
| W3 | `missing_docs` | Partially — it forced the second copy to carry its own doc line, which is why the two doc comments differ in length |
| W4 | A test asserting the two agree | Impossible by construction — only one exists per build |

**W1 is the whole exposure.** An edit to `PaddedCursor::new` that a developer
makes, tests, and lands touches only the `not( loom )` copy. The `loom` copy
compiles the day someone next runs a loom model, which is not part of any
routine verification level:

```sh
cd "$(git rev-parse --show-toplevel)"
out=$( RUSTFLAGS='--cfg loom' cargo test -p ring_publish --test handshake_test 2>&1 )
printf '%s\n' "$out" | command grep -E '^test .+\.\.\. ' | LC_ALL=C sort
printf '%s\n' "$out" | command grep -E '^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
test exhaustive::a_claimed_slot_is_invisible_until_published_over_every_interleaving ... ok
test exhaustive::a_full_ring_stops_the_producer_over_every_interleaving ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

That command is named in `ring_atomic`'s module documentation as the way to run
the seam. It is not in levels 1 through 5.

### The `check-cfg` Declaration, and Its Stale Comment

`loom` is set by `RUSTFLAGS`, not by a feature, so `rustc` would otherwise warn
`unexpected_cfgs` at every one of these sites. The workspace declares it once:

```toml
unexpected_cfgs = { level = "warn", check-cfg = [ 'cfg(loom)' ] }
```

with a comment explaining the placement — inherited workspace-wide, and a crate
cannot both inherit the lints table and add to it. The reasoning is sound and the
census beside it is not:

> Only ring_atomic, ring_cursor and ring_publish read the cfg

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rl 'cfg( *loom *)' --include=*.rs \
  {module,ring}/*/src/ {module,ring}/*/tests/ | LC_ALL=C sort
```

Live output:

```
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ws_verify/src/probe.rs
ring_atomic/src/lib.rs
ring_cursor/src/lib.rs
ring_mpsc/tests/mpsc_test.rs
ring_publish/tests/handshake_test.rs
ring_publish/tests/publish_test.rs
ring_spsc/tests/spsc_test.rs
ring_testkit/src/lib.rs
ring_testkit/tests/exhaustive_test.rs
ring_testkit/tests/testkit_test.rs
```

| Crate | Where | Reads the cfg | Named in the comment? |
|-------|-------|:-------------:|:---------------------:|
| `ring_atomic` | `src/` | ✅ | ✅ |
| `ring_cursor` | `src/` | ✅ | ✅ |
| `ring_publish` | `tests/handshake_test.rs` | ✅ | ✅ |
| `ring_mpsc` | `tests/mpsc_test.rs` | ✅ | ❌ |
| `ring_spsc` | `tests/spsc_test.rs` | ✅ | ❌ |
| `ring_testkit` | `tests/exhaustive_test.rs` | ✅ | ❌ |
| `ws_verify` | `src/probe.rs` | ❌ — prose | ❌ |

**Seven crates match the grep, six read the cfg, three are named.** `ws_verify`
is a workspace-verification harness that only *mentions* `cfg(loom)` in a doc
comment, so it inflates the raw count without adding a site the `check-cfg`
declaration has to cover — the same distinction that separates the file list
above from the crate count here.

The three omitted all read it from `tests/` rather
than `src/`, which is the likely cause — the comment was written about source
files and the test files arrived later.

It is harmless in the direction it drifted: the declaration is workspace-wide, so
the unnamed crates are covered anyway. It would not be harmless if someone used
the comment to decide the declaration could be narrowed.

### CU49 — The Workaround's Other Arm Is Never Built

```
constructors: 4
```

Four constructor bodies exist so that two of them can be `const` in ordinary
builds. The `loom` pair is compiled only under `RUSTFLAGS="--cfg loom"`, which
appears in no verification level this project runs.

**Finding.** The workaround costs a doubled constructor on both types and buys a
`const fn` in the configuration that is always built, at the price of an arm that
is never built. That is a defensible trade and an unmonitored one: the unbuilt
half can stop compiling at any time and the suite will keep passing.

---

### CU50 — The Deletion Condition Is Checkable and Unchecked

The workaround goes away when `loom`'s cell constructor becomes `const`. That is
decidable by compiling the `loom` build and reading one error message.

**Finding.** Nothing runs it. So on the day the constraint lifts, this crate
keeps four constructors, the doc keeps describing a limitation that no longer
exists, and nobody is told — the same failure shape as CU36's stale allowlist,
one crate closer to home. A workaround with a stated exit condition and no check
for it is a permanent workaround.

---

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_padded_cursor.md](../data_structure/001_the_padded_cursor.md) | The type whose constructor is written twice |
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | The larger of the two duplicated bodies |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_four_dependencies_all_used.md](../integration/001_four_dependencies_all_used.md) | The `ring_atomic` edge the constraint arrives through |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_cursor_from_new_to_shared.md](../lifecycle/001_a_cursor_from_new_to_shared.md) | Construction as the first stage, in both builds |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The `const`ness promise, and the build it does not hold in |

### Workarounds

| File | Relationship |
|------|--------------|
| [002_the_trait_must_travel_with_the_type.md](002_the_trait_must_travel_with_the_type.md) | The crate's other language-imposed workaround |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:158-171` | `PaddedCursor::new`, both copies |
| `ring_cursor/src/lib.rs:268-291` | `CursorPair::new`, both copies |
| `ring_atomic/src/lib.rs:43-66, 195-208, 349-376` | The seam, and the two constructors it duplicates there |
| `Cargo.toml:614-619` | The `check-cfg` declaration and its census comment |

### Tests

| File | Relationship |
|------|--------------|
| `ring_publish/tests/handshake_test.rs:60` | The loom model that compiles the `cfg( loom )` copies |
| `tests/cursor_test.rs` | W1 — every test here builds the `not( loom )` copy only |
