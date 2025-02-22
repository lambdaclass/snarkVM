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

impl<E: Environment, I: IntegerType> BitAnd<Integer<E, I>> for Integer<E, I> {
    type Output = Integer<E, I>;

    /// Returns `(self AND other)`.
    fn bitand(self, other: Integer<E, I>) -> Self::Output {
        self & &other
    }
}

impl<E: Environment, I: IntegerType> BitAnd<Integer<E, I>> for &Integer<E, I> {
    type Output = Integer<E, I>;

    /// Returns `(self AND other)`.
    fn bitand(self, other: Integer<E, I>) -> Self::Output {
        self & &other
    }
}

impl<E: Environment, I: IntegerType> BitAnd<&Integer<E, I>> for Integer<E, I> {
    type Output = Integer<E, I>;

    /// Returns `(self AND other)`.
    fn bitand(self, other: &Integer<E, I>) -> Self::Output {
        &self & other
    }
}

impl<E: Environment, I: IntegerType> BitAnd<&Integer<E, I>> for &Integer<E, I> {
    type Output = Integer<E, I>;

    /// Returns `(self AND other)`.
    fn bitand(self, other: &Integer<E, I>) -> Self::Output {
        let mut output = self.clone();
        output &= other;
        output
    }
}

impl<E: Environment, I: IntegerType> BitAndAssign<Integer<E, I>> for Integer<E, I> {
    /// Sets `self` as `(self AND other)`.
    fn bitand_assign(&mut self, other: Integer<E, I>) {
        *self &= &other;
    }
}

impl<E: Environment, I: IntegerType> BitAndAssign<&Integer<E, I>> for Integer<E, I> {
    /// Sets `self` as `(self AND other)`.
    fn bitand_assign(&mut self, other: &Integer<E, I>) {
        // Stores the bitwise AND of `self` and `other` in `self`.
        *self = Self {
            bits_le: self.bits_le.iter().zip_eq(other.bits_le.iter()).map(|(a, b)| a & b).collect(),
            phantom: Default::default(),
        }
    }
}

impl<E: Environment, I: IntegerType> Metrics<dyn BitAnd<Integer<E, I>, Output = Integer<E, I>>> for Integer<E, I> {
    type Case = (Mode, Mode);

    fn count(case: &Self::Case) -> Count {
        match (case.0, case.1) {
            (Mode::Constant, _) | (_, Mode::Constant) => Count::is(0, 0, 0, 0),
            (_, _) => Count::is(0, 0, I::BITS, I::BITS),
        }
    }
}

impl<E: Environment, I: IntegerType> OutputMode<dyn BitAnd<Integer<E, I>, Output = Integer<E, I>>> for Integer<E, I> {
    type Case = (CircuitType<Integer<E, I>>, CircuitType<Integer<E, I>>);

    fn output_mode(case: &Self::Case) -> Mode {
        match (case.0.mode(), case.1.mode()) {
            (Mode::Constant, Mode::Constant) => Mode::Constant,
            (Mode::Constant, mode_b) => match &case.0 {
                // Determine if the constant is all zeros.
                CircuitType::Constant(constant) => match constant.eject_value().is_zero() {
                    true => Mode::Constant,
                    false => mode_b,
                },
                _ => E::halt(format!("The constant is required to determine the output mode of Constant AND {mode_b}")),
            },
            (mode_a, Mode::Constant) => match &case.1 {
                // Determine if the constant is all zeros.
                CircuitType::Constant(constant) => match constant.eject_value().is_zero() {
                    true => Mode::Constant,
                    false => mode_a,
                },
                _ => E::halt(format!("The constant is required to determine the output mode of {mode_a} AND Constant")),
            },
            (_, _) => Mode::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_circuits_environment::Circuit;
    use snarkvm_utilities::{test_rng, UniformRand};

    use std::ops::RangeInclusive;

    const ITERATIONS: u64 = 128;

    fn check_and<I: IntegerType + BitAnd<Output = I>>(name: &str, first: I, second: I, mode_a: Mode, mode_b: Mode) {
        let a = Integer::<Circuit, I>::new(mode_a, first);
        let b = Integer::new(mode_b, second);
        let expected = first & second;
        Circuit::scope(name, || {
            let candidate = (&a).bitand(&b);
            assert_eq!(expected, candidate.eject_value());
            assert_count!(BitAnd(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b));
            assert_output_mode!(BitAnd(Integer<I>, Integer<I>) => Integer<I>, &(CircuitType::from(&a), CircuitType::from(&b)), candidate);
        });
        Circuit::reset();
    }

    fn run_test<I: IntegerType + BitAnd<Output = I>>(mode_a: Mode, mode_b: Mode) {
        for i in 0..ITERATIONS {
            let first: I = UniformRand::rand(&mut test_rng());
            let second: I = UniformRand::rand(&mut test_rng());

            let name = format!("BitAnd: ({} & {}) {}", mode_a, mode_b, i);
            check_and(&name, first, second, mode_a, mode_b);
            check_and(&name, second, first, mode_a, mode_b); // Commute the operation.

            let name = format!("BitAnd Identity: ({} & {}) {}", mode_a, mode_b, i);
            let identity = if I::is_signed() { I::zero() - I::one() } else { I::MAX };
            check_and(&name, identity, first, mode_a, mode_b);
            check_and(&name, first, identity, mode_a, mode_b); // Commute the operation.
        }

        // Check cases common to signed and unsigned integers.
        check_and("0 & MAX", I::zero(), I::MAX, mode_a, mode_b);
        check_and("MAX & 0", I::MAX, I::zero(), mode_a, mode_b);
        check_and("0 & MIN", I::zero(), I::MIN, mode_a, mode_b);
        check_and("MIN & 0", I::MIN, I::zero(), mode_a, mode_b);
    }

    fn run_exhaustive_test<I: IntegerType + BitAnd<Output = I>>(mode_a: Mode, mode_b: Mode)
    where
        RangeInclusive<I>: Iterator<Item = I>,
    {
        for first in I::MIN..=I::MAX {
            for second in I::MIN..=I::MAX {
                let name = format!("BitAnd: ({} & {})", first, second);
                check_and(&name, first, second, mode_a, mode_b);
            }
        }
    }

    test_integer_binary!(run_test, i8, bitand);
    test_integer_binary!(run_test, i16, bitand);
    test_integer_binary!(run_test, i32, bitand);
    test_integer_binary!(run_test, i64, bitand);
    test_integer_binary!(run_test, i128, bitand);

    test_integer_binary!(run_test, u8, bitand);
    test_integer_binary!(run_test, u16, bitand);
    test_integer_binary!(run_test, u32, bitand);
    test_integer_binary!(run_test, u64, bitand);
    test_integer_binary!(run_test, u128, bitand);

    test_integer_binary!(#[ignore], run_exhaustive_test, u8, bitand, exhaustive);
    test_integer_binary!(#[ignore], run_exhaustive_test, i8, bitand, exhaustive);
}
