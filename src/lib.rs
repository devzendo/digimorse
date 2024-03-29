// #![feature(const_fn)]
// #![feature(const_generics)]
//
#[macro_use]
extern crate enum_primitive;
extern crate lazy_static;
extern crate simple_error;

pub mod libs;

#[cfg(test)]
#[macro_use]
extern crate serial_test;
