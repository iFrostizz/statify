use z3::{Config, Context, Optimize};

pub struct Prover {
    ctx: Context,
}

impl Prover {
    pub fn new(cfg: Config) -> Self {
        let ctx = Context::new(&cfg);

        Self { ctx }
    }

    pub fn opt(&mut self) -> Optimize {
        Optimize::new(&self.ctx)
    }
}
