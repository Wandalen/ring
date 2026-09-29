# Decisions: Capacity Is the One Parameter That Is Not a `with_`

### Scope

**Purpose:** Record the shape decision that gives the type one fallible
constructor and four infallible setters, and what the family does with the
fallibility that decision concentrates.

**Responsibility:** Why capacity is a constructor argument, why the other four
fields are not, and the census of who actually constructs a configuration.

**In Scope:** `ring_config/src/lib.rs:65`, `:89`, `:104`, `:122`, `:140`,
`:112-114`.

**Out of Scope:** The clamping half of the same decision is
[`decisions/002`](002_clamping_instead_of_a_question_mark_mid_chain.md). What the
`?` costs the constructor is
[`workaround/001`](../workaround/001_the_question_mark_forecloses_const.md).

---

## Where the Fallibility Is, and Who Meets It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- one fallible constructor, four infallible setters --'
command grep 'pub fn new(\|pub const fn with_' ring_config/src/lib.rs
echo '  -- every construction site outside this crate, in src/, by crate --'
command grep -rc 'RingConfig::new(' --include=*.rs */src | command grep -v ':0$\|^ring_config/' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
echo '  -- how many of those lines sit inside a doc comment --'
command grep -r 'RingConfig::new(' --include=*.rs */src | command grep -v '^ring_config/' | command grep -c '/// \|//! ' || true
echo '  -- and how each construction expression discharges the Result --'
command grep -r 'RingConfig::new( *[0-9_]* *)\.[a-z_]*' --include=*.rs */src | command grep -v '^ring_config/' | command grep -o 'RingConfig::new( *[0-9_]* *)\.[a-z_]*' | command grep -o '\.[a-z_]*$' | sort | uniq -c | sort -rn
```

Live output:

```
  -- one fallible constructor, four infallible setters --
  pub fn new( slots : usize ) -> Result< Self, RingError >
  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
  pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self
  pub const fn with_producers( mut self, producers : usize ) -> Self
  pub const fn with_batch( mut self, batch : usize ) -> Self
  -- every construction site outside this crate, in src/, by crate --
smoke_ring_write_path/src/lane.rs:2
ring_bench/src/lib.rs:4
ring_core/src/lib.rs:8
ring_factory/src/lib.rs:3
ring_flush/src/lib.rs:1
ring_handle/src/lib.rs:2
ring_mpsc/src/lib.rs:1
ring_poll/src/lib.rs:5
ring_registry/src/lib.rs:1
ring_shutdown/src/lib.rs:4
ring_spsc/src/lib.rs:1
ring_testkit/src/lib.rs:3
  -- how many of those lines sit inside a doc comment --
33
  -- and how each construction expression discharges the Result --
     32 .unwrap
```

---

### RC13 — The `?` Is Concentrated at the Head of the Chain So the Chain Has None

Five fields, five ways to set them, and one of the five is not a `with_`.
`capacity` is `new`'s argument and produces a `Result`; `wait`, `overflow`,
`producers` and `batch` are infallible `const fn` setters that cannot reject
anything.

The asymmetry is not arbitrary. Capacity is the only field with an invalid value
that has no obviously right correction: a zero cannot be raised to something, and
an arbitrary integer cannot be rounded to a power of two without silently
resizing the ring by up to a factor of two. Rejection is the only honest response,
and putting the only rejection at the head means every later call is total.

**Finding.** The decision is stated once, and from the other side.
`with_producers`' doc gives the reason for its own clamp — "clamping keeps the
setter infallible so a builder chain does not need a `?` in its middle" — which is
the same decision seen from the field that had a choice. Nothing states the
positive half: that capacity is where the `?` was put *on purpose*, that it is
there because it is the one field where clamping would be dishonest, and that this
is what makes the four setters `const`.

A reader therefore learns why `with_producers` clamps and not why `new` does not.
The rationale for the whole shape is present, attached to the one field it applies
to least.

---

### RC14 — Every Construction but Two Is in a Doc Comment, and the Two Disagree With All of Them

Thirty-five lines across twelve crates mention `RingConfig::new` in a `src/`
file, and thirty-three of them are inside doc comments. Thirty-two of those are
construction expressions and every one discharges the `Result` with `.unwrap()`;
the thirty-third is prose in `ring_bench`'s module comment.

The remaining two are not doctests. `smoke_ring_write_path` constructs a
configuration in `run_arm` and in `run_order_check_over`, and both discharge with
`.map_err( … )?` rather than `.unwrap()` — the second then chains
`.with_overflow` onto the recovered value, which is the chain shape
[`decisions/002`](002_clamping_instead_of_a_question_mark_mid_chain.md)'s
clamping half exists to keep total after the one `?` at its head.

Every other consumer receives a configuration rather than building one — by
shared reference in `ring_core`, `ring_mpsc` and `ring_spsc`, by value in
`ring_factory` and `ring_bench`.

**Finding.** So the fallibility this shape was built around is met outside a doc
comment by exactly one crate, and that crate propagates where every documented
example panics. The doctests are not the family's whole demonstrated answer to a
rejected capacity; they are the loud majority of it, and the single non-doctest
answer contradicts all thirty-two of them.

That contradiction is the recordable part, not the ratio. A reader learning the
constructor from its documentation meets `.unwrap()` thirty-two times and `?`
never, while the only production caller in the workspace does the reverse and has
the better claim to being followed — `run_arm` returns the refusal as a named
`String` rather than aborting a harness arm. Neither convention is written down
anywhere, so which one a reader adopts depends on whether they arrived through
the doctests or through `smoke_ring_write_path`.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_clamping_instead_of_a_question_mark_mid_chain.md) | The other half of the same decision, at the four fields that had a choice |
| [`algorithm/002`](../algorithm/002_one_fallible_path_and_it_is_not_this_crates.md) | What that one `Result` can actually carry |
| [`workaround/001`](../workaround/001_the_question_mark_forecloses_const.md) | What placing the `?` there costs the constructor |
| [`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md) | The builder shape the decision produces |

### Sources

| Fact | Where |
|------|-------|
| The one fallible constructor | `ring_config/src/lib.rs:65` |
| The four infallible setters | `ring_config/src/lib.rs:89`, `:104`, `:122`, `:140` |
| The rationale, stated at `with_producers` | `ring_config/src/lib.rs:112-114` |
| 35 mentions across twelve crates, 33 of them in doc comments | Census above |
| 32 construction expressions in doc comments, all `.unwrap()` | Census above |
| The two constructions outside a doc comment, both `.map_err( … )?` | `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/src/lane.rs:195`, `:265-267` |

### Tests

| Test | Covers |
|------|--------|
| `capacity_is_validated_at_construction` | The rejection the decision concentrates at `new` |
| `zero_producers_clamps_to_one` | The setter that could have rejected and does not |
| `batch_clamps_into_one_through_capacity` | The other one |
| `defaults_are_the_documented_ones` | What the four setters start from |
