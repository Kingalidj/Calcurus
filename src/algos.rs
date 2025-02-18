use std::{borrow::Borrow, cmp, ops, slice};

use crate::{
    atom::{Atom, Expr, Func, Pow, Prod, Real, Sum, SymbolicExpr},
    rational::{binomial_coeff, Int, Rational},
    utils::HashSet,
};

use derive_more::{From, Into};

impl Sum {
    pub fn add_rhs(&mut self, rhs: &Expr) {
        use Atom as A;

        if let Some(A::Undef) = self.first() {
            return;
        }

        let rhs = rhs.flatten();

        if rhs.is_zero() {
            return;
        }

        match rhs.atom() {
            A::Undef => {
                self.args.clear();
                self.args.push(rhs.clone());
            }
            A::Sum(sum) => {
                sum.args.iter().for_each(|a| self.add_rhs(a));
            }
            A::Rational(r1) => {
                for a in &mut self.args {
                    if let A::Rational(r2) = a.atom() {
                        *a = A::Rational(r1.clone() + r2).into();
                        return;
                    }
                }
                self.args.push(rhs.clone())
            }
            _ => {
                if let Some(arg) = self.args.iter_mut().find(|a| {
                    a.non_rational_term()
                        .is_some_and(|t| Some(t) == rhs.non_rational_term())
                }) {
                    *arg = Expr::from(arg.rational_coeff().unwrap() + rhs.rational_coeff().unwrap())
                        * arg.non_rational_term().unwrap()
                } else {
                    self.args.push(rhs.clone())
                }
            }
        }
    }

    fn add_rhs_raw(&mut self, rhs: &Expr) {
        use Atom as A;

        let rhs = rhs.flatten();
        if let Atom::Undef = rhs.atom() {
            self.args.clear();
            self.args.push(Expr::undef());
            return;
        }

        if self.is_undef() || rhs.is_zero() {
            return;
        }
        //match self.first() {
        //    Some(A::Undef | &A::ZERO) => return,
        //    Some(_) | None => (),
        //}

        match rhs.atom() {
            A::Undef => {
                self.args.clear();
                self.args.push(rhs.clone())
            }
            A::Sum(sum) => {
                sum.args.iter().for_each(|a| self.add_rhs_raw(a));
            }
            _ => self.args.push(rhs.clone()),
        }
    }

    fn cmp_args(lhs: &Expr, rhs: &Expr) -> cmp::Ordering {
        match (lhs.atom(), rhs.atom()) {
            (Atom::Prod(p1), Atom::Prod(p2)) => p1.cmp(p2),
            (Atom::Var(a), Atom::Var(b)) => a.cmp(b),
            _ => rhs.cmp(lhs),
        }
    }

    fn add_sorted(lhs: &Expr, rhs: &Expr) -> Sum {
        let mut lhs = lhs;
        let mut rhs = rhs;

        if Self::cmp_args(lhs, rhs).is_ge() {
            std::mem::swap(&mut lhs, &mut rhs)
        }
        let mut sum = Sum::zero();
        sum.add_rhs(&lhs);
        sum.add_rhs(&rhs);
        sum
    }

    fn flat_merge(lhs: &Expr, mut rhs: Sum) -> Sum {
        let lhs = lhs.flatten();

        if lhs.is_zero() {
            return rhs;
        }

        match lhs.atom() {
            Atom::Sum(sum) => {
                let mut sum = sum.clone();
                rhs.args.into_iter().for_each(|a| {
                    sum.add_rhs(&a);
                });
                sum
            }
            _ => {
                let mut res = vec![lhs.clone()];
                res.append(&mut rhs.args);
                rhs.args = res;
                rhs
            }
        }
        //if let Atom::Sum(sum) = lhs.atom() {
        //    let mut sum = sum.clone();
        //    rhs.args.into_iter().for_each(|a| {
        //        sum.add_rhs(&a);
        //    });
        //    sum
        //} else {
        //    let mut res = vec![lhs];
        //    res.extend(rhs.args);
        //    rhs.args = res;
        //    rhs
        //}
    }

    fn merge_args(p: &[Expr], q: &[Expr]) -> Sum {
        // println!("merge_args: {p:?} (+) {q:?}");
        if p.is_empty() {
            //q.into_iter().cloned().collect()
            Sum { args: q.to_vec() }
        } else if q.is_empty() {
            //p.into_iter().cloned().collect()
            Sum { args: p.to_vec() }
        } else {
            let p1 = p.first().unwrap();
            let q1 = q.first().unwrap();
            let p_rest = &p[1..];
            let q_rest = &q[1..];
            let h = Sum::reduce_rec(&[p1.clone(), q1.clone()]).args;
            // println!("merge_args: {p1} (+) {q1} -> {h:?}");
            if h.is_empty() {
                Sum::merge_args(p_rest, q_rest)
            } else if h.len() == 1 {
                Sum::flat_merge(&h[0], Sum::merge_args(p_rest, q_rest))
            } else {
                let rhs = if p1 == &h[0] && q1 == &h[1] {
                    let rhs = Sum::merge_args(p_rest, q);
                    // println!("  merge_args1: {p_rest:?} (+) {q:?} -> {rhs:?}");
                    rhs
                } else if q1 == &h[0] && p1 == &h[1] {
                    //assert!(q1 == &h[0], "{:?} != {:?}", q1, h[0]);
                    let rhs = Sum::merge_args(p, q_rest);
                    // println!("  merge_args2: {p:?} (+) {q_rest:?} -> {rhs:?}");
                    rhs
                } else {
                    panic!("Illegal reduction: {q:?} + {p:?} -> h")
                };

                // println!("flat_merge: {:?} (+) {:?}", h[0], rhs);
                Sum::flat_merge(&h[0], rhs)
            }
        }
    }

    pub(crate) fn reduce_rec(args: &[Expr]) -> Sum {
        //println!("reduce_rec: {args:?}");
        let res = if args.len() < 2 {
            //return args.into_iter().cloned().collect();
            Sum {
                args: args.to_vec(),
            }
        } else if args.len() == 2 {
            //println!("args: {args:?}");
            let lhs = args[0].flatten();
            let rhs = args[1].flatten();
            if let (Atom::Sum(p1), Atom::Sum(p2)) = (lhs.atom(), rhs.atom()) {
                Sum::merge_args(&p1.args, &p2.args)
            } else if let Atom::Sum(p) = lhs.atom() {
                Sum::merge_args(&p.args, slice::from_ref(rhs))
            } else if let Atom::Sum(p) = rhs.atom() {
                Sum::merge_args(slice::from_ref(lhs), &p.args)
            } else if lhs.is_const() || rhs.is_const() {
                Sum::add_sorted(lhs, rhs)
            } else if lhs
                .non_rational_term()
                .is_some_and(|term| Some(term) == rhs.non_rational_term())
            {
                //} else if lhs.term() == rhs.term() && lhs.term().is_some() {
                let e = lhs.rational_coeff().unwrap() + rhs.rational_coeff().unwrap();
                Sum {
                    args: vec![(Expr::from(e) * lhs.non_rational_term().unwrap()).reduce()],
                }
            } else {
                Sum::add_sorted(lhs, rhs)
            }
        } else {
            let lhs = args.first().unwrap().flatten();
            let rhs = Sum::reduce_rec(&args[1..]);

            if let Atom::Sum(p) = lhs.atom() {
                Sum::merge_args(&p.args, &rhs.args)
            } else {
                Sum::merge_args(slice::from_ref(lhs), &rhs.args)
            }
        };
        res
    }
}
impl Prod {
    pub fn mul_rhs_raw(&mut self, rhs: &Expr) {
        use Atom as A;

        let rhs = rhs.flatten();
        if let Atom::Undef = rhs.atom() {
            self.args.clear();
            self.args.push(Expr::undef());
            return;
        }

        if self.is_undef() || self.is_zero() || rhs.is_one() {
            return;
        } else if rhs.is_undef() || rhs.is_zero() {
            self.args.clear();
            self.args.push(rhs.clone());
            return;
        }

        match rhs.atom() {
            A::Prod(prod) => {
                prod.args.iter().for_each(|a| self.mul_rhs_raw(a));
            }
            _ => self.args.push(rhs.clone()),
        }
    }
    pub fn mul_rhs(&mut self, rhs: &Expr) {
        use Atom as A;

        let rhs = rhs.flatten();
        if let Atom::Undef = rhs.atom() {
            self.args.clear();
            self.args.push(Expr::undef());
            return;
        }

        if self.is_undef() || self.is_zero() || rhs.is_one() {
            return;
        } else if rhs.is_undef() || rhs.is_zero() {
            self.args.clear();
            self.args.push(rhs.clone());
            return;
        }

        match rhs.atom() {
            A::Prod(prod) => {
                prod.args.iter().for_each(|a| self.mul_rhs(a));
            }
            A::Rational(r1) => {
                for a in &mut self.args {
                    if let A::Rational(r2) = a.atom() {
                        *a = A::Rational(r1.clone() * r2).into();
                        return;
                    }
                }
                self.args.push(rhs.clone())
            }
            A::Pow(pow) => {
                if pow.base().is_prod() {
                    let rhs_terms = pow.base().args();
                    let expon = pow.exponent();

                    let mut remainder = vec![];

                    for term in rhs_terms {
                        if let Some(arg) = self.args.iter_mut().find(|a| &a.base() == term) {
                            *arg = Expr::pow(arg.base(), arg.exponent() + expon)
                        } else {
                            remainder.push(term.clone())
                        }
                    }

                    if remainder.len() == 1 {
                        let rem = Expr::pow(remainder.remove(0), expon);
                        self.args.push(rem)
                    } else if remainder.len() > 1 {
                        let rem = Expr::pow(&Prod { args: remainder }.into(), expon);
                        self.args.push(rem)
                    }
                } else if let Some(arg) = self.args.iter_mut().find(|a| &a.base() == pow.base()) {
                    *arg = Expr::pow(arg.base(), arg.exponent() + pow.exponent())
                } else {
                    self.args.push(rhs.clone())
                }
            }
            _ => {
                if let Some(arg) = self.args.iter_mut().find(|a| a.base() == rhs.base()) {
                    *arg = Expr::pow(arg.base(), arg.exponent() + rhs.exponent())
                } else {
                    self.args.push(rhs.clone())
                }
            }
        }
    }

    fn cmp_args(lhs: &Expr, rhs: &Expr) -> cmp::Ordering {
        lhs.cmp(rhs)
    }

    fn mul_sorted(lhs: &Expr, rhs: &Expr) -> Prod {
        let mut lhs = lhs;
        let mut rhs = rhs;
        if Self::cmp_args(lhs, rhs).is_ge() {
            std::mem::swap(&mut lhs, &mut rhs)
        }
        let mut mul = Prod::one();
        mul.mul_rhs(&lhs);
        mul.mul_rhs(&rhs);
        mul
    }

    fn expand_mul(lhs: &Expr, rhs: &Expr) -> Expr {
        use Atom as A;
        match (lhs.atom(), rhs.atom()) {
            (A::Prod(_), A::Prod(_)) => {
                let mut res = Prod::one();
                res.mul_rhs(lhs);
                res.mul_rhs(rhs);
                A::Prod(res).into()
            }
            (A::Sum(sum), _) => {
                let mut res = Sum::zero();
                for term in &sum.args {
                    let term_prod = Self::expand_mul(term, rhs);
                    res.add_rhs(&term_prod)
                }
                A::Sum(res).into()
            }
            (_, A::Sum(sum)) => {
                let mut res = Sum::zero();
                for term in &sum.args {
                    let term_prod = Self::expand_mul(lhs, term);
                    res.add_rhs(&term_prod)
                }
                A::Sum(res).into()
            }
            _ => Expr::mul(lhs, rhs),
        }
    }

    fn distribute_first(&self) -> Expr {
        // prod = a * sum * b
        let sum_indx = if let Some(indx) = self
            .args
            .iter()
            .position(|a| matches!(a.atom(), Atom::Sum(_)))
        {
            indx
        } else {
            return Atom::Prod(self.clone()).into();
        };

        let mut a = Prod::one();
        let mut b = Prod::one();
        let sum = self.args[sum_indx].clone();

        for (i, arg) in self.args.iter().enumerate() {
            if i < sum_indx {
                a.args.push(arg.clone())
            } else if i > sum_indx {
                b.args.push(arg.clone())
            }
        }

        let (lhs, rhs): (Expr, Expr) = (Atom::Prod(a).into(), Atom::Prod(b).into());
        let mut res = Sum::zero();

        for term in sum.args() {
            res.add_rhs(&(lhs.clone() * term * rhs.clone()));
        }

        Atom::Sum(res).into()
    }

    fn distribute(&self) -> Expr {
        if self.args.is_empty() {
            return Expr::one();
        } else if self.args.len() == 1 {
            return self.args[0].clone();
        }

        self.args
            .iter()
            .fold(Atom::Prod(Prod::one()).into(), |l, r| {
                Self::expand_mul(&l, r)
            })
    }

    fn flat_merge(lhs: &Expr, mut rhs: Prod) -> Prod {
        let lhs = lhs.flatten();
        if lhs.is_zero() {
            rhs.args = vec![Expr::zero()];
            return rhs;
        } else if lhs.is_one() {
            return rhs;
        }
        match lhs.atom() {
            Atom::Prod(prod) => {
                let mut prod = prod.clone();
                rhs.args.into_iter().for_each(|a| {
                    prod.mul_rhs(&a);
                });
                prod
            }
            _ => {
                let mut res = vec![lhs.clone()];
                res.append(&mut rhs.args);
                rhs.args = res;
                rhs
            }
        }
        //if let Atom::Prod(prod) = lhs.atom() {
        //    let mut prod = prod.clone();
        //    rhs.args.into_iter().for_each(|a| {
        //        prod.mul_rhs(&a);
        //    });
        //    //vec![Atom::Prod(prod).into()]
        //    prod
        //} else {
        //    let mut res = vec![lhs];
        //    res.extend(rhs.args);
        //    rhs.args = res;
        //    rhs
        //}
    }

    fn merge_args(p: &[Expr], q: &[Expr]) -> Prod {
        //println!("merge_args: {p:?} (*) {q:?}");
        if p.is_empty() {
            Prod { args: q.to_vec() }
        } else if q.is_empty() {
            Prod { args: p.to_vec() }
        } else {
            let p1 = p.first().unwrap();
            let q1 = q.first().unwrap();
            let p_rest = &p[1..];
            let q_rest = &q[1..];
            let h = Prod::reduce_rec(&[p1.clone(), q1.clone()]).args;
            //println!("merge_args: {p1} (*) {p_rest:?} (*) {q1} (*) {q_rest:?} -> {h:?}");
            if h.is_empty() {
                Prod::merge_args(p_rest, q_rest)
            } else if h.len() == 1 {
                Prod::flat_merge(&h[0], Prod::merge_args(p_rest, q_rest))
            } else {
                let rhs = if p1 == &h[0] && q1 == &h[1] {
                    Prod::merge_args(p_rest, q)
                } else if q1 == &h[0] && p1 == &h[1] {
                    //println!("h: {h:?}, p1: {p1:?}, q1: {q1:?}");
                    //assert!(q1 == &h[0], "{:?} != {:?}", q1, h[0]);
                    Prod::merge_args(p, q_rest)
                } else {
                    panic!("Illegal reduction: {q:?} * {p:?} -> h")
                };

                Prod::flat_merge(&h[0], rhs)
            }
        }
    }

    pub(crate) fn reduce_rec(args: &[Expr]) -> Prod {
        //println!("reduce_rec: {args:?}");
        if args.len() < 2 {
            //return args.into_iter().cloned().collect();
            Prod {
                args: args.to_vec(),
            }
        } else if args.len() == 2 {
            //println!("args: {args:?}");
            let lhs = args[0].flatten();
            let rhs = args[1].flatten();
            if let (Atom::Prod(p1), Atom::Prod(p2)) = (lhs.atom(), rhs.atom()) {
                Prod::merge_args(&p1.args, &p2.args)
            } else if let Atom::Prod(p) = lhs.atom() {
                Prod::merge_args(&p.args, slice::from_ref(rhs))
            } else if let Atom::Prod(p) = rhs.atom() {
                Prod::merge_args(slice::from_ref(lhs), &p.args)
            } else if lhs.is_const() || rhs.is_const() {
                let res = Prod::mul_sorted(lhs, rhs);

                // numbers if present always at the lhs
                //assert!(
                //    (res.n_args() < 2
                //        || (res.args[0].is_number()
                //            || res.args[1].is_number() && res.args[0].is_number())),
                //);

                res
            } else if lhs.base() == rhs.base() {
                let e = (lhs.exponent() + rhs.exponent()).reduce();
                Prod {
                    args: vec![Expr::pow(lhs.base(), e).reduce()],
                }
            } else {
                let mut lhs = lhs.clone();
                let mut rhs = rhs.clone();
                if rhs < lhs {
                    std::mem::swap(&mut lhs, &mut rhs);
                }

                Prod {
                    args: vec![lhs, rhs],
                }
            }
        } else {
            let lhs = args.first().unwrap().flatten();
            let rhs = Prod::reduce_rec(&args[1..]);

            if let Atom::Prod(p) = lhs.atom() {
                Prod::merge_args(&p.args, &rhs.args)
            } else {
                Prod::merge_args(slice::from_ref(lhs), &rhs.args)
            }
        }
    }
}

impl Pow {
    pub fn expand_pow_rec(&self, recurse: bool) -> Expr {
        use Atom as A;
        let expand_pow = |lhs: &Expr, rhs: &Expr| -> Expr {
            if recurse {
                Expr::pow(lhs, rhs).expand()
            } else {
                Expr::pow(lhs, rhs)
            }
        };
        let expand_mul = |lhs: &Expr, rhs: &Expr| -> Expr {
            if recurse {
                Expr::mul(lhs, rhs).expand()
            } else {
                Expr::mul(lhs, rhs)
            }
        };

        let (base, e) = match (self.base().atom(), self.exponent().atom()) {
            (_, A::Rational(r)) if r.is_one() => {
                return self.base().clone();
            }
            (A::Sum(sum), A::Rational(r))
                if r.is_int() && r > &Rational::ONE && sum.args.len() > 1 =>
            {
                (sum, r.numer().clone())
            }
            (A::Sum(sum), A::Rational(r)) if r > &Rational::ONE && sum.args.len() > 1 => {
                let (div, rem) = r.div_rem();
                return expand_mul(
                    &expand_pow(self.base(), &Expr::from(div)),
                    &expand_pow(self.base(), &Expr::from(rem)),
                );
            }
            (A::Prod(_), _) => {
                return self
                    .base()
                    .clone()
                    .map_args(|a| *a = expand_pow(a, self.exponent()));
                //return args
                //    .iter()
                //    .map(|a| expand_pow(a, self.exponent()))
                //    .fold(Expr::one(), |prod, rhs| prod * rhs)
            }
            _ => {
                return A::Pow(self.clone()).into();
            }
        };

        // (a + b)^exp
        let exp = Expr::from(e.clone());
        let (a, b) = base.as_binary_sum();
        //let mut args = base.args.clone();
        //let a = args.pop_front().unwrap();
        //let b = A::Sum(Sum { args }).into();

        let mut res = Sum::zero();
        for k in 0..=e {
            let rhs = if k == 0 {
                // 1 * a^exp
                expand_pow(&a, &exp)
            } else if k == e {
                // 1 * b^k
                expand_pow(&b, &Expr::from(k.clone()))
            } else {
                // a^k + b^(exp-k)
                let c = binomial_coeff(e, k);
                let k_e = Expr::from(k.clone());

                expand_mul(
                    &Expr::from(c),
                    &expand_mul(
                        &expand_pow(&a, &k_e),
                        &expand_pow(&b, &Expr::from(e.clone() - &k)),
                    ),
                )
            };
            res.add_rhs(&rhs);
        }

        Atom::Sum(res).into()
    }

    pub fn expand(&self) -> Expr {
        self.expand_pow_rec(true)
    }
}

// operations
impl Expr {
    pub fn add(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        use Atom as A;
        let (lhs, rhs): (&Expr, &Expr) = (lhs.borrow().flatten(), rhs.borrow().flatten());

        if lhs.is_undef() || rhs.is_undef() {
            return A::Undef.into();
        } else if lhs.is_zero() {
            return rhs.clone();
        } else if rhs.is_zero() {
            return lhs.clone();
        }

        match (lhs.atom(), rhs.atom()) {
            (A::Rational(r1), A::Rational(r2)) => A::Rational(*r1 + r2).into(),
            (_, _) => {
                let mut sum = Sum::zero();
                sum.add_rhs(lhs);
                sum.add_rhs(rhs);
                A::Sum(sum).into()
            }
        }
    }
    pub fn sub(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        let (lhs, rhs) = (lhs.borrow(), rhs.borrow());
        let min_one = Expr::from(-1);
        let min_rhs = Expr::mul(min_one, rhs);
        Expr::add(lhs, min_rhs)
    }
    pub fn mul(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        use Atom as A;
        let (lhs, rhs): (&Expr, &Expr) = (lhs.borrow(), rhs.borrow());

        if lhs.is_undef() || rhs.is_undef() {
            return Expr::undef();
        } else if lhs.is_zero() || rhs.is_zero() {
            return Expr::zero();
        } else if lhs.is_one() {
            rhs.clone()
        } else if rhs.is_one() {
            lhs.clone()
        } else if let (Ok(r1), Ok(r2)) =
            (lhs.try_unwrap_rational_ref(), rhs.try_unwrap_rational_ref())
        {
            (r1.clone() * r2).into()
        } else if lhs.base() == rhs.base() {
            Expr::pow(lhs.base(), lhs.exponent() + rhs.exponent())
        } else {
            let mut prod = Prod::one();
            prod.mul_rhs(lhs);
            prod.mul_rhs(rhs);
            A::Prod(prod).into()
        }

        /*
        match (lhs.atom(), rhs.atom()) {
            (A::Rational(r1), A::Rational(r2)) => A::Rational(r1.clone() * r2).into(),
            (_, _) => {
                if lhs.base() == rhs.base() {
                    Expr::pow(lhs.base(), lhs.exponent() + rhs.exponent())
                } else {
                    let mut prod = Prod::one();
                    prod.mul_rhs(lhs);
                    prod.mul_rhs(rhs);
                    A::Prod(prod).into()
                }
                //Expr::prod([lhs, rhs]),
            }
        }
        */
        //Expr::prod([lhs.borrow(), rhs.borrow()])
    }
    pub fn div(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        let (lhs, rhs) = (lhs.borrow(), rhs.borrow());

        if lhs.is_undef() || rhs.is_undef() {
            Expr::undef()
        } else if lhs == rhs {
            Expr::one()
        } else {
            let min_one = Expr::from(-1);
            let inv_rhs = Expr::pow(rhs, &min_one);
            Expr::mul(lhs, &inv_rhs)
        }

        /*
        match (lhs.atom(), rhs.atom()) {
            (_, Atom::Pow(pow)) if pow.exponent().is_neg() => {
                let rhs = Expr::pow(pow.base(), Expr::min_one() * pow.exponent());
                lhs * rhs
            }
            _ => {
                let min_one = Expr::from(-1);
                let inv_rhs = Expr::pow(rhs, &min_one);
                Expr::mul(lhs, &inv_rhs)
            }
        }.explain("div", &[lhs, rhs])
        */
    }

    pub fn add_raw(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        let (lhs, rhs) = (lhs.borrow(), rhs.borrow());

        let mut prod = Sum::zero();
        prod.add_rhs_raw(lhs);
        prod.add_rhs_raw(rhs);
        Atom::Sum(prod).into()
    }

    pub fn mul_raw(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        let (lhs, rhs) = (lhs.borrow(), rhs.borrow());

        let mut prod = Prod::one();
        prod.mul_rhs_raw(lhs);
        prod.mul_rhs_raw(rhs);
        Atom::Prod(prod).into()
    }
    pub fn div_raw(lhs: impl Borrow<Expr>, rhs: impl Borrow<Expr>) -> Expr {
        let min_one = Expr::min_one();
        let one_div_rhs = Expr::pow_raw(rhs, min_one);
        Expr::mul_raw(lhs, one_div_rhs)
    }
    pub fn pow_raw(base: impl Borrow<Expr>, exponent: impl Borrow<Expr>) -> Expr {
        Atom::Pow(Pow {
            args: [base.borrow().clone(), exponent.borrow().clone()],
        })
        .into()
    }

    pub fn pow(base: impl Borrow<Expr>, exponent: impl Borrow<Expr>) -> Expr {
        use Atom as A;

        let (base, exponent) = (base.borrow(), exponent.borrow());

        if base.is_undef()
            || exponent.is_undef()
            || (base.is_zero() && exponent.is_zero())
            || (base.is_zero() && exponent.is_neg())
        {
            return Expr::undef();
        }

        if base.is_one() {
            return Expr::one();
        } else if exponent.is_one() {
            return base.clone();
        } else if exponent.is_zero() {
            return Expr::one();
        }
        match (base.atom(), exponent.atom()) {
            (A::Rational(b), A::Rational(e)) if b.is_int() && e.is_int() => {
                let (pow, rem) = b.clone().pow(e.clone());
                assert!(rem.is_zero());
                Expr::from(pow)
            }
            (A::Pow(pow), A::Rational(e)) if e.is_int() => {
                Expr::pow(pow.base(), exponent * pow.exponent())
            }

            //(A::Pow(pow), A::Rational(e2)) if e2.is_int() => {
            //    println!("pow: {base:?}, {exponent:?}");
            //    match pow.exponent().atom() {
            //        A::Rational(e1) => {
            //            Expr::pow(base, Expr::from(e1.clone() * e2))
            //        }
            //        _ => Expr::raw_pow(base, exponent),
            //    }
            //}
            _ => Expr::pow_raw(base, exponent),
        }
    }

    pub fn derivative<T: Borrow<Self>>(&self, x: T) -> Self {
        use Atom as A;
        let x = x.borrow();

        if self == x && !self.is_const() {
            return Expr::one();
        }

        match self.atom() {
            A::Undef => self.clone(),
            A::Irrational(_) | A::Rational(_) => Expr::zero(),
            A::Sum(Sum { args }) => {
                let mut res = Sum::zero();
                args.iter()
                    .map(|a| a.derivative(x))
                    .for_each(|a| res.add_rhs(&a));
                Atom::Sum(res).into()
            }
            A::Prod(prod) => {
                if prod.args.is_empty() {
                    return Expr::zero();
                } else if prod.args.len() == 1 {
                    return prod.args.first().unwrap().derivative(x);
                }
                // prod = term * args
                //let mut args = args.clone();
                //let term = args.pop_front().unwrap();
                //let rest = Atom::Prod(Prod { args }).into();
                let (term, rest) = prod.as_binary_mul();

                // d(a * b)/dx = da/dx * b + a * db/dx
                term.derivative(x) * &rest + term * rest.derivative(x)
            }
            A::Pow(pow) => {
                let v = pow.base();
                let w = pow.exponent();
                // d(v^w)/dx = w * v^(w - 1) * dv/dx + dw/dx * v^w * ln(v)
                w * Expr::pow(v, w - Expr::one()) * v.derivative(x)
                    + w.derivative(x) * Expr::pow(v, w) * Expr::ln(v)
            }
            A::Var(_) => {
                if self.free_of(x) {
                    Expr::zero()
                } else {
                    // TODO: d(f)/d(x^2)
                    todo!()
                }
            }
            A::Func(f) => f.derivative(x),
        }
    }
}

pub trait CostFn {
    fn cost(e: &Expr) -> usize;
}

pub struct ExprCost;
impl CostFn for ExprCost {
    fn cost(e: &Expr) -> usize {
        let mut cost = 1;
        e.iter_args().for_each(|a| cost += Self::cost(a));
        cost
    }
}

impl Expr {
    pub fn simplify(&self) -> Vec<Self> {
        let operations: Vec<(fn(&Expr) -> Expr, &'static str)> = vec![
            (Self::expand, "expand"),
            //(Self::expand_main_op, "expand_main_op"),
            (Self::expand_exponential, "expand_exponential"),
            (Self::expand_trig, "expand_trig"),
            (Self::expand_ln, "expand_ln"),
            (Self::contract_trig, "contract_trig"),
            //(Self::distribute, "distribute"),
            (Self::rationalize, "rationalize"),
            (Self::factor_out, "factor_out"),
            (Self::cancel, "cancel"),
        ];

        if self.is_irreducible() {
            return vec![];
        }

        let mut todo = vec![self.clone()];
        let mut eclass = HashSet::<Self>::default();

        let mut steps = 0;
        while let Some(expr) = todo.pop() {
            for (op, name) in &operations {
                let e = op(&expr).reduce();
                //println!("{name}: {expr} -> {e}");

                if !eclass.contains(&e) {
                    todo.push(e.clone());
                    eclass.insert(e);
                }
            }
            steps += 1;
            if steps > 10 {
                break;
            }
        }

        let mut eclass: Vec<_> = eclass.into_iter().collect();
        eclass.sort_by_key(ExprCost::cost);

        eclass
    }

    pub fn expand(&self) -> Self {
        use Atom as A;
        let expanded = self.clone().map_args(|a| *a = a.expand());
        match expanded.atom() {
            A::Var(_) | A::Undef | A::Rational(_) => expanded.clone(),
            A::Sum(Sum { args }) if args.len() == 1 => args.first().unwrap().expand(),
            A::Prod(prod) => prod.distribute(),
            A::Pow(pow) => pow.expand(),
            _ => expanded.clone(),
        }
    }

    pub fn expand_main_op(&self) -> Self {
        use Atom as A;
        match self.atom() {
            A::Prod(prod) => prod.distribute(),
            A::Pow(pow) => pow.expand_pow_rec(false),
            A::Sum(sum) if sum.n_args() == 1 => sum.args()[0].expand_main_op(),
            A::Irrational(_) | A::Undef | A::Rational(_) | A::Var(_) | A::Sum(_) => self.clone(),
            A::Func(_) => self.clone(),
        }
    }

    /// exponential expansion
    ///
    /// exp(u+v) = exp(u) * exp(v)
    /// exp(w*u) = exp(u)^w
    pub fn expand_exponential(&self) -> Self {
        if self.n_args() == 0 {
            return self.clone();
        }

        let e = self
            .expand_main_op()
            .map_args(|a| *a = a.expand_exponential());

        if e.is_exponential() {
            expand_exponential_arg(&e.exponent())
        } else {
            e
        }
    }

    pub fn simplify_trig(&self) -> Self {
        let e = self.substitute_trig().rationalize();

        let n = e.numerator().expand_trig().contract_trig();
        let d = e.denominator().expand_trig().contract_trig();
        (n / d).reduce()
    }

    pub fn substitute_trig(&self) -> Self {
        use Atom as A;
        use Func as F;

        if self.is_atom() {
            return self.clone();
        }

        let e = self.clone().map_args(|a| *a = a.substitute_trig());

        match e.atom() {
            A::Func(F::Tan(x)) => Expr::sin(x) / Expr::cos(x),
            A::Func(F::Cot(x)) => Expr::cos(x) / Expr::sin(x),
            A::Func(F::Sec(x)) => Expr::one() / Expr::cos(x),
            A::Func(F::Csc(x)) => Expr::one() / Expr::sin(x),
            _ => e,
        }
    }

    pub fn expand_trig(&self) -> Self {
        if self.is_atom() {
            return self.clone();
        }

        let e = self.clone().map_args(|a| *a = a.expand_trig());
        if let Ok(f) = e.try_unwrap_func_ref() {
            match f {
                Func::Sin(phi) => expand_trig_arg(&phi).0,
                Func::Cos(phi) => expand_trig_arg(&phi).1,
                _ => e,
            }
        } else {
            e
        }
    }

    pub fn contract_trig(&self) -> Self {
        if self.is_atom() {
            return self.clone();
        }

        let e = self.clone().map_args(|a| *a = a.contract_trig());
        if e.is_prod() || e.is_pow() {
            contract_trig_arg(&e).reduce()
        } else {
            e
        }
    }

    pub fn expand_ln(&self) -> Self {
        if self.is_atom() {
            return self.clone();
        }

        let e = self.clone().map_args(|a| *a = a.expand_ln());
        if let Ok(log) = e.try_unwrap_func_ref() {
            match log {
                Func::Log(Real::E, x) => expand_ln_arg(&x),
                _ => e,
            }
        } else {
            e
        }
    }

    pub fn distribute(&self) -> Self {
        use Atom as A;
        if let A::Prod(prod) = self.atom() {
            prod.distribute_first()
        } else {
            self.clone()
        }
    }

    pub fn rationalize(&self) -> Expr {
        use Atom as A;
        match self.atom() {
            A::Prod(_) => self.clone().map_args(|a| *a = a.rationalize()),
            A::Sum(Sum { args }) => args
                .iter()
                .map(|a| a.rationalize())
                .fold(Expr::zero(), |sum, r| Self::rationalize_add(&sum, &r)),
            A::Pow(pow) => Expr::pow(pow.base().rationalize(), pow.exponent()),
            _ => self.clone(),
        }
    }

    /// a/b + c/d -> (ad + cb) / (bd)
    fn rationalize_add(lhs: &Self, rhs: &Self) -> Expr {
        let a = lhs.numerator();
        let b = lhs.denominator();
        let c = rhs.numerator();
        let d = rhs.denominator();
        let bd = &b * &d;
        if bd.is_one() {
            lhs + rhs
        } else {
            Self::rationalize_add(&(&a * &d), &(&c * &b)) / bd
        }
    }

    /// divide lhs and rhs by their common factor and
    /// return them in the form (fac, (lhs/fac, rhs/fac)
    pub fn factorize_common_terms(lhs: &Expr, rhs: &Self) -> (Expr, (Expr, Expr)) {
        // println!("begin factor: {lhs}, {rhs}");
        use Atom as A;
        if lhs == rhs {
            // println!("end factor: ({lhs}, (1, 1))");
            return (lhs.clone(), (Expr::one(), Expr::one()));
        }
        let res = match (lhs.atom(), rhs.atom()) {
            (e, _) | (_, e) if e.is_one() || e.is_min_one() => {
                (Expr::one(), (lhs.clone(), rhs.clone()))
            }
            //(one, _) | (_, one) if one.is_one() => {
            //    println!("{}, {}", lhs, rhs);
            //    return (lhs.clone(), (Expr::one(), Expr::one()))
            //}
            (A::Rational(r1), A::Rational(r2)) => match r1.int_gcd(r2) {
                Some(gcd) => {
                    let rgcd = Rational::from(gcd);
                    let l = r1.clone() / &rgcd;
                    let r = r2.clone() / &rgcd;
                    (rgcd.into(), (l.into(), r.into()))
                }
                None => (Expr::one(), (lhs.clone(), rhs.clone())),
            },
            //(A::Rational(r1), A::Rational(r2)) if r1.is_int() && r2.is_int() => {
            //    let (i1, i2) = (r1.to_int().unwrap(), r2.to_int().unwrap());
            //    let gcd = i1.gcd(&i2);
            //    let rgcd = Rational::from(gcd);
            //    let l = r1.clone() / &rgcd;
            //    let r = r2.clone() / &rgcd;
            //    (rgcd.into(), (l.into(), r.into()))
            //}
            (A::Prod(prod), _) if !prod.args.is_empty() => {
                if prod.args.len() == 1 {
                    let lhs = prod.args.first().unwrap();
                    // println!("end factor: ({}, ({}, {}))", res.0, res.1.0, res.1.1);
                    return Self::factorize_common_terms(lhs, rhs);
                }
                /*
                                   (a*x) * (b*y), (u*x*y)
                                   => common(a*x, u*x*y) -> (x, (a, u*y))
                                   => common(b*y, u*y) -> (y, (b, u))
                                   => return (x*y, (a*b, u))

                */
                //let mut args = args.clone();
                let uxy = rhs;
                let (ax, by) = prod.as_binary_mul();

                let (x, (a, uy)) = Self::factorize_common_terms(&ax, uxy);
                let (y, (b, u)) = Self::factorize_common_terms(&by, &uy);
                (x * y, (a * b, u))
            }
            (_, A::Prod(_)) => {
                let (fac, (r, l)) = Self::factorize_common_terms(rhs, lhs);
                (fac, (l, r))
            }
            (A::Sum(sum), _) if !sum.args.is_empty() => {
                if sum.args.len() == 1 {
                    let lhs = sum.args.first().unwrap();
                    let res = Self::factorize_common_terms(lhs, rhs);
                    // println!("end factor: ({}, ({}, {}))", res.0, res.1.0, res.1.1);
                    //return Self::factorize_common_terms(lhs, rhs);
                    return res;
                }
                /*
                abxy + cdxy, u*x*y*a*c
                => common(abxy, uacxy) -> (axy, (b, uc))
                => common(cdxy, uacxy) -> (cxy, (d, ua))
                => common(axy, cxy)    -> (xy, (a, c))
                => common(ua, uc)      -> (u, (a, c))
                => return (xy, (ab + cd, uac))
                */
                //let mut args = args.clone();
                let uacxy = rhs;
                let (abxy, cdxy) = sum.as_binary_sum();

                let (axy, (b, _uc)) = Self::factorize_common_terms(&abxy, uacxy);
                let (cxy, (d, ua)) = Self::factorize_common_terms(&cdxy, uacxy);
                let (xy, (a, c)) = Self::factorize_common_terms(&axy, &cxy);

                //let (u, (_a, _c)) = Self::factorize_common_terms(&ua, &uc);
                //println!("begin factor: {lhs}, {rhs}");
                //println!("abxy + cdxy       : {lhs:?}");
                //println!("uacxy             : {uacxy:?}");
                //println!("abxy              : {abxy:?}");
                //println!("cdxy              : {cdxy:?}");
                //println!("(axy, (b, uc))    : ({axy:?}, ({b:?}, {uc:?}))");
                //println!("(cxy, (d, ua))    : ({cxy:?}, ({d:?}, {ua:?}))");
                //println!("(xy, (a, c))      : ({xy:?}, ({a:?}, {c:?}))");
                //println!("(u, (_a, _c))     : ({u:?}, ({_a:?}, {_c:?}))");
                //let res = (xy, (a * b + c * d, u * _a * _c));
                //println!("({}, ({}, {}))", res.0, res.1.0, res.1.1);
                //println!("");
                (xy, (a * b + &c * d, ua * c))
            }
            (_, A::Sum(_)) => {
                let (fac, (r, l)) = Self::factorize_common_terms(rhs, lhs);
                (fac, (l, r))
            }
            (_, _) => match (lhs.exponent().atom(), rhs.exponent().atom()) {
                (A::Rational(r1), A::Rational(r2))
                    if r1.is_pos() && r2.is_pos() && rhs.base() == lhs.base() =>
                {
                    let e = std::cmp::min(r1, r2).clone();
                    let b = rhs.base();
                    (
                        Expr::pow(&b, Expr::from(e.clone())),
                        (
                            Expr::pow(&b, Expr::from(r1.clone() - &e)),
                            Expr::pow(&b, Expr::from(r2.clone() - e)),
                        ),
                    )
                }
                _ => (Expr::one(), (lhs.clone(), rhs.clone())),
            },
        };
        res
    }

    pub fn common_factors(lhs: &Self, rhs: &Self) -> Expr {
        Expr::factorize_common_terms(lhs, rhs).0
    }

    pub fn factor_out(&self) -> Expr {
        use Atom as A;
        match self.atom() {
            A::Prod(Prod { args }) => args
                .iter()
                .map(|a| a.factor_out())
                .fold(Expr::one(), |prod, rhs| prod * rhs),
            A::Pow(pow) => Expr::pow(pow.base().factor_out(), pow.exponent()),
            A::Sum(Sum { args }) => {
                let s = args
                    .iter()
                    .map(|a| a.factor_out())
                    .fold(Expr::zero(), |sum, rhs| sum + rhs);
                //.reduce();
                if let A::Sum(sum) = s.atom() {
                    // sum = a + b
                    let (a, b) = sum.as_binary_sum();
                    let b = b.factor_out();
                    let (f, (a_div_f, b_div_f)) = Expr::factorize_common_terms(&a, &b);
                    f * (a_div_f + b_div_f)
                } else {
                    s
                }
            }
            _ => self.clone(),
        }
    }

    pub fn separate_factors(&self, x: &Self) -> (Expr, Expr) {
        match self.atom() {
            Atom::Prod(prod) => {
                let mut free_part = Expr::one();
                let mut dep_part = Expr::one();

                for a in prod.args() {
                    if a.free_of(x) {
                        free_part *= a;
                    } else {
                        dep_part *= a;
                    }
                }

                (free_part, dep_part)
            }
            _ => {
                if self.free_of(x) {
                    (self.clone(), Expr::one())
                } else {
                    (Expr::one(), self.clone())
                }
            }
        }
    }

    pub fn cancel(&self) -> Expr {
        let n = self.numerator();
        let d = self.denominator();
        let numer = n.factor_out();
        let denom = d.factor_out();
        (numer / denom).reduce()
    }

    pub fn substitude(&self, from: &Expr, to: &Expr) -> Self {
        self.concurr_substitude([(from, to)])
    }

    pub fn seq_substitude<'a, T>(&self, subs: T) -> Self
    where
        T: IntoIterator<Item = (&'a Expr, &'a Expr)>,
    {
        let mut res = self.clone();
        subs.into_iter().for_each(|(from, to)| {
            res.for_each_compl_sub_expr(|sub_expr| {
                if sub_expr == from {
                    *sub_expr = to.clone();
                }
            });
        });
        res
    }

    pub fn concurr_substitude<'a, T>(&self, subs: T) -> Self
    where
        T: IntoIterator<Item = (&'a Expr, &'a Expr)> + Copy,
    {
        let mut res = self.clone();
        res.try_for_each_compl_sub_expr(|sub_expr| {
            for (from, to) in subs {
                if sub_expr == from {
                    *sub_expr = (*to).clone();
                    return ops::ControlFlow::Break(());
                }
            }
            ops::ControlFlow::Continue(())
        });
        res
    }

    pub fn sort_args(&self) -> Self {
        use Atom as A;
        match self.atom() {
            A::Undef | A::Rational(_) | A::Irrational(_) | A::Var(_) => self.clone(),
            A::Sum(sum) => {
                let mut s = sum.clone().map_args(|a| *a = a.sort_args());
                s.args_mut().sort_by(Sum::cmp_args);
                s.into()
            }
            A::Prod(prod) => {
                let mut p = prod.clone().map_args(|a| *a = a.sort_args());
                p.args_mut().sort_by(Prod::cmp_args);
                p.into()
            }
            A::Pow(pow) => {
                let mut p = pow.clone();
                p.args[0] = p.args[0].sort_args();
                p.args[1] = p.args[1].sort_args();
                p.into()
            }
            A::Func(func) => {
                let f = func.clone().map_args(|a| *a = a.sort_args());
                f.into()
            }
        }
    }

    pub fn try_for_each_compl_sub_expr<F>(&mut self, func: F)
    where
        F: Fn(&mut Expr) -> ops::ControlFlow<()> + Copy,
    {
        if func(self).is_break() {
            return;
        }

        self.atom_mut()
            .args_mut()
            .iter_mut()
            .for_each(|a| a.try_for_each_compl_sub_expr(func))
    }

    pub fn for_each_compl_sub_expr<F>(&mut self, func: F)
    where
        F: Fn(&mut Expr) + Copy,
    {
        self.try_for_each_compl_sub_expr(|expr| {
            func(expr);
            ops::ControlFlow::Continue(())
        });
    }
}

/// helper function for [Expr::expand_exponential]
///
/// will expand the argument to a exponential function
fn expand_exponential_arg(a: &Expr) -> Expr {
    let exp_args = expand_exponential_arg;

    match a.atom() {
        Atom::Sum(s) => {
            let args: Vec<_> = s.iter_args().map(|a| exp_args(a)).collect();
            return Prod { args }.into();
        }
        Atom::Prod(p) => {
            let (p1, p2) = p.as_binary_mul();
            if p1.is_int() {
                return Expr::pow_raw(exp_args(&p2), p1);
            }
        }
        _ => (),
    }

    Expr::exp(a)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum TrigTyp {
    Sin,
    Cos,
}

/// expands the expression sin(n*phi) to
///
/// sin(n*phi) = sum(j=1 && odd(j); n) { (-1)^((j-1)/2) * binom(n,j) * cos(phi)^(n-j) * sin(phi)^j  }
fn expand_sin_n_times_phi(n: i128, phi: &Expr) -> Expr {
    _expand_sin_cos_n_times_phi(n, phi, TrigTyp::Sin)
}

/// expands the expression cos(n*phi) to
///
/// cos(n*phi) = sum(j=0 && even(j); n) { (-1)^(j/2) * binom(n,j) * cos(phi)^(n-j) * sin(phi)^j  }
fn expand_cos_n_times_phi(n: i128, phi: &Expr) -> Expr {
    _expand_sin_cos_n_times_phi(n, phi, TrigTyp::Cos)
}

/// expands the expression sin(n*phi) or cos(n*phi)
///
/// helper functions for [expand_sin_n_times_phi] and [expand_cos_n_times_phi]
fn _expand_sin_cos_n_times_phi(n: i128, phi: &Expr, typ: TrigTyp) -> Expr {
    let mut min_one = false;

    // TODO expand here?
    let (sin, cos) = expand_trig_arg(phi);

    let mut sum = Sum::zero();
    let ne = Expr::from(n.clone());

    for i in 0..=n {
        if typ == TrigTyp::Cos && (i % 2 != 0) || typ == TrigTyp::Sin && (i % 2 == 0) {
            continue;
        }

        min_one = !min_one;

        let sign = match min_one {
            false => Expr::min_one(),
            true => Expr::one(),
        };

        let rhs = if i == 0 {
            // 1 * cos(phi)^n
            Expr::pow(&cos, &ne)
        } else if i == n {
            // 1 * sin(phi)^n
            Expr::pow(&sin, &ne)
        } else {
            // binom(n, i) * sin(phi)^i * cos(phi)^(n-i)
            let b = Expr::from(binomial_coeff(n, i));
            b * Expr::pow(&cos, Expr::from(n - &i)) * Expr::pow(&sin, Expr::from(i))
        };

        sum.add_rhs(&(sign * rhs));
    }

    sum.into()
}

/// return the expanded trigonometric form of sin(x), cos(x)
///
/// helper function for [Expr::expand_trig]
fn expand_trig_arg(a: &Expr) -> (Expr, Expr) {
    let a = a.expand_main_op();
    match a.atom() {
        Atom::Sum(sum) => {
            let sum = sum.as_binary_sum();
            let (f, r) = (expand_trig_arg(&sum.0), expand_trig_arg(&sum.1));
            let s = &f.0 * &r.1 + &f.1 * &r.0;
            let c = f.1 * r.1 - f.0 * r.0;
            return (s, c);
        }
        Atom::Prod(prod) => {
            let prod = prod.as_binary_mul();
            if let Some(n) = prod.0.try_unwrap_int() {
                let n_abs = n.abs();
                let mut s = expand_sin_n_times_phi(n_abs, &prod.1);
                let c = expand_cos_n_times_phi(n_abs, &prod.1);

                if n < 0 {
                    s = Expr::min_one() * s;
                }
                return (s, c);
            }
        }
        _ => (),
    }

    (Expr::sin(&a), Expr::cos(a))
}

fn contract_trig_arg(e: &Expr) -> Expr {
    let e = e.expand_main_op();
    match e.atom() {
        Atom::Pow(_) => contract_trig_pow(&e),
        Atom::Prod(_) => {
            let (r, s) = separate_sin_cos(&e);

            if s.is_one()
                || s.try_unwrap_func_ref()
                    .is_ok_and(|f| f.is_sin() || f.is_cos())
            {
                e
            } else if let Atom::Pow(_) = s.atom() {
                (r * contract_trig_pow(&s)).expand_main_op()
            } else if let Atom::Prod(p) = s.atom() {
                (r * contract_trig_prod(p.args())).expand_main_op()
            } else {
                unreachable!()
            }
        }

        Atom::Sum(s) => s.iter_args().fold(Expr::one(), |lhs, rhs| {
            if rhs.is_prod() || rhs.is_pow() {
                lhs + contract_trig_arg(rhs)
            } else {
                lhs + rhs
            }
        }),
        _ => e.clone(),
    }
}

fn contract_trig_pow(p: &Expr) -> Expr {
    use num::Integer;

    if !p.is_pow() {
        return p.clone();
    }

    let f = p.base();
    let n = p.exponent();

    if !(n.is_int() && n.is_pos()) && !(f.is_sin() || f.is_cos()) {
        return p.clone();
    }

    let func = || {
        let en = n;
        let n = en.unwrap_rational_ref().to_int().unwrap();
        let n_min_one = n - 1;
        let f = f.unwrap_func_ref();

        let half_n = n / 2;

        let binom = |l: i128, r: i128| Expr::from(binomial_coeff(l, r));
        let min_one_pow = |n: &i128| match n % 2 == 0 {
            true => Expr::one(),
            false => Expr::min_one(),
        };

        match f {
            Func::Sin(x) => {
                if n % 2 == 0 {
                    let a = Rational::from(binomial_coeff(n, half_n))
                        / Rational::from(2i128.checked_pow(n.try_into().ok()?)?);
                    let mut b = match half_n.is_even() {
                        true => Rational::ONE,
                        false => Rational::MINUS_ONE,
                    };
                    b /= Rational::from(2i128.checked_pow(n_min_one.try_into().ok()?)?);

                    let mut sum = Expr::zero();
                    for j in 0..=half_n - 1 {
                        let mut term = min_one_pow(&j);
                        term *= binom(n, j);
                        term *= Expr::cos((&en - Expr::from(2 * j)) * x);
                        sum += term;
                    }

                    Some(Expr::from(a) + Expr::from(b) * sum)
                } else {
                    let mut b = match (n_min_one.clone() / 2).is_even() {
                        true => Rational::ONE,
                        false => Rational::MINUS_ONE,
                    };
                    b /= Rational::from(2i128.checked_pow(n_min_one.try_into().ok()?)?);
                    let mut sum = Expr::zero();
                    for j in 0..=half_n {
                        let mut term = min_one_pow(&j);
                        term *= binom(n, j);
                        term *= Expr::sin((&en - Expr::from(2 * j)) * x);
                        sum += term;
                    }
                    Some(Expr::from(b) * sum)
                }
            }
            Func::Cos(x) => {
                if n.is_even() {
                    let mut a = Rational::from(binomial_coeff(n, half_n));
                    a /= Rational::from(2i128.checked_pow(n.try_into().ok()?)?);

                    let b = Rational::ONE
                        / Rational::from(2i128.checked_pow(n_min_one.try_into().ok()?)?);

                    let mut sum = Expr::zero();
                    for j in 0..=half_n - 1 {
                        let mut term = binom(n, j);
                        term *= Expr::cos((&en - Expr::from(2 * j)) * x);
                        sum += term
                    }
                    Some(Expr::from(a) + Expr::from(b) * sum)
                } else {
                    let b = Rational::ONE
                        / Rational::from(2i128.checked_pow(n_min_one.try_into().ok()?)?);
                    let mut sum = Expr::zero();
                    for j in 0..=half_n {
                        let mut term = binom(n, j);
                        term *= Expr::cos((&en - Expr::from(2 * j)) * x);
                        sum += term
                    }
                    Some(Expr::from(b) * sum)
                }
            }
            _ => unreachable!(),
        }
    };
    func().unwrap_or(p.clone())
}
fn contract_trig_prod(p: &[Expr]) -> Expr {
    if p.is_empty() {
        return Expr::one();
    } else if p.len() == 1 {
        return contract_trig_arg(&p[0]);
    }
    if p.len() == 2 {
        let a = &p[0];
        let b = &p[1];

        if let Atom::Pow(_) = a.atom() {
            let a = contract_trig_pow(a);
            return contract_trig_arg(&(a * b));
        } else if let Atom::Pow(_) = b.atom() {
            let b = contract_trig_pow(b);
            return contract_trig_arg(&(a * b));
        } else {
            assert!(a.is_sin() || a.is_cos());
            assert!(b.is_sin() || b.is_cos());

            let theta = &a.args()[0];
            let phi = &b.args()[0];
            let two = Expr::two();

            let cos = |e: Expr| Expr::cos(e);
            let sin = |e: Expr| Expr::sin(e);

            if a.is_sin() && b.is_sin() {
                return cos(theta - phi) / &two - cos(theta + phi) / &two;
            } else if a.is_cos() && b.is_cos() {
                return cos(theta + phi) / &two + cos(theta - phi) / &two;
            } else if a.is_sin() && b.is_cos() {
                return sin(theta + phi) / &two + sin(theta - phi) / &two;
            } else if a.is_cos() && b.is_sin() {
                return sin(theta + phi) / &two + sin(phi - theta) / &two;
            } else {
                unreachable!()
            }
        }
    } else {
        let a = &p[0];
        let b = contract_trig_prod(&p[1..]);
        return contract_trig_arg(&(a * b));
    }
}

fn is_pos_sin_cos_int_pow(e: &Expr) -> bool {
    match e.atom() {
        Atom::Pow(p)
            if (p.base().is_sin() || p.base().is_cos())
                && p.exponent().is_rational_and(|r| r.is_int() && r.is_pos()) =>
        {
            true
        }
        Atom::Func(f) if f.is_sin() || f.is_cos() => true,
        _ => false,
    }
}

pub fn separate_sin_cos(e: &Expr) -> (Expr, Expr) {
    if is_pos_sin_cos_int_pow(e) {
        return (Expr::one(), e.clone());
    }
    match e.atom() {
        Atom::Prod(p) => p
            .iter_args()
            .map(|a| {
                if is_pos_sin_cos_int_pow(a) {
                    Ok(a)
                } else {
                    Err(a)
                }
            })
            .fold((Expr::one(), Expr::one()), |(ll, lr), rhs| match rhs {
                Ok(rhs) => (ll, lr * rhs),
                Err(rhs) => (ll * rhs, lr),
            }),
        _ => (e.clone(), Expr::one()),
    }
}

fn expand_ln_arg(e: &Expr) -> Expr {
    let e = e.expand_main_op();
    match e.atom() {
        Atom::Prod(prod) if prod.args.len() >= 2 => {
            let prod = prod.as_binary_mul();
            expand_ln_arg(&prod.0) + expand_ln_arg(&prod.1)
        }
        Atom::Pow(pow) => pow.exponent() * expand_ln_arg(pow.base()),
        _ => Expr::ln(e),
    }
}

impl<T: Borrow<Expr>> ops::Add<T> for &Expr {
    type Output = Expr;
    fn add(self, rhs: T) -> Self::Output {
        Expr::add(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::Add<T> for Expr {
    type Output = Expr;
    fn add(self, rhs: T) -> Self::Output {
        Expr::add(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::AddAssign<T> for Expr {
    fn add_assign(&mut self, rhs: T) {
        *self = &*self + rhs;
    }
}
impl<T: Borrow<Expr>> ops::Sub<T> for &Expr {
    type Output = Expr;
    fn sub(self, rhs: T) -> Self::Output {
        Expr::sub(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::Sub<T> for Expr {
    type Output = Expr;
    fn sub(self, rhs: T) -> Self::Output {
        Expr::sub(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::SubAssign<T> for Expr {
    fn sub_assign(&mut self, rhs: T) {
        *self = &*self - rhs;
    }
}
impl<T: Borrow<Expr>> ops::Mul<T> for &Expr {
    type Output = Expr;
    fn mul(self, rhs: T) -> Self::Output {
        Expr::mul(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::Mul<T> for Expr {
    type Output = Expr;
    fn mul(self, rhs: T) -> Self::Output {
        Expr::mul(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::MulAssign<T> for Expr {
    fn mul_assign(&mut self, rhs: T) {
        *self = &*self * rhs;
    }
}
impl<T: Borrow<Expr>> ops::Div<T> for &Expr {
    type Output = Expr;
    fn div(self, rhs: T) -> Self::Output {
        Expr::div(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::Div<T> for Expr {
    type Output = Expr;
    fn div(self, rhs: T) -> Self::Output {
        Expr::div(self, rhs)
    }
}
impl<T: Borrow<Expr>> ops::DivAssign<T> for Expr {
    fn div_assign(&mut self, rhs: T) {
        *self = &*self / rhs;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_eq as eq;
    use calcurs_macros::expr as e;

    #[test]
    fn expand() {
        eq!(
            e!(x * (2 + (1 + x) ^ 2)).expand_main_op().sort_args(),
            e!(2*x + (x+1)^2 * x)
        );
        eq!(
            e!((x + (1 + x) ^ 2) ^ 2).expand_main_op().sort_args(),
            e!(x ^ 2 + 2 * x * (1 + x) ^ 2 + (1 + x) ^ 4).sort_args()
        );
        eq!(
            e!((x + 2) * (x + 3) * (x + 4))
                .expand()
                .as_polynomial_view(&[e!(x)].into())
                .collect_terms()
                .unwrap()
                .sort_args(),
            e!(x ^ 3 + 9 * x ^ 2 + 26 * x + 24).sort_args()
        );
        eq!(
            e!((x + 1) ^ 2 + (y + 1) ^ 2).expand().reduce(),
            e!(y^2 + 2*y + x^2 + 2*x + 2),
        );
        eq!(
            e!(((x + 2) ^ 2 + 3) ^ 2).expand().reduce(),
            e!(x ^ 4 + 8 * x ^ 3 + 30 * x ^ 2 + 56 * x + 49).reduce()
        );
    }

    #[test]
    fn distributive() {
        eq!(
            e!(a * (b + c) * (d + e)).distribute(),
            e!(a * b * (d + e) + a * c * (d + e))
        );
        eq!(
            e!((x + y) / (x * y)).distribute(),
            e!(x / (x * y) + y / (x * y))
        );
    }

    #[test]
    fn rationalize() {
        eq!(e!((1 + 1 / x) ^ 2).rationalize(), e!(((x + 1) / x) ^ 2));
        eq!(
            e!((1 + 1 / x) ^ (1 / 2)).rationalize(),
            e!(((x + 1) / x) ^ (1 / 2))
        );
    }

    #[test]
    fn common_factors() {
        eq!(
            Expr::factorize_common_terms(&e!(6 * x * y ^ 3), &e!(2 * x ^ 2 * y * z)),
            (e!(2 * x * y), (e!(3 * y ^ 2), e!(x * z)))
        );
        eq!(
            Expr::factorize_common_terms(&e!(a * (x + y)), &e!(x + y)),
            (e!(x + y), (e!(a), e!(1)))
        );
        eq!(
            Expr::factorize_common_terms(&e!(x - y), &e!(a * (x - y))),
            (e!(x - y), (e!(1), e!(a)))
        );
    }

    #[test]
    fn factor_out() {
        eq!(
            e!((x ^ 2 + x * y) ^ 3).factor_out().expand_main_op(),
            e!(x ^ 3 * (x + y) ^ 3)
        );
        eq!(e!(a * (b + b * x)).factor_out(), e!(a * b * (1 + x)));
        eq!(
            e!(2 ^ (1 / 2) + 2).factor_out(),
            e!(2 ^ (1 / 2) * (1 + 2 ^ (1 / 2)))
        );
        eq!(
            e!(a * b * x + a * c * x + b * c * x).factor_out(),
            e!(x * (a * b + c * (a + b))),
        );
        eq!(e!(a / x + b / x), e!(a / x + b / x))
    }

    #[test]
    fn separate_factors() {
        eq!(
            e!(c * x * sin(x) / 2).separate_factors(&e!(x)),
            (e!(c / 2), e!(x * sin(x)))
        );
    }

    #[test]
    fn reduce() {
        let checks = vec![
            (e!(2 * x), e!(2 * x)),
            (e!(1 + 2), e!(3)),
            (e!(a + undef), e!(undef)),
            (e!(a + (b + c)), e!(a + (b + c))),
            (e!(0 - 2 * b), e!((2 - 4) * b)),
            (e!(a + 0), e!(a)),
            (e!(0 + a), e!(a)),
            (e!(1 + 2), e!(3)),
            (e!(x + 0), e!(x)),
            (e!(0 + x), e!(x)),
            (e!(0 - x), e!((4 - 5) * x)),
            (e!(x - 0), e!(x)),
            (e!(3 - 2), e!(1)),
            (e!(x * 0), e!(0)),
            (e!(0 * x), e!(0)),
            (e!(x * 1), e!(x)),
            (e!(1 * x), e!(x)),
            (e!(0 ^ 0), e!(undef)),
            (e!(0 ^ 1), e!(0)),
            (e!(0 ^ 314), e!(0)),
            (e!(1 ^ 0), e!(1)),
            (e!(314 ^ 0), e!(1)),
            (e!(314 ^ 1), e!(314)),
            (e!(x ^ 1), e!(x)),
            (e!(1 ^ x), e!(1)),
            (e!(1 ^ 314), e!(1)),
            (e!(3 ^ 3), e!(27)),
            (e!(a - b), e!(a + ((2 - 3) * b))),
            (e!(a / b), e!(a * b ^ (2 - 3))),
            (e!((x ^ (1 / 2) ^ (1 / 2)) ^ 8), e!(x ^ 2)),
            (e!(x + x), e!(2 * x)),
            (e!(2 * x + y + x), e!(3 * x + y)),
            (e!(sin(0)), e!(0)),
            (e!(sin(-x)), e!(-1 * sin(x))),
            (e!(cos(-x)), e!(cos(x))),
            (e!(x * y / (y * x)), e!(1)),
            (Expr::ln(Expr::e()), e!(1)),
        ];
        for (calc, res) in checks {
            eq!(calc.reduce(), res.sort_args());
        }
    }

    #[test]
    fn expand_trig() {
        eq!(
            e!((sin(2 * x) - 2 * sin(x) * cos(x)))
                .expand_trig()
                .reduce(),
            e!(0)
        );
        eq!(
            e!(sin(a + b)).expand_trig(),
            e!(sin(a) * cos(b) + cos(a) * sin(b))
        );
        eq!(
            e!(cos(a + b)).expand_trig(),
            e!(cos(a) * cos(b) - sin(a) * sin(b))
        );
        eq!(
            e!(sin(2 * (x + y))).expand_trig().reduce(),
            e!(2 * cos(x) * sin(x) * (cos(y) ^ 2 - sin(y) ^ 2)
                + (cos(x) ^ 2 - sin(x) ^ 2) * 2 * cos(y) * sin(y))
            .reduce()
        );
        eq!(
            e!(sin((x + y) ^ 2)).expand_trig(),
            e!(sin(x ^ 2)
                * ((cos(x * y) ^ 2 - sin(x * y) ^ 2) * cos(y ^ 2)
                    - 2 * cos(x * y) * sin(x * y) * sin(y ^ 2))
                + cos(x ^ 2)
                    * (2 * cos(x * y) * sin(x * y) * cos(y ^ 2)
                        + (cos(x * y) ^ 2 - sin(x * y) ^ 2) * sin(y ^ 2))),
        )
    }

    #[test]
    fn expand_exponential() {
        let pow = |b, e| Expr::pow_raw(b, e);
        let mul = |b, e| Expr::mul_raw(b, e);
        eq!(
            e!(exp(2 * w * x + 3 * y * z)).expand_exponential(),
            pow(e!(exp(w * x)), e!(2)) * pow(e!(exp(y * z)), e!(3))
        );
        eq!(
            e!(exp(2 * (x + y))).expand_exponential(),
            pow(e!(exp(x)), e!(2)) * pow(e!(exp(y)), e!(2))
        );
        eq!(
            e!(exp((x + y) ^ 2)).expand_exponential(),
            mul(
                e!(exp(x ^ 2)),
                mul(pow(e!(exp(x * y)), e!(2)), e!(exp(y ^ 2)))
            )
        )
    }

    #[test]
    fn expand_ln() {
        eq!(
            e!(ln((w * x) ^ a) + ln(y ^ b * z)).expand_ln().reduce(),
            e!(a * ln(w) + a * ln(x) + b * ln(y) + ln(z)).reduce()
        )
    }

    #[test]
    fn cancel() {
        eq!(
            e!(((a + b) * c + (a + b) * d) / (a * e + b * e)).cancel(),
            e!((c + d) / e)
        );

        eq!(
            e!((a * (a + b) - a ^ 2 - a * b + r * s + r * t) / r ^ 2)
                .expand()
                .cancel(),
            e!((s + t) / r)
        );
    }

    #[test]
    fn sort_args() {
        let checks = vec![
            e!(a + b),
            e!(b * c + a),
            e!(sin(x) * cos(x)),
            e!(a * x ^ 2 + b * x + c + 3),
        ];

        for c in checks {
            eq!(c.sort_args(), c)
        }
    }
}
