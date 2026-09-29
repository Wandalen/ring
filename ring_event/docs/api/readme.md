# api

Five public items over 234 lines: two traits, three free functions, one
associated type. It is the smallest surface among the ring crates sampled here,
and deliberately so — the crate adds no capability, it removes the possibility of
two capabilities where the family wants one.

Both instances are about what that removal costs. One takes the surface as
declared and finds an attribute correctly absent and three bounds that do not
compose. The other reads the single write signature against the two shapes it
covers and finds it simultaneously wider and narrower than either needs: it adds
an error case the typed shape cannot produce, and it drops a return value
`ring_slot` documents as the reason that value exists.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_traits_three_functions_one_associated_type.md) | Two Traits, Three Functions, One Associated Type | The whole surface, zero `#[ must_use ]`, and three bounds from two crates |
| [002](002_a_result_one_impl_can_never_return.md) | A `Result` One Impl Can Never Return | The unified signature against both impls, and the value it discards |

## Why Zero Is the Right Number of `#[ must_use ]`

Four neighbouring crates carry the attribute between four and eleven times each —
`ring_config` on eleven of twelve returning functions, `ring_store` on seven of
eleven, `ring_slot` six of ten, `ring_publish` four of six. This crate carries it
zero times on two.

Every returning item here returns `Result< (), RingError >` or an `Option`, and
`core` already marks both. `recycle` returns `()`. So there is no signature in the
crate whose result a caller could drop by accident, and the attribute would be
noise. What is not recorded anywhere is that this is the reason — a sweep for
missing attributes finds a zero and cannot distinguish it from an oversight.

## The Cost of One Signature Over Two Shapes

`Fill::fill` returns `Result< (), RingError >`. `BytesSlot::write` needs the
`Err`; `TypedSlot::set` cannot produce one, and eight typed publishes in the
crate's own doctests and suite discharge it anyway.

The traffic runs the other way too. `TypedSlot::set` returns `Option< T >` — the
displaced payload — and `ring_slot`'s doc names evict-oldest as the reason. The
typed `Fill` impl writes `slot.set( self );` and drops it. A byte slot has no
equivalent to return, so the shared signature is the intersection, and on this
axis the intersection is empty.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the public surface, and what each returning item returns --'
command grep -n 'pub trait\|pub fn\|  type Out<' ring_event/src/lib.rs
command grep -n -o 'fn [a-z_]*[^-]*-> *[A-Za-z<:_ ]*' ring_event/src/lib.rs | sed 's/( [^)]*)//'
echo '  -- must_use against returning functions, five crates --'
for c in ring_event ring_publish ring_slot ring_store ring_config; do
  printf '%-14s %s / %s\n' "$c" "$( command grep -c 'must_use' $c/src/lib.rs || true )" "$( command grep -c 'pub \(const \)\?fn .*->' $c/src/lib.rs || true )"
done
echo '  -- the three bounds, and the value the typed impl drops --'
sed -n '/^  P : Fill< S >,$/p;/^  S : Peek,$/p;/^  S : Slot,$/p;/^    slot\.set( self );$/p' ring_event/src/lib.rs
command grep -m1 -F '  pub fn set( &mut self, value : T ) -> Option< T >' ring_slot/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV5 | `ring_event` | n/a — observation | Four neighbouring crates mark between four and eleven returning functions `#[ must_use ]` and this one marks none of two, which is correct rather than an omission because every returning item in the crate returns `Result< (), RingError >` or an `Option` and `core` marks both already — but the distinguishing fact is stated nowhere, so a reviewer sweeping the family for missing attributes finds a zero indistinguishable from an oversight, on the smallest public surface of the five crates sampled |
| EV6 | `ring_event` | n/a — doc gap | The three free functions carry three different bounds — `P : Fill< S >`, `S : Peek`, `S : Slot`, the last belonging to `ring_slot` — so no two agree and a generic round trip through what the module documentation calls one path costs four bounds across two crates, as the suite's own `land_and_read` helper shows with `S : Peek + Slot + Default`; `pub trait Peek : Slot` would compile today, since `TypedSlot` and `BytesSlot` are the only implementors of either and each implements both, and nothing records that the sets coincide or that keeping them coincident is anyone's job |
| EV7 | `ring_event` | **latent hazard** | `TypedSlot::set` returns the displaced payload and `ring_slot:102-105` states why — so evict-oldest can hand back what it evicted "instead of losing it silently" — while `Fill`'s typed impl at `:66` is `slot.set( self );` and drops it, so a caller publishing into an occupied typed slot through `publish_into` loses the previous value with no diagnostic where the direct call returns it; a byte slot has nothing equivalent to report, making the unified `Result< (), RingError >` an intersection that is empty on this axis, and the family's one capture of a `set( … )` return is a `debug_assert!` at `ring_core:369` that cargo's default release profile compiles out, which is what keeps this latent and also why nobody has noticed |
| EV8 | `ring_event` | n/a — observation | `Fill::fill` must be fallible because `BytesSlot::write` can return `BatchTooLarge`, so eight typed publishes across the crate's doctests and suite discharge a variant no execution can produce — pinned deliberately by `a_typed_publish_cannot_fail`, whose comment says it is not a tautology — and read against EV7 the shared signature is wider and narrower than either shape needs at once, adding an impossible `Err` to the typed side and removing a real `Option< T >` from it, with only the addition documented and neither costing anything at runtime |
