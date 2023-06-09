#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct OpCode(u8);

#[derive(Debug, PartialEq, Eq)]
pub enum OpCodes {
    Invalid,
    Stop,
    Add,
    Mul,
    Sub,
    Div,
    Sdiv,
    Mod,
    Smod,
    Addmod,
    Mulmod,
    Exp,
    Signextend,
    Lt,
    Gt,
    Slt,
    Sgt,
    Eq,
    Iszero,
    And,
    Or,
    Xor,
    Not,
    Byte,
    Shl,
    Shr,
    Sar,
    Sha3,
    Address,
    Balance,
    Origin,
    Caller,
    Callvalue,
    Calldataload,
    Calldatasize,
    Calldatacopy,
    Codesize,
    Codecopy,
    Gasprice,
    Extcodesize,
    Extcodecopy,
    Returndatasize,
    Returndatacopy,
    Extcodehash,
    Blockhash,
    Coinbase,
    Timestamp,
    Number,
    Difficulty,
    Gaslimit,
    Chainid,
    Selfbalance,
    Basefee,
    Pop,
    Mload,
    Mstore,
    Mstore8,
    Sload,
    Sstore,
    Jump,
    Jumpi,
    Pc,
    Msize,
    Gas,
    Jumpdest,
    Push0,
    Push1,
    Push2,
    Push3,
    Push4,
    Push5,
    Push6,
    Push7,
    Push8,
    Push9,
    Push10,
    Push11,
    Push12,
    Push13,
    Push14,
    Push15,
    Push16,
    Push17,
    Push18,
    Push19,
    Push20,
    Push21,
    Push22,
    Push23,
    Push24,
    Push25,
    Push26,
    Push27,
    Push28,
    Push29,
    Push30,
    Push31,
    Push32,
    Dup1,
    Dup2,
    Dup3,
    Dup4,
    Dup5,
    Dup6,
    Dup7,
    Dup8,
    Dup9,
    Dup10,
    Dup11,
    Dup12,
    Dup13,
    Dup14,
    Dup15,
    Dup16,
    Swap1,
    Swap2,
    Swap3,
    Swap4,
    Swap5,
    Swap6,
    Swap7,
    Swap8,
    Swap9,
    Swap10,
    Swap11,
    Swap12,
    Swap13,
    Swap14,
    Swap15,
    Swap16,
    Log0,
    Log1,
    Log2,
    Log3,
    Log4,
    Create,
    Call,
    Callcode,
    Return,
    Delegatecall,
    Create2,
    Staticcall,
    Revert,
    Selfdestruct,
}

impl OpCode {
    pub fn from_u8(opcode: u8) -> OpCode {
        OpCode(opcode)
    }

    pub fn opcode(&self) -> &OpCodes {
        OPCODE_JUMPMAP.get(self.0 as usize).unwrap()
    }

    #[inline(always)]
    pub const fn u8(&self) -> u8 {
        self.0
    }

    pub fn is_push(&self) -> bool {
        self.0 >= 95 && self.0 < 128
    }

    pub fn is_dup(&self) -> bool {
        self.0 >= 128 && self.0 < 144
    }

    pub fn is_swap(&self) -> bool {
        self.0 >= 144 && self.0 < 160
    }

    pub fn push_size(&self) -> Option<u8> {
        if self.is_push() {
            // starts at 0
            Some(self.0 - 95)
        } else {
            None
        }
    }

    pub fn dup_size(&self) -> Option<u8> {
        if self.is_dup() {
            Some(self.0 - 128 + 1)
        } else {
            None
        }
    }

    pub fn swap_size(&self) -> Option<u8> {
        if self.is_swap() {
            Some(self.0 - 144 + 1)
        } else {
            None
        }
    }
}

pub const OPCODE_JUMPMAP: [OpCodes; 256] = [
    /* 0x00 */ OpCodes::Stop,
    /* 0x01 */ OpCodes::Add,
    /* 0x02 */ OpCodes::Mul,
    /* 0x03 */ OpCodes::Sub,
    /* 0x04 */ OpCodes::Div,
    /* 0x05 */ OpCodes::Sdiv,
    /* 0x06 */ OpCodes::Mod,
    /* 0x07 */ OpCodes::Smod,
    /* 0x08 */ OpCodes::Addmod,
    /* 0x09 */ OpCodes::Mulmod,
    /* 0x0a */ OpCodes::Exp,
    /* 0x0b */ OpCodes::Signextend,
    /* 0x0c */ OpCodes::Invalid,
    /* 0x0d */ OpCodes::Invalid,
    /* 0x0e */ OpCodes::Invalid,
    /* 0x0f */ OpCodes::Invalid,
    /* 0x10 */ OpCodes::Lt,
    /* 0x11 */ OpCodes::Gt,
    /* 0x12 */ OpCodes::Slt,
    /* 0x13 */ OpCodes::Sgt,
    /* 0x14 */ OpCodes::Eq,
    /* 0x15 */ OpCodes::Iszero,
    /* 0x16 */ OpCodes::And,
    /* 0x17 */ OpCodes::Or,
    /* 0x18 */ OpCodes::Xor,
    /* 0x19 */ OpCodes::Not,
    /* 0x1a */ OpCodes::Byte,
    /* 0x1b */ OpCodes::Shl,
    /* 0x1c */ OpCodes::Shr,
    /* 0x1d */ OpCodes::Sar,
    /* 0x1e */ OpCodes::Invalid,
    /* 0x1f */ OpCodes::Invalid,
    /* 0x20 */ OpCodes::Sha3,
    /* 0x21 */ OpCodes::Invalid,
    /* 0x22 */ OpCodes::Invalid,
    /* 0x23 */ OpCodes::Invalid,
    /* 0x24 */ OpCodes::Invalid,
    /* 0x25 */ OpCodes::Invalid,
    /* 0x26 */ OpCodes::Invalid,
    /* 0x27 */ OpCodes::Invalid,
    /* 0x28 */ OpCodes::Invalid,
    /* 0x29 */ OpCodes::Invalid,
    /* 0x2a */ OpCodes::Invalid,
    /* 0x2b */ OpCodes::Invalid,
    /* 0x2c */ OpCodes::Invalid,
    /* 0x2d */ OpCodes::Invalid,
    /* 0x2e */ OpCodes::Invalid,
    /* 0x2f */ OpCodes::Invalid,
    /* 0x30 */ OpCodes::Address,
    /* 0x31 */ OpCodes::Balance,
    /* 0x32 */ OpCodes::Origin,
    /* 0x33 */ OpCodes::Caller,
    /* 0x34 */ OpCodes::Callvalue,
    /* 0x35 */ OpCodes::Calldataload,
    /* 0x36 */ OpCodes::Calldatasize,
    /* 0x37 */ OpCodes::Calldatacopy,
    /* 0x38 */ OpCodes::Codesize,
    /* 0x39 */ OpCodes::Codecopy,
    /* 0x3a */ OpCodes::Gasprice,
    /* 0x3b */ OpCodes::Extcodesize,
    /* 0x3c */ OpCodes::Extcodecopy,
    /* 0x3d */ OpCodes::Returndatasize,
    /* 0x3e */ OpCodes::Returndatacopy,
    /* 0x3f */ OpCodes::Extcodehash,
    /* 0x40 */ OpCodes::Blockhash,
    /* 0x41 */ OpCodes::Coinbase,
    /* 0x42 */ OpCodes::Timestamp,
    /* 0x43 */ OpCodes::Number,
    /* 0x44 */ OpCodes::Difficulty,
    /* 0x45 */ OpCodes::Gaslimit,
    /* 0x46 */ OpCodes::Chainid,
    /* 0x47 */ OpCodes::Selfbalance,
    /* 0x48 */ OpCodes::Basefee,
    /* 0x49 */ OpCodes::Invalid,
    /* 0x4a */ OpCodes::Invalid,
    /* 0x4b */ OpCodes::Invalid,
    /* 0x4c */ OpCodes::Invalid,
    /* 0x4d */ OpCodes::Invalid,
    /* 0x4e */ OpCodes::Invalid,
    /* 0x4f */ OpCodes::Invalid,
    /* 0x50 */ OpCodes::Pop,
    /* 0x51 */ OpCodes::Mload,
    /* 0x52 */ OpCodes::Mstore,
    /* 0x53 */ OpCodes::Mstore8,
    /* 0x54 */ OpCodes::Sload,
    /* 0x55 */ OpCodes::Sstore,
    /* 0x56 */ OpCodes::Jump,
    /* 0x57 */ OpCodes::Jumpi,
    /* 0x58 */ OpCodes::Pc,
    /* 0x59 */ OpCodes::Msize,
    /* 0x5a */ OpCodes::Gas,
    /* 0x5b */ OpCodes::Jumpdest,
    /* 0x5c */ OpCodes::Invalid,
    /* 0x5d */ OpCodes::Invalid,
    /* 0x5e */ OpCodes::Invalid,
    /* 0x5f */ OpCodes::Push0,
    /* 0x60 */ OpCodes::Push1,
    /* 0x61 */ OpCodes::Push2,
    /* 0x62 */ OpCodes::Push3,
    /* 0x63 */ OpCodes::Push4,
    /* 0x64 */ OpCodes::Push5,
    /* 0x65 */ OpCodes::Push6,
    /* 0x66 */ OpCodes::Push7,
    /* 0x67 */ OpCodes::Push8,
    /* 0x68 */ OpCodes::Push9,
    /* 0x69 */ OpCodes::Push10,
    /* 0x6a */ OpCodes::Push11,
    /* 0x6b */ OpCodes::Push12,
    /* 0x6c */ OpCodes::Push13,
    /* 0x6d */ OpCodes::Push14,
    /* 0x6e */ OpCodes::Push15,
    /* 0x6f */ OpCodes::Push16,
    /* 0x70 */ OpCodes::Push17,
    /* 0x71 */ OpCodes::Push18,
    /* 0x72 */ OpCodes::Push19,
    /* 0x73 */ OpCodes::Push20,
    /* 0x74 */ OpCodes::Push21,
    /* 0x75 */ OpCodes::Push22,
    /* 0x76 */ OpCodes::Push23,
    /* 0x77 */ OpCodes::Push24,
    /* 0x78 */ OpCodes::Push25,
    /* 0x79 */ OpCodes::Push26,
    /* 0x7a */ OpCodes::Push27,
    /* 0x7b */ OpCodes::Push28,
    /* 0x7c */ OpCodes::Push29,
    /* 0x7d */ OpCodes::Push30,
    /* 0x7e */ OpCodes::Push31,
    /* 0x7f */ OpCodes::Push32,
    /* 0x80 */ OpCodes::Dup1,
    /* 0x81 */ OpCodes::Dup2,
    /* 0x82 */ OpCodes::Dup3,
    /* 0x83 */ OpCodes::Dup4,
    /* 0x84 */ OpCodes::Dup5,
    /* 0x85 */ OpCodes::Dup6,
    /* 0x86 */ OpCodes::Dup7,
    /* 0x87 */ OpCodes::Dup8,
    /* 0x88 */ OpCodes::Dup9,
    /* 0x89 */ OpCodes::Dup10,
    /* 0x8a */ OpCodes::Dup11,
    /* 0x8b */ OpCodes::Dup12,
    /* 0x8c */ OpCodes::Dup13,
    /* 0x8d */ OpCodes::Dup14,
    /* 0x8e */ OpCodes::Dup15,
    /* 0x8f */ OpCodes::Dup16,
    /* 0x90 */ OpCodes::Swap1,
    /* 0x91 */ OpCodes::Swap2,
    /* 0x92 */ OpCodes::Swap3,
    /* 0x93 */ OpCodes::Swap4,
    /* 0x94 */ OpCodes::Swap5,
    /* 0x95 */ OpCodes::Swap6,
    /* 0x96 */ OpCodes::Swap7,
    /* 0x97 */ OpCodes::Swap8,
    /* 0x98 */ OpCodes::Swap9,
    /* 0x99 */ OpCodes::Swap10,
    /* 0x9a */ OpCodes::Swap11,
    /* 0x9b */ OpCodes::Swap12,
    /* 0x9c */ OpCodes::Swap13,
    /* 0x9d */ OpCodes::Swap14,
    /* 0x9e */ OpCodes::Swap15,
    /* 0x9f */ OpCodes::Swap16,
    /* 0xa0 */ OpCodes::Log0,
    /* 0xa1 */ OpCodes::Log1,
    /* 0xa2 */ OpCodes::Log2,
    /* 0xa3 */ OpCodes::Log3,
    /* 0xa4 */ OpCodes::Log4,
    /* 0xa5 */ OpCodes::Invalid,
    /* 0xa6 */ OpCodes::Invalid,
    /* 0xa7 */ OpCodes::Invalid,
    /* 0xa8 */ OpCodes::Invalid,
    /* 0xa9 */ OpCodes::Invalid,
    /* 0xaa */ OpCodes::Invalid,
    /* 0xab */ OpCodes::Invalid,
    /* 0xac */ OpCodes::Invalid,
    /* 0xad */ OpCodes::Invalid,
    /* 0xae */ OpCodes::Invalid,
    /* 0xaf */ OpCodes::Invalid,
    /* 0xb0 */ OpCodes::Invalid,
    /* 0xb1 */ OpCodes::Invalid,
    /* 0xb2 */ OpCodes::Invalid,
    /* 0xb3 */ OpCodes::Invalid,
    /* 0xb4 */ OpCodes::Invalid,
    /* 0xb5 */ OpCodes::Invalid,
    /* 0xb6 */ OpCodes::Invalid,
    /* 0xb7 */ OpCodes::Invalid,
    /* 0xb8 */ OpCodes::Invalid,
    /* 0xb9 */ OpCodes::Invalid,
    /* 0xba */ OpCodes::Invalid,
    /* 0xbb */ OpCodes::Invalid,
    /* 0xbc */ OpCodes::Invalid,
    /* 0xbd */ OpCodes::Invalid,
    /* 0xbe */ OpCodes::Invalid,
    /* 0xbf */ OpCodes::Invalid,
    /* 0xc0 */ OpCodes::Invalid,
    /* 0xc1 */ OpCodes::Invalid,
    /* 0xc2 */ OpCodes::Invalid,
    /* 0xc3 */ OpCodes::Invalid,
    /* 0xc4 */ OpCodes::Invalid,
    /* 0xc5 */ OpCodes::Invalid,
    /* 0xc6 */ OpCodes::Invalid,
    /* 0xc7 */ OpCodes::Invalid,
    /* 0xc8 */ OpCodes::Invalid,
    /* 0xc9 */ OpCodes::Invalid,
    /* 0xca */ OpCodes::Invalid,
    /* 0xcb */ OpCodes::Invalid,
    /* 0xcc */ OpCodes::Invalid,
    /* 0xcd */ OpCodes::Invalid,
    /* 0xce */ OpCodes::Invalid,
    /* 0xcf */ OpCodes::Invalid,
    /* 0xd0 */ OpCodes::Invalid,
    /* 0xd1 */ OpCodes::Invalid,
    /* 0xd2 */ OpCodes::Invalid,
    /* 0xd3 */ OpCodes::Invalid,
    /* 0xd4 */ OpCodes::Invalid,
    /* 0xd5 */ OpCodes::Invalid,
    /* 0xd6 */ OpCodes::Invalid,
    /* 0xd7 */ OpCodes::Invalid,
    /* 0xd8 */ OpCodes::Invalid,
    /* 0xd9 */ OpCodes::Invalid,
    /* 0xda */ OpCodes::Invalid,
    /* 0xdb */ OpCodes::Invalid,
    /* 0xdc */ OpCodes::Invalid,
    /* 0xdd */ OpCodes::Invalid,
    /* 0xde */ OpCodes::Invalid,
    /* 0xdf */ OpCodes::Invalid,
    /* 0xe0 */ OpCodes::Invalid,
    /* 0xe1 */ OpCodes::Invalid,
    /* 0xe2 */ OpCodes::Invalid,
    /* 0xe3 */ OpCodes::Invalid,
    /* 0xe4 */ OpCodes::Invalid,
    /* 0xe5 */ OpCodes::Invalid,
    /* 0xe6 */ OpCodes::Invalid,
    /* 0xe7 */ OpCodes::Invalid,
    /* 0xe8 */ OpCodes::Invalid,
    /* 0xe9 */ OpCodes::Invalid,
    /* 0xea */ OpCodes::Invalid,
    /* 0xeb */ OpCodes::Invalid,
    /* 0xec */ OpCodes::Invalid,
    /* 0xed */ OpCodes::Invalid,
    /* 0xee */ OpCodes::Invalid,
    /* 0xef */ OpCodes::Invalid,
    /* 0xf0 */ OpCodes::Create,
    /* 0xf1 */ OpCodes::Call,
    /* 0xf2 */ OpCodes::Callcode,
    /* 0xf3 */ OpCodes::Return,
    /* 0xf4 */ OpCodes::Delegatecall,
    /* 0xf5 */ OpCodes::Create2,
    /* 0xf6 */ OpCodes::Invalid,
    /* 0xf7 */ OpCodes::Invalid,
    /* 0xf8 */ OpCodes::Invalid,
    /* 0xf9 */ OpCodes::Invalid,
    /* 0xfa */ OpCodes::Staticcall,
    /* 0xfb */ OpCodes::Invalid,
    /* 0xfc */ OpCodes::Invalid,
    /* 0xfd */ OpCodes::Revert,
    /* 0xfe */ OpCodes::Invalid,
    /* 0xff */ OpCodes::Selfdestruct,
];
