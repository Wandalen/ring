# type

Three types, and the compiler holds less of them than the declarations suggest.
Five kinds are written down in three places; the type system ties two of the
three pairings and holds the third in one direction only, which happens to be the
direction that does not drift. The one property the whole design rests on —
`Trace` being shareable across producer threads — is inferred from a field type
and stated nowhere.

Both are cases of a guarantee that looks structural and is partly social. The
family already owns the instrument for the second, uses it 44 times in six
crates, and did not use it here.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_five_discriminants_and_the_array_beside_them.md) | Five Discriminants and the Array Beside Them | Which of the three declarations the compiler ties, both drift directions compiled |
| [002](002_what_the_compiler_knows_about_these_three_types.md) | What the Compiler Knows About These Three Types | The derives, `Trace`'s auto traits, and the assertions six other crates write |

## What `[ Self; 5 ]` Is and Is Not

The array's declared length is a real constraint on the literal beside it — six
entries against `[ Self; 5 ]` gives `error[E0308]`, and the compiler names the
property: "expected an array with a size of 5, found one with a size of 6". It
constrains nothing else. Five entries that repeat `Claim` and omit `Drop` compile
without an error.

So the pairing that is held is the array against its own length, and the pairing
that is not is the array against the enum — the direction a new discriminant
takes, and the one the suite claims is covered. `name()`'s exhaustive match holds
the enum end properly, and `Display` is not a third obligation because it
delegates to `name()` rather than matching again.

## A Derive With No User and a User With No Derive

`TraceOp` derives `Hash` and the crate never hashes anything. It does not derive
`Ord`, and the suite's uniqueness check over `ALL` wants one — it sorts by
`op.name()` and dedups, building a set by hand with the order borrowed from the
strings. `HashSet< TraceOp >` collects `ALL` in one line and makes the same
assertion, using the derive that is already there.

`Trace` derives only `Debug`, and is `Send + Sync` because `Mutex` and `bool`
are. That is the crate's premise, arriving by inference. Six crates in the family
pin this class of property with a two-line generic assertion, 44 times between
them; this crate does it zero times.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- five kinds, three declarations, and what ties them --'
printf '    discriminants: %s   entries in ALL: %s   arms in name(): %s   declared array length: %s\n' \
  "$( command grep -m1 -A12 -F 'pub enum TraceOp' ring_trace/src/lib.rs | command grep -c '^  [A-Z][a-z]*,$' || true )" \
  "$( command grep -m1 -A7 -F '  pub const ALL : [ Self; 5 ] =' ring_trace/src/lib.rs | command grep -c 'Self::' || true )" \
  "$( command grep -m1 -A11 -F '  pub const fn name( self ) -> &'"'"'static str' ring_trace/src/lib.rs | command grep -c 'Self::[A-Z][a-z]* =>' || true )" \
  "$( command grep -h 'pub const ALL' ring_trace/src/lib.rs | command grep -o '[0-9]*' )"
echo '  -- the derives, and what the crate does with them --'
command grep -h '^#\[ derive' ring_trace/src/lib.rs | sed 's/^/    /'
printf '    hashing anywhere in the crate: %s   compile-time type assertions: %s\n' \
  "$( command grep -rc 'HashMap\|HashSet\|\.hash(' --include=*.rs ring_trace/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -rc 'assert_sync\|assert_send\|assert_copy\|assert_clone\|assert_debug' ring_trace/src/lib.rs ring_trace/tests/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
echo '  -- against the rest of the family --'
t=0
for c in ring_*/; do
  k=$( command grep -rc 'assert_sync\|assert_send\|assert_copy\|assert_clone\|assert_debug' "$c"src/lib.rs "$c"tests/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )
  t=$(( t + k ))
done
printf '    compile-time type assertions in 33 crates: %s\n' "$t"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR45 | `ring_trace` | n/a — observation | Five kinds are declared in three places — the enum's discriminants, `ALL`'s five entries, and `name()`'s five match arms, with `Display` not a fourth since it delegates via `f.write_str( self.name() )` — and the compiler ties two of the three pairings while holding the third in one direction only: listing a sixth entry against a declared `[ Self; 5 ]` gives `error[E0308]: mismatched types`, the compiler naming the property exactly, "expected an array with a size of 5, found one with a size of 6", while five entries that repeat `Claim` and omit `Drop` compile with zero errors; so the declared length constrains the literal beside it and nothing else, saying nothing about which discriminants appear or whether one appears twice, and the direction that matters — the enum growing while `ALL` stands still — is the unheld one, which is precisely what `item/002` shows the suite claiming is covered, so one clause on the constant naming which drift the type catches and which a human must catch costs a line and forecloses the reading the suite already fell into |
| TR46 | `ring_trace` | n/a — doc gap | `ALL` is `pub`, and therefore permanent surface a caller may depend on, iterate and hold the length of, while its doc describes an internal audience — "Every discriminant, for a test that must cover all of them" — which is currently exact, since no crate in the family other than `ring_trace` names `TraceOp` or `TraceEntry` at all and the constant's only user is the suite in the same crate; the two cannot stay aligned, because the vocabulary is a real one, five names for five ring operations, and enumerating it is a reasonable thing for an eventual caller to want while the doc tells that caller the constant exists for someone else's tests — so either the constant is public because callers should enumerate the vocabulary, in which case the doc should say that and drop the reference to tests, where a `#[ cfg( test ) ]` constant would otherwise live, or it is public only because an integration test in `tests/` cannot see a private item, which is the actual mechanical reason and worth stating outright since it tells a caller the guarantee is incidental rather than intended |
| TR47 | `ring_trace` | n/a — coverage | `Trace` is `Send` and `Sync` and has to be, every method taking `&self`, the module documentation opening on "a shared trace across producers" and the contention test handing one `&Trace` to four threads at once — yet the property arrives automatically from `Mutex< Vec< TraceEntry > >` and a `bool`, and nothing in the crate states or checks it, in a family that pins exactly this class of property 44 times across six crates (`ring_mpsc` thirteen, `ring_flush` ten, `ring_spsc` nine, `ring_core` six, `ring_handle` four, `ring_registry` two) in both doc examples and test files, `ring_flush`'s version being the one `pattern/002` records as the family's standard for a compile-time claim; this is also the crate where the assertion would carry most weight, `Sync` here being a consequence of a field type rather than a deliberate `unsafe impl`, so swapping `Mutex` for a `RefCell` — which someone optimising the single-threaded case might reasonably try, a disabled trace never contending — silently removes it, the first symptom being a compile error at whatever call site shares the trace, several crates away, where `fn assert_sync< T : Sync >() {}` and one turbofish call turn the premise into something the compiler holds |
| TR48 | `ring_trace` | n/a — observation | `TraceOp` derives `Hash` and the crate hashes nothing — zero `HashMap`, zero `HashSet`, zero `.hash()`, and no crate outside `ring_trace` names the type, so the derive has neither an internal nor an external user — while it does not derive `Ord`, which the suite wanted: its uniqueness check over `ALL` reads `unique.sort_unstable_by_key( \|op\| op.name() )` then `unique.dedup()`, a set built by hand with the total order borrowed from the string names, and asking for the direct form gives `error[E0277]: the trait bound TraceOp: Ord is not satisfied`; the two facts meet, since `TraceOp` being `Hash + Eq` means `HashSet< TraceOp >` collects `ALL` in one line and reports five distinct kinds, which is exactly the assertion the sort-and-dedup pair makes — so the derive the crate has would replace the workaround the crate wrote, twelve lines apart in the same test file, with neither mentioning the other, and `TraceEntry` deriving no `Hash` at all leaves the two types that travel together with different capabilities for no stated reason, the entry rather than the operation being what a caller deduplicating a log would key on |
