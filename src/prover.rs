use crate::{
    bytecode::Mnemonics,
    data::{to_word, Calldata, EVMCalldata, EVMMemory, EVMStack, Memory, RevertReason, U256},
    opcodes::OpCodes::*,
    z3::word_to_bv,
};
use z3::{ast::Bool, Config, Context, Optimize, SatResult, Solver};

pub struct Prover<'a> {
    ctx: Context,
    code: &'a Mnemonics<'a>,
    data: Vec<u8>,
}

/// Prover step for each bytecode instruction
pub struct Step {
    // in: todo!(),
    // out: todo!(),
}

impl<'a> Prover<'a> {
    pub fn new(cfg: Config, code: &'a Mnemonics, data: Vec<u8>) -> Self {
        let ctx = Context::new(&cfg);

        Self { ctx, code, data }
    }

    pub fn run(&mut self) -> Result<Solver, RevertReason> {
        let sol = Solver::new(&self.ctx);

        let mut stack = EVMStack::new();
        let mut memory = EVMMemory::new();
        let calldata = EVMCalldata::from(self.data.clone());

        // let steps = Vec::new();

        for instruction in self.code {
            let op = instruction.op;
            let opcode = op.opcode();
            // dbg!(op);
            match opcode {
                Stop => {
                    // no output for this step
                }
                Revert => {
                    let loc = stack.pop_bv(&self.ctx, "loc")?;
                    let len = stack.pop_bv(&self.ctx, "len")?;
                }
                Lt => {
                    let a = stack.pop_bv(&self.ctx, "a")?;
                    let b = stack.pop_bv(&self.ctx, "b")?;
                    sol.assert(&b.bvult(&a));
                    sol.push();
                }
                Push0 | Push1 | Push2 | Push3 | Push4 | Push5 | Push6 | Push7 | Push8 | Push9
                | Push10 | Push11 | Push12 | Push13 | Push14 | Push15 | Push16 | Push17
                | Push18 | Push19 | Push20 | Push21 | Push22 | Push23 | Push24 | Push25
                | Push26 | Push27 | Push28 | Push29 | Push30 | Push31 | Push32 => {
                    stack.push(to_word(instruction.pushes))?;
                }
                Dup1 | Dup2 | Dup3 | Dup4 | Dup5 | Dup6 | Dup7 | Dup8 | Dup9 | Dup10 | Dup11
                | Dup12 | Dup13 | Dup14 | Dup15 | Dup16 => {
                    let dup_size = op.dup_size().unwrap();
                    stack.dupn((dup_size - 1) as usize)?;
                }
                Pop => {
                    stack.pop()?;
                }
                Calldataload => {
                    let off = stack.pop()?;
                    let val = calldata.load(U256::new(off));
                    stack.push(val)?;
                }
                Eq => {
                    let a = stack.pop()?;
                    let b = stack.pop()?;
                    let mut arr = [0; 32];
                    if a == b {
                        arr[31] = 1;
                    }
                    stack.push(arr)?;

                    // TODO: can also feed in 32 bytes ?
                    // let a = stack.pop_bv(&self.ctx, "a")?;
                    // let b = stack.pop_bv(&self.ctx, "b")?;
                    sol.assert(&Bool::from_bool(&self.ctx, b.eq(&a)));
                    // sol.assert(word_to_bv(&self.ctx, "a", a)._eq(word_to_bv(&self.ctx, "b", b)));
                    sol.push();
                }
                Mload => {
                    let off = stack.pop()?;
                    let mem = memory.mload(U256::new(off));
                    stack.push(mem)?;
                }
                Mstore => {
                    let off = stack.pop()?;
                    let val = stack.pop()?;
                    memory.mstore(U256::new(off), val);
                }
                Return => {
                    let off = U256::new(stack.pop()?);
                    let len = stack.pop()?;
                    let end = off + U256::new(len);
                    let ret = memory.mbig_load(off, end);
                    // dbg!(&ret);
                }
                op => todo!("{:?}", op),
            }
            // dbg!(&stack);
            // dbg!(&memory);
        }

        match sol.check() {
            SatResult::Sat => Ok(sol),
            _ => Err(RevertReason::Unsat),
        }
    }
}

#[cfg(test)]
use crate::to_mnemonics;

#[test]
fn prover_lt() {
    let cfg = Config::default();
    let hex = hex::decode("5F60011000").unwrap();
    // let hex = hex::decode("60015F1000").unwrap();
    let code = to_mnemonics(&hex);
    let mut prover = Prover::new(cfg, &code, vec![]);
    let sol = prover.run().unwrap();
    let model = sol.get_model();
    dbg!(&sol);
    dbg!(&model);
}

#[test]
fn reverts() {
    let cfg = Config::default();
    let hex = hex::decode("5F5FFD").unwrap();
    let code = to_mnemonics(&hex);
    let mut prover = Prover::new(cfg, &code, vec![]);
    let sol = prover.run().unwrap();
    let model = sol.get_model();
    dbg!(&sol);
    dbg!(&model);
}

#[test]
fn password() {
    let cfg = Config::default();
    let hex = hex::decode("5F35611337145F5260205FF3").unwrap();
    let code = to_mnemonics(&hex);
    let mut data = [0u8; 32];
    data[30..].copy_from_slice(&hex::decode("1337").unwrap());
    let mut prover = Prover::new(cfg, &code, data.to_vec());
    let sol = prover.run().unwrap();
    let model = sol.get_model();
    dbg!(&sol);
    dbg!(&model);
}
