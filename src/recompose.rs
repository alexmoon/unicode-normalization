// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{decompose::Decompositions, BufferOverflow};
use core::fmt::{self, Write};

type Buffer = heapless::Vec<char, 32>;

#[derive(Debug, Clone)]
enum RecompositionState {
    Composing,
    Purging(usize),
    Finished(usize),
}

/// External iterator for a string recomposition's characters.
#[derive(Clone)]
pub struct Recompositions<I> {
    iter: Decompositions<I>,
    state: RecompositionState,
    buffer: Buffer,
    composee: Option<char>,
    last_ccc: Option<u8>,
}

#[inline]
pub fn new_canonical<I: Iterator<Item = char>>(iter: I) -> Recompositions<I> {
    Recompositions {
        iter: super::decompose::new_canonical(iter),
        state: self::RecompositionState::Composing,
        buffer: Buffer::new(),
        composee: None,
        last_ccc: None,
    }
}

#[inline]
pub fn new_compatible<I: Iterator<Item = char>>(iter: I) -> Recompositions<I> {
    Recompositions {
        iter: super::decompose::new_compatible(iter),
        state: self::RecompositionState::Composing,
        buffer: Buffer::new(),
        composee: None,
        last_ccc: None,
    }
}

impl<I: Iterator<Item = char>> Iterator for Recompositions<I> {
    type Item = Result<char, BufferOverflow>;

    #[inline]
    fn next(&mut self) -> Option<Result<char, BufferOverflow>> {
        use self::RecompositionState::*;

        loop {
            match self.state {
                Composing => {
                    for ch in self.iter.by_ref() {
                        let ch = match ch {
                            Ok(ch) => ch,
                            Err(err) => return Some(Err(err)),
                        };
                        let ch_class = super::char::canonical_combining_class(ch);
                        let k = match self.composee {
                            None => {
                                if ch_class != 0 {
                                    return Some(Ok(ch));
                                }
                                self.composee = Some(ch);
                                continue;
                            }
                            Some(k) => k,
                        };
                        match self.last_ccc {
                            None => match super::char::compose(k, ch) {
                                Some(r) => {
                                    self.composee = Some(r);
                                    continue;
                                }
                                None => {
                                    if ch_class == 0 {
                                        self.composee = Some(ch);
                                        return Some(Ok(k));
                                    }
                                    if self.buffer.push(ch).is_err() {
                                        return Some(Err(BufferOverflow));
                                    }
                                    self.last_ccc = Some(ch_class);
                                }
                            },
                            Some(l_class) => {
                                if l_class >= ch_class {
                                    // `ch` is blocked from `composee`
                                    if ch_class == 0 {
                                        self.composee = Some(ch);
                                        self.last_ccc = None;
                                        self.state = Purging(0);
                                        return Some(Ok(k));
                                    }
                                    if self.buffer.push(ch).is_err() {
                                        return Some(Err(BufferOverflow));
                                    }
                                    self.last_ccc = Some(ch_class);
                                    continue;
                                }
                                match super::char::compose(k, ch) {
                                    Some(r) => {
                                        self.composee = Some(r);
                                        continue;
                                    }
                                    None => {
                                        if self.buffer.push(ch).is_err() {
                                            return Some(Err(BufferOverflow));
                                        }
                                        self.last_ccc = Some(ch_class);
                                    }
                                }
                            }
                        }
                    }
                    self.state = Finished(0);
                    if self.composee.is_some() {
                        return self.composee.take().map(Ok);
                    }
                }
                Purging(next) => match self.buffer.get(next).cloned() {
                    None => {
                        self.buffer.clear();
                        self.state = Composing;
                    }
                    s => {
                        self.state = Purging(next + 1);
                        return s.map(Ok);
                    }
                },
                Finished(next) => match self.buffer.get(next).cloned() {
                    None => {
                        self.buffer.clear();
                        return self.composee.take().map(Ok);
                    }
                    s => {
                        self.state = Finished(next + 1);
                        return s.map(Ok);
                    }
                },
            }
        }
    }
}

impl<I> Recompositions<I> {
    /// Converts the given value to a [`heapless::String`].
    pub fn to_string<const N: usize>(&self) -> Result<heapless::String<N>, BufferOverflow>
    where
        I: Iterator<Item = char> + Clone,
    {
        let mut res = heapless::String::new();
        for ch in self.clone() {
            res.push(ch?).map_err(|_| BufferOverflow)?;
        }
        Ok(res)
    }
}

impl<I: Iterator<Item = char> + Clone> fmt::Display for Recompositions<I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for c in self.clone() {
            f.write_char(c.map_err(|_| core::fmt::Error)?)?;
        }
        Ok(())
    }
}
