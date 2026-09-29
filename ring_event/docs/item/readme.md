# item

Six declarations, four impl blocks and no attributes. This definition reads the
crate one declaration at a time — where each item sits, how much documentation it
carries, whether that documentation runs, what decorates it, and who calls it —
on the principle that a census of declarations answers questions a census of the
surface cannot. `api/` asks what the shape costs a caller; this asks what the
code actually says.

Two things fall out that neither the surface nor the behaviour would have shown.
`ring_event` is the only one of the family's 33 crates carrying no item
attributes at all, which is right four times over and wrong once. And the
unbounded `Fill` impl, which looks like it consumes the whole typed design space,
claims only the diagonal — leaving `Fill` open as a conversion point the crate
never mentions.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_five_public_items_and_their_traffic.md) | Five Public Items and Their Traffic | Declarations, doc sizes, the empty attribute census, and every call site |
| [002](002_four_impls_and_what_the_blanket_one_does_not_claim.md) | Four Impls, and What the Blanket One Does Not Claim | The diagonal, the conversion point, and the orphan rule's asymmetry |

## The Census

`Fill` at `:53`, `Peek` at `:103`, `Peek::Out` at `:106`, then the three free
functions at `:173`, `:207` and `:229`. Five public items, one associated type,
154 documentation lines in a 234-line file, five doctests — one on every item
except `Out`. `publish_into` carries the longest doc block in the crate at 23
lines above a one-expression body.

Traffic is almost entirely internal: the three free functions are called 38 times
across the crate's own doctests and suite against twice outside it, both in
`ring_tls`'s test file. `Fill::fill` and
`Peek::peek` are never called by name anywhere else in the workspace — the two
apparent hits are `BinaryHeap::peek` and an unrelated `fill` on
another crate's block handle.

## What Nothing Protects

The crate's zero attributes are correct for `#[ derive ]` (no data),
`#[ inline ]` (generic bodies instantiate in the caller) and `publish_into`
(`Result` carries `#[ must_use ]` in `core`). They are not correct for
`drain_from`, whose `Option` return is silently discardable because `Option`
carries no such attribute.

The same gap runs deeper next door. `ring_slot` carries six `#[ must_use ]`
attributes and every one sits on a constructor or a query. `set` and `take` —
the two methods that hand back an owned payload, the two where discarding loses
something — carry none, and the family's three call sites of `set` treat the
return three different ways: bound and debug-asserted in `ring_core`, explicitly
dropped in `ring_spsc`, silently dropped in `ring_mpsc`.

## What the Blanket Impl Leaves Open

`impl< T > Fill< TypedSlot< T > > for T` covers `T` into `TypedSlot< T >` and
nothing else. A downstream `impl Fill< TypedSlot< u64 > > for Millis` compiles
beside it and both stay reachable, selected by the slot type at the call site —
so a payload may transform on the way in, which is a capability the crate's
"deliberately trivial" description of `publish_into` reads as ruling out.

The byte half has the opposite property. Trait, slot and payload are all foreign
to a downstream crate, so `impl Fill< BytesSlot< N > > for Vec< u8 >` is `E0117`,
and the one shipped impl is `for &[ u8 ]` — which a byte-string literal does not
satisfy either, since `&[ u8; 4 ]` is not `&[ u8 ]`. All four byte publishes in
the workspace are written `&b"…"[ .. ]`, because no other form compiles.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every public item, its doc block and whether that block runs --'
awk '
  /^ *\/\/\// { d++; if ( $0 ~ /```/ ) { t = 1 } ; next }
  /^ *(pub (trait|fn) |type Out< .a > where)/ { printf "%-84s line %-4d doc %-3d doctest %s\n", $0, NR, d, ( t ? "yes" : "no" ) }
  { d = 0 ; t = 0 }
' ring_event/src/lib.rs
echo '  -- attributes here, against the family --'
for c in ring_*/; do
  n=$( command grep -rhc '^ *#\[' --include=*.rs "$c"src 2>/dev/null | awk '{ s += $1 } END { print s + 0 }' )
  echo "$n $( basename $c )"
done | sort -n | sed -n '1,5p;31,33p'
echo '  -- and where ring_slot puts the six it has --'
awk '
  /^ *#\[/ { a = $0 ; next }
  /^ *pub (const )?fn |^pub struct / { if ( a != "" ) { printf "%-4d %-34s %s\n", NR, a, $0 } ; a = "" }
' ring_slot/src/lib.rs
echo '  -- the four impls, and the three treatments of what set returns --'
command grep -n '^impl' ring_event/src/lib.rs
command grep -rn '\.set( ' --include=*.rs ring_core/src ring_mpsc/src ring_spsc/src
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV25 | `ring_event` | n/a — observation | This is the only one of the family's 33 crates carrying no item attributes in its sources — against 43 in `ring_bench`, 33 in `ring_core` and between one and three in the quietest four after it — and the absence is right four times over: nothing to derive because the crate declares no data, nothing to mark `#[ non_exhaustive ]` for the same reason, `#[ inline ]` redundant on generic bodies that instantiate in the caller's crate, and `publish_into` already protected by `Result`'s own `#[ must_use ]`, which a probe confirms by warning on exactly that discard; the fifth is wrong, because `drain_from` returns an `Option` and `Option` carries no such attribute, so `drain_from( &slot );` compiles silently in a crate whose whole purpose is to be the read path — harmless as a mutation but meaningless as a statement, and one line fixes it |
| EV26 | `ring_slot` | **latent hazard** | All six `#[ must_use ]` attributes in `ring_slot` sit on constructors and queries — `TypedSlot::empty`, `BytesSlot::empty`, `capacity`, `len`, `is_empty`, `read` — where discarding the result costs nothing, while `set` at `:90` and `take` at `:117`, the two methods that hand back an owned payload, carry none, and `set`'s own doc says the return value is the whole point of the signature because it is what lets `ring_overflow` hand a caller what it evicted "instead of losing it silently"; the family's three call sites treat it three different ways — `ring_core:369` binds and `debug_assert!`s it, `ring_spsc:719` writes `drop( … )`, `ring_mpsc:914` drops it where it falls — all correct today because a claimed slot is empty, but only one of the three says so, and if that ever stopped holding `ring_core` would trip its assertion while `ring_mpsc` silently discarded a live payload |
| EV27 | `ring_event` | n/a — doc gap | `impl< T > Fill< TypedSlot< T > > for T` reads as consuming the entire typed design space and claims only the diagonal: a probe's downstream `impl Fill< TypedSlot< u64 > > for Millis` compiles beside it, both stay reachable, and the slot type at the call site selects between them — landing `3000` in one and `Millis( 3 )` in the other; that makes `Fill` a conversion point, since the payload owns `fill` and may normalise, scale, validate or timestamp on the way in and the ring's publish path will run it, which is arguably the design's best property and is exactly what a reader takes `publish_into`'s "deliberately trivial" to rule out — true of the function, false of the operation it names |
| EV28 | `ring_event` | n/a — doc gap | The extension point is open on the typed side and closed on the byte side: `Fill`, `BytesSlot` and `Vec< u8 >` are all foreign to a downstream crate, so `impl Fill< BytesSlot< N > > for Vec< u8 >` is `E0117` and so is the same impl for `String`, `&str`, `Box< [ u8 ] >` or `Cow< '_, [ u8 ] >`, leaving `for &[ u8 ]` as the only route in — which a byte-string literal does not take either, since `&[ u8; 4 ]` is not `&[ u8 ]` and no unsizing happens while resolving `P : Fill< S >`, so all four byte publishes in the workspace are written `&b"…"[ .. ]` because no other form compiles; neither limit is a defect and rustc names the fix in both diagnostics, but the asymmetry is invisible from the trait declaration and the crate's account of `Fill` as the open extension point does not qualify itself |
