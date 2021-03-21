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

use crate::{fiat_shamir::FiatShamirRng, FiatShamirError, PhantomData};
use snarkvm_fields::{PrimeField, ToConstraintField};
use snarkvm_nonnative::params::OptimizationType;

use core::num::NonZeroU32;
use digest::Digest;
use rand_chacha::ChaChaRng;
use rand_core::{Error, RngCore, SeedableRng};

/// Implements a Fiat-Shamir based Rng that allows one to incrementally update
/// the seed based on new messages in the proof transcript.
/// Use a ChaCha stream cipher to generate the actual pseudorandom bits.
/// Use a digest function to do absorbing.
pub struct FiatShamirChaChaRng<TargetField: PrimeField, BaseField: PrimeField, D: Digest> {
    /// The ChaCha RNG.
    r: Option<ChaChaRng>,
    /// The initial seed for the RNG.
    seed: Option<Vec<u8>>,
    #[doc(hidden)]
    _target_field: PhantomData<TargetField>,
    #[doc(hidden)]
    _base_field: PhantomData<BaseField>,
    #[doc(hidden)]
    _digest: PhantomData<D>,
}

impl<TargetField: PrimeField, BaseField: PrimeField, D: Digest> RngCore
    for FiatShamirChaChaRng<TargetField, BaseField, D>
{
    #[inline]
    fn next_u32(&mut self) -> u32 {
        (&mut self.r)
            .as_mut()
            .map(|r| r.next_u32())
            .expect("Rng was invoked in a non-hiding context")
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        (&mut self.r)
            .as_mut()
            .map(|r| r.next_u64())
            .expect("Rng was invoked in a non-hiding context")
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        (&mut self.r)
            .as_mut()
            .map(|r| r.fill_bytes(dest))
            .expect("Rng was invoked in a non-hiding context")
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        match &mut self.r {
            Some(r) => r.try_fill_bytes(dest),
            None => Err(NonZeroU32::new(rand_core::Error::CUSTOM_START).unwrap().into()),
        }
    }
}

impl<TargetField: PrimeField, BaseField: PrimeField, D: Digest> FiatShamirRng<TargetField, BaseField>
    for FiatShamirChaChaRng<TargetField, BaseField, D>
{
    fn new() -> Self {
        Self {
            r: None,
            seed: None,
            _target_field: PhantomData,
            _base_field: PhantomData,
            _digest: PhantomData,
        }
    }

    fn absorb_nonnative_field_elements(&mut self, elems: &[TargetField], _: OptimizationType) {
        let mut bytes = Vec::new();
        for elem in elems {
            elem.write(&mut bytes).expect("failed to convert to bytes");
        }
        self.absorb_bytes(&bytes);
    }

    fn absorb_native_field_elements<T: ToConstraintField<BaseField>>(&mut self, src: &[T]) {
        let mut elems = Vec::<BaseField>::new();
        for elem in src.iter() {
            elems.append(&mut elem.to_field_elements().unwrap());
        }

        let mut bytes = Vec::new();
        for elem in elems.iter() {
            elem.write(&mut bytes).expect("failed to convert to bytes");
        }
        self.absorb_bytes(&bytes);
    }

    fn absorb_bytes(&mut self, elements: &[u8]) {
        let mut bytes = elements.to_vec();
        // If a seed exists, extend the byte vector to include the existing seed.
        if let Some(seed) = &self.seed {
            bytes.extend_from_slice(seed);
        }

        let new_seed = (*D::digest(&bytes).as_slice()).to_vec();
        self.seed = Some(new_seed.to_vec());

        let mut seed = [0u8; 32];
        for (i, byte) in new_seed.as_slice().iter().enumerate() {
            seed[i] = *byte;
        }

        self.r = Some(ChaChaRng::from_seed(seed));
    }

    fn squeeze_nonnative_field_elements(
        &mut self,
        num: usize,
        _: OptimizationType,
    ) -> Result<Vec<TargetField>, FiatShamirError> {
        // Ensure the RNG is initialized.
        let rng = match &mut self.r {
            Some(rng) => rng,
            None => return Err(FiatShamirError::UninitializedRNG),
        };

        let mut res = Vec::<TargetField>::new();
        for _ in 0..num {
            res.push(TargetField::rand(rng));
        }
        Ok(res)
    }

    fn squeeze_native_field_elements(&mut self, num: usize) -> Result<Vec<BaseField>, FiatShamirError> {
        // Ensure the RNG is initialized.
        let rng = match &mut self.r {
            Some(rng) => rng,
            None => return Err(FiatShamirError::UninitializedRNG),
        };

        let mut res = Vec::<BaseField>::new();
        for _ in 0..num {
            res.push(BaseField::rand(rng));
        }
        Ok(res)
    }

    fn squeeze_128_bits_nonnative_field_elements(&mut self, num: usize) -> Result<Vec<TargetField>, FiatShamirError> {
        // Ensure the RNG is initialized.
        let rng = match &mut self.r {
            Some(rng) => rng,
            None => return Err(FiatShamirError::UninitializedRNG),
        };

        let mut res = Vec::<TargetField>::new();
        for _ in 0..num {
            let mut x = [0u8; 16];
            rng.fill_bytes(&mut x);
            res.push(TargetField::from_random_bytes(&x).unwrap());
        }
        Ok(res)
    }
}