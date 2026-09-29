# Type: Five Discriminants and the Array Beside Them

### Scope

**Purpose:** Establish which of the three declarations that must agree about
`TraceOp`'s five kinds are held by the type system, measure both directions of
the one that is only half-held, and record what `ALL` is as a piece of public API
against what its doc says it is for.

**Responsibility:** The enum's five discriminants, `ALL`'s declared type and its
contents, `name()`'s arms, `Display`'s delegation, and who outside the crate
names either type.

**In Scope:** `ring_trace/src/lib.rs:59-118`; every `ring_*` `src/` and
`tests/`.

**Out of Scope:** That the suite claims a tripwire the type system does not
provide is
[`item/002`](../item/002_traceop_all_and_the_tripwire_that_is_not_one.md). The
exhaustive-match idiom as a family pattern is
[`pattern/002`](../pattern/002_exhaustive_match_as_a_tripwire.md). The derive sets
are [`type/002`](002_what_the_compiler_knows_about_these_three_types.md).

---

## Three Declarations of the Same Five

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the five, and the three places that must agree with them --'
printf '    discriminants declared: %s   entries in ALL: %s   arms in name(): %s\n' \
  "$( command grep -m1 -A12 -F 'pub enum TraceOp' ring_trace/src/lib.rs | command grep -c '^  [A-Z][a-z]*,$' || true )" \
  "$( command grep -m1 -A7 -F '  pub const ALL : [ Self; 5 ] =' ring_trace/src/lib.rs | command grep -c 'Self::' || true )" \
  "$( command grep -m1 -A11 -F '  pub const fn name( self ) -> &'"'"'static str' ring_trace/src/lib.rs | command grep -c 'Self::[A-Z][a-z]* =>' || true )"
printf '    Display, which delegates instead of matching: %s\n' \
  "$( command grep -m1 -A9 -F 'impl fmt::Display for TraceOp' ring_trace/src/lib.rs | command grep 'write_str' | sed 's/^ *//' )"
echo '  -- the constant, its declared type, and what its doc says it is for --'
command grep -m1 -A2 -F '  /// Every discriminant, for a test that must cover all of them.' ring_trace/src/lib.rs
echo '  -- and who outside this crate names either type --'
printf '    crates naming either type outside ring_trace: %s\n' \
  "$( for c in ring_*/; do
        if [ "$( basename "$c" )" = ring_trace ]; then continue; fi
        if command grep -rq 'TraceOp\|TraceEntry' "$c"src "$c"tests 2>/dev/null; then echo x; fi
      done | wc -l )"
```

Live output:

```
  -- the five, and the three places that must agree with them --
    discriminants declared: 5   entries in ALL: 5   arms in name(): 5
    Display, which delegates instead of matching: f.write_str( self.name() )
  -- the constant, its declared type, and what its doc says it is for --
  /// Every discriminant, for a test that must cover all of them.
  pub const ALL : [ Self; 5 ] =
  [
  -- and who outside this crate names either type --
    crates naming either type outside ring_trace: 0
```

## What the Array's Type Actually Checks

*Both compile probes below ran once against a scratch binary under
`-tr_probe/compile/` and were swept afterward per this project's
convention for temporary files — neither can be re-run to reconfirm, and
the exact `error[E0308]` wording is not a stability guarantee across
compiler versions. `TraceOp`'s five discriminants and `ALL`'s
`[ Self; 5 ]` type (`ring_trace/src/lib.rs:59-118`) are unchanged, so
the two structural facts demonstrated — a too-long array is a type error,
a repeated-and-dropped one is not — should still hold.*

```rust
// -tr_probe/compile/-all_too_long.rs — a sixth entry, declared length untouched
pub enum TraceOp { Claim, Publish, Consume, Commit, Drop, Evict }
impl TraceOp
{
  pub const ALL : [ Self; 5 ] =
  [ Self::Claim, Self::Publish, Self::Consume, Self::Commit, Self::Drop, Self::Evict ];
}

// -tr_probe/compile/-all_repeats.rs — five entries, one listed twice, one dropped
pub enum TraceOp { Claim, Publish, Consume, Commit, Drop }
impl TraceOp
{
  pub const ALL : [ Self; 5 ] =
  [ Self::Claim, Self::Claim, Self::Publish, Self::Consume, Self::Commit ];
}
```

```
=== ALL longer than its declared length ===
    error[E0308]: mismatched types
    expected an array with a size of 5, found one with a size of 6
    error: aborting due to 1 previous error
=== ALL repeating a kind and dropping another ===
    errors: 0
```

---

### TR45 — The Array's Type Checks How Many Are Listed and Never Which

Five kinds are declared in three places: the enum's discriminants, `ALL`'s five
entries, and `name()`'s five match arms. `Display` is not a fourth, because it
delegates — `f.write_str( self.name() )` — so it inherits whatever `name()` does.

Two of those three are tied by the compiler. Adding a discriminant forces a new
arm in `name()`, which is the tripwire the source documents and which is real.
The third pairing, enum against `ALL`, is held in one direction only, and the
compiler says which. Listing a sixth entry while leaving the type as `[ Self; 5 ]`
gives `error[E0308]: mismatched types` — "expected an array with a size of 5,
found one with a size of 6". Writing five entries that repeat `Claim` and omit
`Drop` compiles without a single error.

So the declared length is a real constraint on the array and a constraint on
nothing else. It enforces that the list has the length it says it has, which is a
statement about the literal beside it, and it says nothing about which
discriminants appear in it or whether any appears twice.

**Finding.** Recorded as an accurate but narrow guarantee that is easy to read as
a wider one, since `[ Self; 5 ]` and an enum with five variants look like they
check each other. The direction that matters — the enum growing while `ALL`
stands still — is the one nothing holds, and it is exactly the direction
`item/002` shows the suite claiming is covered. One clause on the constant naming
which drift the type catches, and which a human has to catch, costs a line and
prevents the reading the suite already fell into.

---

### TR46 — A Public Constant Documented as a Test Affordance

`ALL` is `pub`, so it is part of the crate's permanent surface: a caller can
depend on it, iterate it, and hold its length. Its doc says what it is for —
"Every discriminant, for a test that must cover all of them" — which describes
the crate's own suite.

That description is currently exact. No crate in the family other than
`ring_trace` names `TraceOp` or `TraceEntry` at all, so the constant's only user
is the test file six directories away in the same crate, and its stated audience
and its actual audience are the same set of one.

The awkwardness is that the two cannot stay the same. `ALL` is public API and
will be read by whatever eventually calls the crate — the vocabulary is a real
vocabulary, five names for five ring operations, and enumerating them is a
reasonable thing for a caller to want. The doc, meanwhile, tells that caller the
constant exists for someone else's tests.

**Finding.** Recorded as a doc gap in the narrow sense and an API question in the
wider one. If `ALL` is public because callers should enumerate the vocabulary,
the doc should say that and drop the reference to tests, which is where a
`#[ cfg( test ) ]` constant would live instead. If it is public only because the
integration test in `tests/` cannot see a private item — which is the actual
mechanical reason, and a common one — then saying so is worth more than the
current sentence, because it tells a caller the guarantee is incidental rather
than intended.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](../item/002_traceop_all_and_the_tripwire_that_is_not_one.md) | The drift direction nothing catches, compiled |
| [`pattern/002`](../pattern/002_exhaustive_match_as_a_tripwire.md) | The idiom the enum-to-`name()` pairing belongs to |
| [`type/002`](002_what_the_compiler_knows_about_these_three_types.md) | What the derives on this enum do and do not provide |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | Why nothing outside the crate names the type |
| [`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | What one discriminant costs in an entry |

### Sources

| Fact | Where |
|------|-------|
| The five discriminants | `ring_trace/src/lib.rs:72-81` |
| `ALL`, its type and its doc | `ring_trace/src/lib.rs:86-94` |
| `name()`'s five arms | `ring_trace/src/lib.rs:111-115` |
| `Display` delegating rather than matching | `ring_trace/src/lib.rs:120-126` |
| Both drift directions, compiled | Probe above |
| No consumer outside the crate | Census above |

### Tests

| Test | Covers |
|------|--------|
| `the_operation_kinds_are_exactly_the_five_declared` | `ALL`'s length and its lack of repeats |
| `every_operation_kind_has_its_own_name` | The enum-to-`name()` pairing |
| `an_operation_prints_as_its_name` | `Display`'s delegation |
