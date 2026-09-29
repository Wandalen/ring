# type

Two type-level declarations carry this crate, and both are maximally permissive.
`Peek::Out` is the family's only generic associated type and carries no trait
bounds, so it can be any type at all — and consequently a generic body holding
one can do nothing with it. `Fill`'s blanket impl carries no bounds either, so
every type in the language implements it, including the crate's own error type
and both slot shapes.

Neither declaration is documented where it is written. The reasoning for both
lives one level up — a `##` section of module documentation for the associated
type, a paragraph of trait documentation for the blanket impl — and both explain
the policy rather than the declaration that delivers it. The crate's one lint,
`deny( missing_docs )`, cannot ask for better: it does not reach impl blocks, and
the four in this file are exactly where the traits acquire their meaning.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_an_associated_type_with_a_lifetime.md) | An Associated Type With a Lifetime | The GAT, its three compiler-forced `where` clauses, and what generic code can do with it |
| [002](002_a_blanket_impl_over_every_type_there_is.md) | A Blanket Impl Over Every Type There Is | An unbounded `T`, the two-impl overlap, and slots nesting inside slots |

## Permissive on Both Halves, and What Each Permission Costs

An unbounded associated type is the right default for a read half serving two
shapes that return genuinely different things, and it costs generic callers
every operation: no `Debug`, no `PartialEq`, no `Clone`. The suite shows the
line exactly — `round_trip` returns `Option< S::Out< '_ > >` untouched and the
thirteen comparisons happen at concrete instantiations, while `land_and_read`,
the one helper that consumes the value inside a generic body, calls `.is_some()`
because that is the only method available.

An unbounded blanket impl is the right default for a write half whose whole
argument is that adding a payload kind never touches the slot types, and it costs
the ability to ever narrow. `T : Send` or `T : 'static` would be ordinary things
to want of a payload crossing a ring; both are foreclosed for every downstream
caller by an impl that has already claimed every type.

## What the Bounds Turn Out to Be

The three `where` clauses on the read half look inconsistent — `Self : 'a` on
the trait, `T : 'a` on the typed impl, nothing on the byte impl — and a probe
that removes each in turn shows every one of them is exactly what the compiler
demands. The trait's is the interesting case: the error text does not call it
sound, it calls it "currently required to ensure that impls have maximum
flexibility" and cites an open language issue soliciting feedback. It is a
placeholder for a question Rust has not settled, sitting undocumented in the
family's only GAT.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the read half: one declaration, two definitions, three where clauses --'
command grep -m1 -A1 -F '  /// What a reader is handed when the slot holds something.' ring_event/src/lib.rs
command grep -m1 -F '  type Out< '"'"'a > = &'"'"'a T where T : '"'"'a;' ring_event/src/lib.rs
command grep -m1 -F '  type Out< '"'"'a > = &'"'"'a [ u8 ];' ring_event/src/lib.rs
echo '  -- and they are the only generic associated types in 33 crates --'
command grep -rn 'type [A-Za-z]*< .a >' --include=*.rs ring_*/src | wc -l
echo '  -- the write half: an impl with no bounds at all --'
command grep -m1 -F 'impl< T > Fill< TypedSlot< T > > for T' ring_event/src/lib.rs
command grep -m1 -F 'impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]' ring_event/src/lib.rs
echo '  -- what documents each, one level up --'
command grep -m1 -A1 -F '//! ## Why the read half is a GAT' ring_event/src/lib.rs
command grep -m1 -A2 -F '/// Implemented on the *payload*, not the slot, so adding a payload kind never' ring_event/src/lib.rs
echo '  -- and the lint that cannot reach either declaration --'
command grep -m1 -F '#![ deny( missing_docs ) ]' ring_event/src/lib.rs
awk '/^impl/{ printf "  line %-4d doc above: %-4s %s\n", NR, ( p ~ /^ *\/\/\// ? "yes" : "no" ), $0 } { p = $0 }' ring_event/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV45 | `ring_event` | n/a — doc gap | The family's only generic associated type carries three `where` clauses that read as inconsistent — `Self : 'a` on the trait declaration, `T : 'a` on the typed impl, none on the byte impl — and a probe removing each in turn shows all three are exactly what the compiler demands and that two of them are coupled, since dropping the trait's bound while keeping the impl's yields `E0276` "impl has stricter requirements than trait" while dropping the impl's yields `E0309`; the trait's bound is the notable one, because the compiler does not call it sound but "currently required to ensure that impls have maximum flexibility", citing rust-lang issue #87479 as still soliciting feedback, so a provisional language rule sits unexplained in the crate's most advanced declaration while the module documentation spends a whole `##` section on why an associated type with a lifetime is the right *choice* and the declaration itself gets one line describing the type and none of its syntax |
| EV46 | `ring_event` | n/a — doc gap | `Peek::Out` declares no supertrait bounds, which is what lets `BytesSlot` return `&[ u8 ]` and `TypedSlot< T >` return `&T` for an unbounded `T`, and the cost is that a generic body holding an `S::Out< '_ >` has no operation available on it — a probe attempting `println!( "{:?}", drain_from( slot ) )` in a generic function fails with `E0277`, `<S as Peek>::Out<'_>` doesn't implement `Debug`; the suite marks the boundary exactly, `round_trip< S, P >` returning the value untouched with its thirteen `assert_eq!` comparisons happening at concrete instantiations after monomorphisation, and `land_and_read< S, P >`, the one helper consuming the value inside a generic body, calling `.is_some()` because that is the only method an unbounded associated type permits, yet nothing records that a future generic reader in `ring_core` or `ring_debug` must supply its own bound, nor that putting `Debug` on the trait would tax every payload type forever |
| EV47 | `ring_event` | n/a — doc gap | `impl< T > Fill< TypedSlot< T > > for T` carries no bounds whatsoever, so the set of types implementing `Fill` is the set of types — a probe lands `()`, the crate's own `RingError` and a `BytesSlot< 2 >` through one generic function without annotation — and two type-level consequences follow that nothing states: `&[ u8 ]` implements `Fill` twice at two different `S` (`TypedSlot< &[ u8 ] >` by the blanket impl, `BytesSlot< N >` by the explicit one), producing results that print identically while typed `Option< &[u8] >` and `Option< &&[u8] >`; and `Fill` and `Slot` are not disjoint, so a slot is a legal payload. Meanwhile the crate's only lint, `#![ deny( missing_docs ) ]`, does not apply to impl blocks, and all four impls in the file carry no doc line above them — the one mechanism enforcing documentation structurally cannot see the four declarations that decide what the traits mean, and narrowing the impl later (to `T : Send`, say) would break every downstream payload |
| EV48 | `ring_event` | n/a — observation | Because a slot is a payload, `TypedSlot< TypedSlot< u32 > >` type-checks and the probe walks it: `drain_from` returns the inner slot rather than its contents, so reading takes one call per level (`TypedSlot(Some(7))`, then `Some(7)`); `recycle` on the outer reports it empty having dropped the inner whole rather than clearing it, which is why the byte-slot residue does not compound through nesting; and `Fill::fill` of an already-occupied `BytesSlot< 2 >` into a `TypedSlot< BytesSlot< 2 > >` returns `Ok( () )`, since a refusal cannot cross a level — only the byte impl can refuse and the byte impl is never the outer one. Every behaviour is sensible and none is written down, and while an accidental nest is caught at compile time by the outer type mismatch, one sentence on `Fill` saying a slot is itself a legal payload and each operation acts on one level converts a discovery into a documented property |
