use petgraph::dot::Dot;
use petgraph::prelude::Graph;

pub fn gen_graph(assertions: Vec<String>) {
    let mut graph = Graph::<&str, &str>::new();
    let origin = graph.add_node("Denver");
    let destination_1 = graph.add_node("San Diego");
    let destination_2 = graph.add_node("New York");

    graph.extend_with_edges(&[
        (origin, destination_1, assertions[0].as_str()),
        (origin, destination_2, "hello"),
    ]);

    println!("{}", Dot::new(&graph));
}

#[cfg(test)]
mod tests {
    use crate::utils::get_artifacts_code;
    use crate::{bytecode::to_mnemonics, prover::Prover};
    use ethabi::Contract;
    use z3::{Config, Context, SatResult};

    #[test]
    fn func_select() {
        let cfg = Config::default();
        let hex = hex::decode(
            "60003560e01c8063123456781461002157806312345679146100235760006000fd5b005b00",
        )
        .unwrap();
        let code = to_mnemonics(&hex);
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        assert_eq!(sol.check(), SatResult::Sat);
        dbg!(&tree);
        dbg!(&sol.get_assertions());
    }

    #[test]
    fn weth() {
        let bytecode = get_artifacts_code("test-data/WETH9.asm").unwrap();
        let code = to_mnemonics(&bytecode);
        let cfg = Config::default();
        let ctx = Context::new(&cfg);
        let mut prover = Prover::new(&ctx, &code, Contract::default());
        let tree = prover.run().unwrap();
        let sol = &tree[&0].0;
        assert_eq!(sol.check(), SatResult::Sat);

        dbg!(&tree);
    }
}
