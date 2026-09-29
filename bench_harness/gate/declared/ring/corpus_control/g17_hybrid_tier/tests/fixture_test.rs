//! Tests the control fixture's instances cite. Nothing here exercises a real
//! crate — the names exist so G16's citation check has something to resolve.

#[ test ]
fn the_first_branch_is_taken()
{
  assert_eq!( 1 + 1, 2 );
}

#[ test ]
fn the_second_branch_is_taken()
{
  assert_eq!( 2 + 2, 4 );
}
