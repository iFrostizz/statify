use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{
    analysis::get_jumpdest,
    bytecode::{Mnemonic, Mnemonics},
    data::{bool_to_bv, to_bv, EVMMemory, EVMStack, RevertReason},
    opcodes::OpCodes::*,
};
use ethabi::Contract;
use z3::{ast::Ast, Context, SatResult, Solver};

pub struct Prover<'a, 'ctx> {
    ctx: &'ctx Context,
    sol: Solver<'ctx>,
    code: &'a Mnemonics<'a>,
    abi: Contract,
    sym: Symbolic<'ctx>,
}

#[derive(Debug, Default, Clone)]
pub struct Ret<'ctx> {
    val: Option<z3::ast::BV<'ctx>>,
    ret: bool,
    /// wether it reverted or not
    rev: bool,
}

impl Ret<'_> {
    pub fn has_ret(&self) -> bool {
        return self.ret || self.rev;
    }
}

pub struct Symbolic<'ctx> {
    calldata: z3::FuncDecl<'ctx>,
    value: z3::FuncDecl<'ctx>,
}

impl<'ctx> Symbolic<'ctx> {
    #[rustfmt::skip]
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            calldata: z3::FuncDecl::new(ctx, "calldata", &[&z3::Sort::bitvector(ctx, 256)], &z3::Sort::bitvector(ctx, 256)),
            value: z3::FuncDecl::new(ctx, "value", &[&z3::Sort::bitvector(ctx, 256)], &z3::Sort::bitvector(ctx, 256)),
        }
    }
}

/// Prover step for each bytecode instruction
#[derive(Debug, Clone)]
pub struct Step<'a> {
    op: Mnemonic<'a>,
    stack: EVMStack<'a>,
    memory: EVMMemory<'a>,
    ret: Ret<'a>,
}

/// The full set of steps indexed by their branch id
pub type Tree<'a> = BTreeMap<usize, Vec<Step<'a>>>;

impl<'a, 'ctx> Prover<'a, 'ctx> {
    pub fn new(ctx: &'ctx Context, code: &'a Mnemonics, abi: Contract) -> Self {
        let sym = Symbolic::new(ctx);
        let sol = Solver::new(ctx);

        Self {
            ctx,
            sol,
            code,
            abi,
            sym,
        }
    }

    /// run the solver constraining algo for the given evm mnemonics.
    /// throw with a "RevertReason" in the case of the main thread having an issue.
    pub fn run(&'a mut self) -> Result<(&Solver, Tree), RevertReason> {
        let jdest = get_jumpdest(self.code.to_vec());

        let stack = EVMStack::new();
        let memory = EVMMemory::new(self.ctx);
        // TODO: extract symbolic calldata from abi

        let (tree, sol, _p) = self.walk()?;

        // output the final solver with constraints
        Ok((sol, tree))
    }

    /// entry point of branching, is the main branch with id 0
    pub fn walk(&'a mut self) -> Result<(Tree, &Solver, usize), RevertReason> {
        let jdest = get_jumpdest(self.code.to_vec());

        // main thread
        let stack = EVMStack::new();
        let memory = EVMMemory::new(self.ctx);
        let last_step = Step {
            op: *self.code.first().unwrap(),
            stack,
            memory,
            ret: Default::default(),
        };

        // TODO: handle main thread **only** stack underflow
        Self::path(
            self.ctx,
            &jdest,
            &self.sol,
            &self.sym.calldata,
            self.code,
            0,
            Default::default(),
            &mut Default::default(),
            last_step,
            0,
        )
    }

    pub fn step(
        ctx: &'a Context,
        sol: &'a Solver,
        calldata: &'a z3::FuncDecl<'ctx>,
        last_step: Step<'a>,
        instruction: Mnemonic<'a>,
    ) -> Result<Step<'a>, RevertReason> {
        let mut step = last_step;
        step.op = instruction;
        // dbg!(&step);

        let op = instruction.op;
        let opcode = op.opcode();
        match opcode {
            Stop => {
                // no output for this step
            }
            Add => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvadd(&b))?;
            }
            Lt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                sol.assert(&a.bvult(&b));
                sol.push();
            }
            Gt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                sol.assert(&a.bvugt(&b));
                sol.push();
            }
            Push0 | Push1 | Push2 | Push3 | Push4 | Push5 | Push6 | Push7 | Push8 | Push9
            | Push10 | Push11 | Push12 | Push13 | Push14 | Push15 | Push16 | Push17 | Push18
            | Push19 | Push20 | Push21 | Push22 | Push23 | Push24 | Push25 | Push26 | Push27
            | Push28 | Push29 | Push30 | Push31 | Push32 => {
                step.stack.push(to_bv(ctx, instruction.pushes))?;
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
                let load = calldata
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
                sol.assert(eq);

                step.stack.push(bool_to_bv(ctx, eq))?;
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
                step = Self::ret(step)?;
            }
            Revert => {
                step = Self::ret(step)?;
                step.ret.rev = true;
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
                step.ret.rev = true;
            }
            Jumpdest => {
                // nothing, handled by branching
            }
            Jump => {
                step.stack.pop()?;
            }
            Jumpi => {
                step.stack.pop()?;
                step.stack.pop()?;
            }
            op => todo!("{:?}", op),
        }

        Ok(step)
    }

    fn ret(mut step: Step<'a>) -> Result<Step<'a>, RevertReason> {
        let off = step.stack.pop32()?.unwrap();
        let len = step.stack.pop32()?.unwrap();
        let ret = step.memory.mbig_load(off, off + len);
        step.ret.val = Some(ret);
        Ok(step)
    }

    /// iterate on a portion of the bytecode, branch when needed
    pub fn path(
        ctx: &'ctx Context,
        jdest: &Vec<u64>,
        sol: &'a Solver<'ctx>,
        calldata: &'a z3::FuncDecl<'ctx>,
        code: &Mnemonics<'a>,
        mut pid: usize,
        tree: Rc<RefCell<Tree<'a>>>,
        vdest: &mut Vec<u64>,
        mut step: Step<'a>,
        pc: usize,
    ) -> Result<(Tree<'a>, &'a Solver<'ctx>, usize), RevertReason> {
        dbg!(&vdest);
        let last_pid = pid;

        // start the execution from the id
        for (i, instruction) in code.iter().enumerate().skip_while(|(_, ins)| ins.pc < pc) {
            let opcode = instruction.opcode();

            if opcode == &Jump || opcode == &Jumpi {
                // find potential jump dests
                let dest = if opcode == &Jump {
                    step.stack.peek(0)
                } else {
                    step.stack.peek(1)
                }?;

                // if symbolic dest, find for all valable destinations
                if !dest.is_const() {
                    for jd in jdest {
                        let dest_int = z3::ast::Int::from_u64(ctx, *jd);
                        sol.push();
                        sol.assert(&dest_int._eq(&dest.to_int(false)).simplify());
                        // check if dest is reachable
                        if sol.check() == SatResult::Sat && !vdest.contains(jd) {
                            vdest.push(*jd);

                            if let Ok((t, s, p)) = Self::path(
                                ctx,
                                jdest,
                                sol,
                                calldata,
                                code,
                                pid + 1,
                                tree.clone(),
                                vdest,
                                step.clone(),
                                *jd as usize,
                            ) {
                                pid = p;
                            }
                        }
                        sol.pop(1);
                    }
                } else if let Some(d) = dest.as_u64() {
                    if !vdest.contains(&d) {
                        vdest.push(d);

                        if jdest.contains(&d) {
                            sol.push();
                            if let Ok((t, s, p)) = Self::path(
                                ctx,
                                jdest,
                                sol,
                                calldata,
                                code,
                                pid + 1,
                                tree.clone(),
                                vdest,
                                step.clone(),
                                d as usize,
                            ) {
                                pid = p;
                            };

                            sol.pop(1);
                        }
                    } else {
                        // already visited
                    }
                } else {
                    step = Self::ret(step)?;
                    step.ret.rev = true;
                }
            }

            // also keep up with the left branch
            step = Self::step(ctx, sol, calldata, step.clone(), *instruction)?;
            let tr = tree.clone();
            let mut t = tr.borrow_mut();
            let val = t.get_mut(&last_pid);
            if let Some(steps) = val {
                steps.push(step.clone());
            } else {
                t.insert(last_pid, vec![step.clone()]);
            };

            if step.ret.has_ret() {
                // main thread has returned, get out
                break;
            }
        }

        // tree
        let tree = tree.clone();
        let tree = tree.borrow();

        Ok((tree.to_owned(), sol, pid + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_mnemonics;
    use z3::{Config, SatResult};

    #[test]
    fn lt() {
        let cfg = Config::default();
        let hex = hex::decode("5F60011000").unwrap();
        // let hex = hex::decode("60015F1000").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
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
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let (sol, _) = prover.run().unwrap();
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn password() {
        let cfg = Config::default();
        let hex = hex::decode("5F35611337145F5260205FF3").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let (sol, _) = prover.run().unwrap();
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn jump() {
        // https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27wWZjump+overqinvalid+and+jusXgoYoqpushk4x0_+++x2+%7Bprevious+instruction+occupies+2+bytes%7DzINVALIDx3_DEST%7E4k1x5%27%7E+wOffseXz%5Cnx+%7Ew%2F%2F+qYhZkzPUSH1+_zJUMPZe+Y+tXt+%01XYZ_kqwxz%7E_&fork=shanghai
        let cfg = Config::default();
        let hex = hex::decode("5F3556FE5B60015B").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let (sol, tree) = prover.run().unwrap();
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 3);
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn jumpi() {
        // https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27qFirstk%20noYjump%2C%20secondkw0%20XRY0w10z2~h4~W_z5w12z7~h9~Z0gINVALIDK11gZ2w_z13%27~%20%7Bprevious%20instruction%20occupiR%202%20bytR%7DgzXseYwgWq%2F%2F%20k%20example%20doRhQI%20%20Kg%5Cn_1%20ZQDESTz1Yt%20X%20qOffWPUSH_ResQJUMPK%20z%01KQRWXYZ_ghkqwz~_
        let cfg = Config::default();
        let hex = hex::decode("6000600a576001600C575BFE5B6001").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let (sol, tree) = prover.run().unwrap();
        println!("{:#?}", &tree);
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 3);
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn infinite() {
        let cfg = Config::default();
        let hex = hex::decode("5B5F56FE").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let (sol, tree) = prover.run().unwrap();
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    #[test]
    fn dyn_jump() {
        let cfg = Config::default();
        let hex = hex::decode("5F35600656FE5B00").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let (sol, tree) = prover.run().unwrap();
        dbg!(&tree);
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
    }

    /// only the main thread make the proving revert, not branches
    #[test]
    fn main_reverts() {
        let cfg = Config::default();
        let hex = hex::decode("5F5050").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        assert!(prover.run().is_err());

        let cfg = Config::default();
        let hex = hex::decode("600160065F5B50").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        assert!(prover.run().is_ok());
    }
}
