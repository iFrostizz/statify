#![feature(step_trait)]

use crate::z3::word_to_bv;
use core::cmp::Ordering;
use std::{
    collections::HashMap,
    fmt::Debug,
    // iter::Step,
    ops::{Add, Range, Sub},
};
use z3::{ast::BV, Context};

pub type Word = [u8; 32];

#[derive(Default)]
pub struct Stack {
    data: Vec<Word>,
}

#[derive(Default)]
pub struct Memory {
    data: Vec<u8>,
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

impl Stack {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(16),
        }
    }

    pub fn push(&mut self, value: Word) -> Result<(), RevertReason> {
        if self.data.len() == 16 {
            return Err(RevertReason::StackOverflow);
        }

        self.data.push(value);

        Ok(())
    }

    pub fn pop(&mut self) -> Result<Word, RevertReason> {
        self.data.pop().ok_or(RevertReason::StackUnderflow)
    }

    pub fn pop_bv<'c>(&mut self, ctx: &'c Context, name: &'c str) -> Result<BV<'c>, RevertReason> {
        let word = word_to_bv(ctx, name, self.pop()?);

        Ok(word)
    }

    /// dup the word at index n on the stack. Returns false if n is out of stack
    pub fn dupn(&mut self, n: usize) -> Result<(), RevertReason> {
        let word = match self.data.get(n) {
            Some(w) => w,
            None => return Err(RevertReason::StackUnderflow),
        };

        self.push(*word)
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

#[derive(Debug)]
pub struct EVMStack {
    stack: Stack,
}

impl EVMStack {
    pub fn new() -> Self {
        Self {
            stack: Stack::new(),
        }
    }

    /// push a 32 bytes value to the stack
    pub fn push(&mut self, value: Word) -> Result<(), RevertReason> {
        self.stack.push(value)
    }

    /// pop the front element of the stack and return it
    pub fn pop(&mut self) -> Result<Word, RevertReason> {
        self.stack.pop()
    }

    /// dup the word at index n on the stack
    pub fn dupn(&mut self, n: usize) -> Result<(), RevertReason> {
        self.stack.dupn(n)
    }

    /// swap the first word with the one at index n on the stack
    pub fn swapn(&mut self, n: usize) -> Result<(), RevertReason> {
        self.stack.swapn(n)
    }

    pub fn pop_bv<'c>(&mut self, ctx: &'c Context, name: &'c str) -> Result<BV<'c>, RevertReason> {
        let word = word_to_bv(ctx, name, self.pop()?);

        Ok(word)
    }
}

impl Memory {
    pub fn new() -> Self {
        Default::default()
    }

    /// set a vec of words in the memory at offset
    pub fn set(&mut self, offset: usize, words: Vec<u8>) {
        // dbg!(offset);
        // dbg!(offset + words.len());
        let len = self.data.len();
        if len <= offset {
            self.data.resize(len + words.len(), 0);
        }

        self.data.splice(offset..(offset + words.len()), words);
    }

    /// get a vec of words in the memory at offset
    pub fn get(&self, r: Range<usize>) -> Vec<u8> {
        r.into_iter()
            .map(|o| *self.data.get(o).unwrap_or(&0u8)) // 0 if out of bounds
            .collect()
    }
}

#[derive(Default, Debug)]
pub struct EVMMemory {
    memory: Memory,
}

impl EVMMemory {
    pub fn new() -> Self {
        Self {
            memory: Memory::new(),
        }
    }

    pub fn mload(&self, offset: U256) -> Word {
        let off: usize = offset.into();
        let mut ret = [0; 32];
        let mem = self.memory.get(off..(off + 32));
        ret.copy_from_slice(&mem);
        ret
    }

    pub fn mstore(&mut self, offset: U256, value: Word) {
        self.memory.set(offset.into(), value.to_vec());
    }

    pub fn mbig_load(&self, from: U256, to: U256) -> Vec<u8> {
        let from: usize = from.into();
        let to: usize = to.into();
        // dbg!(&from, to);
        self.memory.get(from..to)
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
        //         if self > <$From>::max_value() {
        //             <$From>::max_value()
        //         } else {
        //             let diff = self - U256::from(<$From>::max_value());
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

impl Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // stack is easily visualized when reversed
        let mut data = self.data.iter().rev();

        let mut data_str = if let Some(first) = data.next() {
            data.fold(format!("[{:?}", hex::encode(first)), |d, w| {
                format!("{d}, {:?}", hex::encode(w))
            })
        } else {
            String::from("[")
        };

        data_str.push(']');

        write!(f, "{}", data_str)
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = self.data.chunks(32);

        let mut data_str = if let Some(first) = data.next() {
            data.fold(format!("[{:?}", hex::encode(first)), |d, w| {
                format!("{d}, {:?}", hex::encode(w))
            })
        } else {
            String::from("[")
        };

        data_str.push(']');

        write!(f, "{}", data_str)
    }
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

#[test]
fn push_stack() {
    let mut stack = Stack::new();
    let num = convert_to_bytes(100_u16);
    stack.push(num);

    assert_eq!(stack.data, vec![num]);
}

#[test]
fn pop_stack() {
    let mut stack = Stack::new();
    let num = convert_to_bytes(100_u16);
    stack.push(num);
    stack.pop();

    assert!(stack.data.is_empty());
}

#[test]
fn dup_stack() {
    let mut stack = Stack::new();
    let num = convert_to_bytes(100_u16);
    stack.push(num);
    stack.dupn(0);

    assert_eq!(stack.data, vec![num, num]);
}

#[test]
fn swap_stack() {
    let mut stack = Stack::new();
    let num1 = convert_to_bytes(100_u16);
    let num2 = convert_to_bytes(200_u16);
    stack.push(num1);
    stack.push(num2);
    stack.swapn(1);

    assert_eq!(stack.data, vec![num2, num1]);
}

// #[test]
// fn set_mem() {
//     let mut memo = Memory::new();
//     let words = vec![1, 2, 3, 4, 5];
//     memo.set(0, words.clone());
//     assert_eq!(memo.get(0..5), words);
// }
