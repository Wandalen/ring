# Pattern: A Consuming Builder Over a Copy Record

### Scope

**Purpose:** Record the consuming-builder pattern as this crate applies it, and
the one guarantee the pattern loses when the receiver is `Copy`.

**Responsibility:** The four `with_*` setters, the `#[ must_use ]` that is
load-bearing rather than decorative, a family-wide census of every other
consuming builder, and the second `Copy` receiver that gets the same guarantee
from somewhere else.

**In Scope:** `ring_config/src/lib.rs:41`, `:88-143`;
`ring_bench/src/lib.rs:205`, `:196-206`; the consuming-builder census
across this family — `*/src`, flat, one crate per directory.

**Out of Scope:** What the setters do to their arguments is
[`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md). Why
they clamp rather than return `Result` is
[`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md).

---

## Every Consuming Builder in the Workspace

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every method in this family that takes `mut self`, with its receivers derives --'
# The last column is the one that matters. `fn name( mut self` is a syntax, not
# the concept: it also matches a *terminal* consumer that eats the receiver and
# returns something else entirely, which is not a builder and which the
# `must_use` argument below does not apply to. Classifying on the return type
# is what separates them, so the census says which each row is instead of
# calling all of them builders.
#
# The receiver is read from the enclosing `impl`, taking what follows `for`
# when there is one.
#
# Method names, not line numbers: the four `RingConfig` rows need
# distinguishing from each other, `with_wait` does that permanently, and a
# line number stops being true the next time anything above it is edited.
for f in $( command grep -rl 'fn [a-z_]*( mut self' --include=*.rs */src 2>/dev/null | LC_ALL=C sort )
do
  awk -v path="${f#/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/}" '
    FNR == NR {
      if ( $0 ~ /^#\[ derive/ ) { pending = $0 }
      else if ( $0 ~ /^pub struct / ) { name = $3; sub( /[^A-Za-z0-9_].*/, "", name ); drv[ name ] = pending; pending = "" }
      else if ( $0 !~ /^#\[/ ) { pending = "" }
      next
    }
    /^impl/ {
      ty = $0
      sub( /^impl/, "", ty ); sub( /^ *<[^>]*>/, "", ty ); sub( /.* for /, "", ty )
      sub( /^ */, "", ty ); sub( /[^A-Za-z0-9_].*/, "", ty )
    }
    /must_use/ { mu = 1; next }
    /fn [a-z_]*\( mut self/ {
      fn = $0; sub( /.*fn /, "", fn ); sub( /\(.*/, "", fn )
      ret = $0; sub( /.*-> */, "", ret ); sub( / *$/, "", ret )
      kind = ( ret == "Self" ) ? "builder" : ( ret ~ /^Result< *Self/ ) ? "builder, fallible" : "TERMINAL, not a builder"
      printf "%-42s %-19s %-26s must_use=%d  %-23s %s\n", path, ty, fn, mu, kind, drv[ ty ]
      mu = 0; next
    }
    { mu = 0 }
  ' "$f" "$f"
done
echo '  -- and the two that configure a ring, on the same zero --'
command grep -m1 -A9 -F '  pub fn with_producers( mut self, producers : usize ) -> Result< Self, WorkloadError >' ring_bench/src/lib.rs
command grep -m1 -A3 -F '  pub const fn with_producers( mut self, producers : usize ) -> Self' ring_config/src/lib.rs
```

Live output:

```
  -- every method in this family that takes `mut self`, with its receivers derives --
ring_bench/src/lib.rs                 Workload            with_producers             must_use=0  builder, fallible       #[ derive( Debug, Clone, Copy ) ]
ring_bench/src/lib.rs                 Workload            with_records_per_producer  must_use=0  builder, fallible       #[ derive( Debug, Clone, Copy ) ]
ring_bench/src/lib.rs                 Workload            with_batch                 must_use=0  builder, fallible       #[ derive( Debug, Clone, Copy ) ]
ring_bench/src/lib.rs                 Workload            with_cells                 must_use=0  builder, fallible       #[ derive( Debug, Clone, Copy ) ]
ring_bench/src/lib.rs                 Workload            with_semantics             must_use=1  builder                 #[ derive( Debug, Clone, Copy ) ]
ring_config/src/lib.rs                RingConfig          with_wait                  must_use=1  builder                 #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
ring_config/src/lib.rs                RingConfig          with_overflow              must_use=1  builder                 #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
ring_config/src/lib.rs                RingConfig          with_producers             must_use=1  builder                 #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
ring_config/src/lib.rs                RingConfig          with_batch                 must_use=1  builder                 #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
ring_flush/src/lib.rs                 Flusher             with_log                   must_use=0  builder                 #[ derive( Debug ) ]
ring_testkit/src/lib.rs               Script              then                       must_use=1  builder                 #[ derive( Debug, Clone, PartialEq, Eq ) ]
  -- and the two that configure a ring, on the same zero --
  pub fn with_producers( mut self, producers : usize ) -> Result< Self, WorkloadError >
  {
    if producers == 0
    {
      return Err( WorkloadError::ZeroProducers );
    }

    self.producers = producers;
    self.config = self.config.with_producers( producers );
    Ok( self )
  pub const fn with_producers( mut self, producers : usize ) -> Self
  {
    self.producers = if producers == 0 { 1 } else { producers };
    self
```

---

## What a Discarded Setter Actually Does

Three records, one line of code. `Guarded` is `RingConfig`'s exact shape —
`Copy`, with the attribute. `Bare` is the same record with the attribute
dropped. Both get their setter result discarded.

```sh
cd "$(git rev-parse --show-toplevel)"
cat > /tmp/-discard_probe.rs <<'RS'
#[ derive( Debug, Clone, Copy ) ] pub enum WaitKind { Spin, Park }

/// `RingConfig`'s exact shape: `Copy`, and the attribute that compensates for it.
#[ derive( Debug, Clone, Copy ) ] pub struct Guarded { wait : WaitKind }
impl Guarded
{
  #[ must_use ]
  const fn with_wait( mut self, wait : WaitKind ) -> Self { self.wait = wait; self }
}

/// The same record with the attribute dropped — the hazard being described.
#[ derive( Debug, Clone, Copy ) ] pub struct Bare { wait : WaitKind }
impl Bare
{
  const fn with_wait( mut self, wait : WaitKind ) -> Self { self.wait = wait; self }
}

fn main()
{
  let guarded = Guarded { wait : WaitKind::Spin };
  guarded.with_wait( WaitKind::Park );
  println!( "guarded, result discarded: wait {:?}", guarded.wait );

  let bare = Bare { wait : WaitKind::Spin };
  bare.with_wait( WaitKind::Park );
  println!( "bare,    result discarded: wait {:?}", bare.wait );
}
RS
# `--crate-name` is required because the hyphen-prefixed output path is not a
# legal crate name; the `-->` lines are dropped because they carry the probe's
# own /tmp path, which says nothing and differs per reader.
rustc -O -A dead_code --crate-name discard_probe -o /tmp/-discard_probe /tmp/-discard_probe.rs 2>&1 \
  | command grep -v '^ *--> ' | command grep -A6 '^warning: unused return value'
/tmp/-discard_probe
```

Live output:

```
warning: unused return value of `Guarded::with_wait` that must be used
   |
21 |   guarded.with_wait( WaitKind::Park );
   |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
guarded, result discarded: wait Spin
bare,    result discarded: wait Spin
```

Only `Guarded` produces a diagnostic, and both records read back `Spin` — the
setter ran, built a new value, and threw it away. `Bare` does it in silence.

Now the third record: the same setter shape with `Copy` itself removed.

```sh
cd "$(git rev-parse --show-toplevel)"
cat > /tmp/-moved_probe.rs <<'RS'
#[ derive( Debug, Clone, Copy ) ] pub enum WaitKind { Spin, Park }

/// `RingConfig`'s exact setter shape, minus the `Copy` derive.
#[ derive( Debug, Clone ) ] pub struct NotCopy { wait : WaitKind }
impl NotCopy
{
  #[ must_use ]
  const fn with_wait( mut self, wait : WaitKind ) -> Self { self.wait = wait; self }
}

fn main()
{
  let cfg = NotCopy { wait : WaitKind::Spin };
  cfg.with_wait( WaitKind::Park );
  println!( "{:?}", cfg.wait );
}
RS
rustc -O -A dead_code --crate-name moved_probe -o /tmp/-moved_probe /tmp/-moved_probe.rs 2>&1 \
  | command grep -v '^ *--> ' | command grep -B1 -A9 '^error\[E0382\]'
```

Live output:

```
error[E0382]: borrow of moved value: `cfg`
   |
13 |   let cfg = NotCopy { wait : WaitKind::Spin };
   |       --- move occurs because `cfg` has type `NotCopy`, which does not implement the `Copy` trait
14 |   cfg.with_wait( WaitKind::Park );
   |       --------------------------- `cfg` moved due to this method call
15 |   println!( "{:?}", cfg.wait );
   |                     ^^^^^^^^ value borrowed here after move
   |
note: `NotCopy::with_wait` takes ownership of the receiver `self`, which moves `cfg`
```

The line numbers in that diagnostic index the heredoc directly above it, which
is the whole of the program — nothing here refers outward.

**This section used to cite a scratch crate that no longer exists.** Both probes
lived in `-cfg_probe/`, a hyphen-prefixed directory, and the output above was
quoted from a run of it. The directory has since been swept, so for some time
the central evidence of this document was a transcript no reader could reproduce
and no reader could tell was unreproducible — the block looked exactly like
every other quoted result. It was also invisible to the corpus gate that checks
quoted output against a live re-run, because that gate pairs a `sh` block with a
`Live output:` block and this was a bare fence with no command attached to it.
The rewrite above fixes both: the program is now *in* the recipe, so running the
recipe is the only way the quote can exist, and a drift makes the gate fail
rather than making a reader wrong.

---

### RC37 — `Copy` Turns the Pattern's Compile Error Into a Warning, and `must_use` Is What Buys It Back

The consuming builder's safety property is not the fluent syntax. It is that the
receiver is moved. Write `cfg.with_wait( x );` and then use `cfg`, and on an
ordinary builder the compiler stops you: the setter took `self` by value, so the
old binding is gone and reading it is a use-after-move. You cannot lose a setter
by forgetting to bind its result, because forgetting does not compile.

`Copy` deletes that. The three probe records isolate each half against the same
line of code. `Guarded` — `RingConfig`'s shape — takes the discard, compiles,
stays readable, and still reads `Spin`: the setter ran, built a new value, and
threw it away. `NotCopy` is the same setter with the derive removed, and the
same line is `error[E0382]`. So the derive is what turns the error into a
silence.

`Bare` is what says the rest. It keeps the derive and drops only the attribute,
and its discard is silent — no error, no warning, nothing. The pair
`Guarded`/`Bare` isolates the attribute exactly as `Guarded`/`NotCopy` isolates
the derive, and it is why the diagnostic exists at all: all four real setters
carry `#[ must_use ]`, and without it the census's `must_use=1` column would
read `0` and this document would have nothing to point at.

**Finding.** The attribute is load-bearing and reads as hygiene. Nothing in the
crate says that `#[ must_use ]` on these four functions is standing in for a
compile error the `Copy` derive removed — the setters' documentation covers what
they set and what they clamp, and the derive's line says nothing about the
interaction.

Two things follow from that. The first is a real difference in strength:
`unused_must_use` is a warn-level lint, so the discard is a warning under
`cargo build` and an error only under `RUSTFLAGS="-D warnings"`, which the
family's verification levels do set but a caller's own build need not. The
compile error it replaced was unconditional.

The second is that the two attributes are separable by an edit that looks
unrelated. Dropping `#[ must_use ]` from a setter as a formatting tidy, or adding
a fifth setter and not carrying the attribute across, silently restores the
lost-setter case with no other symptom. There is no test for this: the suite's
`each_setter_is_independent` and `setters_commute` both bind every result, as
correct code does, so they exercise the path where the attribute is never
consulted.

---

### RC38 — Two `Copy` Consuming Builders in This Family, Both Configure a Ring, and the Guarantee Tracks Fallibility, Not the Crate

The census finds eleven methods across this family that take `mut self`, and
all eleven are genuine consuming builders across four receiver types — none of
them is a terminal consumer that eats the receiver and hands back something
unrelated. Six of the eleven carry `#[ must_use ]`: this crate's four,
`ring_bench::Workload::with_semantics`, and `ring_testkit::Script::then`.

Sorting the four types by whether the receiver is `Copy` explains the rest.
`Flusher` and `Script` are not `Copy`, so the move already protects them and
the attribute is optional: `Script::then` carries it as belt and braces,
`Flusher`'s sole setter has none and needs none. Exactly two receivers are
`Copy`: `RingConfig` and `ring_bench::Workload`. Both are records that
configure a ring.

`Workload` has five setters, not three. Four of them — `with_producers`,
`with_records_per_producer`, `with_batch` and `with_cells` — return
`Result< Self, WorkloadError >`, so they carry no `must_use` of their own and
are safe regardless, because `Result` is `#[ must_use ]` in the standard
library: discarding one is a warning for free. The fifth, `with_semantics`,
returns `Self` directly, gets none of that free protection, and carries an
explicit `#[ must_use ]` — the same attribute `RingConfig` needs on every one
of its own setters, for the same reason.

**Finding.** So the guarantee `ring_config` pays for by hand, four times,
tracks fallibility rather than the crate it sits in — and fallibility is
exactly what this crate rejected. `decisions/002` records why the setters
clamp instead of returning `Result`: to keep a chain from needing a `?` in the
middle of it. The census puts a price on that choice which the decision does
not mention, and the two crates show it on the same input.
`Workload::with_producers` answers a zero with `Err( WorkloadError::ZeroProducers )`;
`RingConfig::with_producers` answers it with `1`. The harness checks first and
only then writes through, so the clamp is unreachable from the one caller that
had the alternative available. `Workload`'s own fifth setter confirms the split
rather than complicating it: the one time it, too, is infallible, it pays by
hand exactly like `RingConfig`'s four.

The pattern is applied correctly at all eleven builder sites, which is worth
saying plainly. What is missing is any record of why the four attributes have to be
there, in the one place where the derive that makes them necessary and the
attribute that compensates for it sit thirty lines apart in the same file.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_a_derived_reading_instead_of_a_stored_field.md) | The crate's other pattern, and the shadow copy that shows its counter-example |
| [`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md) | The infallibility choice this finding prices |
| [`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md) | What the four setters guarantee about their arguments |
| [`api/001`](../api/001_twelve_functions_eleven_of_them_const.md) | The eleven `must_use` attributes on the surface as a whole |
| [`data_structure/001`](../data_structure/001_twenty_six_bytes_of_fields_in_thirty_two_of_struct.md) | The `Copy` derive itself, and what it costs per hand-off |

### Sources

| Fact | Where |
|------|-------|
| The four setters and their `must_use` attributes | `ring_config/src/lib.rs:88-143` |
| The `Copy` derive that removes the move | `ring_config/src/lib.rs:41` |
| Eleven consuming builders, four receivers, two of them `Copy` | Census above |
| `Workload`'s fallible setters and its `Copy` derive | `ring_bench/src/lib.rs:205`, `:196-206` |
| The discard compiling on `Copy` and failing without it | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `each_setter_is_independent` | Every setter's result bound, the path where `must_use` is never consulted |
| `setters_commute` | The same, across four chained setters |
| `zero_producers_clamps_to_one` | The answer this crate gives where its sibling returns `Err` |
| `the_record_is_copy_and_compares_by_value` | The derive that turns the move into a copy |
