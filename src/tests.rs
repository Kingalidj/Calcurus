#[cfg(hidden)]
mod test_display {
    use calcu_rs::calc;
    use calcu_rs::prelude::*;
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    macro_rules! c {
        ($($x: tt)*) => {
            calc!($($x)*)
        }
    }

    #[test_case(c!(x^(-1)), "1/x")]
    #[test_case(c!(34/3), "34/3")]
    #[test_case(c!(x^(-3)), "x^(-3)")]
    #[test_case(c!(x^2), "x^2")]
    #[test_case(c!(x+x), "2x")]
    #[test_case(c!(1^2), "1")]
    #[test_case(c!((1/2)^2), "1/4")]
    #[test_case(c!((1/3)^(1/100)), "(1/3)^(1/100)")]
    #[test_case(c!((10^15) + 1/1000), "1000000000000000001 e-3")]
    #[test_case(c!((1/3)^(2/1000)), "(1/3)^(1/500)")]
    fn disp_fractions(exp: Expr, res: &str) {
        let fmt = format!("{}", exp);
        assert_eq!(fmt, res);
    }
}
mod test_rational {
    use calcu_rs::prelude::*;
    use pretty_assertions::assert_eq;

    macro_rules! r {
        ($v: literal) => {
            Rational::from($v)
        };

        ($numer: literal / $denom: literal) => {
            Rational::from(($numer as i64, $denom as i64))
        };
    }

    #[test]
    fn exprs() {
        assert_eq!(r!(1) + r!(1), r!(2));
        assert_eq!(r!(1 / 3) + r!(2 / 3), r!(1));
        assert_eq!(r!(1 / 3) - r!(2 / 3), r!(-1 / 3));
        assert_eq!(r!(1 / -3) * r!(3), r!(-1));
        assert!(r!(2) > r!(1));
        assert!(r!(2) >= r!(2));
        assert!(r!(2 / 4) <= r!(4 / 8));
        assert!(r!(5 / 128) > r!(11 / 2516));
    }
}
#[cfg(hide)]
mod test_derivative {
    use calcu_rs::calc;
    use calcu_rs::prelude::*;
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    macro_rules! c {
        ($($x: tt)*) => {
            calc!($($x)*)
        }
    }

    #[test_case(1, c!((x^2) + x*3), c!(2*x + 3))]
    #[test_case(2, c!(1/3 + 3/5),   c!(0))]
    #[test_case(3, c!(x+y),         c!(1))]
    fn sum_rule(_case: u32, f: Expr, df: Expr) {
        assert_eq!(f.derive("x"), df);
    }

    #[test_case(c!((x^2)*y), c!(2*x*y); "1")]
    fn product_rule(f: Expr, df: Expr) {
        assert_eq!(f.derive("x"), df);
    }

    #[test_case(c!(x).derive("x"), c!(1))]
    #[test_case(c!(y).derive("x"), c!(0))]
    #[test_case(c!(x*x).derive("x"), c!(2*x))]
    #[test_case(c!((x^2 - x) / (2 * x)).derive("x"), c!(1 / 2))]
    fn derive(expr: Expr, result: Expr) {
        assert_eq!(expr, result);
    }
}
mod test_operators {
    use calcu_rs::calc;
    use calcu_rs::prelude::*;
    use test_case::test_case;

    macro_rules! c {
        ($($t:tt)*) => {
            calc!($($t)*)
        }
    }

    #[test_case(c!(2 + 3),      c!(5),      "1")]
    #[test_case(c!(1/2 + 1/2),  c!(1),      "2")]
    #[test_case(c!(x + x),      c!(x * 2),  "3")]
    #[test_case(c!(-3 + 1 / 2), c!(-5 / 2), "4")]
    //#[test_case(c!(oo + 4),     c!(oo),     "5")]
    //#[test_case(c!(-oo + 4),    c!(-oo),    "6")]
    //#[test_case(c!(oo + oo),    c!(oo),     "7")]
    //#[test_case(c!(-oo + oo),   c!(undef),  "8")]
    //#[test_case(c!(undef + oo), c!(undef),  "9")]
    #[test_case(c!(4/2 + 0),    c!(2),      "10")]
    fn add(add: Expr, sol: Expr, n: &'static str) {
        let add = add.simplify();
        let sol = sol.simplify();
        assert_eq!(add, sol, "{}: [ {} ] != [ {} ]", n, add, sol);
    }

    #[test_case(c!(-1 - 3),        c!(-4),     "1")]
    #[test_case(c!(-3 - 1 / 2),    c!(-7 / 2), "2")]
    #[test_case(c!(1 / 2 - 1 / 2), c!(0),      "3")]
    //#[test_case(c!(oo - 4),        c!(oo),     "4")]
    //#[test_case(c!(-oo - 4 / 2),   c!(-oo),    "5")]
    //#[test_case(c!(oo - 4),        c!(oo),     "6")]
    //#[test_case(c!(oo - oo),       c!(undef),  "7")]
    //#[test_case(c!(-oo - oo),      c!(-oo),    "8")]
    //#[test_case(c!(undef - oo),    c!(undef),  "9")]
    fn sub(sub: Expr, sol: Expr, n: &'static str) {
        let sub = sub.simplify();
        let sol = sol.simplify();
        assert_eq!(sub, sol, "{}: [ {} ] != [ {} ]", n, sub, sol);
    }

    #[test_case(c!(-1*3),         c!(-3),     "1")]
    #[test_case(c!(-1*0),         c!(0),      "2")]
    #[test_case(c!(-1*3) * c!(0), c!(0),      "3")]
    #[test_case(c!(-3*1 / 2),     c!(-3 / 2), "4")]
    #[test_case(c!(1 / 2*1 / 2),  c!(1 / 4),  "5")]
    //#[test_case(c!(oo*4),         c!(oo),     "6")]
    //#[test_case(c!(-oo * 4/2),    c!(-oo),    "7")]
    //#[test_case(c!(oo*4),         c!(oo),     "8")]
    //#[test_case(c!(oo*-1),        c!(-oo),    "9")]
    //#[test_case(c!(oo*oo),       c!(oo),      "10")]
    //#[test_case(c!(-oo*oo),      c!(-oo),     "11")]
    //#[test_case(c!(undef*oo),    c!(undef),   "12")]
    fn mul(mul: Expr, sol: Expr, n: &'static str) {
        let mul = mul.simplify();
        let sol = sol.simplify();
        assert_eq!(mul, sol, "{}: [ {} ] != [ {} ]", n, mul, sol);
    }

    #[test_case(c!(0/0), c!(undef), "1")]
    #[test_case(c!(0/5), c!(0),     "2")]
    #[test_case(c!(5/0), c!(undef), "3")]
    #[test_case(c!(5/5), c!(1),     "4")]
    #[test_case(c!(1/3), c!(1/3),   "5")]
    #[test_case(c!(x/x), c!(1),     "6")]
    #[test_case(c!((x*x + x) / x), c!(x + 1), "7")]
    #[test_case(c!((x*x + x) / (1 / x)), c!(x*x*x + x*x), "8")]
    fn div(div: Expr, sol: Expr, n: &'static str) {
        let div = div.simplify();
        let sol = sol.simplify();
        assert_eq!(div, sol, "{}: [ {} ] != [ {} ]", n, div, sol);
    }

    #[test_case(c!(1^(1/100)),  c!(1),     "1")]
    #[test_case(c!(4^1),        c!(4),     "2")]
    #[test_case(c!(0^0),        c!(undef), "3")]
    #[test_case(c!(0^(-3/4)),   c!(undef), "4")]
    #[test_case(c!(0^(3/4)),    c!(0),     "5")]
    #[test_case(c!((1/2)^(-1)), c!(4/2),   "6")]
    #[test_case(c!((x^2)^3),    c!(x^6),   "7")]
    #[test_case(c!((x+y)^2),    c!(x^2 + 2*x*y + y^2),   "8")]
    fn pow(pow: Expr, sol: Expr, n: &'static str) {
        let pow = pow.simplify();
        let sol = sol.simplify();
        assert_eq!(pow, sol, "{}: [ {} ] != [ {} ]", n, pow, sol);
    }

    #[test_case(c!(x*x*2 + 3*x + 4/3), c!(4/3 + (x^2) * 2 + 3*x), "1")]
    fn polynom(p1: Expr, p2: Expr, n: &'static str) {
        let p1 = p1.simplify();
        let p2 = p2.simplify();
        assert_eq!(p1, p2, "{}: [ {} ] != [ {} ]", n, p1, p2);
    }
}
