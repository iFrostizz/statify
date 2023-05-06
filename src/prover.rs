use crate::{
    bytecode::Mnemonics,
    data::{bool_to_bv, to_bv, EVMMemory, EVMStack, RevertReason},
    opcodes::OpCodes::*,
};
use ethabi::Contract;
use z3::{ast::Ast, Context, SatResult, Solver};

pub struct Prover<'a, 'ctx> {
    ctx: &'ctx Context,
    code: &'a Mnemonics<'a>,
    data: Vec<u8>, // TODO: switch to symbolic only
    abi: Contract,
    sym: Symbolic<'ctx>,
    ret: Ret<'ctx>,
}

/// Prover step for each bytecode instruction
pub struct Step {
    // in: todo!(),
    // out: todo!(),
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

impl<'a, 'ctx> Prover<'a, 'ctx> {
    pub fn new(ctx: &'ctx Context, code: &'a Mnemonics, data: Vec<u8>, abi: Contract) -> Self {
        // let ctx = Context::new(&cfg);
        let sym = Symbolic::new(ctx);
        let ret = Default::default();

        Self {
            ctx,
            code,
            data,
            abi,
            sym,
            ret,
        }
    }

    pub fn run<'m>(&'m mut self) -> Result<(Solver, Ret), RevertReason> {
        let sol = Solver::new(self.ctx);

        let mut stack = EVMStack::new();
        let mut memory = EVMMemory::new(self.ctx);
        // let calldata = EVMCalldata::from(self.data.clone());
        // TODO: extract symbolic calldata from abi
        // let calldata: z3::ast::BV = z3::ast::BV::from_u64(self.ctx, 0, 32);
        let calldata: z3::ast::BV = z3::ast::BV::new_const(self.ctx, "calldata", 256);
        // let calldata = self.sym.calldata.apply(args)
        let cds = calldata.get_size() as u64;

        // let max_cds = z3::ast::Int::from_u64(self.ctx, u64::max_value());

        // let steps = Vec::new();

        for instruction in self.code.iter() {
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
                    let a = stack.pop()?;
                    let b = stack.pop()?;
                    sol.assert(&a.bvult(&b));
                    sol.push();
                }
                Gt => {
                    let a = stack.pop()?;
                    let b = stack.pop()?;
                    sol.assert(&a.bvugt(&b));
                    sol.push();
                }
                Add => {
                    todo!()
                    // let a = stack.pop()?;
                    // let b = stack.pop()?;
                }
                Push0 | Push1 | Push2 | Push3 | Push4 | Push5 | Push6 | Push7 | Push8 | Push9
                | Push10 | Push11 | Push12 | Push13 | Push14 | Push15 | Push16 | Push17
                | Push18 | Push19 | Push20 | Push21 | Push22 | Push23 | Push24 | Push25
                | Push26 | Push27 | Push28 | Push29 | Push30 | Push31 | Push32 => {
                    stack.push(to_bv(self.ctx, instruction.pushes))?;
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
                    // let val = if let Some(off) = stack.pop64()? {
                    //     if off >= cds + 256 {
                    //         z3::ast::BV::from_u64(self.ctx, 0, 256)
                    //     } else if off >= cds {
                    //         let cd = calldata
                    //             .zero_ext((off + 256 - cds).try_into().unwrap())
                    //             .clone();
                    //         cd.extract((off + 255).try_into().unwrap(), off.try_into().unwrap())
                    //     } else {
                    //         calldata
                    //             .extract((off + 255).try_into().unwrap(), off.try_into().unwrap())
                    //     }
                    // } else {
                    //     panic!("issue popping 64 bits val from stack for calldataload")
                    // };

                    // stack.push(val)?;

                    let off = stack.pop()?;
                    dbg!(&off);
                    let load = self
                        .sym
                        .calldata
                        .apply(&[&off])
                        .as_bv()
                        .expect("couldn't convert calldata into a bitvector");
                    dbg!(&load);

                    stack.push(load)?;
                }
                Eq => {
                    let a = stack.pop()?;
                    let b = stack.pop()?;

                    let eq = &a._eq(&b).simplify();
                    sol.assert(eq);

                    stack.push(bool_to_bv(self.ctx, eq))?;
                }
                Mload => {
                    todo!()
                    // let off = stack.pop()?;
                    // let mem = memory.mload(U256::new(off));
                    // stack.push(mem)?;
                }
                Mstore => {
                    let off = stack.pop32()?.unwrap();
                    let val = stack.pop()?;
                    memory.mstore(off, val);
                }
                Return => {
                    let off = stack.pop32()?.unwrap();
                    let len = stack.pop32()?.unwrap();
                    dbg!(off, len);
                    let ret = memory.mbig_load(off, off + len);
                    self.ret.ret = Some(ret);
                }
                op => todo!("{:?}", op),
            }
            dbg!(op);
            dbg!(&stack);
            dbg!(&memory);
        }

        match sol.check() {
            SatResult::Sat => Ok((sol, Default::default())),
            _ => Err(RevertReason::Unsat),
        }
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
        let (sol, _) = prover.run().unwrap();
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
        let (sol, ret) = prover.run().unwrap();
        let model = sol.get_model();
        dbg!(&sol);
        dbg!(&model);
        dbg!(&ret);
    }
}
