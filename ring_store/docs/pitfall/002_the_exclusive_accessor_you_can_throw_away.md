# Pitfall: The Exclusive Accessor You Can Throw Away

### Scope

**Purpose:** Record that `get_mut` and `at_mut` carry no `#[ must_use ]` while
their shared counterparts do, that discarding either is a silent no-op under the
workspace's own lint settings, and that the workspace does not enable the one
lint that would catch it.

**Responsibility:** The attribute asymmetry across the accessor pairs, and what
it lets compile.

**In Scope:** `ring_store/src/lib.rs:87-252`; `Cargo.toml:222-236`.

**Out of Scope:** The full attribute census is
[`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md). The
borrow discipline itself is
[`pattern/002`](../pattern/002_every_write_is_a_borrow.md).

---

### BF40 — Two of the Six Accessors Can Be Called and Discarded Under `-D warnings`

The attribute lands on some functions and not others:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E -B1 'pub (const )?fn ' ring_store/src/lib.rs | grep -E 'must_use|pub (const )?fn |^--'
```

Live output:

```
  #[ must_use ]
  pub fn new( capacity : Capacity ) -> Self
--
  pub fn clear( &mut self )
--
  #[ must_use ]
  pub fn all_empty( &self ) -> bool
--
  #[ must_use ]
  pub const fn capacity( &self ) -> Capacity
--
  #[ must_use ]
  pub const fn len( &self ) -> usize
--
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
--
  #[ must_use ]
  pub fn get( &self, index : SlotIndex ) -> &S
--
  #[ must_use ]
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
--
  #[ must_use ]
  pub fn at( &self, seq : Seq ) -> &S
--
  #[ must_use ]
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
--
  pub fn iter( &self ) -> core::slice::Iter< '_, S >
--
  pub fn iter_mut( &mut self ) -> core::slice::IterMut< '_, S >
```

`get` is marked, `get_mut` is not. `at` is marked, `at_mut` is not. What that
permits, compiled with the flags the project's own verification levels use:

```
--- (1) discarded exclusive accessors ---
  get_mut(0); and at_mut(Seq(0)); both compiled under -D warnings
  slot 0 still holds Some(7)
```

**Finding.** `buffer.get_mut( index );` and `buffer.at_mut( seq );` are complete,
warning-free statements that do nothing at all. They borrow the buffer
exclusively, produce a `&mut S`, and drop it — no diagnostic, no lint, no effect.
`clear` is the only other function that legitimately returns nothing, so a reader
scanning for "operations that mutate" sees three statement-shaped calls where one
of them is real.

The asymmetry is not arbitrary — `#[ must_use ]` on a `&mut` return is unusual,
and the compiler's own guidance is that an exclusive borrow is normally obtained
in order to be used, which is exactly the assumption that fails here. The two
iterator methods are unmarked too, and are nonetheless safe: `slice::Iter` and
`slice::IterMut` carry their own `#[ must_use ]`
([`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) BF14), so
discarding one is a hard warning. Of six accessors, four are protected and two
are not, and the two unprotected are the two whose entire purpose is to be
written through.

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- get_mut --'
command grep -m1 -B1 -F 'pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S' ring_store/src/lib.rs
echo '  -- at_mut --'
command grep -m1 -B1 -F 'pub fn at_mut( &mut self, seq : Seq ) -> &mut S' ring_store/src/lib.rs
```

Live output:

```
  -- get_mut --
  #[ must_use ]
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
  -- at_mut --
  #[ must_use ]
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
```

**Disposition:** applied — added `#[ must_use ]` to `get_mut` and `at_mut` in
`src/lib.rs`, matching the four accessors that already carry it. Verified no
caller anywhere in the workspace discards either result: this crate's own
tests, `ring_event`, and `ring_tls` all chain a method call or pass the
result straight into a function argument; the other `get_mut`/`at_mut` hits
found workspace-wide belong to unrelated types (`[T]::get_mut` on a batch
slice in `ring_core`/`ring_mpsc`/`ring_spsc`, `HashMap`/`BTreeMap::get_mut`
in `ring_registry`), not this crate's `Buffer`. Both forms in BF41's own
probe — `buffer.get_mut( index );` and `let _ = buffer.get_mut( index );` —
now diverge under `-D warnings`, the same divergence the four already-marked
accessors give. `unused_results` at the workspace level, BF41's other named
option, is out of scope here — it would need auditing all 33 crates, which
this pass does not have the authority or the reach to do. The crate's 17
unit tests plus 8 doctests re-verified passing (`cargo test --all-features`,
2026-09-04). Now prints: `  #[ must_use ]`

---

### BF41 — The Lint That Catches It Is Allow-by-Default and the Workspace Does Not Enable It

`unused_results` reports exactly this shape, and the workspace's lint table does
not contain it:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[workspace.lints.rust\]/,/^\[workspace.lints.clippy\]/p' Cargo.toml
```

Live output:

```
[workspace.lints.rust]
rust_2018_idioms = { level = "warn", priority = -1 }
future_incompatible = { level = "warn", priority = -1 }
missing_docs = "warn"
missing_debug_implementations = "warn"
unsafe-code = "deny"
unsafe_op_in_unsafe_fn = "deny"
unreachable_pub = "warn"
meta_variable_misuse = "warn"
redundant_lifetimes = "warn"
unit_bindings = "warn"
unused_lifetimes = "warn"
unused_macro_rules = "warn"
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }

[workspace.lints.clippy]
```

Adding `#![ deny( unused_results ) ]` to the probe turns both discards into
compile errors — `unused result of type &mut TypedSlot<u32>`, twice — so the
detection exists and is one line away.

**Finding.** The gap is closable two ways and neither costs anything structural.
`#[ must_use ]` on `get_mut` and `at_mut` closes it for these two functions
across every consumer, at the price of an attribute the compiler's conventions do
not usually suggest for a `&mut` return. `unused_results` at the workspace level
closes it for every function in all 33 crates, at the price of a lint that is
allow-by-default for good reason — it fires on a great deal of ordinary code and
would need auditing crate by crate.

Neither is obviously right, which is why this is recorded as a pitfall rather
than a defect. What is worth stating is that the choice was never made: the
comment in the lints table shows the workspace's lint configuration is
deliberate, discussed and maintained, and `unused_results` is absent from it
without a note either way, while the two functions it would cover are absent from
the `#[ must_use ]` pattern their siblings follow. A reader has no way to tell
whether either omission was considered.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](001_the_two_questions_named_is_empty.md) | The other way this API compiles into a no-op |
| [`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) | The full attribute census, and `slice::Iter`'s own protection |
| [`pattern/002`](../pattern/002_every_write_is_a_borrow.md) | Why the exclusive accessors exist at all |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | The six accessors, four of them protected |

### Sources

| Fact | Where |
|------|-------|
| The attribute placement | `ring_store/src/lib.rs:87-252` |
| Both discards compiling | Release probe under `RUSTFLAGS="-D warnings"`, quoted above |
| Both discards rejected by `unused_results` | Same probe with `#![ deny( unused_results ) ]` |
| The workspace lint table | `Cargo.toml:222-236` |
| `slice::Iter`'s own `#[ must_use ]` | Census in `api/001` |

### Tests

| Test | Covers |
|------|--------|
| `indexed_get_and_set_round_trip` | `get_mut` used the way it is meant to be |
| `a_full_lap_overwrites_and_a_partial_one_does_not` | `at_mut` used the way it is meant to be |
| *(to create)* | A compile-fail assertion that a discarded `get_mut` is rejected — once the attribute exists |
