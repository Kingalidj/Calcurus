//! simple symbolic algebra system in rust

use calcrs_internals::Inherited;
use core::fmt::Debug;
use std::ops;

use calcrs_macros::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub struct Base {
    is_number: bool,
    is_atom: bool,
    is_symbol: bool,
    is_function: bool,
    is_add: bool,
    is_mul: bool,
    is_pow: bool,
    is_float: bool,
    is_rational: bool,
    is_integer: bool,
    is_numbersymbol: bool,
    is_order: bool,
    is_derivative: bool,
    is_piecewise: bool,
    is_poly: bool,
    is_algebraicnumber: bool,
    is_relational: bool,
    is_equality: bool,
    is_boolean: bool,
    is_not: bool,
    is_matrix: bool,
    is_vector: bool,
    is_point: bool,
    is_matadd: bool,
    is_matmul: Option<bool>,
    is_real: Option<bool>,
    is_zero: Option<bool>,
    is_negative: Option<bool>,
    is_commutative: Option<bool>,
}

impl Base {
    pub const fn new() -> Self {
        Base {
            is_number: false,
            is_atom: false,
            is_symbol: false,
            is_function: false,
            is_add: false,
            is_mul: false,
            is_pow: false,
            is_float: false,
            is_rational: false,
            is_integer: false,
            is_numbersymbol: false,
            is_order: false,
            is_derivative: false,
            is_piecewise: false,
            is_poly: false,
            is_algebraicnumber: false,
            is_relational: false,
            is_equality: false,
            is_boolean: false,
            is_not: false,
            is_matrix: false,
            is_vector: false,
            is_point: false,
            is_matadd: false,
            is_matmul: None,
            is_real: None,
            is_zero: None,
            is_negative: None,
            is_commutative: None,
        }
    }
}

impl Debug for Base {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Base {{...}}")
    }
}

macro_rules! base {
        ($($field:ident = $value:expr),* $(,)?) => {
            Base {
                $(
                    $field: $value,
                )*
            ..Base::new()
            }
        }
    }

#[derive(Debug, Clone, PartialEq)]
pub enum CalcrsType {
    BooleanAtom(BooleanAtom),
    BooleanTrue(BooleanTrue),
    BooleanFalse(BooleanFalse),
    And(And),
}

macro_rules! map_calcrs_type {
    ($e: expr, $func: expr) => {{
        use CalcrsType::*;
        match $e {
            BooleanAtom(ref x) => $func(x),
            BooleanTrue(ref x) => $func(x),
            BooleanFalse(ref x) => $func(x),
            And(ref x) => $func(x),
        }
    }};
}

impl Inherited<Base> for CalcrsType {
    fn base(&self) -> &Base {
        use CalcrsType::*;

        match self {
            BooleanAtom(x) => x.base(),
            BooleanTrue(x) => x.base(),
            BooleanFalse(x) => x.base(),
            And(x) => x.base(),
        }
    }
}

pub type Eval = CalcrsType;

/// this is only used, because innert attribute macros are unstable
#[init_calcrs_macro_scope]
mod __ {

    pub trait Basic: Debug + Clone + PartialEq + Into<CalcrsType> + Inherited<Base> {
        fn eval(&self) -> CalcrsType {
            self.clone().into()
        }
    }
    pub trait Boolean: Basic {}
    pub trait Application: Basic {}
    pub trait BooleanFunc: Boolean + Application {}

    #[calcrs_type]
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub struct BooleanAtom {}
    impl Basic for BooleanAtom {}
    impl Boolean for BooleanAtom {}

    impl From<BooleanAtom> for CalcrsType {
        fn from(value: BooleanAtom) -> Self {
            CalcrsType::BooleanAtom(value)
        }
    }

    #[calcrs_type]
    #[derive(Debug, Clone, Default, Copy, PartialEq)]
    pub struct BooleanTrue {}
    impl Basic for BooleanTrue {}
    impl Boolean for BooleanTrue {}

    impl BooleanTrue {
        pub const fn new() -> Self {
            let base = base!(is_boolean = true, is_atom = true, is_negative = Some(false));
            BooleanTrue { base }
        }
    }

    impl From<BooleanTrue> for CalcrsType {
        fn from(value: BooleanTrue) -> Self {
            CalcrsType::BooleanTrue(value)
        }
    }

    #[calcrs_type]
    #[derive(Debug, Clone, Default, Copy, PartialEq)]
    pub struct BooleanFalse {}
    impl Basic for BooleanFalse {}
    impl Boolean for BooleanFalse {}

    impl BooleanFalse {
        pub const fn new() -> Self {
            let base = base!(is_boolean = true, is_atom = true, is_negative = Some(true));
            BooleanFalse { base }
        }
    }

    impl From<BooleanFalse> for CalcrsType {
        fn from(value: BooleanFalse) -> Self {
            CalcrsType::BooleanFalse(value)
        }
    }

    #[calcrs_type]
    #[derive(Debug, Clone, PartialEq)]
    pub struct And {
        left: Box<CalcrsType>,
        right: Box<CalcrsType>,
    }

    impl And {
        pub fn new(lhs: impl Boolean, rhs: impl Boolean) -> Self {
            And {
                base: Default::default(),
                left: Box::new(lhs.into()),
                right: Box::new(rhs.into()),
            }
        }
    }

    impl Basic for And {
        fn eval(&self) -> CalcrsType {
            let lhs = map_calcrs_type!(*self.left, Basic::eval);
            let rhs = map_calcrs_type!(*self.right, Basic::eval);

            if lhs.base().is_negative.is_none() || rhs.base().is_negative.is_none() {
                return self.clone().into();
            }

            let lhs = lhs.base().is_negative.unwrap();
            let rhs = rhs.base().is_negative.unwrap();

            match lhs && rhs {
                true => True.into(),
                false => False.into(),
            }
        }
    }
    impl Boolean for And {}
    impl Application for And {}
    impl BooleanFunc for And {}

    impl From<And> for CalcrsType {
        fn from(value: And) -> Self {
            CalcrsType::And(value)
        }
    }
}

impl<U: Boolean> ops::BitAnd<U> for BooleanTrue {
    type Output = And;

    fn bitand(self, rhs: U) -> Self::Output {
        And::new(self, rhs)
    }
}

impl<U: Boolean> ops::BitAnd<U> for BooleanFalse {
    type Output = And;

    fn bitand(self, rhs: U) -> Self::Output {
        And::new(self, rhs)
    }
}

impl<U: Boolean> ops::BitAnd<U> for And {
    type Output = And;

    fn bitand(self, rhs: U) -> Self::Output {
        And::new(self, rhs)
    }
}

#[allow(non_upper_case_globals)]
pub static False: BooleanFalse = BooleanFalse::new();

#[allow(non_upper_case_globals)]
pub static True: BooleanTrue = BooleanTrue::new();

#[cfg(test)]
mod test {

    use crate::*;

    #[test]
    fn boolean() {
        assert_eq!(And::new(True, False), True & False);
        assert_eq!(CalcrsType::from(False), (True & False).eval());
    }
}
