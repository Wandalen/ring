# invariant

Two properties hold in this crate and neither is a contract. The first — that
`peek()` returns `None` exactly when `Slot::is_empty()` is true — is what keeps a
drain from reading a slot the handshake considers empty, and the suite's own
comment says so. The second — that a refused fill leaves the slot untouched — is
what lets a caller keep using a slot after `BatchTooLarge`.

Both are held by the two shipped shapes and required of nobody. `Peek` and `Slot`
are unrelated traits in different crates with no supertrait between them, and
`Fill::fill` takes `&mut S` with no word about the slot's state on `Err`. Both
instances measure the gap the same way: what the code does, what the tests reach,
and what a third implementor arriving through the crate's documented extension
point would be free to do instead.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_readings_of_one_emptiness.md) | Two Readings of One Emptiness | The agreement the drain rests on, and the implementor that breaks it |
| [002](002_a_refused_fill_changes_nothing.md) | A Refused Fill Changes Nothing | The early return, three statements of it, and the geometry that hides the rest |

## Held by Coincidence, by Construction, and by Nothing

`Peek for TypedSlot` calls the inherent `get()`, `self.0.as_ref()`, while
`Slot::is_empty` separately reads `self.0.is_none()` — two expressions over one
field, agreeing because `Option` makes them agree. `Peek for BytesSlot` calls the
inherent `is_empty()`, `self.len == 0`, and `Slot::is_empty` forwards to that same
method — one reading under two names. Above them, `Slot` declares `is_empty` and
`clear` with no supertrait and `Peek` declares `Out` and `peek` with none either.

A probe supplies the third case: a slot whose `Slot::is_empty` always answers
`true` while `drain_from` hands back `Some( 42 )`. It compiles without a warning.

## Verified Over the Part That Can Be Seen

`BytesSlot::write` checks the length before touching either field, so a refused
write is a pure read. Three places say so — a doctest, a test doc comment
promising "the slot is not half-updated", and this crate's assertion message —
and both tests that check it use a `BytesSlot< 4 >` holding four bytes, the one
geometry where `read()` hides nothing and the assertion is therefore total by
accident. At `BytesSlot< 8 >` holding two, the same assertion form leaves six
bytes unchecked. The property does hold there; nothing verifies that it does.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two readings of emptiness, per shape --'
awk '/^  pub const fn get\( &self \) -> Option< &T >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 3 { print } /^impl< T > Slot for TypedSlot< T >$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 5 { print } /^  pub const fn is_empty\( &self \) -> bool$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 3 { print } /^impl< const N : usize > Slot for BytesSlot< N >$/{ n4 = NR } n4 && NR >= n4 + 2 && NR <= n4 + 5 { print }' ring_slot/src/lib.rs
echo '  -- what each Peek impl consults, and whether the traits are related --'
sed -n '/^  fn peek( &self ) -> Option< &T >$/,/^  }$/p;/^  fn peek( &self ) -> Option< &\[ u8 ] >$/,/^  }$/p' ring_event/src/lib.rs
command grep -m1 -A1 -F 'pub trait Slot' ring_slot/src/lib.rs
command grep -m1 -F 'pub trait Peek' ring_event/src/lib.rs
echo '  -- the refusal check, before either field is touched --'
command grep -m1 -A6 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- and every statement in the family about what a refusal preserved --'
command grep -rn 'left the old one\|previous contents intact' --include=*.rs */ 
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV21 | `ring_event` | n/a — unenforced | The agreement between `peek()` and `Slot::is_empty()` is what stops a drain reading a slot the handshake considers empty — stated only in a test comment — and nothing requires it: `Slot` and `Peek` are unrelated traits in different crates with no supertrait either way, and the two shipped shapes satisfy it for different reasons, `TypedSlot` by two independent expressions over one `Option` field and `BytesSlot` by a single reading its `Slot` impl forwards to, so a third implementor writes both halves freely, as a probe shows with a slot whose `Slot::is_empty` answers `true` while `drain_from` returns `Some( 42 )`, compiling without a warning; one sentence on `Peek::peek` would make it a contract and costs nothing today |
| EV22 | `ring_event` | n/a — coverage | `peek_agrees_with_the_slots_own_emptiness` visits two states per shape — fresh, and holding one payload — and misses three that matter: the slot after `clear`, which is what `recycle` produces and the whole third operation of the crate; the byte slot after a zero-length write, the one state where the two readings agree on an answer a caller may not expect and which `Peek`'s own doc documents at length; and the free-function path, since every assertion calls `.peek()` as a method while `drain_from` — the surface the module doc names as the ring's entry point — appears nowhere in the test, leaving the invariant pinned for the trait method and inferred for the documented one |
| EV23 | `ring_event` | n/a — doc gap | `Fill::fill` takes `&mut S` and its `# Errors` section is unusually thorough about which error and why `BatchTooLarge` differs from `Full`, while saying nothing about the slot's state after an `Err` — the sentence a caller most needs, since deciding whether a refused slot is still usable is the whole point of handling the error; the property holds because `BytesSlot::write` checks `payload.len() > N` and returns before touching either field, and three places in the family state it (a doctest, a test doc comment claiming "the slot is not half-updated", and this crate's assertion message) with the trait contract not among them, so the documented extension point admits an implementor that mutates and then fails and nothing would catch it |
| EV24 | `ring_event` | n/a — coverage | Both tests that check the refusal property use a `BytesSlot< 4 >` holding four bytes — this crate's `a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing` and `ring_slot`'s `a_failed_write_leaves_the_previous_contents_intact` — and every assertion in either reads `bytes[ ..len ]` and `len` via `read()`, `len()` or `is_empty()`, so the checks are total only because `len == N` leaves no tail; a probe at `BytesSlot< 8 >` holding two bytes shows the property still holds over both fields while the same assertion form would see only two of eight, meaning a `write` that corrupted the tail before returning `Err` would pass both suites, and `assert_eq!( slot, before )` after a clone covers the whole struct at any geometry in one line |
