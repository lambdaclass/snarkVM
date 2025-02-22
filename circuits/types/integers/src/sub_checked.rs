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

impl<E: Environment, I: IntegerType> Sub<Integer<E, I>> for Integer<E, I> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self - &other
    }
}

impl<E: Environment, I: IntegerType> Sub<Integer<E, I>> for &Integer<E, I> {
    type Output = Integer<E, I>;

    fn sub(self, other: Integer<E, I>) -> Self::Output {
        self - &other
    }
}

impl<E: Environment, I: IntegerType> Sub<&Integer<E, I>> for Integer<E, I> {
    type Output = Self;

    fn sub(self, other: &Self) -> Self::Output {
        &self - other
    }
}

impl<E: Environment, I: IntegerType> Sub<&Integer<E, I>> for &Integer<E, I> {
    type Output = Integer<E, I>;

    fn sub(self, other: &Integer<E, I>) -> Self::Output {
        let mut output = self.clone();
        output -= other;
        output
    }
}

impl<E: Environment, I: IntegerType> SubAssign<Integer<E, I>> for Integer<E, I> {
    fn sub_assign(&mut self, other: Integer<E, I>) {
        *self -= &other;
    }
}

impl<E: Environment, I: IntegerType> SubAssign<&Integer<E, I>> for Integer<E, I> {
    fn sub_assign(&mut self, other: &Integer<E, I>) {
        // Stores the difference of `self` and `other` in `self`.
        *self = self.sub_checked(other);
    }
}

impl<E: Environment, I: IntegerType> SubChecked<Self> for Integer<E, I> {
    type Output = Self;

    #[inline]
    fn sub_checked(&self, other: &Integer<E, I>) -> Self::Output {
        // Determine the variable mode.
        if self.is_constant() && other.is_constant() {
            // Compute the difference and return the new constant.
            match self.eject_value().checked_sub(&other.eject_value()) {
                Some(value) => Integer::constant(value),
                None => E::halt("Integer underflow on subtraction of two constants"),
            }
        } else {
            // Instead of subtracting the bits of `self` and `other` directly, the integers are
            // converted into a field elements, and subtracted, before converting back to integers.
            // Note: This is safe as the field is larger than the maximum integer type supported.
            let difference = self.to_field() + (!other).to_field() + Field::one();

            // Extract the integer bits from the field element, with a carry bit.
            let (difference, carry) = match difference.to_lower_bits_le(I::BITS as usize + 1).split_last() {
                Some((carry, bits_le)) => (Integer::from_bits_le(bits_le), carry.clone()),
                None => E::halt("Malformed difference detected during integer subtraction"),
            };

            // Check for underflow.
            match I::is_signed() {
                // For signed subtraction, overflow and underflow conditions are:
                //   - a > 0 && b < 0 && a - b > 0 (Overflow)
                //   - a < 0 && b > 0 && a - b < 0 (Underflow)
                //   - Note: if sign(a) == sign(b) then over/underflow is impossible.
                //   - Note: the result of an overflow and underflow must be negative and positive, respectively.
                true => {
                    let is_different_signs = self.msb().is_not_equal(other.msb());
                    let is_underflow = is_different_signs & difference.msb().is_equal(other.msb());
                    E::assert_eq(is_underflow, E::zero());
                }
                // For unsigned subtraction, ensure the carry bit is one.
                false => E::assert_eq(carry, E::one()),
            }

            // Return the difference of `self` and `other`.
            difference
        }
    }
}

impl<E: Environment, I: IntegerType> Metrics<dyn Sub<Integer<E, I>, Output = Integer<E, I>>> for Integer<E, I> {
    type Case = (Mode, Mode);

    fn count(case: &Self::Case) -> Count {
        <Self as Metrics<dyn SubChecked<Integer<E, I>, Output = Integer<E, I>>>>::count(case)
    }
}

impl<E: Environment, I: IntegerType> OutputMode<dyn Sub<Integer<E, I>, Output = Integer<E, I>>> for Integer<E, I> {
    type Case = (Mode, Mode);

    fn output_mode(case: &Self::Case) -> Mode {
        <Self as OutputMode<dyn SubChecked<Integer<E, I>, Output = Integer<E, I>>>>::output_mode(case)
    }
}

impl<E: Environment, I: IntegerType> Metrics<dyn SubChecked<Integer<E, I>, Output = Integer<E, I>>> for Integer<E, I> {
    type Case = (Mode, Mode);

    fn count(case: &Self::Case) -> Count {
        match I::is_signed() {
            true => match (case.0, case.1) {
                (Mode::Constant, Mode::Constant) => Count::is(I::BITS, 0, 0, 0),
                (Mode::Constant, _) => Count::is(0, 0, I::BITS + 3, I::BITS + 5),
                (_, Mode::Constant) => Count::is(0, 0, I::BITS + 2, I::BITS + 4),
                (_, _) => Count::is(0, 0, I::BITS + 4, I::BITS + 6),
            },
            false => match (case.0, case.1) {
                (Mode::Constant, Mode::Constant) => Count::is(I::BITS, 0, 0, 0),
                (_, _) => Count::is(0, 0, I::BITS + 1, I::BITS + 3),
            },
        }
    }
}

impl<E: Environment, I: IntegerType> OutputMode<dyn SubChecked<Integer<E, I>, Output = Integer<E, I>>>
    for Integer<E, I>
{
    type Case = (Mode, Mode);

    fn output_mode(case: &Self::Case) -> Mode {
        match (case.0, case.1) {
            (Mode::Constant, Mode::Constant) => Mode::Constant,
            (_, _) => Mode::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_circuits_environment::Circuit;
    use snarkvm_utilities::{test_rng, UniformRand};
    use test_utilities::*;

    use core::{ops::RangeInclusive, panic::RefUnwindSafe};

    const ITERATIONS: u64 = 128;

    fn check_sub<I: IntegerType + RefUnwindSafe>(name: &str, first: I, second: I, mode_a: Mode, mode_b: Mode) {
        let a = Integer::<Circuit, I>::new(mode_a, first);
        let b = Integer::<Circuit, I>::new(mode_b, second);
        match first.checked_sub(&second) {
            Some(expected) => Circuit::scope(name, || {
                let candidate = a.sub_checked(&b);
                assert_eq!(expected, candidate.eject_value());
                assert_count!(Sub(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b));
                assert_output_mode!(Sub(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b), candidate);
            }),
            None => match mode_a.is_constant() && mode_b.is_constant() {
                true => check_operation_halts(&a, &b, Integer::sub_checked),
                false => Circuit::scope(name, || {
                    let _candidate = a.sub_checked(&b);
                    assert_count_fails!(Sub(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b));
                }),
            },
        }
        Circuit::reset();
    }

    fn run_test<I: IntegerType + RefUnwindSafe>(mode_a: Mode, mode_b: Mode) {
        for i in 0..ITERATIONS {
            let name = format!("Sub: {} - {} {}", mode_a, mode_b, i);
            let first: I = UniformRand::rand(&mut test_rng());
            let second: I = UniformRand::rand(&mut test_rng());
            check_sub(&name, first, second, mode_a, mode_b);
        }

        // Overflow
        if I::is_signed() {
            check_sub("MAX - (-1)", I::MAX, I::zero() - I::one(), mode_a, mode_b);
        }
        // Underflow
        check_sub("MIN - 1", I::MIN, I::one(), mode_a, mode_b);
    }

    fn run_exhaustive_test<I: IntegerType + RefUnwindSafe>(mode_a: Mode, mode_b: Mode)
    where
        RangeInclusive<I>: Iterator<Item = I>,
    {
        for first in I::MIN..=I::MAX {
            for second in I::MIN..=I::MAX {
                let name = format!("Sub: ({} - {})", first, second);
                check_sub(&name, first, second, mode_a, mode_b);
            }
        }
    }

    test_integer_binary!(run_test, i8, minus);
    test_integer_binary!(run_test, i16, minus);
    test_integer_binary!(run_test, i32, minus);
    test_integer_binary!(run_test, i64, minus);
    test_integer_binary!(run_test, i128, minus);

    test_integer_binary!(run_test, u8, minus);
    test_integer_binary!(run_test, u16, minus);
    test_integer_binary!(run_test, u32, minus);
    test_integer_binary!(run_test, u64, minus);
    test_integer_binary!(run_test, u128, minus);

    test_integer_binary!(#[ignore], run_exhaustive_test, u8, minus, exhaustive);
    test_integer_binary!(#[ignore], run_exhaustive_test, i8, minus, exhaustive);
}
