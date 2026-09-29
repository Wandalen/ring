# Data Structure: The Default Bound Is a Bound on the Type

### Scope

**Purpose:** Record that the impl blocks split three ways — `Default`,
`Slot + Default`, and no bound — that `Default` alone is therefore what a shape
must satisfy to be *stored* while `Slot` is only what it must satisfy to be
*swept*, and that `ring_slot` — which defines `Slot` — never mentions `Default`
at all.

**Responsibility:** What a slot shape must satisfy to be storable, where that
requirement is written, and where a shape author would look for it.

**In Scope:** `ring_store/src/lib.rs:65, 96, 144`;
`ring_slot/src/lib.rs`; the compile probe.

**Out of Scope:** What `Default` costs at construction is
[`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md). The layout is
[`data_structure/001`](001_three_words_whatever_the_slot_costs.md).

---

## Three Impl Blocks, Three Bounds

```sh
cd "$(git rev-parse --show-toplevel)"
grep '^impl< S' ring_store/src/lib.rs
```

Live output:

```
impl< S : Default > Buffer< S >
impl< S : Slot + Default > Buffer< S >
impl< S > Buffer< S >
```

`new` lives alone under `Default`; `clear` and `all_empty` under
`Slot + Default`; nine functions under no bound at all. The split reads as a
careful minimisation: only the functions that need the traits require them, and
everything that is pure storage works for any `S`.

---

### BF20 — The Constructor's Bound Is What a Shape Must Satisfy, and It Asks for `Default` Only

A slot shape that implements `Slot` and not `Default` cannot be put in a
`Buffer` at all:

```
error[E0277]: the trait bound `NoDefaultSlot: Default` is not satisfied
  --> src/main.rs:17:37
note: required by a bound in `Buffer::<S>::new`
  --> /home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_store/src/lib.rs:55:18
```

There is no second route in. `new` is the only function returning `Self`, the
fields are private, `Buffer` derives only `Debug`, and no `From` impl exists
([`api/002`](../api/002_six_ways_to_reach_a_slot.md) BF17). So `new`'s own bound
is, exactly, the storability requirement for the whole type — and that bound is
`Default`, nothing more.

**Finding.** The entry that stood here previously recorded the opposite: that
`new` sat under `Slot + Default`, so the stronger bound governed the type and the
unbounded block advertised a reachability no caller could use. That was true as
written and is now false — `new` was narrowed to `Default` so that
`Buffer< UnsafeCell< S > >` could be constructed, which is what
[`ring_spsc`](../../../ring_spsc/docs/data_structure/001_two_cursor_ring.md) and
`ring_mpsc` store. `UnsafeCell< S >` is `Default` and is not a `Slot`, so the two
loosest blocks now serve a type the tightest one cannot.

What survives the correction is the reading order, inverted. A shape author still
meets three bounds and has to work out which one is the gate, and the answer is
still not the one printed first: `Slot` is the *conspicuous* requirement — it is
the trait the family is named around and the one `clear` and `all_empty` need —
but it is not the requirement for storage. `Default` is, and it appears in the
narrowest block of the three. BF21 below is the same gap seen from the other
side: the trait that actually gates storage is the one neither crate's
documentation names.

---

### BF21 — `Default` Is Required of Every Slot Shape and `ring_slot` Never Says So

`Slot` is defined in `ring_slot` and requires two methods and no supertraits.
`Default` is a requirement this crate adds. Search both crates' documentation for
it:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'Default' ring_store/src/lib.rs ring_slot/src/lib.rs \
  | grep -E '///|//!' | sed 's|^ring/||' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_store/src/lib.rs://! delegated to `ring_index`. No `unsafe` — a `Box<[S]>` of `Default` slots is
ring_store/src/lib.rs:  /// Bounded on `Default` alone rather than on `Slot`, because allocation needs
ring_store/src/lib.rs:  /// `Default` without being a `Slot`. Wrapping the buffer instead would mean a
```

Three mentions, all in the consuming crate. One is incidental — `:21`, inside a
sentence whose subject is the absence of `unsafe`. The other two are not: `:69`
and `:73` are a deliberate explanation, added to the constructor's own `impl`
block after this finding was filed, of why the bound is `Default` rather than
`Slot` and what would break if the buffer were wrapped instead.

**That makes the asymmetry sharper rather than smaller.** `ring_store` now
states the requirement, argues for it, and names the consequence of the
alternative. `ring_slot` still says nothing: not on the trait, not on either
shape, not in the module comment. A requirement is now thoroughly documented
everywhere except the crate that would have to honour it — which is the one place
an author writing a third slot shape would look. Both shipped shapes do implement
it:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'impl.*Default for' ring_slot/src/lib.rs
```

Live output:

```
impl< T > Default for TypedSlot< T >
impl< const N : usize > Default for BytesSlot< N >
```

**Finding.** Writing a third slot shape is a `ring_slot` activity: implement
`Slot`, two methods, done. The shape then fails to compile in the only container
the family has, with an error pointing at `ring_store/src/lib.rs:65` — a crate
the author had no reason to open.

The requirement is real, load-bearing and cheap to satisfy; the problem is purely
where it is written. `Slot`'s own documentation is the place a shape author
looks, and one sentence there — *shapes must also implement `Default`;
`ring_store` builds a ring's slots with it* — would put the constraint on the
trait that implies it. The alternative fix, a `Default` supertrait on `Slot`,
would state it in the type system and cost `ring_slot` a dependency on nothing at
all, since `Default` is in `core`.

Either way the current arrangement puts a family-wide requirement in one
consuming crate's prose, in a clause about something else.

**Disposition:** declined — both remedies this finding names live in
`ring_slot`, not `ring_store`: a documentation sentence on the `Slot` trait
itself, or a `Default` supertrait added to `Slot`. This disposition pass is
scoped to `ring_align`, `ring_batch`, and `ring_store`'s own `src/` and
`docs/`; `ring_slot` is a different crate and editing it is out of that
scope. `ring_store/src/lib.rs:69-76` already carries the two-sentence
explanation this finding credits it with — there is nothing further to add
on this crate's own side. Mirrors `ring_atomic`'s own AT34 pattern: the fix
belongs to the crate that defines the trait, not the crate that merely
depends on it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_three_words_whatever_the_slot_costs.md) | The layout this bound does not appear in |
| [`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md) | What `Default` costs, called once per slot |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | That `new` is the only constructor, which is what makes its bound the type's |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | What else the buffer asks of a slot |
| [`type/001`](../type/001_one_parameter_three_impl_blocks_one_derive.md) | `S` as a type parameter |

### Sources

| Fact | Where |
|------|-------|
| The three impl blocks | `ring_store/src/lib.rs:65, 96, 144` |
| A `Slot`-not-`Default` shape rejected | Compile probe, quoted above |
| The single documentation mention | `ring_store/src/lib.rs:21` |
| Both shipped `Default` impls | `ring_slot/src/lib.rs:170, 379` |
| `new` as the only constructor | `ring_store/src/lib.rs:88` |

### Tests

| Test | Covers |
|------|--------|
| `every_slot_starts_empty` | That `S::default` produced empty slots |
| `the_same_buffer_type_serves_both_slot_shapes` | Both shipped shapes through the bounded constructor |
| `clear_empties_every_slot_and_keeps_the_allocation` | `clear`, one of the three bounded functions |
| *(to create)* | A compile-fail pinning that a `Slot`-without-`Default` shape is rejected, with the error naming why |
