// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::{
    templates::{
        bls12::{Bls12Parameters, TwistType},
        short_weierstrass_jacobian::{Affine, Projective},
    },
    traits::{AffineCurve, ShortWeierstrassParameters},
};
use snarkvm_fields::{Field, Fp2, One, Zero};
use snarkvm_utilities::{bititerator::BitIteratorBE, serialize::*, ToBytes};

use std::io::{Result as IoResult, Write};

pub type G2Affine<P> = Affine<<P as Bls12Parameters>::G2Parameters>;
pub type G2Projective<P> = Projective<<P as Bls12Parameters>::G2Parameters>;
type CoeffTriplet<T> = (Fp2<T>, Fp2<T>, Fp2<T>);

#[derive(Derivative, CanonicalSerialize, CanonicalDeserialize)]
#[derivative(
    Clone(bound = "P: Bls12Parameters"),
    Debug(bound = "P: Bls12Parameters"),
    PartialEq(bound = "P: Bls12Parameters"),
    Eq(bound = "P: Bls12Parameters")
)]
pub struct G2Prepared<P: Bls12Parameters> {
    // Stores the coefficients of the line evaluations as calculated in
    // https://eprint.iacr.org/2013/722.pdf
    pub ell_coeffs: Vec<CoeffTriplet<P::Fp2Params>>,
    pub infinity: bool,
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = "P: Bls12Parameters"),
    Copy(bound = "P: Bls12Parameters"),
    Debug(bound = "P: Bls12Parameters")
)]
struct G2HomProjective<P: Bls12Parameters> {
    x: Fp2<P::Fp2Params>,
    y: Fp2<P::Fp2Params>,
    z: Fp2<P::Fp2Params>,
}

impl<P: Bls12Parameters> Default for G2Prepared<P> {
    fn default() -> Self {
        Self::from_affine(G2Affine::<P>::prime_subgroup_generator())
    }
}

impl<P: Bls12Parameters> ToBytes for G2Prepared<P> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.ell_coeffs.len() as u32).write_le(&mut writer)?;
        for coeff in &self.ell_coeffs {
            coeff.0.write_le(&mut writer)?;
            coeff.1.write_le(&mut writer)?;
            coeff.2.write_le(&mut writer)?;
        }
        self.infinity.write_le(writer)
    }
}

impl<P: Bls12Parameters> FromBytes for G2Prepared<P> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let ell_coeffs_len: u32 = FromBytes::read_le(&mut reader)?;
        let mut ell_coeffs = Vec::with_capacity(ell_coeffs_len as usize);
        for _ in 0..ell_coeffs_len {
            let coeff_1: Fp2<P::Fp2Params> = FromBytes::read_le(&mut reader)?;
            let coeff_2: Fp2<P::Fp2Params> = FromBytes::read_le(&mut reader)?;
            let coeff_3: Fp2<P::Fp2Params> = FromBytes::read_le(&mut reader)?;
            ell_coeffs.push((coeff_1, coeff_2, coeff_3));
        }

        let infinity: bool = FromBytes::read_le(&mut reader)?;

        Ok(Self { ell_coeffs, infinity })
    }
}

impl<P: Bls12Parameters> G2Prepared<P> {
    pub fn is_zero(&self) -> bool {
        self.infinity
    }

    pub fn from_affine(q: G2Affine<P>) -> Self {
        let two_inv = P::Fp::one().double().inverse().unwrap();
        if q.is_zero() {
            return Self { ell_coeffs: vec![], infinity: true };
        }

        let mut r = G2HomProjective { x: q.x, y: q.y, z: Fp2::one() };

        let bit_iterator = BitIteratorBE::new(P::X);
        let mut ell_coeffs = Vec::with_capacity(bit_iterator.len());

        for i in bit_iterator.skip(1) {
            ell_coeffs.push(doubling_step::<P>(&mut r, &two_inv));

            if i {
                ell_coeffs.push(addition_step::<P>(&mut r, &q));
            }
        }

        Self { ell_coeffs, infinity: false }
    }
}

#[allow(clippy::many_single_char_names)]
fn doubling_step<B: Bls12Parameters>(r: &mut G2HomProjective<B>, two_inv: &B::Fp) -> CoeffTriplet<B::Fp2Params> {
    // Formula for line function when working with
    // homogeneous projective coordinates.

    let mut a = r.x * r.y;
    a.mul_by_fp(two_inv);
    let b = r.y.square();
    let c = r.z.square();
    let e = B::G2Parameters::COEFF_B * (c.double() + c);
    let f = e.double() + e;
    let mut g = b + f;
    g.mul_by_fp(two_inv);
    let h = (r.y + r.z).square() - (b + c);
    let i = e - b;
    let j = r.x.square();
    let e_square = e.square();

    r.x = a * (b - f);
    r.y = g.square() - (e_square.double() + e_square);
    r.z = b * h;
    match B::TWIST_TYPE {
        TwistType::M => (i, j.double() + j, -h),
        TwistType::D => (-h, j.double() + j, i),
    }
}

#[allow(clippy::many_single_char_names)]
fn addition_step<B: Bls12Parameters>(r: &mut G2HomProjective<B>, q: &G2Affine<B>) -> CoeffTriplet<B::Fp2Params> {
    // Formula for line function when working with
    // homogeneous projective coordinates.
    let theta = r.y - (q.y * r.z);
    let lambda = r.x - (q.x * r.z);
    let c = theta.square();
    let d = lambda.square();
    let e = lambda * d;
    let f = r.z * c;
    let g = r.x * d;
    let h = e + f - g.double();
    r.x = lambda * h;
    r.y = theta * (g - h) - (e * r.y);
    r.z *= &e;
    let j = theta * q.x - (lambda * q.y);

    match B::TWIST_TYPE {
        TwistType::M => (j, -theta, lambda),
        TwistType::D => (lambda, -theta, j),
    }
}
