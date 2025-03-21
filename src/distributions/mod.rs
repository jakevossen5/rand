// Copyright 2018 Developers of the Rand project.
// Copyright 2013-2017 The Rust Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Generating random samples from probability distributions
//!
//! This module is the home of the [`Distribution`] trait and several of its
//! implementations. It is the workhorse behind some of the convenient
//! functionality of the [`Rng`] trait, e.g. [`Rng::gen`] and of course
//! [`Rng::sample`].
//!
//! Abstractly, a [probability distribution] describes the probability of
//! occurrence of each value in its sample space.
//!
//! More concretely, an implementation of `Distribution<T>` for type `X` is an
//! algorithm for choosing values from the sample space (a subset of `T`)
//! according to the distribution `X` represents, using an external source of
//! randomness (an RNG supplied to the `sample` function).
//!
//! A type `X` may implement `Distribution<T>` for multiple types `T`.
//! Any type implementing [`Distribution`] is stateless (i.e. immutable),
//! but it may have internal parameters set at construction time (for example,
//! [`Uniform`] allows specification of its sample space as a range within `T`).
//!
//!
//! # The `Standard` distribution
//!
//! The [`Standard`] distribution is important to mention. This is the
//! distribution used by [`Rng::gen`] and represents the "default" way to
//! produce a random value for many different types, including most primitive
//! types, tuples, arrays, and a few derived types. See the documentation of
//! [`Standard`] for more details.
//!
//! Implementing `Distribution<T>` for [`Standard`] for user types `T` makes it
//! possible to generate type `T` with [`Rng::gen`], and by extension also
//! with the [`random`] function.
//!
//! ## Random characters
//!
//! [`Alphanumeric`] is a simple distribution to sample random letters and
//! numbers of the `char` type; in contrast [`Standard`] may sample any valid
//! `char`.
//!
//!
//! # Uniform numeric ranges
//!
//! The [`Uniform`] distribution is more flexible than [`Standard`], but also
//! more specialised: it supports fewer target types, but allows the sample
//! space to be specified as an arbitrary range within its target type `T`.
//! Both [`Standard`] and [`Uniform`] are in some sense uniform distributions.
//!
//! Values may be sampled from this distribution using [`Rng::sample(Range)`] or
//! by creating a distribution object with [`Uniform::new`],
//! [`Uniform::new_inclusive`] or `From<Range>`. When the range limits are not
//! known at compile time it is typically faster to reuse an existing
//! `Uniform` object than to call [`Rng::sample(Range)`].
//!
//! User types `T` may also implement `Distribution<T>` for [`Uniform`],
//! although this is less straightforward than for [`Standard`] (see the
//! documentation in the [`uniform`] module). Doing so enables generation of
//! values of type `T` with  [`Rng::sample(Range)`].
//!
//! ## Open and half-open ranges
//!
//! There are surprisingly many ways to uniformly generate random floats. A
//! range between 0 and 1 is standard, but the exact bounds (open vs closed)
//! and accuracy differ. In addition to the [`Standard`] distribution Rand offers
//! [`Open01`] and [`OpenClosed01`]. See "Floating point implementation" section of
//! [`Standard`] documentation for more details.
//!
//! # Non-uniform sampling
//!
//! Sampling a simple true/false outcome with a given probability has a name:
//! the [`Bernoulli`] distribution (this is used by [`Rng::gen_bool`]).
//!
//! For weighted sampling from a sequence of discrete values, use the
//! [`WeightedIndex`] distribution.
//!
//! This crate no longer includes other non-uniform distributions; instead
//! it is recommended that you use either [`rand_distr`] or [`statrs`].
//!
//!
//! [probability distribution]: https://en.wikipedia.org/wiki/Probability_distribution
//! [`rand_distr`]: https://crates.io/crates/rand_distr
//! [`statrs`]: https://crates.io/crates/statrs

//! [`random`]: crate::random
//! [`rand_distr`]: https://crates.io/crates/rand_distr
//! [`statrs`]: https://crates.io/crates/statrs

use crate::Rng;
use core::iter;

pub use self::bernoulli::{Bernoulli, BernoulliError};
pub use self::float::{Open01, OpenClosed01};
pub use self::other::Alphanumeric;
#[doc(inline)] pub use self::uniform::Uniform;

#[cfg(feature = "alloc")]
pub use self::weighted_index::{WeightedError, WeightedIndex};

mod bernoulli;
pub mod uniform;

#[deprecated(since = "0.8.0", note = "use rand::distributions::{WeightedIndex, WeightedError} instead")]
#[cfg(feature = "alloc")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
pub mod weighted;
#[cfg(feature = "alloc")] mod weighted_index;

#[cfg(feature = "serde1")]
use serde::{Serialize, Deserialize};

mod float;
#[doc(hidden)]
pub mod hidden_export {
    pub use super::float::IntoFloat; // used by rand_distr
}
mod integer;
mod other;
mod utils;

/// Types (distributions) that can be used to create a random instance of `T`.
///
/// It is possible to sample from a distribution through both the
/// `Distribution` and [`Rng`] traits, via `distr.sample(&mut rng)` and
/// `rng.sample(distr)`. They also both offer the [`sample_iter`] method, which
/// produces an iterator that samples from the distribution.
///
/// All implementations are expected to be immutable; this has the significant
/// advantage of not needing to consider thread safety, and for most
/// distributions efficient state-less sampling algorithms are available.
///
/// Implementations are typically expected to be portable with reproducible
/// results when used with a PRNG with fixed seed; see the
/// [portability chapter](https://rust-random.github.io/book/portability.html)
/// of The Rust Rand Book. In some cases this does not apply, e.g. the `usize`
/// type requires different sampling on 32-bit and 64-bit machines.
///
/// [`sample_iter`]: Distribution::sample_iter
pub trait Distribution<T> {
    /// Generate a random value of `T`, using `rng` as the source of randomness.
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> T;

    /// Create an iterator that generates random values of `T`, using `rng` as
    /// the source of randomness.
    ///
    /// Note that this function takes `self` by value. This works since
    /// `Distribution<T>` is impl'd for `&D` where `D: Distribution<T>`,
    /// however borrowing is not automatic hence `distr.sample_iter(...)` may
    /// need to be replaced with `(&distr).sample_iter(...)` to borrow or
    /// `(&*distr).sample_iter(...)` to reborrow an existing reference.
    ///
    /// # Example
    ///
    /// ```
    /// use rand::thread_rng;
    /// use rand::distributions::{Distribution, Alphanumeric, Uniform, Standard};
    ///
    /// let mut rng = thread_rng();
    ///
    /// // Vec of 16 x f32:
    /// let v: Vec<f32> = Standard.sample_iter(&mut rng).take(16).collect();
    ///
    /// // String:
    /// let s: String = Alphanumeric
    ///     .sample_iter(&mut rng)
    ///     .take(7)
    ///     .map(char::from)
    ///     .collect();
    ///
    /// // Dice-rolling:
    /// let die_range = Uniform::new_inclusive(1, 6);
    /// let mut roll_die = die_range.sample_iter(&mut rng);
    /// while roll_die.next().unwrap() != 6 {
    ///     println!("Not a 6; rolling again!");
    /// }
    /// ```
    fn sample_iter<R>(self, rng: R) -> DistIter<Self, R, T>
    where
        R: Rng,
        Self: Sized,
    {
        DistIter {
            distr: self,
            rng,
            phantom: ::core::marker::PhantomData,
        }
    }
}

impl<'a, T, D: Distribution<T>> Distribution<T> for &'a D {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> T {
        (*self).sample(rng)
    }
}


/// An iterator that generates random values of `T` with distribution `D`,
/// using `R` as the source of randomness.
///
/// This `struct` is created by the [`sample_iter`] method on [`Distribution`].
/// See its documentation for more.
///
/// [`sample_iter`]: Distribution::sample_iter
#[derive(Debug)]
pub struct DistIter<D, R, T> {
    distr: D,
    rng: R,
    phantom: ::core::marker::PhantomData<T>,
}

impl<D, R, T> Iterator for DistIter<D, R, T>
where
    D: Distribution<T>,
    R: Rng,
{
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<T> {
        // Here, self.rng may be a reference, but we must take &mut anyway.
        // Even if sample could take an R: Rng by value, we would need to do this
        // since Rng is not copyable and we cannot enforce that this is "reborrowable".
        Some(self.distr.sample(&mut self.rng))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::max_value(), None)
    }
}

impl<D, R, T> iter::FusedIterator for DistIter<D, R, T>
where
    D: Distribution<T>,
    R: Rng,
{
}

#[cfg(features = "nightly")]
impl<D, R, T> iter::TrustedLen for DistIter<D, R, T>
where
    D: Distribution<T>,
    R: Rng,
{
}


/// A generic random value distribution, implemented for many primitive types.
/// Usually generates values with a numerically uniform distribution, and with a
/// range appropriate to the type.
///
/// ## Provided implementations
///
/// Assuming the provided `Rng` is well-behaved, these implementations
/// generate values with the following ranges and distributions:
///
/// * Integers (`i32`, `u32`, `isize`, `usize`, etc.): Uniformly distributed
///   over all values of the type.
/// * `char`: Uniformly distributed over all Unicode scalar values, i.e. all
///   code points in the range `0...0x10_FFFF`, except for the range
///   `0xD800...0xDFFF` (the surrogate code points). This includes
///   unassigned/reserved code points.
/// * `bool`: Generates `false` or `true`, each with probability 0.5.
/// * Floating point types (`f32` and `f64`): Uniformly distributed in the
///   half-open range `[0, 1)`. See notes below.
/// * Wrapping integers (`Wrapping<T>`), besides the type identical to their
///   normal integer variants.
///
/// The `Standard` distribution also supports generation of the following
/// compound types where all component types are supported:
///
/// *   Tuples (up to 12 elements): each element is generated sequentially.
/// *   Arrays (up to 32 elements): each element is generated sequentially;
///     see also [`Rng::fill`] which supports arbitrary array length for integer
///     types and tends to be faster for `u32` and smaller types.
///     When using `rustc` ≥ 1.51, enable the `min_const_gen` feature to support
///     arrays larger than 32 elements.
///     Note that [`Rng::fill`] and `Standard`'s array support are *not* equivalent:
///     the former is optimised for integer types (using fewer RNG calls for
///     element types smaller than the RNG word size), while the latter supports
///     any element type supported by `Standard`.
/// *   `Option<T>` first generates a `bool`, and if true generates and returns
///     `Some(value)` where `value: T`, otherwise returning `None`.
///
/// ## Custom implementations
///
/// The [`Standard`] distribution may be implemented for user types as follows:
///
/// ```
/// # #![allow(dead_code)]
/// use rand::Rng;
/// use rand::distributions::{Distribution, Standard};
///
/// struct MyF32 {
///     x: f32,
/// }
///
/// impl Distribution<MyF32> for Standard {
///     fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MyF32 {
///         MyF32 { x: rng.gen() }
///     }
/// }
/// ```
///
/// ## Example usage
/// ```
/// use rand::prelude::*;
/// use rand::distributions::Standard;
///
/// let val: f32 = StdRng::from_entropy().sample(Standard);
/// println!("f32 from [0, 1): {}", val);
/// ```
///
/// # Floating point implementation
/// The floating point implementations for `Standard` generate a random value in
/// the half-open interval `[0, 1)`, i.e. including 0 but not 1.
///
/// All values that can be generated are of the form `n * ε/2`. For `f32`
/// the 24 most significant random bits of a `u32` are used and for `f64` the
/// 53 most significant bits of a `u64` are used. The conversion uses the
/// multiplicative method: `(rng.gen::<$uty>() >> N) as $ty * (ε/2)`.
///
/// See also: [`Open01`] which samples from `(0, 1)`, [`OpenClosed01`] which
/// samples from `(0, 1]` and `Rng::gen_range(0..1)` which also samples from
/// `[0, 1)`. Note that `Open01` uses transmute-based methods which yield 1 bit
/// less precision but may perform faster on some architectures (on modern Intel
/// CPUs all methods have approximately equal performance).
///
/// [`Uniform`]: uniform::Uniform
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
pub struct Standard;


#[cfg(test)]
mod tests {
    use super::{Distribution, Uniform};
    use crate::Rng;

    #[test]
    fn test_distributions_iter() {
        use crate::distributions::Open01;
        let mut rng = crate::test::rng(210);
        let distr = Open01;
        let mut iter = Distribution::<f32>::sample_iter(distr, &mut rng);
        let mut sum: f32 = 0.;
        for _ in 0..100 {
            sum += iter.next().unwrap();
        }
        assert!(0. < sum && sum < 100.);
    }

    #[test]
    fn test_make_an_iter() {
        fn ten_dice_rolls_other_than_five<'a, R: Rng>(
            rng: &'a mut R,
        ) -> impl Iterator<Item = i32> + 'a {
            Uniform::new_inclusive(1, 6)
                .sample_iter(rng)
                .filter(|x| *x != 5)
                .take(10)
        }

        let mut rng = crate::test::rng(211);
        let mut count = 0;
        for val in ten_dice_rolls_other_than_five(&mut rng) {
            assert!(val >= 1 && val <= 6 && val != 5);
            count += 1;
        }
        assert_eq!(count, 10);
    }
}
