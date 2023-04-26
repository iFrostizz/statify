use crate::{
    bytecode::Mnemonics,
    data::{to_word, EVMStack, Memory, Stack},
    opcodes::OpCodes::*,
    z3::word_to_bv,
};
use z3::{
    ast::{self, BV},
    Config, Context, Optimize, SatResult,
};

pub struct Prover<'a> {
    ctx: Context,
    code: &'a Mnemonics<'a>,
}

/// Prover step for each bytecode instruction
pub struct Step {
    // in: todo!(),
    // out: todo!(),
}

impl<'a> Prover<'a> {
    pub fn new(cfg: Config, code: &'a Mnemonics) -> Self {
        let ctx = Context::new(&cfg);

        Self { ctx, code }
    }

    pub fn run(&mut self) -> Optimize {
        let opt = Optimize::new(&self.ctx);

        let mut stack = EVMStack::new();
        let mut memory = Memory::new();

        // let steps = Vec::new();

        for instruction in self.code {
            let op = instruction.op;
            let opcode = op.opcode();
            match opcode {
                Stop => {
                    // no output for this step
                }
                Revert => {
                    let loc = stack.pop_bv(&self.ctx, "loc").unwrap();
                    let len = stack.pop_bv(&self.ctx, "len").unwrap();
                }
                Lt => {
                    let a = stack.pop_bv(&self.ctx, "a").unwrap();
                    let b = stack.pop_bv(&self.ctx, "b").unwrap();
                    opt.assert(&b.bvult(&a));
                }
                Push0 | Push1 | Push2 | Push3 | Push4 | Push5 | Push6 | Push7 | Push8 | Push9
                | Push10 | Push11 | Push12 | Push13 | Push14 | Push15 | Push16 | Push17
                | Push18 | Push19 | Push20 | Push21 | Push22 | Push23 | Push24 | Push25
                | Push26 | Push27 | Push28 | Push29 | Push30 | Push31 => {
                    stack.push(to_word(instruction.pushes));
                }
                Dup1 | Dup2 | Dup3 | Dup4 | Dup5 | Dup6 | Dup7 | Dup8 | Dup9 | Dup10 | Dup11
                | Dup12 | Dup13 | Dup14 | Dup15 | Dup16 => {
                    let dup_size = op.dup_size().unwrap();
                    stack.dupn((dup_size - 1) as usize);
                }
                Pop => {
                    stack.pop();
                }
                op => todo!("{:?}", op),
            }
        }

        if SatResult::Sat == opt.check(&[]) {
            dbg!("sat");
            opt
        } else {
            opt
        }
    }
}

#[cfg(test)]
use crate::to_mnemonics;

#[test]
fn prover_lt() {
    let cfg = Config::default();
    let hex = hex::decode("5f60011000").unwrap();
    let code = to_mnemonics(&hex);
    let mut prover = Prover::new(cfg, &code);
    let opt = prover.run();
    // let model = opt.get_model();
    // dbg!(&model);

    dbg!(opt.get_objectives());
}

#[test]
fn reverts() {
    let cfg = Config::default();
    let hex = hex::decode("5f5ffd").unwrap();
    let code = to_mnemonics(&hex);
    let mut prover = Prover::new(cfg, &code);
    let opt = prover.run();
    // let model = opt.get_model();
    // dbg!(&model);

    dbg!(opt.get_objectives());
}

#[test]
fn password() {
    let cfg = Config::default();
    let hex = hex::decode("5f3561133714").unwrap();
    let code = to_mnemonics(&hex);
    let mut prover = Prover::new(cfg, &code);
    let opt = prover.run();
    // let model = opt.get_model();
    // dbg!(&model);

    dbg!(opt.get_objectives());
}
