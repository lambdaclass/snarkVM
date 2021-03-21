// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate thiserror;

#[macro_use]
mod macros;

mod errors;
pub use errors::*;

mod fp_256;
pub use fp_256::*;

mod fp_320;
pub use fp_320::*;

mod fp_384;
pub use fp_384::*;

mod fp_768;
pub use fp_768::*;

mod fp_832;
pub use fp_832::*;

mod fp2;
pub use fp2::*;

mod fp3;
pub use fp3::*;

pub mod fp6_2over3;

pub mod fp6_3over2;

mod fp12_2over3over2;
pub use fp12_2over3over2::*;

mod legendre;
pub use legendre::*;

pub mod tests_field;

mod to_field_vec;
pub use to_field_vec::*;

mod traits;
pub use traits::*;

use snarkvm_utilities::{
    biginteger::*,
    bytes::{FromBytes, ToBytes},
    serialize::{
        CanonicalDeserialize,
        CanonicalDeserializeWithFlags,
        CanonicalSerialize,
        CanonicalSerializeWithFlags,
        ConstantSerializedSize,
    },
};

impl_field_into_bigint!(Fp256, BigInteger256, Fp256Parameters);
impl_field_into_bigint!(Fp320, BigInteger320, Fp320Parameters);
impl_field_into_bigint!(Fp384, BigInteger384, Fp384Parameters);
impl_field_into_bigint!(Fp768, BigInteger768, Fp768Parameters);
impl_field_into_bigint!(Fp832, BigInteger832, Fp832Parameters);

impl_prime_field_serializer!(Fp256, Fp256Parameters, 32);
impl_prime_field_serializer!(Fp320, Fp320Parameters, 40);
impl_prime_field_serializer!(Fp384, Fp384Parameters, 48);
impl_prime_field_serializer!(Fp768, Fp768Parameters, 96);
impl_prime_field_serializer!(Fp832, Fp832Parameters, 104);

pub fn batch_inversion<F: Field>(v: &mut [F]) {
    // Montgomery’s Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2

    // First pass: compute [a, ab, abc, ...]
    let mut prod = Vec::with_capacity(v.len());
    let mut tmp = F::one();
    for f in v.iter().filter(|f| !f.is_zero()) {
        tmp.mul_assign(&f);
        prod.push(tmp);
    }

    // Invert `tmp`.
    tmp = tmp.inverse().unwrap(); // Guaranteed to be nonzero.

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v
        .iter_mut()
        // Backwards
        .rev()
        // Ignore normalized elements
        .filter(|f| !f.is_zero())
        // Backwards, skip last element, fill in one for last term.
        .zip(prod.into_iter().rev().skip(1).chain(Some(F::one())))
    {
        // tmp := tmp * g.z; g.z := tmp * s = 1/z
        let newtmp = tmp * &f;
        *f = tmp * &s;
        tmp = newtmp;
    }
}