use self::bytecode::to_mnemonics;

mod analysis;
mod bytecode;
mod data;
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
    let code = [0x60, 0x10];
    let mnemonics = to_mnemonics(&code);

    dbg!(&mnemonics);
}
