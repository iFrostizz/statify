use self::bytecode::to_mnemonics;

mod analysis;
mod bytecode;
mod opcodes;
mod utils;

struct Function {
    name: String,
    calling: Option<Vec<Function>>,
}

// 1. extract all possible call-able functions (public + external)
// 2. use a DFS algo to check if B can be called after A
// 3. ?
// 4. profit

fn main() {
    let code = [0x60, 0x10];
    let mnemonics = to_mnemonics(&code);

    dbg!(&mnemonics);
}
