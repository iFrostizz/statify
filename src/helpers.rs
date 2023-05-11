use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Add, Range, Sub},
};
use z3::{ast::Ast, Context};

pub type Word = [u8; 32];

// calldata inners behaviour is actually very similar to memory
#[derive(Default)]
pub struct Calldata {
    data: Vec<u8>,
}

#[derive(Debug)]
pub enum RevertReason {
    StackUnderflow,
    StackOverflow,
    /// An unsatisfied solve
    Unsat,
    /// Unknown solve status
    Unknown,
}

/// ret a word with 1 if eq, else an empty word
pub fn bool_to_bv<'ctx>(ctx: &'ctx Context, bool: &z3::ast::Bool<'ctx>) -> z3::ast::BV<'ctx> {
    let zero = z3::ast::BV::from_u64(ctx, 0, 256);
    let one = z3::ast::BV::from_u64(ctx, 1, 256);
    bool.ite(&one, &zero)
}

pub fn is_zero<'ctx>(ctx: &'ctx Context, bv: &z3::ast::BV<'ctx>) -> z3::ast::BV<'ctx> {
    let zero = z3::ast::BV::from_u64(ctx, 0, 256);
    let one = z3::ast::BV::from_u64(ctx, 1, 256);

    bv._eq(&zero).ite(&one, &zero)
}

impl Calldata {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, r: Range<usize>) -> Vec<u8> {
        r.into_iter()
            .map(|o| *self.data.get(o).unwrap_or(&0u8)) // 0 if out of bounds
            .collect()
    }
}

#[derive(Default)]
pub struct EVMCalldata {
    calldata: Calldata,
}

impl EVMCalldata {
    pub fn new() -> Self {
        Self {
            calldata: Calldata::new(),
        }
    }

    pub fn from(data: Vec<u8>) -> Self {
        Self {
            calldata: Calldata { data },
        }
    }

    pub fn load(&self, offset: U256) -> Word {
        let off: usize = offset.into();
        let mut ret = [0; 32];
        let mem = self.calldata.get(off..(off + 32));
        ret.copy_from_slice(&mem);
        ret
    }
}

pub type Address = [u8; 20];
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct U256([u8; 32]);

impl U256 {
    pub fn new(arr: [u8; 32]) -> Self {
        Self(arr)
    }

    pub fn min_value() -> Self {
        Self([u8::min_value(); 32])
    }

    pub fn max_value() -> Self {
        Self([u8::max_value(); 32])
    }

    pub fn zero() -> Self {
        Self::min_value()
    }
}

impl Add for U256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let mut result = [0u8; 32];
        let mut carry = false;

        (0..32).for_each(|i| {
            let sum = u16::from(self.0[i]) + u16::from(rhs.0[i]) + u16::from(carry);
            result[i] = sum as u8; // modulo 256
            carry = sum > 0xFF; // remove any number in bounds
        });

        // if the last carry is still on, handle wrapping
        if carry {
            (0..32).for_each(|i| {
                let sum = u16::from(result[i]) + u16::from(carry);
                result[i] = sum as u8;
                carry = sum > 0xFF;
            });
        }

        Self(result)
    }
}

impl Sub for U256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = [0u8; 32];
        let mut carry = false;

        (0..32).for_each(|i| {
            let sum = u16::from(self.0[i])
                .overflowing_sub(u16::from(rhs.0[i]))
                .0
                .overflowing_sub(u16::from(carry))
                .0;
            result[i] = sum as u8; // modulo 256
            carry = sum > 0xFF; // remove any number in bounds
        });

        // if the last carry is still on, handle wrapping
        if carry {
            (0..32).for_each(|i| {
                let sum = u16::from(result[i]).overflowing_sub(u16::from(carry)).0;
                result[i] = sum as u8;
                carry = sum > 0xFF;
            });
        }

        Self(result)
    }
}

// impl Mul for U256 {
//     fn mul(self, rhs: Self) -> Self::Output {

//     }
// }

// impl Step for U256 {
//     fn steps_between(start: &Self, end: &Self) -> Option<usize> {
//         let diff = *end - *start;
//         if diff > U256::from(usize::max_value()) {
//             None
//         } else {
//             let mut out = usize::min_value().to_le_bytes();
//             let val = &diff.0[0..(out.len())];
//             out.copy_from_slice(val);

//             Some(usize::from_le_bytes(out))
//         }
//     }

//     fn forward_checked(start: Self, count: usize) -> Option<Self> {
//         Some(start + U256::from(count))
//     }

//     fn backward_checked(start: Self, count: usize) -> Option<Self> {
//         Some(start - U256::from(count))
//     }
// }

macro_rules! impl_num {
    ($From: ty) => {
        impl From<$From> for U256 {
            fn from(value: $From) -> Self {
                let mut as_bytes = value.to_le_bytes().to_vec();
                let mut with_zeros = (0..(32 - as_bytes.len())).fold(Vec::new(), |mut vec, _| {
                    vec.push(0);
                    vec
                });
                with_zeros.append(&mut as_bytes);

                let mut inner = [0; 32];
                inner.copy_from_slice(&with_zeros);

                U256(inner)
            }
        }

        impl From<U256> for $From {
            fn from(value: U256) -> $From {
                const MAX: usize = <$From>::max_value().to_le_bytes().len();
                let mut ret = [0; MAX];
                ret.copy_from_slice(&value.0[(32 - MAX)..]);
                <$From>::from_be_bytes(ret)
            }
        }

        // impl Into<$From> for U256 {
        //     fn into(self) -> $From {
        //         let max_from = U256::from(<$From>::max_value());
        //         if self > max_from {
        //             <$From>::max_value()
        //         } else {
        //             let diff = self - max_from;
        //             let mut out = <$From>::min_value().to_le_bytes();
        //             let val = &diff.0[0..(out.len())];
        //             out.copy_from_slice(val);

        //             <$From>::from_le_bytes(out)
        //         }
        //     }
        // }

        // impl PartialEq<$From> for U256 {
        //     fn eq(&self, other: &$From) -> bool {
        //         let other = [0; 32];

        //         self.0 == other
        //     }

        //     fn ne(&self, other: &$From) -> bool {
        //         let other = [0; 32];

        //         self.0 != other
        //     }
        // }

        // impl PartialOrd<$From> for U256 {
        //     fn partial_cmp(&self, other: &$From) -> Option<Ordering> {
        //         if self > &U256::from(*other) {
        //             Some(Ordering::Greater)
        //         } else if self < &U256::from(*other) {
        //             Some(Ordering::Less)
        //         } else {
        //             Some(Ordering::Equal)
        //         }
        //     }

        //     fn lt(&self, other: &$From) -> bool {
        //         let other = [0; 32];

        //         self.0 < other
        //     }

        //     fn le(&self, other: &$From) -> bool {
        //         let other = [0; 32];

        //         self.0 <= other
        //     }

        //     fn gt(&self, other: &$From) -> bool {
        //         let other = [0; 32];

        //         self.0 > other
        //     }

        //     fn ge(&self, other: &$From) -> bool {
        //         let other = [0; 32];

        //         self.0 >= other
        //     }
        // }
    };
}

impl_num!(u8);
impl_num!(u16);
impl_num!(u32);
impl_num!(u64);
impl_num!(usize);
impl_num!(u128);

impl ToString for U256 {
    fn to_string(&self) -> String {
        let mut rems = Vec::new();
        let mut bytes = self.0.to_vec();

        while !bytes.is_empty() {
            let mut rem = 0;
            for b in bytes.iter_mut() {
                let loaned = rem << 8u8 | *b as u16;
                *b = (loaned / 10) as u8;
                rem = loaned % 10;
            }
            rems.push(rem);
            if bytes[0] == 0 {
                bytes = bytes[1..].to_vec();
            }
        }

        let as_str: String = rems
            .into_iter()
            .rev()
            .skip_while(|n| n == &0)
            .map(|n| n.to_string())
            .collect();

        if as_str.is_empty() {
            String::from("0")
        } else {
            as_str
        }
    }
}

#[test]
fn add_u256() {
    let a = U256::from(1u8);
    let b = U256::from(2u8);
    assert_eq!(a + b, U256::from(3u8));

    let a = U256::from(127u8);
    let b = U256::from(128u8);
    assert_eq!(a + b, U256::from(u8::max_value()));

    // wrapping
    let a = U256::max_value();
    let b = U256::max_value();
    assert_eq!(a + b, U256::max_value());

    let a = U256::max_value();
    let b = U256::from(1u8);
    assert_eq!(a + b, b);
}

#[test]
fn sub_u256() {
    let a = U256::from(1u8);
    let b = U256::from(2u8);
    assert_eq!(b - a, U256::from(1u8));

    let a = U256::from(127u8);
    let b = U256::from(128u8);
    assert_eq!(b - a, U256::from(1u8));

    let a = U256::max_value();
    let b = U256::max_value();
    assert_eq!(b - a, U256::min_value());

    // wrapping
    let a = U256::max_value();
    let b = U256::from(1u8);
    assert_eq!(b - a, b);

    let a = U256::from(1u8);
    let b = U256::from(2u8);
    assert_eq!(a - b, U256::max_value() - a); // -1
}

impl Iterator for U256 {
    type Item = U256;

    fn next(&mut self) -> Option<Self::Item> {
        // let one = U256::from(1u8);

        // Some(*self + one)

        todo!()
    }
}

pub struct Env {
    caller: Address,
    origin: Address,
    coinbase: Address,
    value: U256,
    gas_limit: u64,
    gas_price: u64,
    nonce: u64,
    timestamp: u32,
    difficulty: U256,
    number: u64,
}

pub struct State {
    storage: HashMap<Address, HashMap<U256, U256>>,
    code: HashMap<Address, Vec<u8>>,
    balance: HashMap<Address, U256>,
}

pub fn convert_to_bytes<N: Into<u128>>(n: N) -> Word {
    let bytes = n.into().to_le_bytes();
    let mut result = [0; 32];
    result[..16].copy_from_slice(&bytes);
    result[16..].copy_from_slice(&bytes);
    result
}

pub fn to_word(val: &[u8]) -> Word {
    let mut slice = [0u8; 32];

    let len = slice.len().min(val.len());
    slice[(32 - len)..].copy_from_slice(&val[..len]);

    slice
}

pub fn to_bv<'ctx>(ctx: &'ctx Context, val: &[u8]) -> z3::ast::BV<'ctx> {
    // println!("{:#?}", &val);
    assert!(val.len() <= 32);
    let mut result: [u8; 32] = [0; 32];
    // extend bytes slice
    result[(32 - val.len())..].copy_from_slice(val);

    // zero out any untouched value
    // result
    //     .iter_mut()
    //     .skip((32 - val.len()))
    //     .take(32 - val.len())
    //     .for_each(|x| *x = 0);

    // println!("{:?}", &result);
    let num = U256::new(result);
    let as_str = num.to_string();
    // dbg!(&as_str);
    let as_int = z3::ast::Int::from_str(ctx, &as_str).unwrap_or(z3::ast::Int::from_u64(ctx, 0));
    z3::ast::BV::from_int(&as_int, 256)
}
