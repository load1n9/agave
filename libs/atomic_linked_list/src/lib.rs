// ported from https://github.com/theseus-os/Theseus/tree/theseus_main/libs/atomic_linked_list
#![no_std]
#![feature(stmt_expr_attributes)]

extern crate alloc;

/// A basic atomic linked list.
pub mod atomic_linked_list;

/// A basic map structure which is backed by an atomic linked list.
pub mod atomic_map;
