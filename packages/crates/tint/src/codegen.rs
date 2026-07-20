use alloy_primitives::U256;
use ark_bn254::Bn254;
use ark_groth16::VerifyingKey;

/// Generates the Solidity source for a Groth16 verifier contract from a
/// verifying key.
///
/// Based on SnarkJS's groth16 verifier contract, remixed to use string formatting
/// instead of EJS.
pub fn groth16_verifier_solidity(vk: &VerifyingKey<Bn254>) -> String {
    let n_public = vk.gamma_abc_g1.len() - 1;

    let (alphax, alphay) = g1(&vk.alpha_g1);
    let (betax1, betax2, betay1, betay2) = g2(&vk.beta_g2);
    let (gammax1, gammax2, gammay1, gammay2) = g2(&vk.gamma_g2);
    let (deltax1, deltax2, deltay1, deltay2) = g2(&vk.delta_g2);

    let ic_constants: String = vk
        .gamma_abc_g1
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (x, y) = g1(p);
            format!("    uint256 constant IC{i}x = {x};\n    uint256 constant IC{i}y = {y};\n")
        })
        .collect();

    let check_field_loop: String = (0..n_public)
        .map(|i| {
            format!(
                "            checkField(calldataload(add(_pubSignals, {})))\n",
                i * 32
            )
        })
        .collect();

    let mul_acc_loop: String = (1..=n_public)
        .map(|i| {
            format!(
                "                g1_mulAccC(_pVk, IC{i}x, IC{i}y, calldataload(add(pubSignals, {})))\n",
                (i - 1) * 32
            )
        })
        .collect();

    format!(
        r#"// SPDX-License-Identifier: GPL-3.0
/*
    Copyright 2021 0KIMS association.

    This file is generated with [snarkJS](https://github.com/iden3/snarkjs).

    snarkJS is a free software: you can redistribute it and/or modify it
    under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    snarkJS is distributed in the hope that it will be useful, but WITHOUT
    ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public
    License for more details.

    You should have received a copy of the GNU General Public License
    along with snarkJS. If not, see <https://www.gnu.org/licenses/>.
*/
// AUTO-GENERATED — do not edit; see tint_rs::codegen

pragma solidity ^0.8.24;

import {{IVerifier}} from "./interfaces/IVerifier.sol";

contract Groth16Verifier is IVerifier {{
    // Scalar field size
    uint256 constant r    = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant q   = 21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // Verification Key data
    uint256 constant alphax  = {alphax};
    uint256 constant alphay  = {alphay};
    uint256 constant betax1  = {betax1};
    uint256 constant betax2  = {betax2};
    uint256 constant betay1  = {betay1};
    uint256 constant betay2  = {betay2};
    uint256 constant gammax1 = {gammax1};
    uint256 constant gammax2 = {gammax2};
    uint256 constant gammay1 = {gammay1};
    uint256 constant gammay2 = {gammay2};
    uint256 constant deltax1 = {deltax1};
    uint256 constant deltax2 = {deltax2};
    uint256 constant deltay1 = {deltay1};
    uint256 constant deltay2 = {deltay2};

{ic_constants}
    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    function verifyProof(uint[2] calldata _pA, uint[2][2] calldata _pB, uint[2] calldata _pC, uint[{n_public}] calldata _pubSignals) public view returns (bool) {{
        assembly {{
            function checkField(v) {{
                if iszero(lt(v, r)) {{
                    mstore(0, 0)
                    return(0, 0x20)
                }}
            }}

            // G1 function to multiply a G1 value(x,y) to value in an address
            function g1_mulAccC(pR, x, y, s) {{
                let success
                let mIn := mload(0x40)
                mstore(mIn, x)
                mstore(add(mIn, 32), y)
                mstore(add(mIn, 64), s)

                success := staticcall(sub(gas(), 2000), 7, mIn, 96, mIn, 64)

                if iszero(success) {{
                    mstore(0, 0)
                    return(0, 0x20)
                }}

                mstore(add(mIn, 64), mload(pR))
                mstore(add(mIn, 96), mload(add(pR, 32)))

                success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)

                if iszero(success) {{
                    mstore(0, 0)
                    return(0, 0x20)
                }}
            }}

            function checkPairing(pA, pB, pC, pubSignals, pMem) -> isOk {{
                let _pPairing := add(pMem, pPairing)
                let _pVk := add(pMem, pVk)

                mstore(_pVk, IC0x)
                mstore(add(_pVk, 32), IC0y)

                // Compute the linear combination vk_x
{mul_acc_loop}
                // -A
                mstore(_pPairing, calldataload(pA))
                mstore(add(_pPairing, 32), mod(sub(q, calldataload(add(pA, 32))), q))

                // B
                mstore(add(_pPairing, 64), calldataload(pB))
                mstore(add(_pPairing, 96), calldataload(add(pB, 32)))
                mstore(add(_pPairing, 128), calldataload(add(pB, 64)))
                mstore(add(_pPairing, 160), calldataload(add(pB, 96)))

                // alpha1
                mstore(add(_pPairing, 192), alphax)
                mstore(add(_pPairing, 224), alphay)

                // beta2
                mstore(add(_pPairing, 256), betax1)
                mstore(add(_pPairing, 288), betax2)
                mstore(add(_pPairing, 320), betay1)
                mstore(add(_pPairing, 352), betay2)

                // vk_x
                mstore(add(_pPairing, 384), mload(add(pMem, pVk)))
                mstore(add(_pPairing, 416), mload(add(pMem, add(pVk, 32))))


                // gamma2
                mstore(add(_pPairing, 448), gammax1)
                mstore(add(_pPairing, 480), gammax2)
                mstore(add(_pPairing, 512), gammay1)
                mstore(add(_pPairing, 544), gammay2)

                // C
                mstore(add(_pPairing, 576), calldataload(pC))
                mstore(add(_pPairing, 608), calldataload(add(pC, 32)))

                // delta2
                mstore(add(_pPairing, 640), deltax1)
                mstore(add(_pPairing, 672), deltax2)
                mstore(add(_pPairing, 704), deltay1)
                mstore(add(_pPairing, 736), deltay2)


                let success := staticcall(sub(gas(), 2000), 8, _pPairing, 768, _pPairing, 0x20)

                isOk := and(success, mload(_pPairing))
            }}

            let pMem := mload(0x40)
            mstore(0x40, add(pMem, pLastMem))

            // Validate that all evaluations ∈ F
{check_field_loop}
            // Validate all evaluations
            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)

            mstore(0, isValid)
             return(0, 0x20)
         }}
     }}
 }}
"#
    )
}

/// Per EIP-197, G1 coordinates map directly: `(x, y)`.
fn g1(p: &ark_bn254::G1Affine) -> (String, String) {
    let x: U256 = p.x.into();
    let y: U256 = p.y.into();
    (x.to_string(), y.to_string())
}

/// Per EIP-197, the `ecPairing` precompile wants each G2 coordinate as
/// `(c1, c0)`, not the natural `(c0, c1)` ark/mathematical order.
fn g2(p: &ark_bn254::G2Affine) -> (String, String, String, String) {
    let x_c1: U256 = p.x.c1.into();
    let x_c0: U256 = p.x.c0.into();
    let y_c1: U256 = p.y.c1.into();
    let y_c0: U256 = p.y.c0.into();
    (
        x_c1.to_string(),
        x_c0.to_string(),
        y_c1.to_string(),
        y_c0.to_string(),
    )
}
