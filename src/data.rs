use std::collections::HashMap;
use std::ops::Add;
use std::{fmt::Debug, ops::Range};

use z3::ast::BV;
use z3::Context;

use crate::z3::word_to_bv;

pub type Word = [u8; 32];

pub struct Stack {
    data: Vec<Word>,
}

#[derive(Default)]
pub struct Memory {
    data: Vec<u8>,
}

#[derive(Debug)]
pub enum RevertReason {
    StackUnderflow,
    StackOverflow,
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
    pub fn swapn(&mut self, n: usize) -> bool {
        if self.data.get(n).is_none() {
            return false;
        }

        self.data.swap(0, n);
        true
    }
}

pub struct EVMStack {
    stack: Stack,
}

impl EVMStack {
    pub fn new() -> Self {
        Self {
            stack: Stack::new(),
        }
    }

    pub fn push(&mut self, value: Word) -> Result<(), RevertReason> {
        self.stack.push(value)
    }

    pub fn pop(&mut self) -> Result<Word, RevertReason> {
        self.stack.pop()
    }

    /// dup the word at index n on the stack. Returns false if n is out of stack
    pub fn dupn(&mut self, n: usize) -> Result<(), RevertReason> {
        self.stack.dupn(n)
    }

    /// swap the first word with the one at index n on the stack. Returns false if n is out of stack
    pub fn swapn(&mut self, n: usize) -> bool {
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
        let len = self.data.len();
        if len <= offset {
            self.data.resize(len + words.len(), 0);
        }

        self.data.splice(offset..(offset + words.len()), words);
    }

    /// get a vec of words in the memory at offset
    pub fn get(&self, r: Range<U256>) -> Vec<u8> {
        // r.into_iter()
        //     .map(|o| *self.data.get(o).unwrap_or(&0))
        //     .collect()

        todo!();
    }
}

#[derive(Default)]
pub struct EVMMemory {
    memory: Memory,
}

impl EVMMemory {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn mload(&self, offset: U256) -> Word {
        // let mut slice = [0u8; 32];
        // let vec = self.memory.get(offset..(offset + 32));
        // slice.copy_from_slice(&vec);
        // slice

        todo!()
    }
}

pub type Address = [u8; 20];
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct U256([u8; 32]);

impl U256 {
    fn max_value() -> Self {
        Self([u8::max_value(); 32])
    }
}

impl Add for U256 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut result = [0u8; 32];
        let mut carry = false;

        (0..32).for_each(|i| {
            let sum = u16::from(self.0[i]) + u16::from(other.0[i]) + u16::from(carry);
            result[i] = sum as u8;
            carry = sum > 0xFF;
        });

        // TODO: add the last carry in the loop
        result[0] = result[0].saturating_add(u8::from(carry));

        Self(result)
    }
}

macro_rules! impl_from {
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
    };
}

impl_from!(u8);
impl_from!(u16);
impl_from!(u32);
impl_from!(u64);
impl_from!(u128);

#[test]
fn add_u256() {
    let a = U256::from(1u8);
    let b = U256::from(2u8);
    assert_eq!(a + b, U256::from(3u8));

    let a = U256::from(127u8);
    let b = U256::from(128u8);
    assert_eq!(a + b, U256::from(u8::max_value()));

    let a = U256::max_value();
    let b = U256::max_value();
    assert_eq!(a + b, U256::max_value());
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
        // stack is easily visualized reversed
        let mut data = self
            .data
            .iter()
            .rev()
            .fold(String::from("["), |d, w| format!("{d}, {}", hex::encode(w)));

        data.push(']');

        write!(f, "{}", data)
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut data = self
            .data
            .chunks(32)
            .fold(String::from("["), |d, w| format!("{d}, {}", hex::encode(w)));

        data.push(']');

        write!(f, "{}", data)
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
    slice[..len].copy_from_slice(&val[..len]);

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
