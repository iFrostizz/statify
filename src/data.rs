use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Add, Range, Sub},
};
use z3::{ast::Ast, Context};

pub type Word = [u8; 32];

// TODO: allow for symbolic stack elements
#[derive(Default, Debug, Clone)]
pub struct Stack<'ctx> {
    data: Vec<z3::ast::BV<'ctx>>,
}

#[derive(Debug, Default, Clone)]
pub struct Memory<'ctx> {
    data: Option<z3::ast::BV<'ctx>>,
}

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

impl<'ctx> Stack<'ctx> {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(16),
        }
    }

    pub fn push(&mut self, value: z3::ast::BV<'ctx>) -> Result<(), RevertReason> {
        if self.data.len() == 16 {
            return Err(RevertReason::StackOverflow);
        }

        self.data.push(value.simplify());

        Ok(())
    }

    pub fn pop(&mut self) -> Result<z3::ast::BV<'ctx>, RevertReason> {
        self.data.pop().ok_or(RevertReason::StackUnderflow)
    }

    /// dup the word at index n on the stack. Returns false if n is out of stack
    pub fn dupn(&mut self, n: usize) -> Result<(), RevertReason> {
        let word = match self.data.get(n) {
            Some(w) => w,
            None => return Err(RevertReason::StackUnderflow),
        };

        self.push(word.clone())
    }

    pub fn get(&self, n: usize) -> Result<z3::ast::BV<'ctx>, RevertReason> {
        self.data
            .get(
                self.data
                    .len()
                    .checked_sub(n + 1)
                    .ok_or(RevertReason::StackUnderflow)?,
            )
            .ok_or(RevertReason::StackUnderflow)
            .cloned()
    }

    /// swap the first word with the one at index n on the stack. Returns false if n is out of stack
    pub fn swapn(&mut self, n: usize) -> Result<(), RevertReason> {
        match self.data.get(n) {
            Some(_) => {
                self.data.swap(0, n);
                Ok(())
            }
            None => Err(RevertReason::StackUnderflow),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EVMStack<'ctx> {
    stack: Stack<'ctx>,
}

impl<'ctx> EVMStack<'ctx> {
    pub fn new() -> Self {
        Self {
            stack: Stack::new(),
        }
    }

    /// push a 32 bytes value to the stack
    pub fn push(&mut self, value: z3::ast::BV<'ctx>) -> Result<(), RevertReason> {
        assert_eq!(value.get_size(), 256);
        self.stack.push(value)
    }

    /// pop the front element of the stack and return it
    pub fn pop(&mut self) -> Result<z3::ast::BV<'ctx>, RevertReason> {
        self.stack.pop()
    }

    pub fn peek(&self, n: usize) -> Result<z3::ast::BV<'ctx>, RevertReason> {
        self.stack.get(n)
    }

    pub fn pop64(&mut self) -> Result<Option<u64>, RevertReason> {
        let val = self.stack.pop()?;

        for i in (1..4).rev() {
            let ex = val.extract((i + 1) * 64 - 1, i * 64).simplify();
            if ex.as_u64().unwrap() != 0 {
                return Ok(None);
            }
        }

        Ok(Some(val.extract(63, 0).simplify().as_u64().unwrap()))
    }

    pub fn pop32(&mut self) -> Result<Option<u32>, RevertReason> {
        let val = self.stack.pop()?;

        for i in (1..8).rev() {
            let ex = val.extract((i + 1) * 32 - 1, i * 32).simplify();
            if ex.as_u64().unwrap() != 0 {
                return Ok(None);
            }
        }

        Ok(Some(
            val.extract(31, 0)
                .simplify()
                .as_u64()
                .unwrap()
                .try_into()
                .unwrap(),
        ))
    }

    /// dup the word at index n on the stack
    pub fn dupn(&mut self, n: usize) -> Result<(), RevertReason> {
        self.stack.dupn(n)
    }

    /// swap the first word with the one at index n on the stack
    pub fn swapn(&mut self, n: usize) -> Result<(), RevertReason> {
        self.stack.swapn(n)
    }
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

impl<'ctx> Memory<'ctx> {
    pub fn new() -> Self {
        Default::default()
    }

    /// set a vec of words in the memory at offset
    pub fn set(&mut self, offset: u32, words: z3::ast::BV<'ctx>) {
        let data = if let Some(data) = &self.data {
            let size = data.get_size();
            let wsize = words.get_size();
            if offset > size {
                let low_data = data.zero_ext(offset - size);
                low_data.concat(&words)
            } else if offset + wsize > size {
                let low_data = data.extract(offset, 0);
                low_data.concat(&words)
            } else {
                let low_data = data.extract(offset, 0);
                let up_data = data.extract(size, offset + wsize);
                low_data.concat(&words.concat(&up_data))
            }
        } else {
            words
        };

        self.data = Some(data.simplify());
    }

    /// Get a `BV` representing the data in memory in the range `r`.
    pub fn get(&mut self, ctx: &'ctx Context, r: Range<u32>) -> z3::ast::BV<'ctx> {
        let (low, high) = (r.start, r.end);
        if low == high {
            return z3::ast::BV::from_u64(ctx, 0, 1);
        }

        let data = std::mem::replace(&mut self.data, None)
            .unwrap_or_else(|| z3::ast::BV::from_u64(ctx, 0, high));

        if high > data.get_size() {
            let extended_data = data.zero_ext(high - data.get_size());
            self.data.replace(extended_data);
        }

        self.data.get_or_insert(data).extract(high - 1, low)
    }
}

#[derive(Debug, Clone)]
pub struct EVMMemory<'ctx> {
    ctx: &'ctx Context,
    memory: Memory<'ctx>,
}

impl<'ctx> EVMMemory<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            ctx,
            memory: Memory::new(),
        }
    }

    pub fn mload(&mut self, off: u32) -> z3::ast::BV {
        let ret = self.memory.get(self.ctx, off..(off + 256));
        assert_eq!(ret.get_size(), 256, "mload val len != 256b");
        ret
    }

    pub fn mstore(&mut self, offset: u32, value: z3::ast::BV<'ctx>) {
        assert_eq!(value.get_size(), 256);
        self.memory.set(offset, value);
    }

    pub fn mbig_load(&mut self, from: u32, to: u32) -> z3::ast::BV<'ctx> {
        self.memory.get(self.ctx, from..to)
    }

    pub fn mbig_store(&mut self, offset: u32, value: z3::ast::BV<'ctx>) {
        self.memory.set(offset, value);
    }
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

// impl<'a> Debug for Stack<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         todo!()
//         // // stack is easily visualized when reversed
//         // let mut data = self.data.iter().rev();

//         // let mut data_str = if let Some(first) = data.next() {
//         //     data.fold(format!("[{:?}", hex::encode(first)), |d, w| {
//         //         format!("{d}, {:?}", hex::encode(w))
//         //     })
//         // } else {
//         //     String::from("[")
//         // };

//         // data_str.push(']');

//         // write!(f, "{}", data_str)
//     }
// }

// impl Debug for Memory<'ctx> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let mut data = self.data.chunks(32);

//         let mut data_str = if let Some(first) = data.next() {
//             data.fold(format!("[{:?}", hex::encode(first)), |d, w| {
//                 format!("{d}, {:?}", hex::encode(w))
//             })
//         } else {
//             String::from("[")
//         };

//         data_str.push(']');

//         write!(f, "{}", data_str)
//     }
// }

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

// TODO: revive the tests
// #[test]
// fn push_stack() {
//     let mut stack = Stack::new();
//     let num = convert_to_bytes(100_u16);
//     stack.push(num);

//     assert_eq!(stack.data, vec![num]);
// }

// #[test]
// fn pop_stack() {
//     let mut stack = Stack::new();
//     let num = convert_to_bytes(100_u16);
//     stack.push(num);
//     stack.pop();

//     assert!(stack.data.is_empty());
// }

// #[test]
// fn dup_stack() {
//     let mut stack = Stack::new();
//     let num = convert_to_bytes(100_u16);
//     stack.push(num);
//     stack.dupn(0);

//     assert_eq!(stack.data, vec![num, num]);
// }

// #[test]
// fn swap_stack() {
//     let mut stack = Stack::new();
//     let num1 = convert_to_bytes(100_u16);
//     let num2 = convert_to_bytes(200_u16);
//     stack.push(num1);
//     stack.push(num2);
//     stack.swapn(1);

//     assert_eq!(stack.data, vec![num2, num1]);
// }

// #[test]
// fn set_mem() {
//     let mut memo = Memory::new();
//     let words = vec![1, 2, 3, 4, 5];
//     memo.set(0, words.clone());
//     assert_eq!(memo.get(0..5), words);
// }
