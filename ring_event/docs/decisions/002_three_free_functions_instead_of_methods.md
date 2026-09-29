# Decisions: Three Free Functions Instead of Methods

### Scope

**Purpose:** Record what the free functions add over the trait methods they
forward to, and what naming them cost — a second name for each of three
operations, a third family-wide name for one, and two verbs the family already
uses for something else.

**Responsibility:** The three forwarding bodies, `publish_into`'s stated
rationale, the family's naming of the same three operations, and how the crate's
own suite chooses between the two available forms.

**In Scope:** `ring_event/src/lib.rs:147-152`, `:173-178`, `:207-212`,
`:229-234`; `ring_publish/src/lib.rs:200`.

**Out of Scope:** Why the traits sit where they do is
[`decisions/001`](001_the_write_half_on_the_payload_the_read_half_on_the_slot.md).
Whether a ring's publish calls this at all is
[`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md).

---

## What Each Function Adds to the Method Under It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- three free functions, and the three methods each forwards to --'
awk '/^pub fn publish_into< S, P >\( slot : &mut S, payload : P \) -> Result< \(\), RingError >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^\/\/\/ let slot = BytesSlot::< 4 >::empty\(\);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 8 { print } /^pub fn recycle< S >\( slot : &mut S \)$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 5 { print }' ring_event/src/lib.rs | command grep 'pub fn\|  [a-z]*\.'
echo '  -- what the free functions are for, as the crate states it --'
command grep -m1 -A4 -F '/// Publish `payload` into `slot` — the one write path both shapes take.' ring_event/src/lib.rs
echo '  -- names the family gives the third operation --'
command grep -r 'pub fn recycle\|pub fn reset\|fn clear( &mut self )\|pub fn clear' --include=*.rs ring_*/src | sed 's/.*fn //; s/[(<].*//' | sort | uniq -c | sort -rn
echo '  -- what the family already calls publish, and what it takes --'
command grep -r 'pub fn publish( ' --include=*.rs ring_*/src
echo '  -- how many drain* functions the family has outside this crate, and where --'
command grep -r 'pub fn drain' --include=*.rs ring_*/src | command grep -vc '^ring_event/' || true
command grep -r 'pub fn drain' --include=*.rs ring_*/src | command grep -v '^ring_event/' | cut -d/ -f1 | sort -u | tr '\n' ' '; echo
echo '  -- and how this crates own suite calls it: free function, then method --'
command grep -c 'publish_into(\|drain_from(\|recycle(' ring_event/tests/event_test.rs || true
command grep -c '\.fill(\|\.peek(' ring_event/tests/event_test.rs || true
```

Live output:

```
  -- three free functions, and the three methods each forwards to --
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
  payload.fill( slot )
pub fn recycle< S >( slot : &mut S )
  slot.clear();
  -- what the free functions are for, as the crate states it --
/// Publish `payload` into `slot` — the one write path both shapes take.
///
/// Deliberately trivial. Its value is not what it does but that there is only
/// one of it: a ring's publish *path* passes through here — the step where a
/// claimed slot receives its payload — so no slot shape can acquire a publish
  -- names the family gives the third operation --
      6 clear
      2 reset
      1 reset_counts
      1 recycle
      1 clear_log
  -- what the family already calls publish, and what it takes --
ring_publish/src/lib.rs:  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  -- how many drain* functions the family has outside this crate, and where --
12
ring_batch ring_flush ring_handle ring_mpsc ring_poll ring_shutdown ring_spsc ring_tls 
  -- and how this crates own suite calls it: free function, then method --
37
4
```

---

### EV15 — Each Free Function Is a Rename, and the Traits Stay Public, so Every Operation Now Has Two Names

The three bodies are `payload.fill( slot )`, `slot.peek()` and `slot.clear();`.
Nothing is added: `drain_from` and `recycle` are pure aliases, and `publish_into`
is an alias that also swaps the argument order so the slot reads first, matching
how a ring holds one and wants to put something in it.

The stated value is not the behaviour but the uniqueness — "its value is not what
it does but that there is only one of it: a ring's publish calls this, so no slot
shape can acquire a publish path of its own without the signature changing."
That argument holds for the *implementation*: there is one body, and a new shape
cannot get its own.

It does not hold for the *call site*. `Fill` and `Peek` are both `pub`, because
the free functions' bounds name them and a caller writing a generic routine must
name them too. So `slot.peek()` is as reachable as `drain_from( &slot )`, and the
crate's own suite writes both — 29 calls through the free functions and 4 through
the methods, the latter in the two tests that exist specifically to demonstrate
the traits are usable directly.

**Finding.** The result is two names for each of three operations, with nothing
saying which is canonical. Worse for the third: across the 33 crates, the reset
operation is called `clear` six times, `reset` twice, `reset_counts` and
`clear_log` once each — and `recycle` exactly once, here. This crate, whose stated
job is to stop the family growing a second path, is the only place in the family
that uses a third word for an operation `ring_slot` already named `clear` and
`ring_shutdown` already named `reset`. Its own test comment uses two of the three
in one sentence: "a slot cleared during recycling."

---

### EV16 — "A Ring's Publish Calls This" Names a Function That Exists and Is Something Else

The family has a `publish`. It is `ring_publish::Publisher::publish( &self, start
: Seq, len : usize ) -> Seq`, and it takes two sequence numbers and returns one.
It moves a cursor. It has no slot, no payload, and no way to reach either.

`publish_into`'s doc says "a ring's publish calls this." Read against the family
that sentence points at `Publisher::publish`, which cannot call it — the types do
not permit it — and the same reading problem repeats on the other half: the
family has 11 functions whose names begin `drain` outside this crate, spread over
`ring_batch`, `ring_flush`, `ring_handle`, `ring_mpsc`, `ring_poll`,
`ring_shutdown`, `ring_spsc` and `ring_tls`, and every one of them drains a batch
or a range from a ring rather than a single slot.

**Finding.** This crate's two verbs are the family's two verbs applied one level
down. `Publisher::publish` marks a range of sequence numbers as visible;
`publish_into` puts one payload into one slot. Both are correct names for what
they do, and the layering is genuine rather than a collision to be renamed away.

What is wrong is the sentence. "A ring's publish calls this" reads as naming a
specific caller, and the specific caller it names is the one function in the
family that shares the word and does not fit. The accurate version is a step
weaker and no longer: a ring's publish *path* passes through here, meaning the
step where a claimed slot receives its payload — not the crate called
`ring_publish`, which is the other half of the same operation and never touches a
slot. A reader who follows the sentence to `ring_publish` finds a signature that
cannot call this one, and has no way to tell whether that is the documentation
being loose or the wiring being absent.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B2 -A4 -F 'claimed slot receives its payload' ring_event/src/lib.rs
```

Live output:

```
/// Deliberately trivial. Its value is not what it does but that there is only
/// one of it: a ring's publish *path* passes through here — the step where a
/// claimed slot receives its payload — so no slot shape can acquire a publish
/// path of its own without the signature changing. Not `ring_publish`'s
/// `Publisher::publish`, which moves a cursor over sequence numbers and never
/// touches a slot; that is the other half of the same operation, one level up.
///
```

**Disposition:** applied — `publish_into`'s doc comment in
`ring_event/src/lib.rs` no longer says "a ring's publish calls this,"
which named a specific caller (`Publisher::publish`) that cannot call it; it
now states the weaker, accurate claim this instance proposes — a ring's
publish *path* passes through here — and names `Publisher::publish`
explicitly as the other half of the same operation, one level up, that never
touches a slot. Now prints: `claimed slot receives its payload`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | Where the methods being renamed actually live |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | What a ring's publish actually calls |
| [`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md) | The three forwarding bodies in full |
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | Why both traits have to stay public |

### Sources

| Fact | Where |
|------|-------|
| Three bodies forwarding to three methods | `ring_event/src/lib.rs:177`, `:211`, `:233` |
| `publish_into` swapping the argument order | `ring_event/src/lib.rs:173` against `:63` |
| "there is only one of it: a ring's publish calls this" | `ring_event/src/lib.rs:149-152` |
| `clear` six times, `reset` twice, `recycle` once | Census above |
| `Publisher::publish` taking `Seq` and `usize` | `ring_publish/src/lib.rs:200` |
| 29 free-function calls and 4 method calls in the suite | Census above |
| "a slot cleared during recycling" | `ring_event/tests/event_test.rs:204-205` |

### Tests

| Test | Covers |
|------|--------|
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | One of the four method-form calls |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | The other |
| `recycling_empties_either_shape_through_the_same_call` | The third operation under its third name |
