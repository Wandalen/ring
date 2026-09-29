# Pattern: Try-and-Loop Over Compare-Exchange

### Scope

- **Purpose**: Record the `try_X` / `X` pairing as this crate instantiates it, and place it against the twelve other `try_*` methods in the family — none of which has a blocking twin.
- **Responsibility**: State the pattern's three parts, show the census, identify what makes this crate the only legitimate instance, and record the two ways the pattern is normally wrong.
- **In Scope**: The `try_publish` / `publish` pairing and the loop that connects them.
- **Out of Scope**: Why the loop needs no budget — see [`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md).

### The Pattern, in Three Parts

1. **A fallible primitive** that attempts one compare-exchange and reports the
   outcome without retrying — `try_publish`.
2. **A blocking wrapper** that calls it in a loop until it succeeds, discarding
   the failure information — `publish`.
3. **A hint, not a yield**, between attempts — `core::hint::spin_loop()`.

```rust
pub fn publish( &self, start : Seq, len : usize ) -> Seq
{
  loop
  {
    if let Ok( end ) = self.try_publish( start, len )
    {
      return end;
    }
    core::hint::spin_loop();
  }
}
```

Nine lines, and the whole pattern. The wrapper adds no state, no parameters and
no error type; it converts *"not yet"* into *"eventually"* and nothing else.

Two properties make the conversion legitimate here, and both are unusual:

- **The retry target is fixed.** `start` and `len` do not change between
  attempts. Contrast `ring_claim`, whose retry loop feeds the failure value back
  in as the next attempt's `current`
  ([`algorithm/001`](../algorithm/001_the_compare_exchange_that_refuses.md)
  § PB7) — that is a race for a moving target, this is a wait for a turn.
- **Success is guaranteed, not merely likely.** The predecessor is committed to
  publishing, so *"loop until it works"* is a termination argument rather than
  optimism
  ([`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md)).

### PB33 — Thirteen `try_*` Methods in the Family, and Exactly One Has a Blocking Twin

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^\s*pub (const )?fn try_[a-z_]+' ring_*/src/*.rs
grep -rE '^\s*pub (const )?fn (push|push_batch|recv|recv_batch|clone|publish)\b' ring_*/src/*.rs
```

Live output:

```
ring_core/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), T >
ring_core/src/lib.rs:  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
ring_core/src/lib.rs:  pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
ring_core/src/lib.rs:  pub fn try_recv( &mut self ) -> Option< T >
ring_core/src/lib.rs:  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
ring_handle/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), T >
ring_handle/src/lib.rs:  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
ring_handle/src/lib.rs:  pub fn try_recv( &mut self ) -> Option< T >
ring_handle/src/lib.rs:  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
ring_publish/src/lib.rs:  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
ring_shutdown/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), Refusal< T > >
ring_shutdown/src/lib.rs:  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
ring_spsc/src/lib.rs:  pub fn try_push( &mut self, record : T ) -> Result< (), T >
ring_mpsc/src/lib.rs:  pub fn push( &self, value : T ) -> Result< Seq, RingError >
ring_poll/src/lib.rs:  pub fn push< T : Send >( &mut self, producer : &mut Producer< '_, T >, record : T )
ring_poll/src/lib.rs:  pub fn push_batch< T : Send >
ring_poll/src/lib.rs:  pub fn recv< T : Send >( &mut self, consumer : &mut Consumer< '_, T > ) -> Option< T >
ring_publish/src/lib.rs:  pub fn publish( &self, start : Seq, len : usize ) -> Seq
ring_tls/src/lib.rs:  pub fn push( &mut self, item : T ) -> Result< (), RingError >
```

| Crate | `try_*` methods | Blocking twin of the same name |
|-------|-----------------|-------------------------------|
| `ring_core` | `try_push`, `try_push_batch`, `try_clone`, `try_recv`, `try_recv_batch` | **none** |
| `ring_handle` | `try_push`, `try_push_batch`, `try_recv`, `try_recv_batch` | **none** |
| `ring_shutdown` | `try_push`, `try_push_batch` | **none** |
| `ring_spsc` | `try_push` | **none** |
| **`ring_publish`** | **`try_publish`** | **`publish`** |

Thirteen `try_*` methods across five crates. Twelve of them are the *only* form
their operation has — there is no `Producer::push` to go with
`Producer::try_push`, no `Consumer::recv` to go with `try_recv`.

The three blocking-sounding `push` methods that do exist belong to types with no
`try_` twin at all, and none of them loops:

| Method | Returns | Blocks |
|--------|---------|:------:|
| `ring_mpsc::Ring::push` | `Result< Seq, RingError >` | no — `Err( Full )` immediately |
| `ring_tls::TlsBuffer::push` | `Result< (), RingError >` | no — staging, not a ring write |
| `ring_poll::…::push` | — | no — a tick path, blocking is the thing it forbids |

So the family's rule is **the caller decides what to do when it cannot proceed**,
and `ring_publish` is the single documented exception. The exception is defensible
for exactly the reason the rule exists: everywhere else, "cannot proceed" means
*the ring is full* or *there is no data*, both of which are states a peer may
never leave. Here it means *it is not your turn yet*, and turns always arrive.

`src/lib.rs:183-189` states the trade the exception makes:

> Never. The loop exits when the predecessor publishes, which it is committed to
> doing; a caller that publishes a range it never claimed deadlocks here instead,
> which is a caller bug this crate cannot detect — `try_publish` is the variant
> for a caller that wants to decide for itself.

The last clause is the concession: keeping the fallible form public means the
family's rule is still available to anyone who wants it, and the blocking form is
an affordance rather than a mandate.

### The Two Ways This Pattern Is Normally Wrong

| Failure | What it looks like | Why this instance avoids it |
|---------|--------------------|-----------------------------|
| **Livelock** | two parties each retry a CAS that the other keeps invalidating; neither makes progress | the retry target is fixed and the frontier is monotone, so each attempt's precondition can become true and never becomes false again |
| **Silent unboundedness** | a loop that "usually" ends, documented as if it always does | the `# Panics` section names the precondition that makes it end and the failure mode when it is broken ([`item/002`](../item/002_the_two_publications.md) § PB23) |

The first is what makes bare CAS loops dangerous in general and is why
`ring_claim`'s loop — which *is* a race — carries an explicit `Full` exit rather
than spinning forever. The second is a documentation failure rather than a code
one, and is the reason
[`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md) exists.

### The Loop Census, and What the Others Are

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs; do
  n=$( grep -vE '^[[:space:]]*//' "$f" \
       | grep -cE '^[[:space:]]*loop[[:space:]]*\{?[[:space:]]*$' )
  [ "$n" -gt 0 ] && printf "%-40s %s\n" "$f" "$n"
done
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_bench/src/lib.rs               5
ring_publish/src/lib.rs             1
```

| Crate | Bare `loop` blocks | Exit condition | Depends on another thread |
|-------|-------------------:|----------------|:-------------------------:|
| `ring_bench` | 5 | `if taken == 0 { break }` — drain until empty | **no** |
| **`ring_publish`** | **1** | `if let Ok( .. ) { return }` — a peer's publication | **yes** |

All five `ring_bench` loops are the same shape: `let taken = consumer.drain()
.len()` or `try_recv_batch`, then break on zero. They end when the *local*
observation says there is nothing left, which is a condition the loop's own thread
establishes.

`publish`'s loop is the only one in the family whose exit condition is another
thread's action. That is the structural fact underneath every argument in
[`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md) and
[`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md):
it is not that the loop is long, it is that its termination is somebody else's
responsibility.

### Where the Pattern Is Checked

| Check | What it establishes |
|-------|--------------------|
| `tests/publish_test.rs:116-124` | The wrapper returns what the primitive would have |
| `tests/publish_test.rs:126-155` | The wrapper waits, and nothing but the predecessor releases it |
| `tests/publish_test.rs:157-181` | Four wrappers spinning at once converge to a contiguous frontier |
| `tests/manual/readme.md § P4` | The loop retries the same exchange rather than scanning for a reorderable position |

P4 is the one that keeps the pattern *this* pattern. Its grep bans `while`, `for`,
`max(`, `contiguous`, `bitmap` and `highest` from `src/lib.rs`, so a `publish`
that grew a scan — the natural first step toward the rejected designs — fires the
check rather than passing silently.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | Part 1 of the pattern, and the contrast with `ring_claim`'s moving target |
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | Part 2, and the termination argument |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_plain_spin_rather_than_a_wait_kind.md](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | Why part 3 is a hint and not a strategy |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | Both halves of the pair, contract by contract |

### Patterns

| File | Relationship |
|------|--------------|
| [002_the_named_ordering_constant.md](002_the_named_ordering_constant.md) | The other pattern the same nine lines instantiate |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The precondition that turns the loop from bounded to infinite |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:161-210` | Both halves, and the `# Panics` concession |
| `ring_claim/src/lib.rs` | The same primitive used as a race rather than a turn-gate |
| `ring_core/src/lib.rs:373-620` | Five `try_*` methods with no blocking twin |
| `ring_bench/src/lib.rs:1092,1144,1176,1226,1262` | The family's other five bare loops, all locally bounded |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:116-124` | Wrapper and primitive agreeing |
| `tests/publish_test.rs:126-155` | The wait, isolated to one releasing event |
| `tests/publish_test.rs:157-181` | Four concurrent wrappers converging |
| `tests/manual/readme.md § P4` | No scan, no second repetition |
