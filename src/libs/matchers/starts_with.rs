extern crate hamcrest2;

use std::fmt;
use std::vec::Vec;
use hamcrest2::core::{Matcher, MatchResult, success};


#[derive(Clone)]
pub struct StartsWith<T> {
    items: Vec<T>,
}

impl<T> StartsWith<T> {
    /// Constructs new `StartsWith` matcher with the default options.
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
        }
    }
}

impl<T> From<Vec<T>> for StartsWith<T> {
    fn from(items: Vec<T>) -> StartsWith<T> {
        StartsWith::new(items)
    }
}

impl<T> From<T> for StartsWith<T> {
    fn from(item: T) -> StartsWith<T> {
        StartsWith::new(vec![item])
    }
}

impl<T: fmt::Debug> fmt::Display for StartsWith<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "starting with {}", Pretty(&self.items))
    }
}

impl<'a, T: fmt::Debug + PartialEq + Clone> Matcher<&'a [T]> for StartsWith<T> {
    fn matches(&self, actual: &[T]) -> MatchResult {
        let rem = actual.to_vec();
        if actual.len() < self.items.len() {
            return Err(format!("{} isn't at least {} long", Pretty(&actual), self.items.len()))
        }

        for (pos, item) in self.items.iter().enumerate() {
            if rem[pos] != *item {
                return Err(format!("{} does not contain {:?} at index {}", Pretty(&actual), *item, pos));
            }
        }

        success()
    }
}

/// Creates matcher that checks if actual data starts with given item(s).
pub fn starts_with<T, I>(item: I) -> StartsWith<T>
    where
        I: Into<StartsWith<T>>,
{
    item.into()
}




// Had to copy this since it's pub(crate) in hamcrest2.
pub struct Pretty<'a, T: 'a>(pub &'a [T]);

impl<'a, T: fmt::Debug> fmt::Display for Pretty<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, t) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", t)?;
        }
        write!(f, "]")
    }
}
