//! # reverse
//!
//! `reverse` is a light-weight, zero-dependency crate for performing **reverse**-mode automatic
//! differentiation in Rust. This is useful when you have functions with many inputs producing a
//! small number of outputs, as the gradients for all inputs with respect to a particular output
//! can be computed in a single pass.
//!
//! # Usage
//!
//! A tape (also called a Wengert list) is created with `Tape::new()`. Variables can then
//! be added to the tape, either individually (`.add_var`) or as a slice (`.add_vars`).
//! This yields differentiable variables with type `Var<'a>`.
//!
//! Differentiable variables can be manipulated like `f64`s, are tracked with the tape,
//! and gradients with respect to other variables can be calculated. Operations can
//! be performed between variables and normal `f64`s as well, and the `f64`s are treated as
//! constants with no gradients.
//!
//! You can define functions that have `Var<'a>` as an input (potentially along with other fixed
//! data of type `f64`) and as an output, and the function will be differentiable. For example:
//!
//! ```rust
//! use reverse::*;
//!
//! fn main() {
//!     let tape = Tape::new();
//!     let params = tape.add_vars(&[5., 2., 0.]);
//!     let data = [1., 2.];
//!     let result = diff_fn(&params, &data);
//!     let gradients = result.grad();
//!     println!("{:?}", gradients.wrt(&params));
//! }
//!
//! fn diff_fn<'a>(params: &[Var<'a>], data: &[f64]) -> Var<'a> {
//!     params[0].powf(params[1]) + data[0].sin() - params[2].asinh() / data[1]
//! }
//! ```

#![allow(clippy::suspicious_arithmetic_impl)]
mod ops;

use std::{cell::RefCell, fmt::Display};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Node {
    weights: [f64; 2],
    dependencies: [usize; 2],
}

#[derive(Debug, Clone, Copy)]
/// Differentiable variable. This is the main type that users will interact with.
pub struct Var<'a> {
    /// Value of the variable.
    pub val: f64,
    /// Location that can be referred to be nodes in the tape.
    location: usize,
    /// Reference to a tape that this variable is associated with.
    pub tape: &'a Tape,
}

#[derive(Debug, Clone)]
/// Tape (Wengert list) that tracks differentiable variables, intermediate values, and the
/// operations applied to each.
pub struct Tape {
    /// Variables and operations that are tracked.
    nodes: RefCell<Vec<Node>>,
}

impl Tape {
    /// Create a new tape.
    pub fn new() -> Self {
        Self {
            nodes: RefCell::new(vec![]),
        }
    }
    /// Gets the number of nodes (differentiable variables and intermediate values) in the tape.
    pub fn len(&self) -> usize {
        self.nodes.borrow().len()
    }
    /// Checks whether the tape is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn add_node(&self, loc1: usize, loc2: usize, grad1: f64, grad2: f64) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        let n = nodes.len();
        nodes.push(Node {
            weights: [grad1, grad2],
            dependencies: [loc1, loc2],
        });
        n
    }

    /// Add a variable with value `val` to the tape. Returns a `Var<'a>` which can be used like an `f64`.
    pub fn add_var(&self, val: f64) -> Var {
        let len = self.len();
        Var {
            val,
            location: self.add_node(len, len, 0., 0.),
            tape: self,
        }
    }

    /// Add a slice of variables to the tape. See `add_var` for details.
    pub fn add_vars<'a>(&'a self, vals: &[f64]) -> Vec<Var<'a>> {
        vals.iter().map(|&x| self.add_var(x)).collect()
    }

    /// Zero out all the gradients in the tape.
    pub fn zero_grad(&self) {
        self.nodes
            .borrow_mut()
            .iter_mut()
            .for_each(|n| n.weights = [0., 0.]);
    }

    /// Clear the tape by deleting all nodes (useful for clearing out intermediate values).
    pub fn clear(&self) {
        self.nodes.borrow_mut().clear();
    }
}

impl Default for Tape {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Var<'a> {
    /// Get the value of the variable.
    pub fn val(&self) -> f64 {
        self.val
    }

    /// Calculate the gradients of this variable with respect to all other (possibly intermediate)
    /// variables that it depends on.
    pub fn grad(&self) -> Vec<f64> {
        let n = self.tape.len();
        let mut derivs = vec![0.; n];
        derivs[self.location] = 1.;

        for (idx, n) in self.tape.nodes.borrow().iter().enumerate().rev() {
            derivs[n.dependencies[0]] += n.weights[0] * derivs[idx];
            derivs[n.dependencies[1]] += n.weights[1] * derivs[idx];
        }

        derivs
    }

    pub fn recip(&self) -> Self {
        Self {
            val: self.val.recip(),
            location: self.tape.add_node(
                self.location,
                self.location,
                -1. / (self.val.powi(2)),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn sin(&self) -> Self {
        Self {
            val: self.val.sin(),
            location: self
                .tape
                .add_node(self.location, self.location, self.val.cos(), 0.),
            tape: self.tape,
        }
    }

    pub fn cos(&self) -> Self {
        Self {
            val: self.val.cos(),
            location: self
                .tape
                .add_node(self.location, self.location, -self.val.sin(), 0.),
            tape: self.tape,
        }
    }

    pub fn tan(&self) -> Self {
        Self {
            val: self.val.tan(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / self.val.cos().powi(2),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn ln(&self) -> Self {
        Self {
            val: self.val.ln(),
            location: self
                .tape
                .add_node(self.location, self.location, 1. / self.val, 0.),
            tape: self.tape,
        }
    }

    pub fn log(&self, base: f64) -> Self {
        Self {
            val: self.val.log(base),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (self.val * base.ln()),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn log10(&self) -> Self {
        self.log(10.)
    }

    pub fn log2(&self) -> Self {
        self.log(2.)
    }

    pub fn ln_1p(&self) -> Self {
        Self {
            val: self.val.ln_1p(),
            location: self
                .tape
                .add_node(self.location, self.location, 1. / (1. + self.val), 0.),
            tape: self.tape,
        }
    }

    pub fn asin(&self) -> Self {
        Self {
            val: self.val.asin(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (1. - self.val.powi(2)).sqrt(),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn acos(&self) -> Self {
        Self {
            val: self.val.acos(),
            location: self.tape.add_node(
                self.location,
                self.location,
                -1. / (1. - self.val.powi(2)).sqrt(),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn atan(&self) -> Self {
        Self {
            val: self.val.atan(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (1. + self.val.powi(2)),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn sinh(&self) -> Self {
        Self {
            val: self.val.sinh(),
            location: self
                .tape
                .add_node(self.location, self.location, self.val.cosh(), 0.),
            tape: self.tape,
        }
    }

    pub fn cosh(&self) -> Self {
        Self {
            val: self.val.cosh(),
            location: self
                .tape
                .add_node(self.location, self.location, self.val.sinh(), 0.),
            tape: self.tape,
        }
    }

    pub fn tanh(&self) -> Self {
        Self {
            val: self.val.tanh(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (self.val.cosh().powi(2)),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn asinh(&self) -> Self {
        Self {
            val: self.val.asinh(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (1. + self.val.powi(2)).sqrt(),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn acosh(&self) -> Self {
        Self {
            val: self.val.acosh(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (self.val.powi(2) - 1.).sqrt(),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn atanh(&self) -> Self {
        Self {
            val: self.val.atanh(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (1. - self.val.powi(2)),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn exp(&self) -> Self {
        Self {
            val: self.val.exp(),
            location: self
                .tape
                .add_node(self.location, self.location, self.val.exp(), 0.),
            tape: self.tape,
        }
    }

    pub fn exp2(self) -> Self {
        Self {
            val: self.val.exp2(),
            location: self.tape.add_node(
                self.location,
                self.location,
                self.val.exp2() * 2_f64.ln(),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn sqrt(&self) -> Self {
        Self {
            val: self.val.sqrt(),
            location: self.tape.add_node(
                self.location,
                self.location,
                1. / (2. * self.val.sqrt()),
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn cbrt(&self) -> Self {
        self.powf(1. / 3.)
    }

    pub fn abs(&self) -> Self {
        let val = self.val.abs();
        Self {
            val,
            location: self.tape.add_node(
                self.location,
                self.location,
                if self.val == 0. {
                    f64::NAN
                } else {
                    self.val / val
                },
                0.,
            ),
            tape: self.tape,
        }
    }

    pub fn powi(&self, n: i32) -> Self {
        Self {
            val: self.val.powi(n),
            location: self.tape.add_node(
                self.location,
                self.location,
                n as f64 * self.val.powi(n - 1),
                0.,
            ),
            tape: self.tape,
        }
    }
}

impl<'a> Display for Var<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl<'a> PartialEq for Var<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.val.eq(&other.val)
    }
}

impl<'a> PartialOrd for Var<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.val.partial_cmp(&other.val)
    }
}

impl<'a> PartialEq<f64> for Var<'a> {
    fn eq(&self, other: &f64) -> bool {
        self.val.eq(other)
    }
}

impl<'a> PartialOrd<f64> for Var<'a> {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.val.partial_cmp(other)
    }
}

impl<'a> PartialEq<Var<'a>> for f64 {
    fn eq(&self, other: &Var<'a>) -> bool {
        other.val.eq(self)
    }
}

impl<'a> PartialOrd<Var<'a>> for f64 {
    fn partial_cmp(&self, other: &Var<'a>) -> Option<std::cmp::Ordering> {
        other.val.partial_cmp(self)
    }
}

/// Calculate gradients with respect to particular variables.
pub trait Gradient<T, S> {
    /// Calculate the gradient with respect to variable(s) `v`.
    fn wrt(&self, v: T) -> S;
}

/// Calculate the gradient with respect to variable `v`.
impl<'a> Gradient<&Var<'a>, f64> for Vec<f64> {
    fn wrt(&self, v: &Var) -> f64 {
        self[v.location]
    }
}

/// Calculate the gradient with respect to all variables in `v`. Returns a vector, where the items
/// in the vector are the gradients with respect to the variable in the original list `v`, in the
/// same order.
impl<'a> Gradient<&Vec<Var<'a>>, Vec<f64>> for Vec<f64> {
    fn wrt(&self, v: &Vec<Var<'a>>) -> Vec<f64> {
        let mut jac = vec![];
        for i in v {
            jac.push(self.wrt(i));
        }
        jac
    }
}

/// Calculate the gradient with respect to all variables in `v`. Returns a vector, where the items
/// in the vector are the gradients with respect to the variable in the original list `v`, in the
/// same order.
impl<'a> Gradient<&[Var<'a>], Vec<f64>> for Vec<f64> {
    fn wrt(&self, v: &[Var<'a>]) -> Vec<f64> {
        let mut jac = vec![];
        for i in v {
            jac.push(self.wrt(i));
        }
        jac
    }
}

/// Calculate the gradient with respect to all variables in `v`. Returns a vector, where the items
/// in the vector are the gradients with respect to the variable in the original list `v`, in the
/// same order.
impl<'a, const N: usize> Gradient<[Var<'a>; N], Vec<f64>> for Vec<f64> {
    fn wrt(&self, v: [Var<'a>; N]) -> Vec<f64> {
        let mut jac = vec![];
        for i in v {
            jac.push(self.wrt(&i));
        }
        jac
    }
}

/// Calculate the gradient with respect to all variables in `v`. Returns a vector, where the items
/// in the vector are the gradients with respect to the variable in the original list `v`, in the
/// same order.
impl<'a, const N: usize> Gradient<&[Var<'a>; N], Vec<f64>> for Vec<f64> {
    fn wrt(&self, v: &[Var<'a>; N]) -> Vec<f64> {
        let mut jac = vec![];
        for i in v {
            jac.push(self.wrt(i));
        }
        jac
    }
}

/// Trait for calculating expressions and tracking gradients for float power operations.
pub trait Powf<Rhs = Self> {
    type Output;

    /// Calculate `powf` for self, where `other` is the power to raise `self` to.
    fn powf(self, other: Rhs) -> Self::Output;
}

#[cfg(test)]
mod test {
    use super::*;
    use approx_eq::assert_approx_eq;

    #[test]
    fn test_ad0() {
        let g = Tape::new();
        let a = g.add_var(2.);
        let b = a.exp() / 5.;
        let c = a.exp2() / 5.;
        let gradb = b.grad().wrt(&a);
        let gradc = c.grad().wrt(&a);
        assert_eq!(gradb, 2_f64.exp() / 5.);
        assert_eq!(gradc, 1. / 5. * 2_f64.exp2() * 2_f64.ln());
    }

    #[test]
    fn test_ad1() {
        let tape = Tape::new();
        let vars = (0..6).map(|x| tape.add_var(x as f64)).collect::<Vec<_>>();
        let res =
            -vars[0] + vars[1].sin() * vars[2].ln() - vars[3] / vars[4] + 1.5 * vars[5].sqrt();
        let grads = res.grad();
        let est_grads = vars.iter().map(|v| grads.wrt(v)).collect::<Vec<_>>();
        let true_grads = vec![
            -1.,
            2_f64.ln() * 1_f64.cos(),
            1_f64.sin() / 2.,
            -1. / 4.,
            3. / 4_f64.powi(2),
            0.75 / 5_f64.sqrt(),
        ];
        for i in 0..6 {
            assert_approx_eq!(est_grads[i], true_grads[i]);
        }
    }

    #[test]
    fn test_ad2() {
        fn f<'a>(a: Var<'a>, b: Var<'a>) -> Var<'a> {
            (a / b - a) * (b / a + a + b) * (a - b)
        }

        let g = Tape::new();
        let a = g.add_var(230.3);
        let b = g.add_var(33.2);
        let y = f(a, b);
        let grads = y.grad();
        assert_approx_eq!(grads.wrt(&a), -153284.83150602411);
        assert_approx_eq!(grads.wrt(&b), 3815.0389441500993);
    }

    #[test]
    fn test_ad3() {
        let g = Tape::new();
        let a = g.add_var(10.1);
        let b = g.add_var(2.5);
        let c = g.add_var(4.0);
        let x = g.add_var(1.0);
        let y = g.add_var(2.0);
        let res = a.powf(b) - c * x / y;
        let grads = res.grad();
        assert_approx_eq!(grads.wrt(&a), 2.5 * 10.1_f64.powf(2.5 - 1.));
        assert_approx_eq!(grads.wrt(&b), 10.1_f64.powf(2.5) * 10.1_f64.ln());
        assert_approx_eq!(grads.wrt(&c), -1. / 2.);
        assert_approx_eq!(grads.wrt(&x), -4. / 2.);
        assert_approx_eq!(grads.wrt(&y), 4. * 1. / (2_f64.powi(2)));
    }

    #[test]
    fn test_ad4() {
        let g = Tape::new();
        let params = (0..5).map(|x| g.add_var(x as f64)).collect::<Vec<_>>();
        let sum = params.iter().copied().sum::<Var>();
        let derivs = sum.grad();
        for i in derivs.wrt(&params) {
            assert_approx_eq!(i, 1.);
        }
    }

    #[test]
    fn test_ad5() {
        let g = Tape::new();
        let a = g.add_var(2.);
        let b = g.add_var(3.2);
        let c = g.add_var(-4.5);
        let res = a.exp2() / (b.powf(c) + 5.).sqrt();
        let est_grads = res.grad().wrt(&[a, b, c]);
        let true_grads = vec![
            2_f64.exp2() * 2_f64.ln() / ((3.2_f64).powf(-4.5) + 5.).sqrt(),
            -((2. - 1_f64).exp2() * (-4.5) * (3.2_f64).powf(-4.5 - 1.))
                / ((3.2_f64.powf(-4.5) + 5.).powf(1.5)),
            -((2. - 1_f64).exp2() * (3.2_f64).powf(-4.5) * (3.2_f64).ln())
                / ((3.2_f64).powf(-4.5) + 5.).powf(1.5),
        ];
        for i in 0..3 {
            assert_approx_eq!(est_grads[i], true_grads[i]);
        }
    }

    #[test]
    fn test_ad6() {
        let g = Tape::new();
        let a = g.add_var(10.1);
        let b = g.add_var(2.5);
        let c = g.add_var(4.0);
        let x = g.add_var(-1.0);
        let y = g.add_var(2.0);
        let z = g.add_var(-5.);
        let params = [a, b, c, x, y, z];
        let res = a.tan() * b.log2() + c.exp() / (x.powi(2) + 2.) - y.powf(z);
        let est_grads = res.grad().wrt(&params);
        let true_grads = vec![
            2.5_f64.ln() / (2_f64.ln() * 10.1_f64.cos().powi(2)),
            10.1_f64.tan() / (2.5 * 2_f64.ln()),
            4_f64.exp() / ((-1_f64).powi(2) + 2.),
            -2. * 4_f64.exp() * (-1_f64) / ((-1_f64).powi(2) + 2.).powi(2),
            -5_f64 * -2_f64.powf(-5. - 1.),
            -2_f64.powf(-5.) * 2_f64.ln(),
        ];
        for i in 0..6 {
            assert_approx_eq!(est_grads[i], true_grads[i]);
        }
    }

    #[test]
    fn test_ad7() {
        let g = Tape::new();
        let v = g.add_var(0.5);

        let res = v.powi(2) + 5.;
        let grad = res.grad().wrt(&v);
        assert_approx_eq!(grad, 2. * 0.5);

        let res = (v.powi(2) + 5.).powi(2);
        let grad = res.grad().wrt(&v);
        assert_approx_eq!(grad, 4. * 0.5 * (0.5_f64.powi(2) + 5.));

        let res = (v.powi(2) + 5.).powi(2) / 2.;
        let grad = res.grad().wrt(&v);
        assert_approx_eq!(grad, 2. * 0.5 * (0.5_f64.powi(2) + 5.));

        let res = (v.powi(2) + 5.).powi(2) / 2. - v;
        let grad = res.grad().wrt(&v);
        assert_approx_eq!(grad, 2. * 0.5 * (0.5_f64.powi(2) + 5.) - 1.);

        let res = (v.powi(2) + 5.).powi(2) / 2. - v.powi(3);
        let grad = res.grad().wrt(&v);
        assert_approx_eq!(grad, 0.5 * (2. * 0.5_f64.powi(2) - 3. * 0.5 + 10.));

        let res = ((v.powi(2) + 5.).powi(2) / 2. - v.powi(3)).powi(2);
        let grad = res.grad().wrt(&v);
        assert_approx_eq!(
            grad,
            0.5 * (2. * 0.5_f64.powi(2) - 3. * 0.5 + 10.)
                * (0.5_f64.powi(4) - 2. * 0.5_f64.powi(3) + 10. * 0.5_f64.powi(2) + 25.)
        );
    }

    #[test]
    fn test_rosenbrock() {
        let g = Tape::new();
        let x = g.add_var(5.);
        let y = g.add_var(-2.);

        let res = (1. - x).powi(2);
        let grad = res.grad().wrt(&[x, y]);
        assert_approx_eq!(grad[0], -2. * (1. - 5.));
        assert_approx_eq!(grad[1], 0.);

        let res = 100. * (y - x.powi(2)).powi(2);
        let grad = res.grad().wrt(&[x, y]);
        assert_approx_eq!(grad[0], -400. * 5. * (-2. - 5_f64.powi(2)));
        assert_approx_eq!(grad[1], 200. * (-2. - 5_f64.powi(2)));

        let res = (1. - x).powi(2) + 100. * (y - x.powi(2)).powi(2);
        let grad = res.grad().wrt(&[x, y]);
        assert_approx_eq!(
            grad[0],
            2. * (200. * 5_f64.powi(3) - 200. * 5. * -2. + 5. - 1.)
        );
        assert_approx_eq!(grad[1], 200. * (-2. - 5_f64.powi(2)));
    }

    #[test]
    fn test_assign() {
        let g = Tape::new();
        let a = g.add_var(1.);
        let mut b = a * 1.0;
        b *= 3.0;
        b /= 2.0;
        b += 5.0;
        b -= 4.0;
        let gradb = b.grad().wrt(&a);
        assert_eq!(gradb, 1.5);
        assert_eq!(b.val(), 2.5);
    }
}
