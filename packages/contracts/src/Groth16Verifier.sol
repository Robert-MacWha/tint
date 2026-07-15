// SPDX-License-Identifier: GPL-3.0
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

import {IVerifier} from "./interfaces/IVerifier.sol";

contract Groth16Verifier is IVerifier {
    // Scalar field size
    uint256 constant r =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant q =
        21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // Verification Key data
    uint256 constant alphax =
        5695647891058145426960992256924239258977162663247491423090033033549927848147;
    uint256 constant alphay =
        12733265912285760475369614862274621513389353530522689426312383006520304007458;
    uint256 constant betax1 =
        17188853177100231683318768507376651386297005843729275101947347654738824529982;
    uint256 constant betax2 =
        16046772795261360631872770483206825907800984977136063169057074951177603730360;
    uint256 constant betay1 =
        12292672574052723815432127860729230633063172427493062034153727604500388164809;
    uint256 constant betay2 =
        2598678333051668525174856434856193461287086780911027270199639036205042539859;
    uint256 constant gammax1 =
        11628222563324298181230674495008344240186186826325519137593610615358287529212;
    uint256 constant gammax2 =
        6831243439432830324813084301481941947356974712036823415176253777732738556231;
    uint256 constant gammay1 =
        17925384280287611628018084016142832400779395652863340582106143584039524919439;
    uint256 constant gammay2 =
        18846298836546160555052373845605078349585884040720348961824903102337542184854;
    uint256 constant deltax1 =
        3628883038028850944881513950572053331780075276182269323324470636960766857522;
    uint256 constant deltax2 =
        18369593787142627228396437495565997248027757595014760737103508814658377944098;
    uint256 constant deltay1 =
        7177405643285582574332637269969080867215958208610218360308679444926037813928;
    uint256 constant deltay2 =
        13560294035408069076432193212197627500290128495326813965045145908416813321341;

    uint256 constant IC0x =
        9976545216855601863930020519738638691313081613217047313558203078274141683013;
    uint256 constant IC0y =
        4900173075559919623938817698769702282998819613298291229841748076283248175340;
    uint256 constant IC1x =
        3788500002196738637042886800805528461016941575791715623386443338809578168855;
    uint256 constant IC1y =
        2924736555346114988024979375821783370154785433133489463434856719387412193452;
    uint256 constant IC2x =
        13702402314049447905718877826543166165608183163456391023820721383495776584076;
    uint256 constant IC2y =
        5629143293024359893125160191736076975526999840196224196841084112831388614667;
    uint256 constant IC3x =
        16714455026935579588548607646845796253241758921347560864586167364598066608995;
    uint256 constant IC3y =
        14624232722468568300645414825404749063291885483443125559647862392114222489915;
    uint256 constant IC4x =
        6549849609985811362846500229011031404750585640059635588548596557576057677421;
    uint256 constant IC4y =
        13245058149458866024331159148848408212067650602676108841256555328372181125384;
    uint256 constant IC5x =
        16853492602825111162860054224032070394034577555433006403021098566285420221592;
    uint256 constant IC5y =
        1248154832392561331469600746172489339062497941655911648090348123749049668189;
    uint256 constant IC6x =
        21057589910030226423605126285746460742547127540052146523506719676307167125587;
    uint256 constant IC6y =
        15769378960328707682741919782958623780158694451414939107400709283922299709317;
    uint256 constant IC7x =
        2738802846711495785012126796111200182977251261584716462287154250135638668667;
    uint256 constant IC7y =
        5396648217293571599824170880991633516087691458711328762698871891867417476922;
    uint256 constant IC8x =
        15535906718799442161695354887339478757080957917722114181439152635365572394855;
    uint256 constant IC8y =
        6014936177910927632912444548082521633883275739156192526322849533489820545429;
    uint256 constant IC9x =
        3782631932770031074885333651208868565960136965896408146044658269060099445767;
    uint256 constant IC9y =
        18878447567214523597868810420873945837206575162382700314535545481847609055427;
    uint256 constant IC10x =
        18896835956086874631328616299550503124073137077493221843178372339288076370921;
    uint256 constant IC10y =
        5269574364028052406971994316209812338343423422386097662502123643452179912758;
    uint256 constant IC11x =
        7962013772390067739148441699689349827142805889302753664962414939217249232337;
    uint256 constant IC11y =
        1992062819611548832362003787480552180636369674119176603932239385705039132820;
    uint256 constant IC12x =
        4320917212734948733259147259472742159257042311142776854098735827170595877320;
    uint256 constant IC12y =
        1242860683953763643350526269119731193222100521944049205684266195049402020124;
    uint256 constant IC13x =
        7837420027660810310166368945660989422795754432220526787729047305059881268700;
    uint256 constant IC13y =
        17477028612030842620467647154712072109773990024319686329868339821715446840022;
    uint256 constant IC14x =
        2378329787220641840003924366307481649935931266155160349324832828400117352127;
    uint256 constant IC14y =
        20988248539508161065926062267639602196739621623123233152163118752949054808247;
    uint256 constant IC15x =
        11167725146304060211321852141610914626470022161451296980086442144599990022837;
    uint256 constant IC15y =
        19078276785488316686779225760489401498271642631320622031542753521936212269464;
    uint256 constant IC16x =
        13620667012139451959667006434147590081379170913323366911029870744380720386369;
    uint256 constant IC16y =
        3826428293126418755022241058960981520157115438683987868616958257098311703819;
    uint256 constant IC17x =
        20616063974868678303224025346438081978985602499640240255366664555880234964869;
    uint256 constant IC17y =
        15340449876713756364229856899706244567939259097805428959673201460658754845609;
    uint256 constant IC18x =
        15872781157044519886549377851978765578947893930716737037786746325332609335557;
    uint256 constant IC18y =
        16638707654538264564038752547275788475630514926238393222485422748775453530471;
    uint256 constant IC19x =
        19538675258925785407313102332866271874187775356156640374658798385744004222900;
    uint256 constant IC19y =
        13238675095916652860461415820619647022120949168008811850706469009075236130704;
    uint256 constant IC20x =
        6005533127831898520969769133365193697173235771765099067275467300912388507305;
    uint256 constant IC20y =
        9881206399806128135457034664007968661425940022007130915126753541139400231962;
    uint256 constant IC21x =
        11367419134070066597389002570913823209433995141381176726797666206806266790901;
    uint256 constant IC21y =
        1766796699434525607958354634008423641344795083164492048925351959053185288088;
    uint256 constant IC22x =
        21224558214050389206111742099771928421133311093026529572771946309944110262975;
    uint256 constant IC22y =
        3422855695746264529348770015341367476971392277897631968878733485875350812546;
    uint256 constant IC23x =
        13182772995210798304098166290083306308189458222793325974124079289772585353981;
    uint256 constant IC23y =
        7380809232318938087710587585842618563868814361134882697043377388072958738462;
    uint256 constant IC24x =
        19142232200741849922281430721233387641282234678615069075791335568310437601714;
    uint256 constant IC24y =
        11558441153249432553628943640484832820005096139516380892067179310465808410389;
    uint256 constant IC25x =
        5395432199362562690669272028834884632026337528753018938112516141834512579847;
    uint256 constant IC25y =
        4757326077020314843924911262483154401773267767131082863851887759001784126738;
    uint256 constant IC26x =
        17325253126807971807565264565265162302506001307814961916137546381995431949480;
    uint256 constant IC26y =
        5576498542090535348351018997772800031947758625241440039156567553097158217396;

    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    function verifyProof(
        uint[2] calldata _pA,
        uint[2][2] calldata _pB,
        uint[2] calldata _pC,
        uint[26] calldata _pubSignals
    ) public view returns (bool) {
        assembly {
            function checkField(v) {
                if iszero(lt(v, r)) {
                    mstore(0, 0)
                    return(0, 0x20)
                }
            }

            // G1 function to multiply a G1 value(x,y) to value in an address
            function g1_mulAccC(pR, x, y, s) {
                let success
                let mIn := mload(0x40)
                mstore(mIn, x)
                mstore(add(mIn, 32), y)
                mstore(add(mIn, 64), s)

                success := staticcall(sub(gas(), 2000), 7, mIn, 96, mIn, 64)

                if iszero(success) {
                    mstore(0, 0)
                    return(0, 0x20)
                }

                mstore(add(mIn, 64), mload(pR))
                mstore(add(mIn, 96), mload(add(pR, 32)))

                success := staticcall(sub(gas(), 2000), 6, mIn, 128, pR, 64)

                if iszero(success) {
                    mstore(0, 0)
                    return(0, 0x20)
                }
            }

            function checkPairing(pA, pB, pC, pubSignals, pMem) -> isOk {
                let _pPairing := add(pMem, pPairing)
                let _pVk := add(pMem, pVk)

                mstore(_pVk, IC0x)
                mstore(add(_pVk, 32), IC0y)

                // Compute the linear combination vk_x
                g1_mulAccC(_pVk, IC1x, IC1y, calldataload(add(pubSignals, 0)))
                g1_mulAccC(_pVk, IC2x, IC2y, calldataload(add(pubSignals, 32)))
                g1_mulAccC(_pVk, IC3x, IC3y, calldataload(add(pubSignals, 64)))
                g1_mulAccC(_pVk, IC4x, IC4y, calldataload(add(pubSignals, 96)))
                g1_mulAccC(_pVk, IC5x, IC5y, calldataload(add(pubSignals, 128)))
                g1_mulAccC(_pVk, IC6x, IC6y, calldataload(add(pubSignals, 160)))
                g1_mulAccC(_pVk, IC7x, IC7y, calldataload(add(pubSignals, 192)))
                g1_mulAccC(_pVk, IC8x, IC8y, calldataload(add(pubSignals, 224)))
                g1_mulAccC(_pVk, IC9x, IC9y, calldataload(add(pubSignals, 256)))
                g1_mulAccC(
                    _pVk,
                    IC10x,
                    IC10y,
                    calldataload(add(pubSignals, 288))
                )
                g1_mulAccC(
                    _pVk,
                    IC11x,
                    IC11y,
                    calldataload(add(pubSignals, 320))
                )
                g1_mulAccC(
                    _pVk,
                    IC12x,
                    IC12y,
                    calldataload(add(pubSignals, 352))
                )
                g1_mulAccC(
                    _pVk,
                    IC13x,
                    IC13y,
                    calldataload(add(pubSignals, 384))
                )
                g1_mulAccC(
                    _pVk,
                    IC14x,
                    IC14y,
                    calldataload(add(pubSignals, 416))
                )
                g1_mulAccC(
                    _pVk,
                    IC15x,
                    IC15y,
                    calldataload(add(pubSignals, 448))
                )
                g1_mulAccC(
                    _pVk,
                    IC16x,
                    IC16y,
                    calldataload(add(pubSignals, 480))
                )
                g1_mulAccC(
                    _pVk,
                    IC17x,
                    IC17y,
                    calldataload(add(pubSignals, 512))
                )
                g1_mulAccC(
                    _pVk,
                    IC18x,
                    IC18y,
                    calldataload(add(pubSignals, 544))
                )
                g1_mulAccC(
                    _pVk,
                    IC19x,
                    IC19y,
                    calldataload(add(pubSignals, 576))
                )
                g1_mulAccC(
                    _pVk,
                    IC20x,
                    IC20y,
                    calldataload(add(pubSignals, 608))
                )
                g1_mulAccC(
                    _pVk,
                    IC21x,
                    IC21y,
                    calldataload(add(pubSignals, 640))
                )
                g1_mulAccC(
                    _pVk,
                    IC22x,
                    IC22y,
                    calldataload(add(pubSignals, 672))
                )
                g1_mulAccC(
                    _pVk,
                    IC23x,
                    IC23y,
                    calldataload(add(pubSignals, 704))
                )
                g1_mulAccC(
                    _pVk,
                    IC24x,
                    IC24y,
                    calldataload(add(pubSignals, 736))
                )
                g1_mulAccC(
                    _pVk,
                    IC25x,
                    IC25y,
                    calldataload(add(pubSignals, 768))
                )
                g1_mulAccC(
                    _pVk,
                    IC26x,
                    IC26y,
                    calldataload(add(pubSignals, 800))
                )

                // -A
                mstore(_pPairing, calldataload(pA))
                mstore(
                    add(_pPairing, 32),
                    mod(sub(q, calldataload(add(pA, 32))), q)
                )

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

                let success := staticcall(
                    sub(gas(), 2000),
                    8,
                    _pPairing,
                    768,
                    _pPairing,
                    0x20
                )

                isOk := and(success, mload(_pPairing))
            }

            let pMem := mload(0x40)
            mstore(0x40, add(pMem, pLastMem))

            // Validate that all evaluations ∈ F
            checkField(calldataload(add(_pubSignals, 0)))
            checkField(calldataload(add(_pubSignals, 32)))
            checkField(calldataload(add(_pubSignals, 64)))
            checkField(calldataload(add(_pubSignals, 96)))
            checkField(calldataload(add(_pubSignals, 128)))
            checkField(calldataload(add(_pubSignals, 160)))
            checkField(calldataload(add(_pubSignals, 192)))
            checkField(calldataload(add(_pubSignals, 224)))
            checkField(calldataload(add(_pubSignals, 256)))
            checkField(calldataload(add(_pubSignals, 288)))
            checkField(calldataload(add(_pubSignals, 320)))
            checkField(calldataload(add(_pubSignals, 352)))
            checkField(calldataload(add(_pubSignals, 384)))
            checkField(calldataload(add(_pubSignals, 416)))
            checkField(calldataload(add(_pubSignals, 448)))
            checkField(calldataload(add(_pubSignals, 480)))
            checkField(calldataload(add(_pubSignals, 512)))
            checkField(calldataload(add(_pubSignals, 544)))
            checkField(calldataload(add(_pubSignals, 576)))
            checkField(calldataload(add(_pubSignals, 608)))
            checkField(calldataload(add(_pubSignals, 640)))
            checkField(calldataload(add(_pubSignals, 672)))
            checkField(calldataload(add(_pubSignals, 704)))
            checkField(calldataload(add(_pubSignals, 736)))
            checkField(calldataload(add(_pubSignals, 768)))
            checkField(calldataload(add(_pubSignals, 800)))

            // Validate all evaluations
            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)

            mstore(0, isValid)
            return(0, 0x20)
        }
    }
}
