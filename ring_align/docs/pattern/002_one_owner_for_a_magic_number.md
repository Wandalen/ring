# Pattern: One Owner for a Magic Number

### Scope

- **Purpose**: State the pattern this crate is the family's second instance of — give a cross-cutting constant exactly one declaring home — and identify, from the family's own two instances, the structural condition that decides whether it holds.
- **Responsibility**: State the shape, measure both instances, name the condition that separates them, and state the limit that applies to both.
- **In Scope**: `CACHE_LINE` and `RingError` as instances; the reach measurement.
- **Out of Scope**: The family-wide restriction as an invariant, which is [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md); which crate should own this particular constant, which is [`integration/002`](../integration/002_why_the_constant_lives_here.md).

### The Shape

A value that many crates would otherwise each write down — a platform constant,
a shared error enum, a protocol version, a buffer size — is declared **once**,
in a crate named for owning it, and reached by dependency rather than by
retyping. Changing it later is one edit.

The pattern is not "avoid magic numbers". A literal `64` inlined at one site is
not a problem. The pattern is about a number that appears in more than one
crate, where the failure is not readability but **divergence**: two copies that
agree today and are edited separately tomorrow.

### The Family Runs It Twice

| Instance | Owner | Tier | Declared | Consumers | Duplicates found |
|----------|-------|:----:|----------|----------:|-----------------:|
| The error enum | `ring_types` | 0 | `error.rs:44` — `pub enum RingError` | **31** | 0 |
| The cache-line size | `ring_align` | 1 | `lib.rs:35` — `pub const CACHE_LINE` | **1** | **1** |

**Correction (2026-09-28):** the error enum's Consumers cell read `32`.
`ring_align` was one of those 32 and dropped its `ring_types` dependency in
commit `ce60ae6e8`, so the count is 31 now — the crate this pattern document
is itself about is the reason its comparison instance's own count moved.

```sh
cd "$(git rev-parse --show-toplevel)"
# excludes an untracked, in-progress workspace-restructuring manifest
# (`ring/Cargo.toml`) and trybuild's scratch/build-output copies under
# `target/`/`-target_gate/` — none of the three is a real crate.
grep -rl 'ring_types' --include=Cargo.toml . \
  | command grep -vE '^ring/Cargo\.toml$|/target/|/-target' | wc -l   # 32 = 31 consumers + itself
grep -rl 'ring_align' --include=Cargo.toml . \
  | command grep -vE '^ring/Cargo\.toml$|/target/|/-target' | wc -l   # 2  = 1 consumer + itself
ls -d ring_*/ | wc -l                                 # 33
```

Live output:

```
32
2
33
```

`ring_types` states its own instance of the pattern in the same terms
(`ring_types/src/error.rs:3-7`):

> One enum rather than one per crate: a consumer sits behind the five-crate
> export surface […] and never names the 28 internal crates, so per-crate error
> types would have to be converted into a shared one at the surface anyway.
> This is that shared one, declared once at tier 0.

### What Separates the Two Instances

Not discipline, and not how well either is documented — both are documented
well. **Reachability.**

`ring_types` is a universal dependency: every one of the other 31 crates
already lists it. A developer needing `RingError` has it in scope, and writing
a private error enum instead would be more work, not less. The pattern is
enforced by the path of least resistance.

`ring_align` is listed by one crate. A developer in `ring_mpsc` needing a
cache-line number has nothing in scope — the existing `ring_mpsc → ring_cursor`
edge does not carry it, because `ring_cursor` re-exports only
`ring_atomic::SeqCell` (`ring_cursor/src/lib.rs:69`). Reaching the owned
constant means editing a manifest; typing `64` means typing two characters.
**The path of least resistance runs the other way, and the duplicate that
exists is exactly where it points**
(→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) § The
Duplicate Already Exists).

**The generalisation, stated as a rule:** single ownership holds where the
owner is already reachable from every plausible consumer, and degrades to a
convention wherever reaching the owner costs a manifest edit. One instance each
way is thin evidence, but the mechanism is not a statistical claim — it is
about which option is cheaper at the moment a developer needs the value.

### The Limit That Applies to Both

**An owner makes duplication visible, not impossible.** Nothing in the language
or the build stops a second declaration. What single ownership actually
provides is:

| Provides | Does not provide |
|----------|------------------|
| A place to look when the value must change | Any signal that a second copy was written |
| A name for the concept (`CACHE_LINE`, not `64`) | Any obligation to use that name |
| A greppable target for an audit that someone chooses to run | The audit |

The audit is Q4 in
[`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md) — a
gate rejecting a bare layout literal in `ring_*` — and it does not
exist. **Until it does, both instances above are conventions with different
amounts of luck**, and the difference in their outcomes is explained entirely
by the reachability asymmetry rather than by either being better governed.

### Applying It Elsewhere

Three tests before adopting the pattern for a new value, in the order they
actually decide the question:

1. **Will more than one crate need this value?** If not, a local `const` is
   correct and a crate is ceremony.
2. **Is the owner reachable from every crate that will need it — today, without
   a manifest edit?** If not, expect the `CACHE_LINE` outcome. Either place the
   value in a crate that is already universal, or re-export it from one that
   is.
3. **Is there a mechanical check that a second copy has not appeared?** If not,
   the pattern is documentation. That may be enough; it should be recorded as
   what it is.

Test 2 is the one this crate would fail, and it has a cheap remedy that does
not require moving anything: a re-export from `ring_cursor`, which every
consumer of padded cursors already depends on
(→ [`integration/002`](../integration/002_why_the_constant_lives_here.md)).

### AL39 — Single Ownership Buys the Grep, Not the Absence

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- declared owners of the line size, family-wide --'
command grep -rc 'pub const CACHE_LINE' */src/lib.rs | command grep -v ':0$'
echo '  -- copies the owner does not prevent --'
command grep -e 'abs_diff( consume ) >= 64' -e 'addr() % 64' \
  ring_mpsc/src/lib.rs ring_cursor/src/lib.rs
```

Live output:

```
  -- declared owners of the line size, family-wide --
ring_align/src/lib.rs:1
  -- copies the owner does not prevent --
ring_mpsc/src/lib.rs:    claim.abs_diff( consume ) >= 64
ring_cursor/src/lib.rs:  /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
```

One declaration across 33 crates — the pattern working — and two live copies the
ownership cannot reach.

**Finding.** What an owner delivers is a well-defined question with one right
answer, so a copy becomes findable and nameable rather than merely suspicious.
It does not deliver the copies' absence, and this crate's own regenerate block
is the grep that finds them. The pattern's limit is precise and should be stated
as part of the pattern: *an owner makes duplication visible, not impossible*.

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_the_constant_is_not_conditional.md](../decisions/001_the_constant_is_not_conditional.md) | "One edit at port time" is the payoff this pattern is supposed to deliver, and it is only as good as the ownership |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_why_the_constant_lives_here.md](../integration/002_why_the_constant_lives_here.md) | Test 2 applied to this crate specifically, with the re-export remedy |
| [../integration/001_one_dependency_one_consumer.md](../integration/001_one_dependency_one_consumer.md) | The consumer count the reachability argument rests on |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | This pattern as a restriction on the family, with the live violation |

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_newtype_as_layout_carrier.md](001_the_newtype_as_layout_carrier.md) | K3 — why the constant cannot be carried into the wrapper generically, which is what forces a second literal even inside this crate |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | The owned declaration, and the `E0693` bind that puts a second literal beside it |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/error.rs:1-10` | The family's other instance of the pattern, stated by its own author |
| `ring_types/src/lib.rs:3-8` | Why tier 0 is universally reachable — the property that makes that instance hold |
| [`../../../readme.md`](../../../readme.md) | The 33-crate decomposition both instances range over |

### Tests

| File | Relationship |
|------|--------------|
| `src/lib.rs` doctest | `assert_eq!( ring_align::CACHE_LINE, 64 )` — pins the owned value; nothing pins that it is the only one |
| `tests/manual/readme.md` | No check covers ownership. The grep in [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md) is the closest thing and it is unwired |
