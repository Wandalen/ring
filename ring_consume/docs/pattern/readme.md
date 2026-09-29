# pattern

Two shapes this crate shares with siblings. The first is a value — start plus
length, sixteen bytes, `Copy`, half-open — which three crates arrived at
independently and agreed on in everything but the length type. The second is a
protocol — ask, act, report — which the two halves of the handshake run in
opposite directions.

Both instances end at the same place: the convergence is real and strong, and
the one thing nobody wrote down is that it is a pattern at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Half-Open Range as a Value](001_the_half_open_range_as_a_value.md) | CN39, CN40 — six shared decisions and one divergent length type, and the `must_use` rule the family follows four times and states never |
| 002 | [Read Then Report](002_read_then_report.md) | CN41, CN42 — neither report operation taking the value its ask produced, and an exact fallibility inversion between the halves |

### Where the Seam Falls

| Phase | Write half | Read half |
|-------|-----------|-----------|
| Ask | `claim( count ) -> Result< Claim, RingError >` | `available() -> Available` |
| Act | write the slots | read the slots |
| Report | `publish( start, len ) -> Seq` | `commit( through ) -> Result< Seq, RingError >` |

Read down each column and the two halves look like the same protocol. Read
across and the fallibility inverts exactly: the write half can fail to *get* and
never fails to *report*; the read half never fails to get and can fail to report.

Both are right — fallibility tracks where a caller can actually be wrong — and
the result is that four operations that form one protocol do not look like one
protocol from any call site.

### The Family's Range Type, Three Times

| | `Claim` | `BatchClaim` | `Available` |
|--|--------|--------------|-------------|
| Crate | `ring_claim` | `ring_batch` | `ring_consume` |
| Start | `Seq` | `Seq` | `Seq` |
| Length | `usize` | `usize` (`count`) | **`u64`** |
| Size | 16 | 16 | 16 |
| `Copy` | ✔ | ✔ | ✔ |
| Derived `end()` | ✔ | ✔ | ✔ |
| Iterator | ✔ | ✔ | ✔ |
| Messaged `must_use` | ✔ severe | **none** | none — correctly |

Six of seven rows agree. The length type is the divergence, and `Available`'s
`u64` is arguably the correct one — `ring_seqno::pending` returns `u64` and
`Available::len` is exactly that value — which means the newest instance got it
right and the other two are the majority.

### The Rule the Family Follows and Never States

Every messaged `must_use` in all 33 crates names a consequence of *dropping* the
value, and there are exactly four. `Available` correctly has none: dropping a
permission costs nothing. `Claim` correctly has the most severe: dropping an
obligation strands slots forever.

`BatchClaim` carries `Claim`'s consequence and no annotation at all. That is a
`ring_batch` finding, visible only from here, because only the three-way
comparison makes the rule legible in the first place.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the three range types, side by side
grep -A5 'pub struct Claim$'      ring_claim/src/lib.rs
grep -A5 'pub struct BatchClaim'  ring_batch/src/lib.rs
grep -A5 'pub struct Available'   ring_consume/src/lib.rs

# every messaged must_use in the family — the rule, and its one gap
command grep -r 'must_use = ' ring_*/src/*.rs | sed 's|ring/||'

# the four operations of the protocol, with their fallibility
grep -E '^\s*pub (const )?fn' ring_publish/src/lib.rs
grep -E '^\s*pub fn (claim|claim_up_to)' ring_claim/src/lib.rs
grep -E '^\s*pub fn (available|commit)' ring_consume/src/lib.rs

# the one test that exercises all four
grep -c 'fn ' ring_publish/tests/handshake_test.rs

# the drop-guard alternative, where the family took it
grep 'must_use = ' ring_spsc/src/lib.rs
```

Live output:

```
pub struct Claim
{
  start : Seq,
  len : usize,
}

pub struct BatchClaim
{
  start : Seq,
  count : usize,
}

pub struct Available
{
  start : Seq,
  len : u64,
}

ring_atomic/src/lib.rs:  #[ must_use = "the returned sequence is the claim — dropping it claims a range nobody will use" ]
ring_claim/src/lib.rs:#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_flush/src/lib.rs:#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
ring_shutdown/src/lib.rs:  #[ must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else" ]
ring_shutdown/src/lib.rs:  #[ must_use = "this is the record itself, not a copy — dropping it loses it" ]
ring_shutdown/src/lib.rs:#[ must_use = "a Wake::Closed means stop, not publish" ]
ring_slot/src/lib.rs:  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
ring_spsc/src/lib.rs:#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again" ]
ring_testkit/src/lib.rs:  #[ must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed" ]
  pub fn new() -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub fn published( &self ) -> Seq
  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  pub fn is_published( &self, seq : Seq ) -> bool
  pub fn claimed( &self ) -> Seq
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
14
#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN39 | family | n/a — duplication | Three crates converged on six decisions and diverged on the length type; the minority instance is the one that matches `ring_seqno`'s return type |
| CN40 | `ring_batch` | n/a — coverage | The family's `must_use` rule — a range whose drop strands a slot gets a messaged annotation — is followed four times, stated never, and `BatchClaim` is its one gap |
| CN41 | family | n/a — doc gap | Neither half's report operation takes the value its ask produced, so `Claim`'s `must_use` is the entire obligation mechanism rather than a supplement to one |
| CN42 | family | n/a — doc gap | The two halves have exactly inverted fallibility in both phases; every cell is right and nothing records that the pattern is a pattern |
