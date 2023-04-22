use crate::{
    bytecode::Mnemonics,
    data::{to_word, Memory, Stack},
    opcodes::OpCodes::*,
    z3::word_to_bv,
};
use z3::{ast::BV, Config, Context, Optimize};

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

        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // let steps = Vec::new();

        for instruction in self.code {
            match instruction.op.opcode() {
                Stop => {
                    // no output for this step
                }
                Lt => {
                    let a: BV = word_to_bv(
                        &self.ctx,
                        // &format!("a_{}", instruction.pc),
                        "a",
                        stack.pop().unwrap(),
                    );
                    let b: BV = word_to_bv(&self.ctx, "b", stack.pop().unwrap());
                    opt.assert(&b.bvult(&a));
                }
                Push0 | Push1 => {
                    // TODO other pushes
                    stack.push(to_word(instruction.pushes)).unwrap();
                }
                op => todo!("{:?}", op),
            }
        }

        opt
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
    let model = opt.get_model();
    dbg!(&model);
}
