# workaround

Two things this crate's users carry that nothing writes down. One is syntactic:
every byte payload published through `publish_into` must be sliced, because
unsizing coercion does not fire at a generic parameter, so `&b"abcd"[ .. ]` is
required where a direct call to the slot accepts `b"abcd"`. The other is
documentary: both prose documents describing this crate name a third dependency
the manifest does not declare and the source never imports.

The two share a mechanism. In each case the artefact a machine enforces is
correct and silent — the compiler demands the slice without explaining why, the
build declares two dependencies without saying the third was dropped — while the
artefact a human reads either does not mention the constraint or states the
opposite. Nothing here is broken; both are things a reader has to rediscover.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_slicing_expression_at_every_generic_call_site.md) | A Slicing Expression at Every Generic Call Site | The forced `[ .. ]`, the perfect call-site split, and the impl that removes it |
| [002](002_a_third_dependency_two_documents_still_name.md) | A Third Dependency Two Documents Still Name | The readme, the task file, the manifest, and the decomposition seam behind them |

## A Perfect Split Nobody Marked

Thirty-three byte writes in the workspace call `BytesSlot::write` directly and
every one passes a plain literal, `b"abcd"`. Five calls reach a byte slot
through `publish_into` and every one passes a slicing expression. Not a single
call site in either group uses the other form, because in each group the other
form does not compile.

`write` takes `payload : &[ u8 ]`, a concrete type, so an array unsizes at the
call site. `publish_into` takes `payload : P where P : Fill< S >`, and coercion
does not fire into an inference variable, so `&[ u8; 4 ]` stays itself and finds
no impl. This crate's own suite writes both forms fifty-six lines apart and
draws no attention to it; the crate's one byte doctest teaches the ritual as
though it were how one writes a byte literal.

## The Artefact the Build Checks Is the One That Got Fixed

Three documents state this crate's dependencies. `readme.md` and task 115 name
three crates; `Cargo.toml` declares two; the source imports from the third zero
times. The edge came from the design decomposition, was found unnecessary during
implementation, and was removed from exactly the one place where leaving it
would have failed a build.

The family census shows both directions of the same seam — `ring_trace`
carrying the identical undeclared claim, `ring_atomic` declaring an edge its
source never uses — and the underlying cause is a decomposition in which the
`Seq` type lives in `ring_types` while the arithmetic over it lives in the crate
named `ring_seqno`. A crate that holds a sequence without computing on one needs
the first and not the second, which is not what the names suggest.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two signatures, one generic and one concrete --'
command grep -m1 -A2 -F 'pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >' ring_event/src/lib.rs
command grep -m1 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- direct writes with a plain literal, then with a slice --'
command grep -rn '\.write( b"' --include=*.rs */ | wc -l
command grep -rn '\.write( &[a-z_]*\[ *\.\. *\]\|\.write( &b"' --include=*.rs */ | wc -l
echo '  -- and every byte fill that goes through this crate --'
command grep -rn 'publish_into( *&mut [a-z_]*, *&\[\|publish_into( *&mut [a-z_]*, *&b"' --include=*.rs */ 
echo '  -- the three dependency statements, and how they disagree --'
command grep -m1 -F 'Depends on [`ring_types`](../ring_types/readme.md), [`ring_slot`](../ring_slot/readme.md), [`ring_seqno`](../ring_seqno/readme.md).' ring_event/readme.md
command grep -m1 -F '**Depends on:** `ring_types`, `ring_slot`, `ring_seqno`' ring_event/task/unverified/115_implement_ring_event.md
sed -n '/^\[dependencies\]/,/^$/p' ring_event/Cargo.toml | command grep '^ring_'
echo '  -- the same seam across the family: readme / manifest / src --'
for c in ring_*/; do
  n=$( basename "$c" )
  r=$( command grep -c '^Depends on.*ring_seqno' "$c"readme.md 2>/dev/null || true )
  m=$( command grep -c '^ring_seqno *=' "$c"Cargo.toml 2>/dev/null || true )
  s=$( command grep -rc 'ring_seqno::' --include=*.rs "$c"src 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )
  if [ "$r" != 0 ] || [ "$m" != 0 ] || [ "$s" != 0 ]; then printf '  %-16s %-8s %-9s %s\n' "$n" "$r" "$m" "$s"; fi
done
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV49 | `ring_event` | n/a — doc gap | The call-site census splits perfectly: thirty-three direct `BytesSlot::write` calls across five crates all pass a plain byte-string literal and none passes a slice expression, while all five byte fills that go through `publish_into` — four tests and a doctest — pass a slice expression and none passes a literal, because `write` takes a concrete `payload : &[ u8 ]` into which `&[ u8; 4 ]` unsizes at the call site whereas `publish_into` takes `payload : P where P : Fill< S >` and coercion does not fire into an inference variable, so a probe passing `b"abcd"` fails with `E0277`, "not implemented for `&[u8; 4]` but it is implemented for `&[u8]`"; the crate whose stated argument is that there should be exactly one path thus makes that path the one costing a syntactic ritual at every call site, demonstrates both forms fifty-six lines apart in its own test file at `:183` and `:239`, and explains the difference nowhere, so the doctest at `:170` transmits the form by example without the reason |
| EV50 | `ring_event` | n/a — unadopted | A third impl removes the ritual and a replica probe shows it collides with nothing: `impl< const M : usize, const N : usize > Fill< BytesSlot< N > > for &[ u8; M ]` forwarding to `&self[ .. ]` coexists with both the blanket `impl< T > Fill< TypedSlot< T > > for T` and the existing slice impl, since the blanket impl's `S` is `TypedSlot< T >` and the new one's is `BytesSlot< N >`, and with it `publish_into( &mut c, b"abcd" )` compiles and lands four bytes while the sliced form keeps working; whether to take those six lines is a genuine tradeoff rather than an obvious yes, because a second byte impl is a second thing to keep in step and the crate's stated virtue is exactly one of everything, but as it stands neither the impl nor a recorded decision against it exists and the ritual propagates by example |
| EV51 | `ring_event` | **wrong doc** | Three artefacts state this crate's dependencies and the two prose ones agree with each other rather than with the code: `readme.md:5` names `ring_types`, `ring_slot` and `ring_seqno` with a working link to the third, task 115's `**Depends on:**` line names the same three, `Cargo.toml` declares two, and the source imports from `ring_seqno` zero times — both documents descending from the same early decomposition and both preserving an edge the implementation found unnecessary, with the manifest the only artefact corrected because it is the only one a build enforces; the same seam runs both directions across the family, `ring_trace` carrying the identical undeclared pair and `ring_atomic` declaring an edge its source never imports, and since this is the second staleness in the same task file after `lifecycle/002`'s `❓ Unverified` marker, the two one-line fixes sit in the same two files and should land together |
| EV52 | `ring_seqno` | n/a — inconsistency | The edge looked right because the decomposition's names imply a layout it does not have: this crate's tests genuinely use sequences — `land_and_read< S, P >( buffer, seq : Seq, payload : P )` is one of two generic helpers and four of sixteen tests address storage by one — but `Seq` is declared at `ring_types/src/id.rs:25` and nowhere in `ring_seqno/src`, which holds arithmetic over sequences rather than the type, and `event_test.rs:23` accordingly imports it from `ring_types`; a consumer therefore needs `ring_types` to hold a `Seq`, `ring_seqno` to compute on one, or both, and nothing in either crate says so, so the mistake will recur for every future crate that touches a position without measuring one — one sentence in `ring_seqno`'s readme stating that `Seq` itself lives in `ring_types` would have made this crate's readme visibly wrong instead of plausible |
