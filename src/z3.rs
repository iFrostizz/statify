use crate::data::Word;
use z3::{ast::BV, Context};

pub fn word_to_bv<'c>(ctx: &'c Context, name: &'c str, word: Word) -> BV<'c> {
    let bv = BV::new_const(ctx, name, 32);

    word.chunks_exact(8).rev().fold(bv, |vec, bytes| {
        let num = u64::from_le_bytes(bytes.try_into().unwrap());
        vec.concat(&BV::from_u64(ctx, num, 8))
    })
}
