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

use super::*;

impl<A: Aleo> FromBits for Identifier<A> {
    type Boolean = Boolean<A>;

    /// Initializes a new identifier from a list of little-endian bits.
    fn from_bits_le(bits_le: &[Self::Boolean]) -> Self {
        // Ensure the number of bits does not exceed the size in bits of the field.
        // This check is not sufficient to ensure the identifier is of valid size,
        // the final step checks the byte-aligned field element is within the data capacity.
        debug_assert!(bits_le.len() <= A::BaseField::size_in_bits(), "Identifier exceeds the maximum bits allowed");

        // Recover the field element from the bits.
        let field = Field::from_bits_le(bits_le);

        // Eject the bits in **little-endian** form, and determine the number of bytes.
        let num_bytes = match console::Identifier::<A::Network>::from_bits_le(&bits_le.eject_value()) {
            Ok(console_identifier) => console_identifier.size_in_bits() / 8,
            Err(error) => A::halt(format!("Failed to recover an identifier from bits: {error}")),
        };

        // Ensure identifier fits within the data capacity of the base field.
        let max_bytes = A::BaseField::size_in_data_bits() / 8; // Note: This intentionally rounds down.
        match num_bytes as usize <= max_bytes {
            // Return the identifier.
            true => Self(field, num_bytes),
            false => A::halt("Identifier exceeds the maximum capacity allowed"),
        }
    }

    /// Initializes a new identifier from a list of big-endian bits.
    fn from_bits_be(bits_be: &[Self::Boolean]) -> Self {
        Self::from_bits_le(bits_be.iter().rev().cloned().collect::<Vec<_>>().as_slice())
    }
}

#[cfg(all(test, console))]
mod tests {
    use super::*;
    use crate::{data::identifier::tests::sample_console_identifier, Circuit};

    use anyhow::Result;

    const ITERATIONS: u64 = 100;

    fn check_from_bits_le(num_constants: u64, num_public: u64, num_private: u64, num_constraints: u64) -> Result<()> {
        for _ in 0..ITERATIONS {
            // Initialize the console identifier.
            let console_identifier = sample_console_identifier::<Circuit>()?;
            // Initialize the circuit list of bits.
            let circuit_bits: Vec<_> = Inject::constant(console_identifier.to_bits_le());

            Circuit::scope("Identifier FromBits", || {
                let candidate = Identifier::<Circuit>::from_bits_le(&circuit_bits);
                assert_eq!(Mode::Constant, candidate.eject_mode());
                assert_eq!(console_identifier, candidate.eject_value());
                assert_scope!(num_constants, num_public, num_private, num_constraints);
            });
            Circuit::reset();
        }
        Ok(())
    }

    fn check_from_bits_be(num_constants: u64, num_public: u64, num_private: u64, num_constraints: u64) -> Result<()> {
        for _ in 0..ITERATIONS {
            // Initialize the console identifier.
            let console_identifier = sample_console_identifier::<Circuit>()?;
            // Initialize the circuit list of bits.
            let circuit_bits: Vec<_> = Inject::constant(console_identifier.to_bits_be());

            Circuit::scope("Identifier FromBits", || {
                let candidate = Identifier::<Circuit>::from_bits_be(&circuit_bits);
                assert_eq!(Mode::Constant, candidate.eject_mode());
                assert_eq!(console_identifier, candidate.eject_value());
                assert_scope!(num_constants, num_public, num_private, num_constraints);
            });
            Circuit::reset();
        }
        Ok(())
    }

    #[test]
    fn test_from_bits_le() -> Result<()> {
        check_from_bits_le(0, 0, 0, 0)
    }

    #[test]
    fn test_from_bits_be() -> Result<()> {
        check_from_bits_be(0, 0, 0, 0)
    }
}
