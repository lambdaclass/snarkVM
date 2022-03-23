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

impl<E: Environment> Double for Group<E> {
    type Output = Group<E>;

    fn double(self) -> Self::Output {
        (&self).double()
    }
}

impl<E: Environment> Double for &Group<E> {
    type Output = Group<E>;

    fn double(self) -> Self::Output {
        // Determine the variable mode.
        let mode = match self.is_constant() {
            true => Mode::Constant,
            false => Mode::Private,
        };

        let a = Field::new(Mode::Constant, E::AffineParameters::COEFF_A);
        let two = Field::one() + Field::one();

        // Compute xy, xx, yy, axx.
        let xy = &self.x * &self.y;
        let x2 = self.x.square();
        let y2 = self.y.square();
        let ax2 = &x2 * &a;

        // Compute x3 and y3.
        let (x3, y3) = {
            let xy = xy.eject_value();
            let x2 = x2.eject_value();
            let y2 = y2.eject_value();
            let ax2 = ax2.eject_value();
            let two = E::BaseField::one() + E::BaseField::one();

            // Assign x3 = (2xy) / (ax^2 + y^2)
            let x3 = {
                let t0 = xy.double();
                let t1 = (E::AffineParameters::COEFF_A * x2) + y2;
                let t0_div_t1 = t0 * t1.inverse().expect("Failed to compute x-coordinate");
                Field::new(mode, t0_div_t1)
            };

            // Assign y3 = (y^2 - ax^2) / (2 - ax^2 - y^2)
            let y3 = {
                let t0 = y2 - ax2;
                let t1 = two - ax2 - y2;
                let t0_div_t1 = t0 * t1.inverse().expect("Failed to compute y-coordinate");
                Field::new(mode, t0_div_t1)
            };

            (x3, y3)
        };

        // Ensure x3 is well-formed.
        // x3 * (ax^2 + y^2) = 2xy
        let ax2_plus_y2 = &ax2 + &y2;
        let two_xy = xy.double();
        E::enforce(|| (&x3, &ax2_plus_y2, two_xy));

        // Ensure y3 is well-formed.
        // y3 * (2 - (ax^2 + y^2)) = y^2 - ax^2
        let y2_minus_a_x2 = y2 - ax2;
        let two_minus_ax2_minus_y2 = two - ax2_plus_y2;
        E::enforce(|| (&y3, two_minus_ax2_minus_y2, y2_minus_a_x2));

        Group { x: x3, y: y3 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_circuits_environment::Circuit;
    use snarkvm_curves::ProjectiveCurve;
    use snarkvm_utilities::{test_rng, UniformRand};

    const ITERATIONS: usize = 250;

    #[test]
    fn test_double() {
        // Constant variables
        for i in 0..ITERATIONS {
            // Sample a random element.
            let point = <Circuit as Environment>::Affine::rand(&mut test_rng()).into_projective();
            let expected = point.double();
            let point = point.into_affine();

            let affine =
                Group::<Circuit>::new(Mode::Constant, (point.to_x_coordinate(), Some(point.to_y_coordinate())));

            Circuit::scope(&format!("Constant {}", i), || {
                let candidate = affine.double();
                assert_eq!(expected, candidate.eject_value());
                assert_scope!(3, 0, 0, 0);
            });
            Circuit::reset();
        }

        // Public variables
        for i in 0..ITERATIONS {
            // Sample a random element.
            let point = <Circuit as Environment>::Affine::rand(&mut test_rng()).into_projective();
            let expected = point.double();
            let point = point.into_affine();

            let affine = Group::<Circuit>::new(Mode::Public, (point.to_x_coordinate(), Some(point.to_y_coordinate())));

            Circuit::scope(&format!("Public {}", i), || {
                let candidate = affine.double();
                assert_eq!(expected, candidate.eject_value());
                assert_scope!(1, 0, 5, 5);
            });
            Circuit::reset();
        }

        // Private variables
        for i in 0..ITERATIONS {
            // Sample a random element.

            let point = <Circuit as Environment>::Affine::rand(&mut test_rng()).into_projective();
            let expected = point.double();
            let point = point.into_affine();

            let affine = Group::<Circuit>::new(Mode::Private, (point.to_x_coordinate(), Some(point.to_y_coordinate())));

            Circuit::scope(&format!("Private {}", i), || {
                let candidate = affine.double();
                assert_eq!(expected, candidate.eject_value());
                assert_scope!(1, 0, 5, 5);
            });
            Circuit::reset();
        }
    }

    #[test]
    fn test_double_matches() {
        // Sample two random elements.
        let a = <Circuit as Environment>::Affine::rand(&mut test_rng()).into_projective();
        let expected = a + a;
        let a = a.into_affine();

        // Constant
        let candidate_a =
            Group::<Circuit>::new(Mode::Constant, (a.to_x_coordinate(), Some(a.to_y_coordinate()))).double();
        assert_eq!(expected, candidate_a.eject_value());

        // Private
        let candidate_b =
            Group::<Circuit>::new(Mode::Private, (a.to_x_coordinate(), Some(a.to_y_coordinate()))).double();
        assert_eq!(expected, candidate_b.eject_value());
    }
}
