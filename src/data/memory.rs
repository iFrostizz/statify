use std::ops::Range;
use z3::{ast::Ast, Context};

#[derive(Debug, Default, Clone)]
pub struct Memory<'ctx> {
    data: Option<z3::ast::BV<'ctx>>,
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
            dbg!(&offset, &size, &wsize, &words);
            if offset > size {
                let low_data = data.zero_ext(offset - size);
                low_data.concat(&words)
            } else if offset + wsize > size {
                let low_data = data.extract(offset - 1, 0);
                low_data.concat(&words)
            } else {
                let low_data = data.extract(offset - 1, 0);
                // we cut the legs of data so we must sub off - 1
                // old was "extract(size - 1, wsize + offset)"
                let up_data = data.extract(size - offset, wsize);
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

// #[test]
// fn set_mem() {
//     let mut memo = Memory::new();
//     let words = vec![1, 2, 3, 4, 5];
//     memo.set(0, words.clone());
//     assert_eq!(memo.get(0..5), words);
// }
