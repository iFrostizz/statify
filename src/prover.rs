use std::collections::BTreeMap;

use crate::{
    analysis::get_jumpdest,
    bytecode::{Mnemonic, Mnemonics},
    data::{bool_to_bv, to_bv, EVMMemory, EVMStack, RevertReason},
    opcodes::OpCodes::{self, *},
};
use ethabi::Contract;
use z3::{ast::Ast, Context, SatResult, Solver};

pub struct Prover<'a, 'ctx> {
    ctx: &'ctx Context,
    sol: Solver<'ctx>,
    code: &'a Mnemonics<'a>,
    data: Vec<u8>, // TODO: switch to symbolic only
    abi: Contract,
    sym: Symbolic<'ctx>,
    ret: Ret<'ctx>,
    /// last assigned path id
    last_id: usize,
}

#[derive(Debug, Default)]
pub struct Ret<'ctx> {
    ret: Option<z3::ast::BV<'ctx>>,
}

pub struct Symbolic<'ctx> {
    calldata: z3::FuncDecl<'ctx>,
    value: z3::FuncDecl<'ctx>,
}

impl<'ctx> Symbolic<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            calldata: z3::FuncDecl::new(
                ctx,
                "calldata",
                &[&z3::Sort::bitvector(ctx, 256)],
                &z3::Sort::bitvector(ctx, 256),
            ),
            value: z3::FuncDecl::new(
                ctx,
                "value",
                &[&z3::Sort::bitvector(ctx, 256)],
                &z3::Sort::bitvector(ctx, 256),
            ),
        }
    }
}

/// Prover step for each bytecode instruction
#[derive(Debug, Clone)]
pub struct Step<'a> {
    stack: EVMStack<'a>,
    memory: EVMMemory<'a>,
}

/// The full set of steps indexed by their branch id
pub type Tree<'a> = BTreeMap<usize, Vec<Step<'a>>>;

impl<'a, 'ctx> Prover<'a, 'ctx> {
    pub fn new(ctx: &'ctx Context, code: &'a Mnemonics, data: Vec<u8>, abi: Contract) -> Self {
        let sym = Symbolic::new(ctx);
        let sol = Solver::new(ctx);
        let ret = Default::default();

        Self {
            ctx,
            sol,
            code,
            data,
            abi,
            sym,
            ret,
            last_id: 0,
        }
    }

    /// run the solver constraining algo for the given evm mnemonics.
    /// throw with a "RevertReason" in the case of the main thread having an issue.
    pub fn run(&mut self) -> Result<(&Solver, Tree, Ret), RevertReason> {
        let jdest = get_jumpdest(self.code.to_vec());

        let stack = EVMStack::new();
        let memory = EVMMemory::new(self.ctx);
        // TODO: extract symbolic calldata from abi

        let (tree, sol) = self.walk();
        dbg!(&tree);

        // output the final solver with constraints
        Ok((sol, Default::default(), Default::default()))
    }

    /// entry point of branching, is the main branch with id 0
    pub fn walk(&mut self) -> (Tree, &Solver) {
        let jdest = get_jumpdest(self.code.to_vec());

        // main thread
        let stack = EVMStack::new();
        let memory = EVMMemory::new(self.ctx);
        let last_step = Step { stack, memory };
        // TODO: handle main thread stack underflow
        self.path(&mut 0, &mut Default::default(), last_step, 0)
    }

    pub fn step(
        &mut self,
        last_step: Step<'ctx>,
        instruction: Mnemonic,
    ) -> Result<Step, RevertReason> {
        let mut step = last_step;

        let op = instruction.op;
        let opcode = op.opcode();
        match opcode {
            Stop => {
                // no output for this step
            }
            Revert => {
                todo!()
                // let loc = stack.pop()?;
                // let len = stack.pop()?;
            }
            Lt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                self.sol.assert(&a.bvult(&b));
                self.sol.push();
            }
            Gt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                self.sol.assert(&a.bvugt(&b));
                self.sol.push();
            }
            Add => {
                todo!()
                // let a = stack.pop()?;
                // let b = stack.pop()?;
            }
            Push0 | Push1 | Push2 | Push3 | Push4 | Push5 | Push6 | Push7 | Push8 | Push9
            | Push10 | Push11 | Push12 | Push13 | Push14 | Push15 | Push16 | Push17 | Push18
            | Push19 | Push20 | Push21 | Push22 | Push23 | Push24 | Push25 | Push26 | Push27
            | Push28 | Push29 | Push30 | Push31 | Push32 => {
                step.stack.push(to_bv(self.ctx, instruction.pushes))?;
            }
            Dup1 | Dup2 | Dup3 | Dup4 | Dup5 | Dup6 | Dup7 | Dup8 | Dup9 | Dup10 | Dup11
            | Dup12 | Dup13 | Dup14 | Dup15 | Dup16 => {
                let dup_size = op.dup_size().unwrap();
                step.stack.dupn((dup_size - 1) as usize)?;
            }
            Pop => {
                step.stack.pop()?;
            }
            Calldataload => {
                let off = step.stack.pop()?;
                let load = self
                    .sym
                    .calldata
                    .apply(&[&off])
                    .as_bv()
                    .expect("couldn't convert calldata into a bitvector")
                    .simplify();

                step.stack.push(load)?;
            }
            Eq => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;

                let eq = &a._eq(&b).simplify();
                self.sol.assert(eq);

                step.stack.push(bool_to_bv(self.ctx, eq))?;
            }
            Mload => {
                todo!()
                // let off = stack.pop()?;
                // let mem = memory.mload(U256::new(off));
                // stack.push(mem)?;
            }
            Mstore => {
                let off = step.stack.pop32()?.unwrap();
                let val = step.stack.pop()?;
                step.memory.mstore(off, val);
            }
            Return => {
                let off = step.stack.pop32()?.unwrap();
                let len = step.stack.pop32()?.unwrap();
                let ret = step.memory.mbig_load(off, off + len);
                self.ret.ret = Some(ret);
            }
            // Jump => {
            //     let to = stack.pop()?.to_int(false);
            //     for dest in &jdest {
            //         self.sol.push();
            //         let dest = z3::ast::Int::from_u64(self.ctx, *dest);
            //         self.sol.assert(&dest._eq(&to).simplify());
            //         if self.sol.check() == SatResult::Sat {
            //             // create branch
            //             dbg!("sat!");
            //         } else {
            //             dbg!("unsat!", dest);
            //         }
            //         self.sol.pop(1);
            //     }
            // }
            Invalid => {
                // revert with (0, 0)
            }
            Jumpdest | Jump | Jumpi => {
                // nothing, handled by branching
            }
            op => todo!("{:?}", op),
        }

        Ok(step)
    }

    /// iterate on a portion of the bytecode, branch when needed
    pub fn path(
        &mut self,
        pid: &mut usize,
        tree: &mut Tree,
        step: Step,
        inst_id: usize,
    ) -> (Tree, &Solver) {
        let id = self.last_id;

        // start the execution from the id
        for (i, instruction) in self.code.iter().enumerate().skip(inst_id) {
            let opcode = instruction.opcode();

            if opcode == &Jump || opcode == &Jumpi {
                // create a new path by following the potential jump destinations
                // TODO: handle while dropping this path if any stack out of bounds or invalid instruction
                *pid += 1;
                // self.path(pid, tree, step, i);
            }

            // let step = self.step(step, *instruction).unwrap();
            // let mut val = tree
            //     .get_mut(&self.last_id)
            //     .get_or_insert(&mut Default::default());
            // val.push(step);
        }

        self.last_id += 1;

        // tree
        (Default::default(), &self.sol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_mnemonics;
    use z3::Config;

    #[test]
    fn prover_lt() {
        let cfg = Config::default();
        let hex = hex::decode("5F60011000").unwrap();
        // let hex = hex::decode("60015F1000").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, vec![], Contract::default());
        let (sol, ..) = prover.run().unwrap();
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn reverts() {
        let cfg = Config::default();
        let hex = hex::decode("5F5FFD").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, vec![], Contract::default());
        let (sol, ..) = prover.run().unwrap();
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn password() {
        let cfg = Config::default();
        let hex = hex::decode("5F35611337145F5260205FF3").unwrap();
        let code = to_mnemonics(&hex);
        // let mut data = [0u8; 32];
        // data[30..].copy_from_slice(&hex::decode("1337").unwrap());
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(
            &ctx,
            &code,
            // data.to_vec(),
            vec![],
            Contract::default(),
        );
        let (sol, _, ret) = prover.run().unwrap();
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
        dbg!(&ret);
    }

    #[test]
    fn jump() {
        // https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27wWZjump+overqinvalid+and+jusXgoYoqpushk4x0_+++x2+%7Bprevious+instruction+occupies+2+bytes%7DzINVALIDx3_DEST%7E4k1x5%27%7E+wOffseXz%5Cnx+%7Ew%2F%2F+qYhZkzPUSH1+_zJUMPZe+Y+tXt+%01XYZ_kqwxz%7E_&fork=shanghai
        let cfg = Config::default();
        let hex = hex::decode("5F3556FE5B60015B").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, vec![], Contract::default());
        let (sol, _, ret) = prover.run().unwrap();
        assert_eq!(sol.check(), SatResult::Sat);
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
        dbg!(&ret);
    }
}
