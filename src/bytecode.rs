// turns hex into mnemonics

use std::fmt::Display;

use crate::opcodes::OpCodes;
use crate::{opcodes::OpCode, utils::get_slice};

#[derive(Debug, Clone, Copy)]
pub struct Mnemonic<'a> {
    pub pc: usize,
    pub op: OpCode,
    // pub pushes: [u8; 32],
    pub pushes: &'a [u8],
}

impl<'a> Mnemonic<'a> {
    pub fn opcode(&self) -> &OpCodes {
        self.op.opcode()
    }
}

impl Display for Mnemonic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self, self.opcode())
    }
}

pub type Mnemonics<'a> = Vec<Mnemonic<'a>>;

pub fn to_mnemonics(bytecode: &[u8]) -> Mnemonics {
    let (mut code, mut pc) = (Vec::new(), 0);

    while let Some(b) = bytecode.get(pc) {
        let op = OpCode::from_u8(*b);

        let (_pc, pushes) = if let Some(push_size) = op.push_size() {
            // write in buffer an skip until stop

            let range = (pc + 1)..(pc + 1 + push_size as usize);

            let mut _pc = pc + push_size as usize;

            let new_slice = get_slice(bytecode, range);
            (_pc, new_slice)
        } else {
            // non-push opcode

            // zero
            (pc, &[][..])
        };

        code.push(Mnemonic { pc, op, pushes });

        pc = _pc + 1;
    }

    code
}
