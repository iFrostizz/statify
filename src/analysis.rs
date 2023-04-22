use crate::bytecode::Mnemonics;

pub fn get_selectors(mnemo: Mnemonics) -> Vec<u32> {
    mnemo
        .into_iter()
        .filter(|mn| mn.pushes.len() == 4)
        .map(|mn| {
            let mut array = [0u8; 4];
            array.copy_from_slice(&mn.pushes);

            u32::from_be_bytes(array)
        })
        .collect()
}

#[cfg(test)]
use crate::{bytecode::to_mnemonics, utils::get_artifacts_code};

#[test]
fn weth() {
    let bytecode = get_artifacts_code("test-data/WETH9.asm").unwrap();
    let mnemonics = to_mnemonics(&bytecode);
    let selectors = get_selectors(mnemonics);

    let expected = vec![
        0x06fdde03, // name()
        0x18160ddd, // totalSupply()
        0x313ce567, // decimals()
        0x70a08231, // balanceOf(address)
        0x95d89b41, // symbol()
        0xdd62ed3e, // allowance(address,address)
        0x095ea7b3, // approve(address,uint256)
        0x23b872dd, // transferFrom(address,address,uint256)
        0x2e1a7d4d, // withdraw(uint256)
        0xa9059cbb, // transfer(address,uint256)
        0xd0e30db0, // deposit()
    ];

    assert!(expected.iter().all(|sel| selectors.contains(sel)));
}
