mod unary {
    use std::{iter::Sum, ops::Neg};

    use crate::Var;

    #[opimps::impl_uni_ops(Neg)]
    fn neg<'a>(self: Var<'a>) -> Var<'a> {
        self * -1.0f64
    }

    impl<'a> Sum<Var<'a>> for Var<'a> {
        fn sum<I: Iterator<Item = Var<'a>>>(iter: I) -> Self {
            iter.reduce(|a, b| a + b).unwrap()
        }
    }
}

mod add {
    use crate::{Tape, Var};
    use std::ops::{Add, AddAssign};

    #[opimps::impl_ops(Add)]
    fn add<'a>(self: Var<'a>, rhs: Var<'a>) -> Var<'a> {
        assert_eq!(self.tape as *const Tape, rhs.tape as *const Tape);
        Self::Output {
            val: self.val + rhs.val,
            location: self.tape.add_node(self.location, rhs.location, 1., 1.),
            tape: self.tape,
        }
    }

    #[opimps::impl_ops_rprim(Add)]
    fn add<'a>(self: Var<'a>, rhs: f64) -> Var<'a> {
        Self::Output {
            val: self.val + rhs,
            location: self.tape.add_node(self.location, self.location, 1., 0.),
            tape: self.tape,
        }
    }

    #[opimps::impl_ops_lprim(Add)]
    fn add<'a>(self: f64, rhs: Var<'a>) -> Var<'a> {
        rhs + self
    }

    #[opimps::impl_ops_assign(AddAssign)]
    fn add_assign<'a>(self: Var<'a>, rhs: Var<'a>) {
        *self = (&*self) + rhs;
    }

    #[opimps::impl_op_assign(AddAssign)]
    fn add_assign<'a>(self: Var<'a>, rhs: f64) {
        *self = (&*self) + rhs;
    }
}

mod sub {
    use crate::Var;
    use std::ops::{Neg, Sub, SubAssign};

    #[opimps::impl_ops(Sub)]
    fn sub<'a>(self: Var<'a>, rhs: Var<'a>) -> Var<'a> {
        self + rhs.neg()
    }

    #[opimps::impl_ops_lprim(Sub)]
    fn sub<'a>(self: f64, rhs: Var<'a>) -> Var<'a> {
        Self::Output {
            val: self - rhs.val,
            location: rhs.tape.add_node(rhs.location, rhs.location, 0., -1.),
            tape: rhs.tape,
        }
    }

    #[opimps::impl_ops_rprim(Sub)]
    fn sub<'a>(self: Var<'a>, rhs: f64) -> Var<'a> {
        self + rhs.neg()
    }

    #[opimps::impl_ops_assign(SubAssign)]
    fn sub_assign<'a>(self: Var<'a>, rhs: Var<'a>) {
        *self = (&*self) - rhs;
    }

    #[opimps::impl_op_assign(SubAssign)]
    fn sub_assign<'a>(self: Var<'a>, rhs: f64) {
        *self = (&*self) - rhs;
    }
}

mod mul {
    use crate::{Tape, Var};
    use std::ops::{Mul, MulAssign};

    #[opimps::impl_ops(Mul)]
    fn mul<'a>(self: Var<'a>, rhs: Var<'a>) -> Var<'a> {
        assert_eq!(self.tape as *const Tape, rhs.tape as *const Tape);
        Self::Output {
            val: self.val * rhs.val,
            location: self
                .tape
                .add_node(self.location, rhs.location, rhs.val, self.val),
            tape: self.tape,
        }
    }

    #[opimps::impl_ops_rprim(Mul)]
    fn mul<'a>(self: Var<'a>, rhs: f64) -> Var<'a> {
        Self::Output {
            val: self.val * rhs,
            location: self.tape.add_node(self.location, self.location, rhs, 0.),
            tape: self.tape,
        }
    }

    #[opimps::impl_ops_lprim(Mul)]
    fn mul<'a>(self: f64, rhs: Var<'a>) -> Var<'a> {
        rhs * self
    }

    #[opimps::impl_ops_assign(MulAssign)]
    fn mul_assign<'a>(self: Var<'a>, rhs: Var<'a>) {
        *self = (&*self) * rhs;
    }

    #[opimps::impl_op_assign(MulAssign)]
    fn mul_assign<'a>(self: Var<'a>, rhs: f64) {
        *self = (&*self) * rhs;
    }
}

mod div {
    use crate::Var;
    use std::ops::{Div, DivAssign};

    #[opimps::impl_ops(Div)]
    fn div<'a>(self: Var<'a>, rhs: Var<'a>) -> Var<'a> {
        self * rhs.recip()
    }

    #[opimps::impl_ops_rprim(Div)]
    fn div<'a>(self: Var<'a>, rhs: f64) -> Var<'a> {
        self * rhs.recip()
    }

    #[opimps::impl_ops_lprim(Div)]
    fn div<'a>(self: f64, rhs: Var<'a>) -> Var<'a> {
        Self::Output {
            val: self / rhs.val,
            location: rhs
                .tape
                .add_node(rhs.location, rhs.location, 0., -1. / rhs.val),
            tape: rhs.tape,
        }
    }

    #[opimps::impl_ops_assign(DivAssign)]
    fn div_assign<'a>(self: Var<'a>, rhs: Var<'a>) {
        *self = (&*self) / rhs;
    }

    #[opimps::impl_op_assign(DivAssign)]
    fn div_assign<'a>(self: Var<'a>, rhs: f64) {
        *self = (&*self) / rhs;
    }
}

mod powf {
    use crate::{Powf, Tape, Var};

    #[opimps::impl_ops(Powf)]
    fn powf<'a>(self: Var<'a>, rhs: Var<'a>) -> Var<'a> {
        assert_eq!(self.tape as *const Tape, rhs.tape as *const Tape);

        Self::Output {
            val: self.val.powf(rhs.val),
            location: self.tape.add_node(
                self.location,
                rhs.location,
                rhs.val * f64::powf(self.val, rhs.val - 1.),
                f64::powf(self.val, rhs.val) * f64::ln(self.val),
            ),
            tape: self.tape,
        }
    }

    #[opimps::impl_ops_rprim(Powf)]
    fn powf<'a>(self: Var<'a>, rhs: f64) -> Var<'a> {
        Self::Output {
            val: f64::powf(self.val, rhs),
            location: self.tape.add_node(
                self.location,
                self.location,
                rhs * f64::powf(self.val, rhs - 1.),
                0.,
            ),
            tape: self.tape,
        }
    }

    #[opimps::impl_ops_lprim(Powf)]
    fn powf<'a>(self: f64, rhs: Var<'a>) -> Var<'a> {
        Self::Output {
            val: f64::powf(self, rhs.val),
            location: rhs.tape.add_node(
                rhs.location,
                rhs.location,
                0.,
                rhs.val * f64::powf(self, rhs.val - 1.),
            ),
            tape: rhs.tape,
        }
    }
}
