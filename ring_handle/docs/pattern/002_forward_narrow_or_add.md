# Pattern: Forward, Narrow, or Add

### Scope

- **Purpose**: State the three-way rule every declaration in this crate obeys, and the test that decides which of the three a new declaration belongs to.
- **Responsibility**: Name the categories, give the criterion, and say what happens when a declaration fits none.
- **In Scope**: The classification and its application to the seventeen declarations.
- **Out of Scope**: Why narrowing is the enforcement mechanism at all (→ [`pattern/001`](001_enforce_by_withholding.md)); the catalogue itself (→ [`item/002`](../item/002_twelve_verbs_eight_bare_forwards.md)).

### Context

A wrapper crate with one dependency has exactly three things it can do with any
capability the dependency offers: pass it through, refuse it, or build something
new on top. Left unstated, the choice is made per-method by whoever is writing
that method, and the crate drifts into a mixture where a reader cannot tell
whether a missing method is a decision or an oversight.

This crate states the rule instead.

### Pattern

**Every public declaration is exactly one of three kinds, and the kind is
decided before the body is written.**

| Kind | Rule | Body shape |
|------|------|------------|
| **Forward** | The capability is safe under this crate's invariant and is offered unchanged | `self.inner.<same name>( <same args> )` — nothing but the call, or the call plus a rewrapping of its result |
| **Narrow** | The capability would break the invariant, so no declaration exists | *No code at all.* A `tests/ui/` case asserts the absence |
| **Add** | The capability does not exist below and is built here | Whatever it takes; it is this crate's own code and its own liability |

`Split::new` is outside the three: it constructs the entry point and has nothing
below to forward to, narrow, or add against.

### Criterion

The question that decides the kind is: **can two callers hold this capability at
once?**

- If holding it twice is harmless — `len`, `is_empty`, `free_capacity`,
  `is_full`, and every push and recv, all of which take `&mut self` or `&self`
  and are therefore already exclusive per handle — it **forwards**.
- If holding it twice produces a second `Producer` or a second `Consumer` over
  one ring — `try_clone` — it is **narrowed**.
- If the capability is a shape the answer does not exist for below — bounded
  iteration — it is **added**.

Applied to the seventeen: ten forward — eight as a bare call, two rewrapping the
result — one adds, one constructs, and five are the structs the other eleven
hang off. The narrow is the eighteenth declaration, the one that is not there.

### Consequences

**A forward is not allowed to grow a body.** The moment a method needs a second
statement, it stops being a forward and becomes an add — which means it becomes
this crate's liability, needs its own test, and needs a line in
[`item/002`](../item/002_twelve_verbs_eight_bare_forwards.md). The rule is what
makes the ten cheap: they cannot be wrong because there is nothing in them to be
wrong.

**A narrow needs a `tests/ui/` case or it is not a narrow.** An absent method
with no test asserting the absence is indistinguishable from a method nobody
got around to writing. This is the whole reason the crate carries a `trybuild`
dev-dependency.

**An add needs the reason it could not go below.** `Drain` lives here because
`ring_core` has no snapshot notion, not because it was convenient. The next add
must say the same kind of thing or it belongs one crate down.

**The three kinds are also where review risk concentrates.** Eight lines of
bare forwards carry zero lines of unique test — the coverage that touches them
is `ring_core`'s, exercised through this crate's names. The one add (`drain`)
and the seven narrow-enforcement `tests/ui/` cases carry effectively all of the
crate's local risk, which is where review attention should go first (→ HD19).

### Patterns

| File | Relationship |
|------|--------------|
| [001_enforce_by_withholding.md](001_enforce_by_withholding.md) | Why the narrow category exists; this instance is its generalization to the other two |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_twelve_verbs_eight_bare_forwards.md](../item/002_twelve_verbs_eight_bare_forwards.md) | The seventeen declarations, classified |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | The forward category, as an algorithm |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_one_struct_that_is_not_a_newtype.md](../data_structure/002_the_one_struct_that_is_not_a_newtype.md) | The one add, worked out |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Where all three kinds appear |
| [`tests/ui/producer_try_clones.rs`](../../tests/ui/producer_try_clones.rs) | The narrow, asserted |

### Tests

| Test | Relationship |
|------|--------------|
| `free_capacity_and_is_full_agree` | Two forwards, cross-checked |
| `drain_is_bounded_at_the_call_that_made_it` | The one add |
| `no_parking_shaped_name_appears_in_the_source` | A narrow of a different shape — a whole vocabulary refused |

### HD17 — The Rule Is Obeyed Everywhere and Written Down Nowhere in the Source

Every body in the crate fits one of the three kinds:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- forwards: nothing but the call --'
command grep -cE '^    self\.inner\.[a-z_]+\(' ring_handle/src/lib.rs
echo '  -- the four bodies that are something else --'
for f in 'pub const fn new' 'pub fn ends' 'pub fn split' 'pub fn drain'; do
  printf '  %-8s %s\n' "${f##* }" \
    "$( sed -n "/  $f(/,/^  }/p" ring_handle/src/lib.rs \
         | command grep -cE '^    [^ ]' )"
done
# grep -c exits 1 on zero matches, so both counts go through printf
printf '  the words, used descriptively: %s\n' \
  "$( command grep -cinE 'forward|narrow|passthrough|pass-through' ring_handle/src/lib.rs )"
printf '  the rule, stated as a rule:    %s\n' \
  "$( command grep -cinE 'three kinds|categor' ring_handle/src/lib.rs )"
```

Live output:

```
  -- forwards: nothing but the call --
8
  -- the four bodies that are something else --
  new      1
  ends     1
  split    2
  drain    2
  the words, used descriptively: 2
  the rule, stated as a rule:    0
```

Eight bare forwards. `new` and `ends` are one statement each but neither is a
forward — one constructs, one rewraps. `split` and `drain` are two statements
each.

**The words appear in the source and the rule does not.** The doc comments
explain what individual methods do and use "forward" descriptively; none of them
says that a method with a second statement has changed category, and nothing
mechanical checks it. A contributor adding a null check to `try_push` — a
plausible, well-meant edit — turns a forward into an add without any signal that
they have taken on a liability, and the only artefact that would tell them is
this instance.

That is a documentation-side gap, not a code defect. The rule is real and
uniformly obeyed; it is simply not where the person about to break it is
looking.

### HD18 — The Narrow Category Has One Member and Seven Enforcement Cases

Absence is asserted seven times for one absent method:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- ui cases --'
ls ring_handle/tests/ui/*.rs | sed 's|.*/||'
echo '  -- what each one tries to do --'
for f in ring_handle/tests/ui/*.rs; do
  printf '  %-34s ' "$( basename "$f" )"
  command grep -oE '\.(try_clone|clone|try_push|try_recv|drain|split|ends)\(' "$f" \
    | sort -u | tr -d '.()' | tr '\n' ' '
  echo
done
```

Live output:

```
  -- ui cases --
consumer_clones.rs
consumer_publishes.rs
producer_clones.rs
producer_drains.rs
producer_shared_across_threads.rs
producer_try_clones.rs
ring_used_after_split.rs
  -- what each one tries to do --
  consumer_clones.rs                 clone ends split 
  consumer_publishes.rs              ends split try_push 
  producer_clones.rs                 clone ends split 
  producer_drains.rs                 ends split try_recv 
  producer_shared_across_threads.rs  ends split 
  producer_try_clones.rs             ends split try_clone 
  ring_used_after_split.rs           
```

Seven cases; only one of them (`producer_try_clones`) targets the method the
crate actually withholds. The other six assert consequences of the *type* shapes
— that a producer cannot drain, that a consumer cannot publish, that neither is
`Clone`, that a producer is not `Sync`-shareable, that the ring cannot be used
after splitting.

**Six of the seven are testing `ring_core`'s guarantees through this crate's
types.** That is not wasted — a regression in `ring_core` that made `Producer`
`Clone` would break them here, and breaking loudly at the wrapper is exactly
what the wrapper is for. But it means the narrow category's own enforcement is
one file, and the impression of a heavily-guarded boundary comes mostly from
tests that would pass whatever this crate did.

### HD19 — Two of the Three Kinds Have No Cost and the Third Has All of It

The classification predicts where the tests are, and it does:

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  tests in the integration file: %s\n' \
  "$( command grep -c '^#\[ test \]' ring_handle/tests/handle_test.rs )"
printf '  of those, named for the add:   %s\n' \
  "$( command grep -cE '^fn (drain|draining)_' ring_handle/tests/handle_test.rs )"
printf '  ui cases, asserting narrows:   %s\n' \
  "$( ls ring_handle/tests/ui/*.rs | wc -l )"
printf '  source lines behind the eight: %s\n' \
  "$( command grep -cE '^    self\.inner\.[a-z_]+\(' ring_handle/src/lib.rs )"
```

Live output:

```
  tests in the integration file: 19
  of those, named for the add:   4
  ui cases, asserting narrows:   7
  source lines behind the eight: 8
```

Eighteen `#[ test ]` in the integration file, four of them named for the add,
plus seven compile-fail cases. Eight lines carry every bare forward in the
crate.

**The bare forwards consume eight lines of source and zero lines of unique
test.** The tests that touch them — `free_capacity_and_is_full_agree`,
`try_push_batch_reports_partial_acceptance`, `the_in_house_backends_behave_alike`
— are exercising `ring_core`'s behaviour, correctly, through this crate's names.
Every genuinely local risk sits in `drain` and in the seven ui cases.

A reader budgeting review attention across the crate would, without this
instance, spread it across seventeen declarations. The classification says to
spend it on two.

**Disposition:** applied — the pattern's own Consequences section now states
the cost distribution this finding measured: bare forwards carry no unique
test coverage, and the add plus the narrow-enforcement cases carry the
crate's local risk, matching this section's own Live output.
Now prints: `source lines behind the eight: 8`
