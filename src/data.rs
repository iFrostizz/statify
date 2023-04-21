use std::collections::VecDeque;
use std::{fmt::Debug, ops::Range};

pub type Word = [u8; 32];

pub struct Stack {
    data: Vec<Word>,
}

pub struct Memory {
    data: VecDeque<Word>,
}

// TODO implement meaningful errors (stack underflow, overflow)
impl Stack {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(16),
        }
    }

    pub fn push(&mut self, value: Word) -> bool {
        if self.data.len() == 16 {
            return false;
        }

        self.data.push(value);
        true
    }

    pub fn pop(&mut self) -> bool {
        self.data.pop().is_some()
    }

    /// dup the word at index n on the stack. Returns false if n is out of stack
    pub fn dupn(&mut self, n: usize) -> bool {
        let word = match self.data.get(n) {
            Some(w) => w,
            None => return false,
        };

        self.push(*word)
    }

    /// swap the first word with the one at index n on the stack. Returns false if n is out of stack
    pub fn swapn(&mut self, n: usize) -> bool {
        let word = match self.data.get(n) {
            Some(w) => w,
            None => return false,
        };

        self.data.swap(0, n);
        true
    }
}

impl Memory {
    /// set a vec of words in the memory at offset
    pub fn set(&mut self, w: VecDeque<Word>, offset: usize) {
        // let mut rem = &w[..];
        let mut rem = w;

        // write rem byte by byte in the memory
        while !rem.is_empty() {
            // word is aligned
            if offset % 32 == 0 {
                let word = rem.pop_front();
            }
        }
    }

    /// get a vec of words in the memory at offset
    pub fn get(&self, r: Range<usize>) -> Vec<Word> {
        todo!()
    }
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
            .iter()
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
