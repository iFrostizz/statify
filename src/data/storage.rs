use std::collections::HashMap;

use z3::Context;

// we don't need a generalized data structure, that would be complex for no reason
#[derive(Debug, Default, Clone)]
pub struct EVMStorage<'ctx>(HashMap<z3::ast::BV<'ctx>, z3::ast::BV<'ctx>>);

impl<'ctx> EVMStorage<'ctx> {
    pub fn new() -> Self {
        Default::default()
    }

    /// store a value at key
    pub fn sstore(&mut self, ctx: &Context, key: z3::ast::BV, value: z3::ast::BV) {
        assert_eq!(key.get_size(), 256);
        assert_eq!(value.get_size(), 256);

        let off = key.bvurem(&z3::ast::BV::from_u64(ctx, 32, 256));
        // if off.bvugt(&z3::ast::BV::from_u64(ctx, 0, 256)) {
        //     // store at low_key + high
        // } else {
        //     self.0.insert(key, value);
        // }
        // let low_key = key.bvsub(&off);
    }
}
