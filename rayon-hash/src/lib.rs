#![feature(alloc)]
#![feature(core_intrinsics)]
#![feature(dropck_eyepatch)]
#![feature(fused)]
#![feature(generic_param_attrs)]
#![feature(heap_api)]
#![feature(oom)]
#![feature(placement_new_protocol)]
#![feature(pub_restricted)]
#![feature(shared)]
#![feature(sip_hash_13)]
#![feature(unique)]

#![cfg_attr(test, feature(placement_in_syntax))]
#![cfg_attr(test, feature(test))]

extern crate alloc;
extern crate rand;
extern crate rayon;

pub use map::HashMap;
pub use set::HashSet;

mod bench;
mod hash;

pub mod map {
    //! A hash map implementation which uses linear probing with Robin
    //! Hood bucket stealing.
    pub use super::hash::map::*;
}

pub mod set {
    //! An implementation of a hash set using the underlying representation of a
    //! HashMap where the value is ().
    pub use super::hash::set::*;
}
