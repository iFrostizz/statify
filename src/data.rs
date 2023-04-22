use std::collections::HashMap;
use std::{fmt::Debug, ops::Range};

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

// TODO implement meaningful errors (e.g stack underflow, overflow)
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

    /// dup the word at index n on the stack. Returns false if n is out of stack
    pub fn dupn(&mut self, n: usize) -> Result<(), RevertReason> {
        let word = match self.data.get(n) {
            Some(w) => w,
            None => return Err(RevertReason::StackUnderflow),
        };

        self.push(*word);

        Ok(())
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
    pub fn get(&self, r: Range<usize>) -> Vec<u8> {
        r.into_iter()
            .map(|o| *self.data.get(o).unwrap_or(&0))
            .collect()
    }
}

pub type Address = [u8; 20];
pub type U256 = [u8; 32];

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

#[test]
fn set_mem() {
    let mut memo = Memory::new();
    let words = vec![1, 2, 3, 4, 5];
    memo.set(0, words.clone());
    assert_eq!(memo.get(0..5), words);
}
