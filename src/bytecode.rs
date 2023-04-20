// turns hex into mnemonics

use crate::{opcodes::OpCode, utils::get_slice};

#[derive(Debug)]
pub enum Op {
    OpCode(OpCode),
    U8(u8),
}

#[derive(Debug)]
pub struct Mnemonic<'a> {
    pub pc: usize,
    pub op: Op,
    pub pushes: &'a [u8],
}

pub type Mnemonics<'a> = Vec<Mnemonic<'a>>;

pub fn to_mnemonics(bytecode: &[u8]) -> Mnemonics {
    let (mut code, mut pc) = (Vec::new(), 0);

    while let Some(b) = bytecode.get(pc) {
        let (op, pushes) = if let Some(op) = OpCode::try_from_u8(*b) {
            let pushes = if let Some(push_size) = op.push_size() {
                // write in buffer an skip until stop

                let range = (pc + 1)..(pc + 1 + push_size as usize);

                pc += push_size as usize;

                get_slice(bytecode, range)
            } else {
                // non-push opcode

                &[]
            };

            (Op::OpCode(op), pushes)
        } else {
            // invalid opcode

            (Op::U8(*b), &[] as &[u8])
        };

        code.push(Mnemonic { pc, op, pushes });

        pc += 1;
    }

    code
}
