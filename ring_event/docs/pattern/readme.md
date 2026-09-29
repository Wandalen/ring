# pattern

One shape explains this entire crate: put the varying behaviour behind a trait
implemented on whichever side actually varies, then write the operation once as a
free function generic over that trait. All three functions are that shape applied
three times — `Fill` on the payload for the write, `Peek` on the slot for the
read, `ring_slot`'s existing `Slot` for the reset.

The second instance turns the same shape on the tests. Instead of asserting
parity twice, the suite writes the round-trip once with no shape named and
instantiates it at both — so "identical path" is discharged by the type checker
rather than by a pair of matching assertion lists.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_trait_on_the_varying_side_and_one_function_over_it.md) | A Trait on the Varying Side and One Function Over It | The shape, every instance of it in the family, and the one that is wired |
| [002](002_proof_by_monomorphisation.md) | Proof by Monomorphisation | The suite's structural parity proof, and exactly where it stops |

## Half the Family's Abstraction, in Two Hundred Lines

Thirty-three crates declare four public traits between them, and this crate
declares two of them. Its `Peek::Out` and the two definitions of it are the only
generic associated types anywhere in the family — the most advanced type-level
construct in the codebase, in the crate with the fewest executable statements and
no data of its own.

The pattern is not exotic here, only concentrated. `ring_atomic::SeqCell` is the
same construct, and it works across a crate boundary: three implementors spread
over `ring_atomic` and `ring_cursor`, consumed by `ring_batch::claim< C : SeqCell >`,
which `ring_tls` calls from production code. `Fill` and `Peek` have four impls
and all four sit in the crate that declares the traits. The difference between
the two is a caller, not a design.

## What a Shared Body Does and Does Not Prove

`round_trip< S, P >` is written once, names no shape, and its doc comment calls
itself "the whole point". Three tests instantiate it and its storage-level
sibling; the remaining thirteen name a concrete shape, eleven of them writing
`TypedSlot::<` or `BytesSlot::<` directly.

That division is the argument. Monomorphisation shares source, not conduct: the
same body, run at three instantiations, answers `true`, `true` and `false` — the
byte shape at a zero-length payload returning `None` where no typed payload can.
The three generic tests prove the path is one path; the thirteen concrete ones
cover everywhere that sharing a path does not make two shapes the same. The file
is sectioned accordingly and never says that is what the sections are for.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every public trait in the 33-crate family, and every impl of one --'
command grep -rn '^pub trait ' --include=*.rs ring_*/src
command grep -rn 'impl.* SeqCell for \|impl.* Slot for \|impl.* Fill< \|impl.* Peek for ' --include=*.rs ring_*/src
echo '  -- every generic associated type in the family --'
command grep -rn 'type [A-Za-z]*< .a >' --include=*.rs ring_*/src
echo '  -- the production call that crosses a boundary through a generic function --'
command grep -n 'let claim = claim(' ring_tls/src/lib.rs
command grep -m1 -F 'pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim' ring_batch/src/lib.rs
echo '  -- the suites generic helper, and how many tests reach one --'
command grep -m1 -B1 -A8 -F '/// The whole point: one body, no shape named, used by both shapes below.' ring_event/tests/event_test.rs
awk '/^fn /{ f = $0; hit = 0 } /round_trip\(|land_and_read\(/{ if ( f != "" && !hit ) { print "  " f; hit = 1 } }' ring_event/tests/event_test.rs
command grep -c '^#\[ test \]' ring_event/tests/event_test.rs || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV37 | `ring_event` | n/a — doc gap | Thirty-three crates declare four public traits between them and this one declares two of them, `Fill` and `Peek`, while `Peek::Out` and its two definitions are the only generic associated types anywhere in the family — the most advanced type-level construct in the codebase, in a 204-line crate with two executable statements and no data of its own; all three of its functions are one shape applied three times (the varying behaviour behind a trait on whichever side varies, the operation written once as a free function generic over it), and the module documentation explains each decision individually in the language of its specific problem while never naming the shape or observing that the three are the same shape, which for a crate whose stated value is that there is only one of each operation is the generalisation most worth writing down |
| EV38 | `ring_event` | n/a — unadopted | The family already runs this exact pattern across a crate boundary and it works: `ring_atomic::SeqCell` has three implementors spread over two crates — `AtomicSeq` and `CountingSeq` in `ring_atomic`, `PaddedCursor` in `ring_cursor` — consumed by `ring_batch::claim< C : SeqCell >`, which `ring_tls/src/lib.rs:280` calls from production code without knowing which implementor it holds, whereas `Fill` and `Peek` have four impls between them and all four sit in the crate that declares the traits, with no production caller for any generic function over them; the usual doubt about an abstraction like this — whether it will be taken up — has therefore already been answered next door, and `ring_cursor` needed neither a new crate nor a redesign to answer it, only the trait's crate as a dependency and an impl block |
| EV39 | `ring_event` | n/a — observation | This crate's "identical path" claim is discharged structurally rather than by assertion: `round_trip< S, P >` is written once, names no shape, carries the doc comment "The whole point: one body, no shape named, used by both shapes below", and is instantiated at `TypedSlot` and `BytesSlot` by two tests named after the argument, with `land_and_read< S, P >` repeating it one level up through `ring_store::Buffer< S >` — so if the body compiles at both, there is exactly one source path and no later edit can give a shape its own without changing the shared signature; this is the strongest form the claim can take from inside the crate and it costs three tests where a conventional suite needs six, which is also why `integration/002`'s finding is easy to miss from here, the proof being genuine and complete about the source while the unrealised half lives one crate away |
| EV40 | `ring_event` | n/a — doc gap | Monomorphisation shares source and not conduct — the compiler emits one instantiation per `(S, P)` pair, each calling that pair's own impls — and the suite's own helper run at three instantiations returns `true`, `true` and `false`, the byte shape at a zero-length payload answering `None` where no typed payload can; the suite is not naive about this, since only three of its sixteen tests go through a generic helper and the other thirteen name a concrete shape (eleven writing `TypedSlot::<` or `BytesSlot::<` directly) precisely to cover the divergences, but nothing says that is the division of labour, so a reader taking "the whole point" at face value reads the concrete tests as ordinary coverage rather than as the other half of the argument — one sentence above the concrete section would make the file self-describing |
