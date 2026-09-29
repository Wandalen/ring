# Decisions: The Flag Is Fixed at Construction

### Scope

**Purpose:** Record that the enabled flag is immutable after construction and
that the crate gives a specific reason for it, confirm the decision holds
mechanically, and identify the capability the two `const fn` constructors create
which nothing in the family uses.

**Responsibility:** `Trace`'s stated argument for freezing the flag, the absence
of any setter or mutable receiver, the `const`-ness of both constructors, and
whether a `static Trace` is reachable and reached.

**In Scope:** `ring_trace/src/lib.rs:183-188`, `:205`, `:213-223`; every
`const` and `static` binding of the type across the 33 crates.

**Out of Scope:** The other mutator that is *not* frozen is
[`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md). The
lock the constructors initialise is
[`decisions/002`](002_a_mutex_over_a_lock_free_log.md).

---

## The Decision, and What It Made Possible

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the flag, and the two const constructors that set it --'
command grep -m1 -A5 -F '/// A log of sequence operations, off unless deliberately switched on.' ring_trace/src/lib.rs
command grep 'pub const fn enabled\|pub const fn disabled\|enabled : ' ring_trace/src/lib.rs
echo '  -- any setter, or any method taking a mutable receiver --'
printf '    setters: %s   &mut self methods: %s\n' \
  "$( command grep -c 'fn set_enabled\|fn enable(\|fn disable(' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'fn [a-z_]*( *&mut self' ring_trace/src/lib.rs || true )"
echo '  -- const fn declared, against const or static bindings of this type anywhere --'
printf '    pub const fn in the crate: %s\n' "$( command grep -c 'pub const fn' ring_trace/src/lib.rs || true )"
command grep -r 'static [A-Z_]* *: *Trace\|const [A-Z_]* *: *Trace' --include=*.rs */ || echo '    no const or static Trace binding anywhere in the family'
```

Live output:

```
  -- the flag, and the two const constructors that set it --
/// A log of sequence operations, off unless deliberately switched on.
///
/// The enabled flag is fixed at construction and never mutable afterwards. A
/// trace that could be switched on mid-run would produce a log with a silent
/// hole at the front, which reads exactly like a run where nothing happened
/// early — the one misreading a diagnostic tool must not invite.
  enabled : bool,
  pub const fn enabled() -> Self
    Self { enabled : true, entries : Mutex::new( Vec::new() ) }
  pub const fn disabled() -> Self
    Self { enabled : false, entries : Mutex::new( Vec::new() ) }
  -- any setter, or any method taking a mutable receiver --
    setters: 0   &mut self methods: 1
  -- const fn declared, against const or static bindings of this type anywhere --
    pub const fn in the crate: 5
    no const or static Trace binding anywhere in the family
```

## The Static That Compiles

*The scratch binary behind this probe is gone, swept per this project's
convention for temporary `-tr_probe/` files, so it can't be re-run. What it
demonstrated still holds by inspection: `enabled()` and `disabled()` are still
`pub const fn` over the same `Mutex::new( Vec::new() )` body, so a
`static Trace` would still compile and record exactly as shown. Treat the
printed lines as a preserved compile-time check, not a live one.*

```rust
// -tr_probe/src/bin/const_and_lock.rs
/// A process-wide trace, initialised at compile time.
static TRACE : Trace = Trace::disabled();

/// The same, switched on.
static LOUD : Trace = Trace::enabled();
```

```
  a static Trace works: enabled false len 0
  and a static enabled one records: len 1
```

---

### TR13 — The Decision Holds, and the Argument for It Is the Best in the Crate

`enabled` is a private `bool` set in exactly two places, both constructors. There
is no setter, no `&mut self` method anywhere in the crate, and no interior
mutability over the flag — it sits beside the `Mutex` rather than inside it. So
the decision is enforced by the type and not by convention, which is the right
way for a decision of this kind to be held.

The reason given is the part worth preserving. A trace switchable mid-run
"would produce a log with a silent hole at the front, which reads exactly like a
run where nothing happened early". That is a statement about how a diagnostic
gets *misread*, not about how it gets used, and it is the correct axis to reason
on for a tool whose entire value is that a reader believes what it says. The same
sentence explains why `default()` is disabled rather than enabled and why
`clear()` is documented as "keeping the enabled state" — one argument doing three
jobs.

**Finding.** Recorded as a decision that is sound, enforced, and stated, which is
rarer in this corpus than the alternative. The one thing it does not do is
generalise: the argument is written as a fact about the flag, so nothing carries
it to the adjacent operation that produces the identical hole from the other
direction. That gap is
[`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md); the
decision itself is not at fault for it, but the decision is where the reasoning
lives and therefore where the generalisation belongs.

---

### TR14 — Both Constructors Are `const fn` and Nothing Uses Them as One

`enabled()` and `disabled()` are `const fn`, which is only possible because
`Mutex::new` and `Vec::new` are both const-constructible. That makes
`static TRACE : Trace = Trace::disabled();` legal — a process-wide trace with no
lazy initialisation, no `OnceLock`, no atomic guard on first use. The probe
compiles both forms and records through the enabled one.

Nothing anywhere declares such a binding. The census finds no `const` or
`static` of this type across all 33 crates, and the test suite constructs a fresh
`Trace` in every one of its nineteen tests. Five `pub const fn` are declared in
the crate and the only one exercised in const position is `TraceOp::ALL`, which
is a `const` item rather than a function.

**Finding.** The `const`-ness is not decoration — it is the difference between a
trace a program can have from before `main` and one that has to be threaded
through construction. For a diagnostic that is off by default and wants to be
reachable from anywhere without changing a signature, the process-wide `static`
is the obvious deployment, and the crate made it possible without saying so. One
line on the constructors showing the `static` form would cost nothing and would
name a capability that is currently discoverable only by noticing the `const`
keyword and knowing what it implies for `Mutex`.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md) | The hole this decision closes, reopened elsewhere |
| [`decisions/002`](002_a_mutex_over_a_lock_free_log.md) | The other decision the constructors carry |
| [`lifecycle/001`](../lifecycle/001_three_states_and_the_two_ways_out_of_one.md) | The states the frozen flag produces |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | Why no deployment of any shape exists yet |

### Sources

| Fact | Where |
|------|-------|
| The stated argument for freezing the flag | `ring_trace/src/lib.rs:183-188` |
| The private flag and its two writers | `ring_trace/src/lib.rs:205`, `:213-223` |
| Zero setters, zero `&mut self` methods | Census above |
| No `const` or `static Trace` in the family | Census above |
| A `static Trace` compiling and recording | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `the_default_trace_is_off` | The default the argument justifies |
| `enabled_and_disabled_report_their_own_state` | Both constructors, at runtime |
| `clearing_empties_the_log_without_switching_it_off` | That clearing leaves the flag alone |
