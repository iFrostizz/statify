use crate::helpers::RevertReason;
use z3::ast::Ast;

#[derive(Default, Debug, Clone)]
pub struct Stack<'ctx> {
    data: Vec<z3::ast::BV<'ctx>>,
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
