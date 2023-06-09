use crate::{
    analysis::get_jumpdest,
    bytecode::{Mnemonic, Mnemonics},
    data::{EVMMemory, EVMStack},
    helpers::{bool_to_bv, is_zero, to_bv, RevertReason},
    opcodes::OpCodes::*,
};
use ethabi::Contract;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
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
        self.ret || self.rev
    }
}

#[derive(Debug)]
pub struct Symbolic<'ctx> {
    calldata: z3::FuncDecl<'ctx>,
    value: z3::FuncDecl<'ctx>,
    address: z3::FuncDecl<'ctx>,
    caller: z3::FuncDecl<'ctx>,
    origin: z3::FuncDecl<'ctx>,
    balance_of: z3::FuncDecl<'ctx>,
    calldatasize: z3::FuncDecl<'ctx>,
    codesize: z3::FuncDecl<'ctx>,
    gasprice: z3::FuncDecl<'ctx>,
}

impl<'ctx> Symbolic<'ctx> {
    #[rustfmt::skip]
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            calldata: z3::FuncDecl::new(ctx, "calldata", &[&z3::Sort::bitvector(ctx, 256)], &z3::Sort::bitvector(ctx, 256)),
            value: z3::FuncDecl::new(ctx, "value", &[&z3::Sort::bitvector(ctx, 256)], &z3::Sort::bitvector(ctx, 256)),
            caller: z3::FuncDecl::new(ctx, "caller", &[&z3::Sort::bitvector(ctx, 256)], &z3::Sort::bitvector(ctx, 256)),
            origin: z3::FuncDecl::new(ctx, "origin", &[], &z3::Sort::bitvector(ctx, 256)),
            address: z3::FuncDecl::new(ctx, "address", &[], &z3::Sort::bitvector(ctx, 256)),
            balance_of: z3::FuncDecl::new(ctx, "balance_of", &[&z3::Sort::bitvector(ctx, 256)], &z3::Sort::bitvector(ctx, 256)),
            calldatasize: z3::FuncDecl::new(ctx, "calldatasize", &[], &z3::Sort::bitvector(ctx, 256)),
            codesize: z3::FuncDecl::new(ctx, "codesize", &[], &z3::Sort::bitvector(ctx, 256)),
            gasprice: z3::FuncDecl::new(ctx, "gasprice", &[], &z3::Sort::bitvector(ctx, 256)),
        }
    }
}

/// Prover step for each bytecode instruction
#[derive(Debug, Clone)]
pub struct Step<'a, 'ctx> {
    op: Mnemonic<'a>,
    stack: EVMStack<'ctx>,
    memory: EVMMemory<'ctx>,
    ret: Ret<'ctx>,
}

/// The full set of steps indexed by their branch id
pub type Tree<'a, 'ctx> = BTreeMap<usize, (Solver<'ctx>, Vec<Step<'a, 'ctx>>)>;

// lifetime of the prover should outlive its context
impl<'a: 'ctx, 'ctx> Prover<'a, 'ctx> {
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
    pub fn run(&'a self) -> Result<Tree<'a, 'ctx>, RevertReason> {
        let jdest = get_jumpdest(self.code.to_vec());

        let stack = EVMStack::new();
        let memory = EVMMemory::new(self.ctx);
        // TODO: extract symbolic calldata from abi

        let (tree, _p) = self.walk()?;

        // output the final solver with constraints
        Ok(tree)
    }

    /// entry point of branching, is the main branch with id 0
    pub fn walk(&'a self) -> Result<(Tree<'a, 'ctx>, usize), RevertReason> {
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

        Self::path(
            self.ctx,
            &jdest,
            &self.sym,
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
        sym: &'a Symbolic<'ctx>,
        last_step: Step<'a, 'ctx>,
        instruction: Mnemonic<'a>,
    ) -> Result<Step<'a, 'ctx>, RevertReason> {
        let mut step = last_step;
        step.op = instruction;

        let op = instruction.op;
        let opcode = op.opcode();
        // dbg!(&opcode);
        match opcode {
            Stop => {
                // no output for this step
            }
            Add => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvadd(&b))?;
            }
            Mul => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvmul(&b))?;
            }
            Sub => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvsub(&b))?;
            }
            Div => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvudiv(&b))?;
            }
            Sdiv => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvsdiv(&b))?;
            }
            Mod => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvurem(&b))?;
            }
            Smod => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvsmod(&b))?;
            }
            Addmod => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                let n = step.stack.pop()?;
                step.stack.push(a.bvadd(&b).bvurem(&n))?;
            }
            Mulmod => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                let n = step.stack.pop()?;
                step.stack.push(a.bvmul(&b).bvurem(&n))?;
            }
            // Exp => {
            //     todo!();
            // }
            Signextend => {
                let a = step.stack.pop()?;
                let b = step.stack.pop32()?.unwrap();
                step.stack.push(a.sign_ext(b))?;
            }
            Lt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(bool_to_bv(ctx, &a.bvult(&b)))?;
            }
            Gt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(bool_to_bv(ctx, &a.bvugt(&b)))?;
            }
            Slt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(bool_to_bv(ctx, &a.bvslt(&b)))?;
            }
            Sgt => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(bool_to_bv(ctx, &a.bvsgt(&b)))?;
            }
            Eq => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                let eq = &a._eq(&b).simplify();
                step.stack.push(bool_to_bv(ctx, eq))?;
            }
            Iszero => {
                let a = step.stack.pop()?;
                step.stack.push(is_zero(ctx, &a))?;
            }
            And => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvand(&b))?;
            }
            Or => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvor(&b))?;
            }
            Xor => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvxor(&b))?;
            }
            Not => {
                let a = step.stack.pop()?;
                step.stack.push(a.bvnot())?;
            }
            Byte => {
                let i = step.stack.pop()?;
                let res = if let Some(x) = step.stack.pop32()? {
                    if x < u32::max_value() - 32 {
                        i.extract(x + 255, x)
                    } else {
                        z3::ast::BV::from_u64(ctx, 0, 256)
                    }
                } else {
                    z3::ast::BV::from_u64(ctx, 0, 256)
                };

                step.stack.push(res)?;
            }
            Shl => {
                let shift = step.stack.pop()?;
                let value = step.stack.pop()?;
                step.stack.push(value.bvshl(&shift))?;
            }
            Shr => {
                let shift = step.stack.pop()?;
                let value = step.stack.pop()?;
                step.stack.push(value.bvlshr(&shift))?;
            }
            Sar => {
                let a = step.stack.pop()?;
                let b = step.stack.pop()?;
                step.stack.push(a.bvashr(&b))?;
            }
            Sha3 => {
                let off = step.stack.pop32()?.unwrap();
                let size = step.stack.pop32()?.unwrap();
                let part = step.memory.mbig_load(off, off + size);
                dbg!(&part);
                let hash = Self::sha3(ctx, &part);
                step.stack.push(hash)?;
            }
            Address => {
                step.stack.push(sym.address.apply(&[]).as_bv().unwrap())?;
            }
            Balance => {
                let address = step.stack.pop()?;
                step.stack
                    .push(sym.balance_of.apply(&[&address]).as_bv().unwrap())?;
            }
            Origin => {
                step.stack.push(sym.origin.apply(&[]).as_bv().unwrap())?;
            }
            Caller => {
                // step.stack.push(sym.caller.apply(&[]).as_bv().unwrap())?;
                // TODO: should it be constant or not ?
                // Probably should, and write the caller address in step
                step.stack.push(
                    sym.caller
                        .apply(&[&z3::ast::BV::from_u64(ctx, 0, 256)])
                        .as_bv()
                        .unwrap(),
                )?;
            }
            Callvalue => {
                // step.stack.push(
                //     sym.value
                //         .apply(&[&z3::ast::BV::from_u64(ctx, 0, 256)])
                //         .as_bv()
                //         .unwrap(),
                // )?;
                step.stack.push(z3::ast::BV::from_u64(ctx, 0, 256))?;
            }
            Calldataload => {
                let off = step.stack.pop()?;
                let load = sym
                    .calldata
                    .apply(&[&off])
                    .as_bv()
                    .expect("couldn't convert calldata into a bitvector")
                    .simplify();

                step.stack.push(load)?;
            }
            Calldatasize => {
                step.stack
                    .push(sym.calldatasize.apply(&[]).as_bv().unwrap())?;
            }
            Codesize => {
                let address = sym.address.apply(&[]).as_bv().unwrap();
                step.stack
                    .push(sym.codesize.apply(&[&address]).as_bv().unwrap())?;
            }
            Codecopy => {
                let addr = sym.address.apply(&[]).as_bv().unwrap();
                let dest_off = step.stack.pop32()?.unwrap();
                let off = step.stack.pop32()?.unwrap();
                let size = step.stack.pop32()?.unwrap();
                step = Self::code_copy(ctx, addr, dest_off, off, size, step)?;
            }
            Gasprice => {
                step.stack.push(sym.gasprice.apply(&[]).as_bv().unwrap())?;
            }
            Extcodesize => {
                let address = step.stack.pop()?;
                step.stack
                    .push(sym.codesize.apply(&[&address]).as_bv().unwrap())?;
            }
            Extcodecopy => {
                let addr = step.stack.pop()?;
                let dest_off = step.stack.pop32()?.unwrap();
                let off = step.stack.pop32()?.unwrap();
                let size = step.stack.pop32()?.unwrap();
                step = Self::code_copy(ctx, addr, dest_off, off, size, step)?;
            }
            Returndatasize => {
                let size = if let Some(val) = &step.ret.val {
                    val.get_size()
                } else {
                    0
                };
                step.stack
                    .push(z3::ast::BV::from_u64(ctx, size.into(), 256))?;
            }
            // Returndatacopy => {
            //     todo!();
            // }
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
            Swap1 | Swap2 | Swap3 | Swap4 | Swap5 | Swap6 | Swap7 | Swap8 | Swap9 | Swap10
            | Swap11 | Swap12 | Swap13 | Swap14 | Swap15 | Swap16 => {
                let swap = op.swap_size().unwrap();
                step.stack.swapn((swap - 1) as usize)?;
            }
            Pop => {
                step.stack.pop()?;
            }
            Mload => {
                let off = step.stack.pop32()?.unwrap();
                let mem = step.memory.mload(off);
                step.stack.push(mem)?;
            }
            Mstore => {
                let off = step.stack.pop32()?.unwrap();
                let val = step.stack.pop()?;
                step.memory.mstore(off, val);
            }
            Return => {
                step = Self::ret(ctx, step)?;
            }
            Revert => {
                step = Self::ret(ctx, step)?;
                step.ret.rev = true;
            }
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

        // dbg!(&step);

        Ok(step)
    }

    fn ret(ctx: &'a Context, mut step: Step<'a, 'ctx>) -> Result<Step<'a, 'ctx>, RevertReason> {
        dbg!(&step.stack);
        let len = step.stack.peek(1)?;
        if len.eq(&z3::ast::BV::from_u64(ctx, 0, 256)) {
            step.ret.val = None;
        } else {
            let len = len.as_u64().unwrap() as u32;
            let off = step.stack.pop32()?.unwrap();
            step.stack.pop()?;
            let ret = step.memory.mbig_load(off, off + len);
            step.ret.val = Some(ret);
        }

        Ok(step)
    }

    fn code_copy(
        ctx: &'a Context,
        addr: z3::ast::BV<'a>,
        dest_off: u32,
        off: u32,
        size: u32,
        mut step: Step<'a, 'ctx>,
    ) -> Result<Step<'a, 'ctx>, RevertReason> {
        let codecopy = z3::FuncDecl::new(
            ctx,
            "codecopy",
            &[
                &z3::Sort::bitvector(ctx, 256),
                &z3::Sort::bitvector(ctx, 256),
                &z3::Sort::bitvector(ctx, 256),
            ],
            &z3::Sort::bitvector(ctx, size),
        );

        let code = codecopy
            .apply(&[
                &addr,
                &z3::ast::BV::from_u64(ctx, off.into(), 256),
                &z3::ast::BV::from_u64(ctx, size.into(), 256),
            ])
            .as_bv()
            .unwrap();

        step.memory.mbig_store(dest_off, code);

        Ok(step)
    }

    /// compute the symbolic keccak256 of an arbitrary length bitvector
    fn sha3(ctx: &'a Context, part: &z3::ast::BV<'a>) -> z3::ast::BV<'a> {
        let sha3 = z3::FuncDecl::new(
            ctx,
            "sha3",
            &[&z3::Sort::bitvector(ctx, part.get_size())],
            &z3::Sort::bitvector(ctx, 256),
        );

        sha3.apply(&[part]).as_bv().unwrap()
    }

    /// iterate on a portion of the bytecode, branch when needed
    fn path(
        ctx: &'ctx Context,
        jdest: &Vec<u64>,
        sym: &'a Symbolic<'ctx>, // unhappy path's solver
        code: &Mnemonics<'a>,
        mut pid: usize,
        tree: Rc<RefCell<Tree<'a, 'ctx>>>,
        vdest: &mut Vec<u64>,
        mut step: Step<'a, 'ctx>,
        pc: usize,
    ) -> Result<(Tree<'a, 'ctx>, usize), RevertReason> {
        let last_pid = pid;
        let trc = tree.clone();
        let t: &BTreeMap<_, (_, _)> = &trc.as_ref().borrow().clone();
        let sol = match &t.get(&pid) {
            Some(v) => v.0.clone(),
            None => Solver::new(ctx),
        };
        drop(trc);

        // start the execution from the id
        for instruction in code.iter().skip_while(|ins| ins.pc < pc) {
            let opcode = instruction.opcode();

            if opcode == &Jump || opcode == &Jumpi {
                // find potential jump dests
                let dest = step.stack.peek(0)?;

                dbg!(&dest);
                // dbg!(&dest.is_app());
                // dbg!(&dest.is_const());

                let cond = if opcode == &Jumpi {
                    step.stack.peek(1)?
                } else {
                    z3::ast::BV::from_u64(ctx, 1, 256)
                };

                // dbg!(&cond);

                // Y shouldn't it be 0 tho ?
                // let is_reachable = cond.ne(&z3::ast::BV::from_u64(ctx, 0, 256)) || !cond.is_const();
                // if opcode == &Jumpi {
                //     sol.assert(
                //         &cond
                //             ._eq(&z3::ast::BV::from_u64(ctx, 0, 256))
                //             .not()
                //             .simplify(),
                //     );
                // }

                if true {
                    // if symbolic dest, find for all valable destinations
                    if !dest.is_const() {
                        // for each potential jump dest
                        for jd in jdest {
                            let dest_int = z3::ast::Int::from_u64(ctx, *jd);
                            sol.push();
                            sol.assert(&dest_int._eq(&dest.to_int(false)).simplify());
                            // check if dest is reachable
                            if sol.check() == SatResult::Sat && !vdest.contains(jd) {
                                vdest.push(*jd);

                                // TODO: watch out for infinite loops !
                                if let Ok((t, p)) = Self::path(
                                    ctx,
                                    jdest,
                                    sym,
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
                                if let Ok((t, p)) = Self::path(
                                    ctx,
                                    jdest,
                                    sym,
                                    code,
                                    pid + 1,
                                    tree.clone(),
                                    vdest,
                                    step.clone(),
                                    d as usize,
                                ) {
                                    pid = p;
                                }

                                sol.pop(1);
                            }
                        } else {
                            // already visited
                        }
                    } else {
                        step = Self::ret(ctx, step)?;
                        step.ret.rev = true;
                    }
                } else {
                    dbg!("unreachable!");
                }
            }

            // also keep up with the left branch
            step = Self::step(ctx, sym, step.clone(), *instruction)?;
            let tr = tree.clone();
            let mut t = tr.borrow_mut();
            if let Some((_sol, steps)) = t.get_mut(&last_pid) {
                steps.push(step.clone());
                // keep up with this solver
                *_sol = sol.clone();
            } else {
                t.insert(last_pid, (sol.clone(), vec![step.clone()]));
            };

            if step.ret.has_ret() {
                // main thread has returned, get out
                break;
            }
        }

        // tree
        let tree = tree.clone();
        let tree: &BTreeMap<_, _> = &tree.as_ref().borrow();

        Ok((tree.to_owned(), pid + 1))
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
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        let model = sol.get_model();
        dbg!(&sol);
        // dbg!(&model);
    }

    #[test]
    fn reverts() {
        let cfg = Config::default();
        let hex = hex::decode("5F5FFD").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        let model = sol.get_model();
        dbg!(&sol);
        // dbg!(&model);
    }

    #[test]
    fn password() {
        let cfg = Config::default();
        let hex = hex::decode("5F35611337145F5260205FF3").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        let model = sol.get_model();
        dbg!(&sol);
        // dbg!(&model);
    }

    #[test]
    fn jump() {
        // https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27wWZjump+overqinvalid+and+jusXgoYoqpushk4x0_+++x2+%7Bprevious+instruction+occupies+2+bytes%7DzINVALIDx3_DEST%7E4k1x5%27%7E+wOffseXz%5Cnx+%7Ew%2F%2F+qYhZkzPUSH1+_zJUMPZe+Y+tXt+%01XYZ_kqwxz%7E_&fork=shanghai
        let cfg = Config::default();
        let hex = hex::decode("5F3556FE5B60015B").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        dbg!(&sol);
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 3);
        let model = sol.get_model();
        // dbg!(&model);
    }

    #[test]
    fn jumpi() {
        // https://www.evm.codes/playground?unit=Wei&codeType=Mnemonic&code=%27qFirstk%20noYjump%2C%20secondkw0%20XRY0w10z2~h4~W_z5w12z7~h9~Z0gINVALIDK11gZ2w_z13%27~%20%7Bprevious%20instruction%20occupiR%202%20bytR%7DgzXseYwgWq%2F%2F%20k%20example%20doRhQI%20%20Kg%5Cn_1%20ZQDESTz1Yt%20X%20qOffWPUSH_ResQJUMPK%20z%01KQRWXYZ_ghkqwz~_
        let cfg = Config::default();
        let hex = hex::decode("6000600a576001600C575BFE5B6001").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        assert_eq!(sol.check(), SatResult::Sat);
        // has 2 JUMPI, but only one branch is reachable
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        dbg!(&sol);
        // dbg!(&model);
    }

    #[test]
    fn infinite() {
        let cfg = Config::default();
        let hex = hex::decode("5B5F56FE").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        dbg!(&sol);
        // dbg!(&model);
    }

    #[test]
    fn dyn_jump() {
        let cfg = Config::default();
        let hex = hex::decode("5F35600656FE5B00").unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        dbg!(&sol);
        // dbg!(&model);
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

    #[test]
    fn payable() {
        // https://github.com/huff-language/huffmate/blob/main/src/auth/NonPayable.huff
        let cfg = Config::default();
        let hex = hex::decode(
            "6000600b34156100225763e342daa4600052602060045260245260445260806000fd5b5050",
        )
        .unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        // dbg!(&tree);
        assert_eq!(sol.check(), SatResult::Sat);
        assert_eq!(tree.keys().len(), 2);
        let model = sol.get_model();
        dbg!(&sol);
        let assertions = &sol.get_assertions();
        for assertion in assertions {
            dbg!(&assertion);
        }
        // dbg!(&model);
    }
}
