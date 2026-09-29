# Item: Twelve Verbs, Eight of Them a Bare Forward

### Scope

- **Purpose**: Catalogue the crate's public methods and establish, per method, whether it forwards, narrows, or adds.
- **Responsibility**: State each verb's receiver, its body, and what it does that its callee does not.
- **In Scope**: The twelve `pub fn` declarations and the `Iterator` impl; the one method deliberately absent.
- **Out of Scope**: The structs carrying them (→ [`item/001`](001_five_nouns_four_of_them_the_same_width.md)); the reason for withholding (→ [`pattern/001`](../pattern/001_enforce_by_withholding.md)).

### The Twelve

| Verb | Receiver | Body | Kind |
|------|----------|------|------|
| `Split::new` | — | `Self { ring }` | Construct |
| `Split::ends` | `&mut self` | `Ends { inner : self.ring.ends() }` | Forward + rewrap |
| `Ends::split` | `&'a mut self` | destructure, rewrap both halves | Forward + rewrap |
| `Producer::try_push` | `&mut self` | `self.inner.try_push( record )` | Forward |
| `Producer::try_push_batch` | `&mut self` | `self.inner.try_push_batch( records )` | Forward |
| `Producer::free_capacity` | `&self` | `self.inner.free_capacity()` | Forward |
| `Producer::is_full` | `&self` | `self.inner.is_full()` | Forward |
| `Consumer::try_recv` | `&mut self` | `self.inner.try_recv()` | Forward |
| `Consumer::try_recv_batch` | `&mut self` | `self.inner.try_recv_batch( out )` | Forward |
| `Consumer::len` | `&self` | `self.inner.len()` | Forward |
| `Consumer::is_empty` | `&self` | `self.inner.is_empty()` | Forward |
| `Consumer::drain` | `&mut self` | snapshot `len()`, hand it to `Drain` | **Add** |

Plus `Drain::next` and `Drain::size_hint`, which are the `Iterator` impl and the
only place in the crate where control flow exists.

### The One That Is Not Here

`ring_core::Producer::try_clone` returns `Option< Producer< 'a, T > >`. It is
not forwarded, and no method on this crate's `Producer` reaches it. That single
omission is the crate's entire reason for existing
(→ [`decisions/001`](../decisions/001_what_this_crate_is_for.md)), and it is
asserted by `tests/ui/producer_try_clones.rs` rather than by anything positive.

### Items

| File | Relationship |
|------|--------------|
| [001_five_nouns_four_of_them_the_same_width.md](001_five_nouns_four_of_them_the_same_width.md) | The structs these verbs hang off |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | The forwarding rule these eleven follow |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The four publishing verbs, as a contract |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The five draining verbs |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_forward_narrow_or_add.md](../pattern/002_forward_narrow_or_add.md) | The three-way classification this table applies |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The twelve declarations and their bodies |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | The eleven callees, plus the one that has no caller here |

### Tests

| Test | Relationship |
|------|--------------|
| `drain_is_bounded_at_the_call_that_made_it` | The one added verb's added semantics |
| `free_capacity_and_is_full_agree` | Two forwards, cross-checked against each other |

### HD27 — Eight Bare Forwards, Two Rewrappings, One Addition, and a Subtraction That Is the Point

The bodies are countable and the count is the crate:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- bodies that are a bare forward and nothing else --'
command grep -E '^    self\.inner\.[a-z_]+\(' ring_handle/src/lib.rs
echo '  -- public methods declared here --'
command grep -cE '^  pub (const )?fn ' ring_handle/src/lib.rs
echo '  -- the four that are not a bare forward --'
for f in 'pub const fn new' 'pub fn ends' 'pub fn split' 'pub fn drain'; do
  printf '  %-16s ' "${f##* }"
  sed -n "/  $f(/,/^  }/p" ring_handle/src/lib.rs \
    | command grep -vE '^\s*(///|\{|\}|  pub )' | sed 's/^ *//' | tr '\n' '; '
  echo
done
echo '  -- the method that is not forwarded --'
command grep 'try_clone' ring_core/src/lib.rs ring_handle/src/lib.rs \
  | command grep -vE ':\s*(///|//!)' | sed 's|ring/||'
```

Live output:

```
  -- bodies that are a bare forward and nothing else --
    self.inner.try_push( record )
    self.inner.try_push_batch( records )
    self.inner.free_capacity()
    self.inner.is_full()
    self.inner.try_recv()
    self.inner.try_recv_batch( out )
    self.inner.len()
    self.inner.is_empty()
  -- public methods declared here --
12
  -- the four that are not a bare forward --
  new              Self { ring };
  ends             Ends { inner : self.ring.ends() };
  split            let ( producer, consumer ) = self.inner.split();;( Producer { inner : producer }, Consumer { inner : consumer } );
  drain            let remaining = self.inner.len();;Drain { consumer : self, remaining };
  -- the method that is not forwarded --
ring_core/src/lib.rs:  pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
```

Eight bodies are literally `self.inner.<same name>( <same args> )` and nothing
else. Two more — `Split::ends` and `Ends::split` — forward and rewrap the
result. `Split::new` constructs. `Consumer::drain` adds. That is twelve.

**The subtraction does not appear in any of these counts, and it is the crate.**
`try_clone` exists on `ring_core::Producer`, is public, is tested there, and has
no forwarder here. A catalogue of what a crate declares cannot show what it
declines to declare, which is why this instance names it explicitly: the reader
who counts ten forwards and asks "why not just re-export?" has the answer only
if the eleventh — the one that is absent — is on the page.

### HD28 — The Added Verb Is the Only One That Can Be Wrong

`drain` is the single method whose behaviour is not its callee's:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the added method, in full --'
sed -n '/pub fn drain( &mut self )/,/^  }/p' ring_handle/src/lib.rs
echo '  -- and the only branch in the crate --'
command grep -vE '^\s*(///|//!)' ring_handle/src/lib.rs \
  | command grep -E '^\s*(if|match|while|for|return) '
echo '  -- tests whose body drains --'
awk '/^fn /{n=$2; sub(/\(.*/,"",n)} /\.drain\(/{print "  "n}' ring_handle/tests/handle_test.rs | sort -u
```

Live output:

```
  -- the added method, in full --
  pub fn drain( &mut self ) -> Drain< '_, 'a, T >
  {
    let remaining = self.inner.len();
    Drain { consumer : self, remaining }
  }
  -- and the only branch in the crate --
    if self.remaining == 0
      return None;
  -- tests whose body drains --
  draining_at_the_same_point_is_deterministic
  drain_is_bounded_at_the_call_that_made_it
  drain_of_an_empty_ring_yields_nothing
  drain_stops_when_the_ring_empties_first
  every_handle_can_be_printed
  the_crossbeam_backend_behaves_alike
  the_in_house_backends_behave_alike
```

`drain` snapshots `self.inner.len()` at the moment of the call and hands that
number to `Drain`, which decrements it. The bound is therefore *the length when
`drain` was called*, not the length as iteration proceeds — a producer pushing
concurrently cannot extend the iteration, and a ring that empties early ends it
sooner.

**Both halves of that are tested and the pairing is the interesting part.**
`drain_is_bounded_at_the_call_that_made_it` covers the upper bound;
`drain_stops_when_the_ring_empties_first` covers the lower. Between them they
pin a semantics that has no counterpart below, which is why they are among the
few tests in the crate that could fail for a reason that is not `ring_core`'s.

**Seven tests call `.drain()` and only four are about `Drain`.** The other three
— `every_handle_can_be_printed`, `the_in_house_backends_behave_alike`,
`the_crossbeam_backend_behaves_alike` — reach for it as the convenient way to
read a ring out, so a regression in the snapshot bound would fail seven tests
while three of the failures pointed at unrelated claims. That is a diagnostic
cost, not a correctness one, and it is invisible from the test names.

The single `if` in the crate is `Drain::next`'s `remaining == 0` check. The ten
forwards contribute no branch at all, so every other behaviour this crate
appears to have is a behaviour it inherits — and a bug found in any of them is,
by construction, a bug to file one crate down.
