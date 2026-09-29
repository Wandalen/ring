# decisions

Two decisions make this crate what it is, and they are both about placement
rather than behaviour. The first puts the write half on the payload and the read
half on the slot — opposite sides of the same operation, each open to extension in
the direction the other is closed. The second wraps both in free functions that
add no behaviour at all, so the family has one named path instead of two method
calls a shape could quietly diverge from.

Both instances weigh what the decision bought against what it cost. The
extension points turn out to be wider than documented and completely unused. The
free functions turn out to be renames, and naming them gave the family a second
word for each of three operations, a third word for one of them, and two verbs
already in use one level up.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | The Write Half on the Payload, the Read Half on the Slot | The asymmetry, both extension points, and their zero users |
| [002](002_three_free_functions_instead_of_methods.md) | Three Free Functions Instead of Methods | What each rename adds, and what the names collide with |

## The Asymmetry, in Four Impl Headers

`impl< T > Fill< TypedSlot< T > > for T` and
`impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]` against
`impl< T > Peek for TypedSlot< T >` and
`impl< const N : usize > Peek for BytesSlot< N >`.

`Fill` carries the slot as a type parameter and lands on the payload; `Peek`
carries no parameter and lands on the slot, with the payload side moved into an
associated type. So a new payload is a new `Fill` impl and no slot changes, and a
new slot shape is a new `Peek` impl and no payload changes. Probes confirm both
directions work from a downstream crate — including a third slot type the family
does not have, which passes through all three free functions untouched.

`Fill`'s doc states its half of the reasoning in one sentence. `Peek`'s says
nothing about its own placement, and no sentence anywhere presents the two as a
pair.

## What Naming the Path Cost

The three free functions forward to `payload.fill( slot )`, `slot.peek()` and
`slot.clear()`. The first also swaps the argument order so the slot reads first;
the other two are pure aliases. Their stated value is that there is only one of
them, which is true of the implementation and false of the call site — both traits
must stay `pub` for the bounds to be nameable, so both forms are always callable,
and the crate's own suite uses 29 of one and 4 of the other.

Across the family the reset operation is `clear` six times, `reset` twice, and
`recycle` exactly once — here. And `publish_into`'s doc says "a ring's publish
calls this" while the family's `publish` is `ring_publish::Publisher::publish`,
which takes two sequence numbers and never sees a slot.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the asymmetry, in four impl headers --'
command grep -n 'pub trait\|^impl' ring_event/src/lib.rs
echo '  -- the three free functions and the methods under them --'
awk '/^pub fn publish_into< S, P >\( slot : &mut S, payload : P \) -> Result< \(\), RingError >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^\/\/\/ let slot = BytesSlot::< 4 >::empty\(\);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 8 { print } /^pub fn recycle< S >\( slot : &mut S \)$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 5 { print }' ring_event/src/lib.rs | command grep 'pub fn\|  [a-z]*\.'
echo '  -- implementors of either trait outside the crate --'
command grep -rn 'impl.*Fill<\|impl.*Peek for' --include=*.rs /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep -vc '^ring_event/src' || true
echo '  -- what the family calls the third operation, and what it calls publish --'
command grep -rn 'pub fn recycle\|pub fn reset\|fn clear( &mut self )\|pub fn clear' --include=*.rs ring_*/src | sed 's/.*fn //; s/[(<].*//' | sort | uniq -c | sort -rn
command grep -rn 'pub fn publish( ' --include=*.rs ring_*/src
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV13 | `ring_event` | n/a — doc gap | `Fill< S >` takes the slot as a type parameter and is implemented on the payload while `Peek` takes no parameter and is implemented on the slot with the payload side moved into an associated type, so each trait is extensible along the axis the other one fixes — a new payload needs no slot change, a new slot shape needs no payload change — and only `Fill` documents its half, in the crate's clearest sentence at `:41-43`; `Peek`'s doc opens by naming what the trait is and then spends every remaining line on why `Out` carries a lifetime, never saying that its placement is the mirror image or that the two traits are a pair, so the symmetry that is the reason the crate has two traits rather than one is visible only by reading the four impl headers side by side |
| EV14 | `ring_event` | n/a — unadopted | Outside `ring_event/src` the workspace contains zero `Fill` impls and zero `Peek` impls — not in another crate, not in a test, not in a doctest — and the suite's `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` does not add a payload kind either, calling `.fill()` on a `u32` the blanket impl already covers; probes show both directions work, a local payload taking `Fill< BytesSlot< 8 > >` while keeping the blanket impl's `Fill< TypedSlot< MyId > >`, and a local slot type taking `Slot`, `Peek` and an inbound `Fill< CountingSlot > for u32` and round-tripping through all three free functions unmodified, so the crate has built a path any number of shapes can take, which is a better property than the two-shape one it claims, and nothing documents it or pins it |
| EV15 | `ring_event` | n/a — inconsistency | The three free functions forward to `payload.fill( slot )`, `slot.peek()` and `slot.clear()` — one argument swap and two pure aliases — and their stated value at `:149-150` is that "there is only one of it", which holds for the implementation and not for the call site, since both traits must stay `pub` for the bounds to be nameable and the suite writes 29 calls in the free-function form and 4 in the method form; the result is two names for each of three operations with nothing marking either canonical, and for the third operation a family-wide third name, since across 33 crates the reset is `clear` six times, `reset` twice, `reset_counts` and `clear_log` once each, and `recycle` exactly once, in the one crate whose stated job is to stop the family growing a second path — its own test comment using two of the three words in one sentence |
| EV16 | `ring_event` | **misleading doc** | `publish_into`'s doc says "a ring's publish calls this", and the family has a `publish` — `ring_publish::Publisher::publish( &self, start : Seq, len : usize ) -> Seq` — which moves a cursor, has no slot and no payload, and could not call it if it wanted to; the same reading problem repeats on the read half, where 11 `drain*` functions across eight other crates all drain a batch or range from a ring rather than a single slot, so this crate's two verbs are the family's two verbs one level down and the sentence names, out of everything it could have named, the one function that shares the word and does not fit, leaving a reader who follows it to `ring_publish` unable to tell whether the documentation is loose or the wiring is missing |
