# Pattern: Channel-to-Ring Binding

### Scope

- **Purpose**: Fix how a declarative channel description — a name, a scope, a payload kind, a handful of policy attributes — becomes one concrete, compile-time-typed ring instance, so a manifest can describe channels without a live process ever being asked to invent a new type.
- **Responsibility**: State the recurring problem, the two-part solution (an identity rule fixing which declarations share a ring, and a typing rule fixing how a ring's compile-time slot type is bound), the conditions under which each part applies to this crate specifically, and what adopting it costs.
- **In Scope**: The relationship between a named channel declaration and the ring instance realizing it, for the `scope: thread` case this crate implements; the compile-time-versus-declarative tension a generic ring slot type creates, and the two ways of resolving it.
- **Out of Scope**: The concrete ring mechanism itself — claim algorithm, cursor layout, memory ordering (→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md), [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md), [Publication Ordering](../invariant/002_publication_ordering.md)); whether a Disruptor-shaped ring is the mechanism that ships at all, which the family and this crate's own decisions (→ [decisions/readme.md](../decisions/readme.md)) still own; the overflow policy a full ring applies, already specified per priority class (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)); a declarative channel-manifest syntax as a shipped artifact — no manifest, parser, or build-time codegen exists in this crate or its consumers today, and this doc does not propose adding one, only records how an earlier design resolves the tension if one is ever built.

### Problem

A ring implementation in the Disruptor family is generic over its slot type,
fixed once at compile time — `RingBuffer<T>` — because that is what buys the
lock-free claim and the unsynchronized payload write this crate's own cost
model depends on (→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md)).
(This sentence said *wait-free* until a later revision; the claim is a compare-exchange loop
and always had to be on a bounded ring, a correction already made at the
family level. The correction does not touch the argument: what the
compile-time `T` buys is the *unsynchronized payload write*, which is
orthogonal to the claim's progress class.)
A declarative description of "what channels exist" — a name, where its
buffer lives, what kind of payload it carries, a handful of policy
attributes — is exactly the kind of thing a designer wants to keep as data,
inspectable and changeable without touching Rust source. The two pull
against each other: data can describe a new channel at any time; a generic
ring's `T` cannot come from data, because a new concrete type cannot appear
from a config file at runtime. Closing the gap by brute force costs
something no matter which side gives: a fat enum over every event's shape
adds a branch and worse cache behavior to the hottest path in the crate, and
an all-bytes slot throws away the type safety a generic slot exists to buy.

A second, related question sits next to the first: given several declared
channels, when do they share one ring and when does each need its own? The
failure mode of getting this wrong in either direction is concrete — one
ring for every declared stream means one full queue stalls all of them at
once, and a separate ring per declared *reader* multiplies ring count for
something a Disruptor ring already handles without one: a ring is
multicast, open to as many readers as needed.

### Solution

Two rules, independent of each other, both stated in earlier design work
for a channel-declaration language and recorded here as the shape a
`scope: thread` binding follows if this crate's ring ever becomes the
target of one.

**Rule 1 — ring identity is the declared channel, not the reader count.**
A reader is not a new ring: a ring is one channel, and readers are however
many systems consume from the same declared name. A worked example makes
the shape concrete: two systems, a mutator and a read-only auditor, both
declared as consumers of the same command channel — two consumers, one
ring. A second ring is warranted only when the *stream* itself differs, not
the reader count, and four concrete things make a stream count as
different: the policy (fail versus drop); the source (thread-local systems
versus the network); what gets written into the world's log; who may stall
whom. Stated as a rule directly: a new ring is a new channel; a new reader
of the same channel is one more consume, and the ring stays the same.
Stated at the whole-process grain: a ring is one channel instance, never
one global ring for the whole process — the narrower and more common form
of the same point being one ring per declared channel that has thread scope
or takes input from outside.

**On terminology: "thread buffer" names the buffer this rule calls
`scope: thread`, and means neither encryption nor the network.** An earlier
ambiguity around the abbreviation "TLS" was flagged and corrected in the
design work this pattern draws on: the intended meaning was always
thread-local storage — memory private to a thread — never Transport Layer
Security and never the network, and "thread buffer" is the term adopted
from that point on. `scope: remote` is the network case, realized by a
different crate entirely in the wider family; this crate's own ring is a
`scope: thread` realization, and "thread buffer" is the term this pattern
uses for it rather than the ambiguous "TLS."

**Rule 2 — a ring's compile-time slot type is bound one of two ways,
chosen by whether channels need distinct shapes.** The classical Disruptor
is generic over a compile-time `T`, and a new concrete type cannot appear
from a config file at runtime. Two resolutions, not one:

- **Shared T, config stays data.** The slot is always one shape — an
  opcode-and-bytes command, or an opaque blob. Different channels become
  different rings of that same `T`, differing only in capacity and
  overflow policy. The config can then remain data even without codegen.
  Declared channels differ in name, scope, capacity, and policy, never in
  the Rust type occupying the slot.
- **Codegen, when a genuinely distinct T is wanted.** The alternative
  compiles the channel manifest into Rust: a build step reads the
  declarative config and generates a module with a concrete `T` and a
  const capacity, zero cost, like a hand-written Disruptor. Changing
  capacity or adding a channel then means a rebuild, not a hot addition of
  a type in a live process. Codegen is needed specifically when a separate
  `T` per channel is wanted — two channels whose payloads are genuinely
  different shapes and sizes. The config is still the source of truth; it
  just passes through a build script rather than being read at runtime,
  and a live process never invents new types.

### Applicability

Applies wherever a declarative source of truth for "what channels exist"
is contemplated ahead of, or alongside, this crate's own ring type —
neither exists in this crate or its prospective consumers today, and
adopting this pattern is not itself a decision this doc makes. Two
conditions narrow it further for this crate specifically, and both must be
read before applying either rule above at face value:

- **Rule 1's reader-count half does not transfer to this crate as
  decided.** The ring-identity rule above holds a Disruptor ring
  open to as many readers as needed, with no new ring required per extra
  reader. This crate's own contract instead fixes exactly one consumer per
  ring: "a second consumer is out of contract, not a degraded mode" (→
  [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)).
  [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)
  independently names the same boundary — a second reader "splits the
  element stream between the two" rather than adding throughput, and is
  "deliberately out of contract here." What transfers from Rule 1 is only
  its other half — a new ring is warranted when the *stream* differs
  (policy, source, log-inclusion, stall relationship), never merely to add
  capacity for a reader this crate's own contract does not admit in the
  first place.
- **Whether this crate's own ring is the mechanism a `scope: thread`
  channel ultimately binds to is still open.** [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
  gates adoption on the family's own verdict against five other
  candidates, at least two of which — a shared exclusive-access buffer,
  declared read/write DAG scheduling — are not ring-shaped at all and
  would not face either rule above the same way. This pattern describes
  the binding a Disruptor-shaped winner would need; it is not evidence for
  which candidate wins.

Does not apply to `scope: remote` channels, whose ring-equivalent
structure and constraints belong to a different crate in the family
entirely and are not restated here — that crate's channels are QUIC
streams/datagrams, not slot arrays, and face no compile-time-`T` question
at all, because their payload is always opaque bytes with no type
parameter.

### Consequences

- **Ring count tracks declared streams, not declared readers, once a
  manifest exists — but the two-readers-one-ring case itself has no
  realization here.** The worked example Rule 1 cites (a mutator and a
  read-only auditor sharing one `commands` ring) costs nothing in ring
  count under the general rule; for this crate specifically it does not
  arise at all, because this crate admits one reader per ring, full stop,
  regardless of what the rule would otherwise permit.
- **The shared-T path costs nothing today, and the two open items it was
  said to be independent of have since closed in its favour.** Capacity is
  power-of-two-constrained (→ [Capacity](../type/002_capacity.md)) and the
  ring's overflow policy is Fail
  (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)),
  with Block and Drop left to the caller. Both closures *strengthen* the
  independence this consequence claimed rather than weakening it: `CAPACITY`
  is a runtime constructor argument rather than a const generic, so a manifest
  supplies it as data on either slot-typing path, and the per-class policy
  question moved off the ring entirely, so a manifest's `overflow` attribute
  is realized above the ring rather than compiled into it. The prediction held
  and the reason it held is now concrete.
- **The codegen path trades hot-reload for zero cost, and nothing in this
  crate's own docs currently needs to make that trade.** Changing capacity
  or adding a channel under codegen means a rebuild, not a hot addition of
  a type in a live process — a cost specific to wanting a genuinely
  distinct `T` per channel; this crate's own data structure is already generic over
  one `T` per ring instance (→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md),
  `slots` field) and states no requirement for multiple concrete `T`s to
  coexist, so nothing here currently forces the codegen path over the
  shared-T one.
- **The channel language's `batch` attribute has a direct realization, and
  this consequence's own prediction is what produced it.** A `batch`
  attribute describing how many slots the consumer takes at once implies a
  per-invocation ceiling. This instance recorded that
  [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)'s
  drain walked the entire published range with no cap, and that honoring
  `batch` as a hard per-call ceiling would need the drain algorithm to gain
  one. It gained one: `Consumer::drain_up_to( n )` takes at most `n`
  records and leaves the rest drainable, alongside the uncapped
  `Consumer::drain()`. A `batch` value of 256 binds to
  `drain_up_to( 256 )` directly.

  Worth naming as a method rather than as a win: the gap was found by writing
  down a shape difference between two documents that were each internally
  consistent, and declining to reconcile it. The unreconciled note is what
  survived to be actioned; a smoothed-over "these are compatible in spirit"
  would not have been.
- **The channel language's `overflow` values partially correspond to this
  crate's own overflow policy, under different names — but the
  correspondence is this doc's own bridge, not something either source
  states outright, and one leg of it is genuinely ambiguous rather than
  settled.** The channel language names three overflow values — fail,
  stall, drop — with drop gated to apply only where reliability is marked
  unreliable; for reliable channels the choice is stall or fail. One leg is
  unambiguous: `overflow: stall` ("wait for the consumer") is
  [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)'s
  Block policy exactly — the producer waits either way. The other two legs
  are this doc's own reading, not a source-stated equivalence:
  `overflow: fail` (framed as "the simulation has no right to silently
  lose intents") reads as Fail the claim, but an earlier gloss for the
  same attribute described it as "stop the world" — a halt, not the
  non-blocking per-call return Fail the claim actually is — and the two
  framings are not reconciled here. `overflow: drop` is read as Overwrite
  oldest because a ring-claim architecture was assumed to have no cheap
  way to reject only the newest arrival without a pre-claim check — but
  neither framing says whether "drop" destroys an already-published slot
  or discards the new element before it is written, and this pattern does
  not resolve that gap, only names it. Where the existing NFR's match is
  unambiguous (Block) the correspondence is stated as fact; where it is not
  (Fail the claim, Overwrite oldest) it is this doc's own reading, not the
  source's. The existing NFR remains authoritative regardless; this pattern
  does not restate its content, only offers a terminology bridge with its
  own uncertainty flagged where uncertainty exists.

  **The implementation changed one leg of this and sharpened a second.**
  `overflow: fail` now binds to something concrete: `Producer::push` returns
  `Err( RingError::Full )` carrying nothing and destroying nothing, which
  matches the "simulation has no right to silently lose intents" reading
  and not the "stop the world" one. The two framings are still unreconciled
  *as documents* — this crate simply cannot implement the "stop the world"
  reading, since a ring has no mechanism to halt anything outside itself.
  That is a narrowing by construction rather than a resolution by argument,
  and it is the weaker of the two.

  The `overflow: drop` leg's ambiguity, by contrast, turned out to be the more
  important half and it resolved the *other* way. This doc read "drop" as
  overwrite-oldest because a ring-claim architecture was assumed to have no
  cheap way to reject only the newest arrival without a pre-claim check.
  The premise is false: the claim's capacity check is
  *fused into* the compare-exchange rather than preceding it, so rejecting the
  newest arrival costs nothing extra — it is the failure return the claim
  already has. `let _ = producer.push( value )` is drop-newest, one line, no
  pre-claim check. So both readings of "drop" are cheap, the original
  ambiguity is still unresolved, and this crate's answer is that drop-newest
  is available to any caller while overwrite-oldest is refused to all of them
  (→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)).
  The reasoning that produced the wrong reading is worth keeping visible: it
  inferred a cost from an architecture rather than from the architecture's
  actual claim operation, and the actual operation was not written yet.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | Its drain walks the full published range in one call, unbounded — the channel language's `batch` attribute would cap it per call, a difference not reconciled here |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The generic slot type `T` Rule 2 binds — already fixed per ring instance, which is why the shared-T path costs this crate nothing today |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | Fixes exactly one consumer per ring, which is why Rule 1's reader-count half never activates here regardless of how many systems might declare a consume |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | Gates whether a Disruptor-shaped ring — the only shape either rule above assumes — is the mechanism a `scope: thread` channel ultimately binds to at all |
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | The authoritative overflow policy; this pattern only bridges its terminology to the channel language's `overflow` attribute, never restates it |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Consumer::drain_up_to` is the `batch` attribute's realization; `Producer::push`'s `Err( RingError::Full )` is `overflow: fail`'s. No manifest, no codegen, and no channel type exists — this crate supplies the primitives a binding would use, not the binding |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::drain_up_to_zero_takes_nothing_and_leaves_the_records_drainable` | The `batch` cap's degenerate end — a zero ceiling takes nothing and destroys nothing, which is what makes the attribute safe to bind directly from a manifest value |
| `tests/mpsc_test.rs::drain_up_to_more_than_capacity_is_capped_rather_than_scanning_past_the_ring` | The `batch` cap's other end — a ceiling above the ring's own size is harmless, so a manifest need not validate `batch` against `capacity` |
| `tests/mpsc_test.rs::a_claim_past_capacity_reports_full_rather_than_overwriting` | `overflow: fail` as this crate realizes it, and simultaneously the refusal of the overwrite-oldest reading of `overflow: drop` |
| `tests/mpsc_test.rs::a_config_supplies_the_capacity_and_its_other_fields_are_deliberately_unread` | The shared-T path's premise: capacity arrives as data at construction, so a manifest needs no codegen to vary it |

### MP41 — The Binding Pattern Has One Instantiation and It Is `ring_core`

A pattern documented from a single instance is worth marking as such. The
second candidate — a `ring_handle`-style narrowing — deliberately does not
forward the multi-producer capability at all, so it does not instantiate the
pattern.

Until a second binding exists, the pattern's rules are a description of
`ring_core`'s MPSC arm rather than a generalization tested against
variation.
