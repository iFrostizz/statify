use self::bytecode::to_mnemonics;
use crate::{fsm::gen_graph, prover::Prover};
use ::z3::{Config, Context, SatResult};
use ethabi::Contract;

mod analysis;
mod bytecode;
mod config;
mod data;
mod fsm;
mod helpers;
mod opcodes;
mod prover;
mod utils;
mod z3;

struct Function {
    name: String,
    calling: Option<Vec<Function>>,
}

// 1. extract all possible call-able functions (public + external)
// 2. use a DFS algo to check if B can be called after A
// 3. ?
// 4. profit

// parallelize z3: https://stackoverflow.com/questions/53246030/parallel-solving-in-z3

fn main() {
    let code = [0x5F, 0x35, 0x60, 0xFF, 0x14];
    let mnemonics = to_mnemonics(&code);
    let cfg = Config::default();
    let ctx = Context::new(&cfg);
    let mut prover = Prover::new(&ctx, &mnemonics, Contract::default());
    let tree = prover.run().unwrap();
    dbg!(&tree);

    let sol = &tree[&0].0;
    assert_eq!(sol.check(), SatResult::Sat, "Cannot be satisfied");
    let assertions = sol
        .get_assertions()
        .into_iter()
        .map(|a| format!("{:#?}", a))
        .collect();

    gen_graph(assertions);
}
