use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashSet;

// This is a trait mostly just so I don't mess up the lifetimes.
// It also lets me implement it on `Properties` (type alias for `Hashmap`)
pub trait AddDependencies {
    fn add_dependencies<'a: 'b, 'b>(&'a self, dependencies: &mut HashSet<&'b str>);
}