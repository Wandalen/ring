# decisions

Open questions and the closed ones worth keeping the reasoning for. Every entry
is either **Closed** with what settled it, or **Pending** with what would settle
it.

## Closed

### Closed 1 — `Entry` or `insert`?

**`Entry`.** `HashMap::insert` maintains one-value-per-key by *replacing*, and a
discarded return value drops the previous ring and every unread record in it. The
implementations are indistinguishable to every count-based and lookup-based
test; only a drop counter separates them, which is why this crate's own
invariant record asks for one.
→ [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md).

### Closed 2 — Should `register` hand the rejected ring back?

**Yes.** Returning `Result< (), ( RegistryError, Split< T > ) >` rather than
`Result< (), RegistryError >` means a name collision does not destroy the
caller's ring.

**Two costs, both measured rather than predicted, and both discovered by a
failing build rather than by reasoning about the signature:**

| # | Cost | How it surfaced |
|---|---|---|
| 1 | A `Debug` bound on the record type at `.unwrap()`/`.expect()` sites | Three tests failed to compile — `Result::expect` requires `E : Debug`, and `E` now transitively contains `T` |
| 2 | A `Result` **448 bytes** wide, on the `Ok` path as well as the `Err` path | `clippy::result_large_err` failed the build under `-D warnings`, and reported the width |

A caller who matches rather than `.expect()`s pays nothing for cost 1. Nobody
avoids cost 2 — but `register` is a setup-time call, once per ring, never in a
loop, so the width lands where it does not matter.

**Cost 2 is suppressed rather than fixed, and the suppression is narrow.** The
lint offers two remedies, and each undoes the decision this entry records:
shrinking the payload means dropping the ring, which is Closed 1's data loss
moved from a successful registration to a failed one; boxing it allocates on the
failure path to narrow a `Result` whose bytes the caller already moves *in*,
since `register` takes the same `Split< T >` by value. The `allow` is written on
`register` alone with a `reason =`, never crate-wide, so a different oversized
`Result` appearing later still fails the build.

Recorded because "add a `Debug` bound to your record type" and "we turned a lint
off" are both surprising things for a registry to ask, and the reasons should be
findable. → [`api/001`](../api/001_the_registry_surface.md).

### Closed 3 — Is there an immutable `get`?

**No, and it is not an omission.** `ring_handle::Split< T >` has two methods:
`new`, and `ends( &mut self )`. A `&Split< T >` therefore permits no operation
whatsoever, so an immutable `get` would compile, return `Some`, and be useless.
`contains` is the immutable query that is actually answerable.

Would become worth revisiting if `Split` ever grew a `&self` accessor.

### Closed 4 — Three dependency edges or one?

**One.** The initial design assigned `ring_handle`, `ring_core`, `ring_types`. The
registry never inspects what it stores — it hashes a name, holds a value, lends
it, drops it — so `T` is opaque and `ring_types` never appears. `ring_core`
appears only in tests. → [`integration/001`](../integration/001_one_declared_edge_of_three.md).

## Pending

### Pending 1 — Should a registry hold rings of different record types?

**What is undecided:** `Registry< T >` is generic over one record type, so a
program with an events ring of `Event` and a telemetry ring of `Sample` needs two
registries.

**Why it is not decided:** heterogeneity needs `Box< dyn Any >` and a downcast at
every retrieval, which turns a `get_mut` that cannot fail into one that can fail
for a second reason — wrong type, as against absent name. Nothing asks for it:
no requirement speaks to `T`'s heterogeneity, and `ring_factory`, the only
consumer, builds one ring at a time.

**What would settle it:** a consumer that genuinely holds rings of two record
types and wants them in one map. Until then two registries cost a binding each
and lose nothing.

### Pending 2 — Should names be validated?

**What is undecided:** `register` takes `impl Into< String >` and accepts
anything — the empty string, a string of spaces, one containing a newline. All
are pinned as valid by `unusual_names_are_ordinary_names`.

**The argument each way:** a registry whose names appear in log lines or a
diagnostic dump benefits from names that are printable and non-empty. But
validation means a second failure mode on `register`, a rule that has to be
documented and agreed, and a decision about what happens to the many names that
are merely odd rather than harmful.

**What would settle it:** a consumer that formats registry names into output
where a newline or an empty string actually breaks something. The current test
would then be the one to change, deliberately, rather than a behaviour that
drifted.

### Pending 3 — Should `names()` be ordered?

**What is undecided:** `names()` yields in `HashMap` order, which is unspecified
and varies between runs.

**Why it is currently unordered:** an ordered map would make iteration
deterministic and costs `O(log n)` lookups instead of `O(1)` on the operation
that actually matters — retrieval. Below roughly a dozen names, `BTreeMap`
is the faster of the two on `get_mut` — the choice buys headroom, not speed today.

**What would settle it:** a consumer that needs a stable listing — a diagnostic
dump compared between runs, say. The cheaper answer would then be for the caller
to sort, and this entry exists so that answer is reached deliberately rather than
by someone changing the map type.

### Pending 4 — Should the registry expose a way to visit every ring?

**What is undecided:** the surface has no `values_mut`, `iter_mut`, `drain` or
`retain`. Visiting every ring compiles only by collecting `names()` into owned
`String`s and looking each one up again, since `names()` and `get_mut` do not
compose under the borrow checker.

**Why it is not decided:** nobody is paying the cost today — every `.names()`
call site in the workspace is immutable and sorts a snapshot rather than
visiting rings. Adding an accessor is a real question with an argument on each
side, not an obvious yes.

**What would settle it:** a consumer that needs to visit every ring — flushing
all of them, draining all of them at shutdown, reporting depth per ring. Until
then the workaround costs a `String` and a second lookup per ring, measured at
roughly thirty-fold the `values_mut()` sweep the wrapped `HashMap` already
supports.

---

Everything above is the record itself; everything below is the apparatus over it.
The two are in this order, rather than the family's usual one, because this file
is a primary source: four findings cite its lines by number and four regenerate
blocks in three other definitions grep it live, so nothing may be inserted ahead
of line 113.

Eight entries, four settled and four open, and they sort by where their evidence
came from. Seven were settled or deferred by reading — a signature, a dependency
list, what a consumer builds — which is the right method for every one of them,
since whether a `&Split< T >` allows an operation is a fact about a declaration
and not something to benchmark. One was settled by a machine refusing: three
tests that would not compile, and a lint that stopped the build and printed a
width. That one entry is the only place in the file where evidence arrived from
outside somebody's head, and it is the only quantity the file contains.

The four findings here follow that split. Two are about the closed entries — the
absence of any measurement the crate went and took, and a contract argued for at
length whose only consumer discards it on purpose with reasons written down. Two
are about the open ones, and both turn on the same fact: all three of the
original Pending entries defer to a consumer, one consumer exists, and it has
already answered one of the questions and measurably contradicts the pricing
of another. Pending 4 was added afterward and defers to a consumer too, but
neither finding here is about it — [RG8](../api/002_the_receiver_split_and_the_sweep_it_forbids.md)
is.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_four_closed_questions_and_the_one_measurement_none_took.md) | Four Closed Questions and the One Measurement None Took | What settled each of the four, and the one that a failing build settled |
| [002](002_three_pending_questions_and_the_one_consumer.md) | Three Pending Questions and the One Consumer | The settling conditions, the consumer that meets one of them, and the ordered map measured |

## Deferral That Names Its Own Trigger

The first three open entries are well-formed in the way that matters: each names a
falsifiable condition rather than deferring indefinitely, and each condition is
about a consumer rather than about somebody's future opinion. That is the
correct shape, and Pending 1 even goes and checks — it cites `ring_factory`
building one ring at a time as evidence for *not* deciding.

The gap is that only one of the three did the checking. `ring_factory` calls
seven of the registry's eight methods across its source and tests, and at
`factory_test.rs:344-346` it collects the unordered listing, sorts it, and
compares against a literal — which is precisely the answer Pending 3 predicted a
consumer would reach and exists to make deliberate. It happened, deliberately,
and the entry does not know. Pending 2's condition, by contrast, is genuinely
still unmet: the same consumer passes every name in from its own caller and
never formats one into output. Saying so is the difference between an open
question and an unexamined one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order. This file is a primary
# source: `decisions/001` greps it for every bolded byte count and every
# complexity class and expects three hits, all in the record above, so nothing
# in this apparatus may reproduce that notation.
d=ring_registry/docs/decisions/readme.md
echo '  -- the record: four settled, four open --'
command grep -n '^### Closed\|^### Pending' "$d" | cut -c1-88 | sed 's/^/    /'
echo '  -- what each open entry names as the evidence that would settle it --'
command grep -n '^\*\*What would settle it' "$d" | cut -c1-88 | sed 's/^/    /'
echo '  -- the only consumer that could supply it --'
command grep -rln 'ring_registry' --include=Cargo.toml */  |
  command grep -v '^ring_registry/' | sed 's/^/    manifest: /'
echo '  -- and the answer it has already given --'
command grep -n 'registry.names()\|names.sort_unstable\|assert_eq!( names' \
  ring_factory/tests/factory_test.rs | sed 's/^/    /'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG13 | `ring_registry` | n/a — observation | Four entries are recorded as closed and their evidence sorts into two kinds: three were settled by reading — no count-based or lookup-based test can tell `Entry` and `insert` apart, `Split` has two methods and one takes `&mut self`, the registry is opaque in `T` — which is the correct method for all three, and one was settled by a machine refusing, three tests that would not compile plus a lint that stopped the build and reported a width; that one entry is the only place in the file where evidence arrived from outside somebody's head and its width is the only quantity the file contains, the two remaining notations being asymptotic classes rather than measurements of anything; recorded as an evidence-type imbalance and not a wrong decision, since all four conclusions survive measurement — but Closed 1 is worth annotating, because [RG2](../algorithm/001_two_branches_and_what_the_refusal_costs.md) finds the `Entry` form measurably slower on exactly the path Closed 1 is about, a conclusion the entry still reaches correctly by an argument that never touched the thing that turns out to be false, and its own follow-through is the counter-example worth crediting since it named the discriminator it lacked and one was then built |
| RG14 | `ring_registry` | n/a — unadopted | Closed 2 is the crate's most argued decision — `register` returns the rejected ring alongside the error so a name collision does not destroy it — and three costs are paid for it: a 448-byte `Result` on both paths, a `Debug` bound on `T` at `.expect()` sites, and a suppressed lint with a written `reason =`; the sole external manifest naming this crate is `ring_factory`, whose single call site at `ring_factory/src/lib.rs:214` matches `Err( ( RegistryError::NameTaken { .. }, _refused ) )` and drops both halves on purpose, the ring because "the caller never held it" and the name because "the caller passed it in and still has it" and carrying it "would also put a `String` in a `Copy` error type for no new information" — both reasons good and neither specific to `ring_factory`, since the build-then-register shape is the one the module doc, readme and doctests all use, so the capability is available precisely to callers that construct a ring at one point and register it at another, of which there are none; recorded as an unexercised contract rather than a wrong one, cheap to keep and expensive to reverse, wanting one line so the entry records the state of the world and not only the intent |
| RG15 | `ring_registry` | n/a — drift | All three open entries name a consumer as the evidence that would settle them, which is the right shape, and one consumer exists: `ring_factory` calls seven of the registry's eight methods across its source and tests — `contains`, `get_mut`, `is_empty`, `len`, `names`, `register`, `remove` — and Pending 1 already consults it, correctly, as evidence for *not* deciding; Pending 3 was not consulted and the answer is there, because its own text predicts a consumer would sort at the call site rather than change the map type, and at `factory_test.rs:344-346` the consumer collects `names()` into a `Vec< &str >`, calls `sort_unstable()`, and compares against a literal — the predicted answer reached deliberately by the one consumer, unknown to the entry recording the prediction; recorded as a stale open question whose repair is to move Pending 3 into the settled column with that test as what settled it, and to note at the same time that Pending 2's condition is genuinely still unmet since the same consumer never formats a name into output |
| RG16 | `ring_registry` | **measured cost** | Pending 3 prices the ordered map by complexity class — logarithmic lookups against constant-time ones "on the operation that actually matters — retrieval" — which is sound as analysis and names the right operation, and never checks whether `n` gets large enough for the classes to separate; measured on `get_mut` itself, `BTreeMap` is **8.6 ns against `HashMap`'s 30.4 at one name, 72% faster**, and 16.8 against 30.5 at four, 45% faster, with the crossover between 4 and 16 and the entry's direction becoming correct by 16 where the ordered map costs 26–33% more, both runs reproducing to a few hundredths of a nanosecond; the largest registry anything in this repository asserts on holds four and the consumer's own doctest holds one, and the hash of a key string does not get cheaper as `n` falls while a single comparison does, which is why the gap is widest exactly where the registries are; the repair is not to switch the map but to say that the choice buys headroom rather than present speed — the same inversion [RG4](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md) records against the source doc's version of the claim, extended here to the operation this entry singles out |
