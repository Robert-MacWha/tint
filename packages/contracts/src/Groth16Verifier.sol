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
    uint256 constant r    = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    // Base field size
    uint256 constant q   = 21888242871839275222246405745257275088696311157297823662689037894645226208583;

    // Verification Key data
    uint256 constant alphax  = 5695647891058145426960992256924239258977162663247491423090033033549927848147;
    uint256 constant alphay  = 12733265912285760475369614862274621513389353530522689426312383006520304007458;
    uint256 constant betax1  = 17188853177100231683318768507376651386297005843729275101947347654738824529982;
    uint256 constant betax2  = 16046772795261360631872770483206825907800984977136063169057074951177603730360;
    uint256 constant betay1  = 12292672574052723815432127860729230633063172427493062034153727604500388164809;
    uint256 constant betay2  = 2598678333051668525174856434856193461287086780911027270199639036205042539859;
    uint256 constant gammax1 = 11628222563324298181230674495008344240186186826325519137593610615358287529212;
    uint256 constant gammax2 = 6831243439432830324813084301481941947356974712036823415176253777732738556231;
    uint256 constant gammay1 = 17925384280287611628018084016142832400779395652863340582106143584039524919439;
    uint256 constant gammay2 = 18846298836546160555052373845605078349585884040720348961824903102337542184854;
    uint256 constant deltax1 = 3628883038028850944881513950572053331780075276182269323324470636960766857522;
    uint256 constant deltax2 = 18369593787142627228396437495565997248027757595014760737103508814658377944098;
    uint256 constant deltay1 = 7177405643285582574332637269969080867215958208610218360308679444926037813928;
    uint256 constant deltay2 = 13560294035408069076432193212197627500290128495326813965045145908416813321341;

    uint256 constant IC0x = 18839922286385797179182112312849839462957956678362999510022574463116388730328;
    uint256 constant IC0y = 9905646485695784431796960053633406987440061355014083043966789225048385653092;
    uint256 constant IC1x = 13680317893104451728790506494912181155321416073145943368710824470602937184607;
    uint256 constant IC1y = 2399779250059275733831844366014157673182550481133077265059401508446058066432;
    uint256 constant IC2x = 12161131198858523848410287433117723182946027447519764131582272285185950339350;
    uint256 constant IC2y = 3472554245431260724885455388987488076775681987586151394150548061090276646756;
    uint256 constant IC3x = 12463631492840572648565264427373893798173531067215717937498722832502644481870;
    uint256 constant IC3y = 16810655352308616411966945319520205621025895090180128854734684072780399907630;
    uint256 constant IC4x = 4240689959062049462610207757480545025607019641815771602215222015563706269595;
    uint256 constant IC4y = 12881040187516849688525803337781010667858918202301176425519957620926679998;
    uint256 constant IC5x = 20348843665266018703743407405315246507586636947724134150536057202048441450614;
    uint256 constant IC5y = 13151209694424064358215461246276863821638056657077997289887819751406625417087;
    uint256 constant IC6x = 12755134198228979934007124685219641584275630779961750055702820294562804079599;
    uint256 constant IC6y = 800583474759158668794707288324645942781524188725756874996875543146195786564;
    uint256 constant IC7x = 9687636110899416272529915960091880119890642895701456710798807204341141577264;
    uint256 constant IC7y = 16212971133717840253228405740814427960474878855296493500372587054220572465320;
    uint256 constant IC8x = 2987077005469489783915488732833987648080106881002790165174987288435179319173;
    uint256 constant IC8y = 12770625593172428543009334350440691230399462068442670992827718588384346309407;
    uint256 constant IC9x = 9630037538961252034876586108975443950882352208928668123081880016619656993967;
    uint256 constant IC9y = 3989368762881261593695947242814444084539602478022695605339892953340746233095;
    uint256 constant IC10x = 20103980942236008518958780506503874974126321304833433750676090765033606611033;
    uint256 constant IC10y = 11879975453912302041816181016850982908977781867106148141339595391618639894789;
    uint256 constant IC11x = 17447372645633169306772209783623779012860802211983100588443431649183993515194;
    uint256 constant IC11y = 8987903653302450757914043674041119784481182870718965367679373256580542557229;
    uint256 constant IC12x = 12236300714038204631169927777276429442452915522297788680604899031096879002518;
    uint256 constant IC12y = 10521868923312456326840126069987197746329354477449418765282164150863493479784;
    uint256 constant IC13x = 9376754472650085694189605030806986154019937800011801689389632427906378927817;
    uint256 constant IC13y = 16026776156143670003180641005168351913534765613009618818192195459018738518714;
    uint256 constant IC14x = 7616813958590737364713370206179689719839073209782200796572402151806165258133;
    uint256 constant IC14y = 1111524608169000128738960272459113051263639510539424241248023003745189761129;
    uint256 constant IC15x = 3920296642418579390416317491938204872701762129032263429430936349469270307485;
    uint256 constant IC15y = 10283069093029289745456558372710308109672993060037609367202229173877126027245;
    uint256 constant IC16x = 3881982972746047542938839388072610225439577789747361394005044543714126873834;
    uint256 constant IC16y = 7595891523504835610020962198150998067260861725592913663994374836923133581851;
    uint256 constant IC17x = 3239507078054356113998606629348619133710356234124902443188616376412538105803;
    uint256 constant IC17y = 15808942245983511798158857668424982968840905523363848809347033815620558277881;
    uint256 constant IC18x = 14742747961218628338963107461470197890802128630020330978505686562617299521899;
    uint256 constant IC18y = 9765125328898744578687953859279809488947012371362499140793373670605295738130;
    uint256 constant IC19x = 13028904606849912920217069729053880365546249233944697952035976381924244376019;
    uint256 constant IC19y = 3137132284321863211698302742271476638499569148138860919351071348565389104934;
    uint256 constant IC20x = 9806321696374446055399575087023945759092108157652939345660109128466538904415;
    uint256 constant IC20y = 14214891861521477583403387425990752078423560509045010054921501294471788260212;
    uint256 constant IC21x = 15137252648368526790491909441169473623549083743851704135581202637624870019664;
    uint256 constant IC21y = 8493594707875230985222385237513387420008199721996888929960081443677507183746;
    uint256 constant IC22x = 14595369046676318521642406647575403410354119732172434201984463298220211862188;
    uint256 constant IC22y = 16379612399735141832938611280306771538029145098346589396035572952380840773620;
    uint256 constant IC23x = 10981092975533390983078476252776103418253215703180892243102404567783860744675;
    uint256 constant IC23y = 15539718188521613556457668092331391151988910818259536571693033856139082499573;
    uint256 constant IC24x = 12133824141452220238531756950229230326292059587358926143786079811022403217043;
    uint256 constant IC24y = 19124762630161491133323210888353017497146389012973421085918458675061272651012;
    uint256 constant IC25x = 21003456505539156297423158682855959786321100288661661150365783585826948084451;
    uint256 constant IC25y = 19847942941786027511712941168564792544254029302508272798480005317099138758344;
    uint256 constant IC26x = 17271910341556926760877688840679414599823734776814576090423452877490877294939;
    uint256 constant IC26y = 7104373943566439694581010526502932015756504597271537957995426148144741762130;

    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    function verifyProof(uint[2] calldata _pA, uint[2][2] calldata _pB, uint[2] calldata _pC, uint[26] calldata _pubSignals) public view returns (bool) {
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
                g1_mulAccC(_pVk, IC10x, IC10y, calldataload(add(pubSignals, 288)))
                g1_mulAccC(_pVk, IC11x, IC11y, calldataload(add(pubSignals, 320)))
                g1_mulAccC(_pVk, IC12x, IC12y, calldataload(add(pubSignals, 352)))
                g1_mulAccC(_pVk, IC13x, IC13y, calldataload(add(pubSignals, 384)))
                g1_mulAccC(_pVk, IC14x, IC14y, calldataload(add(pubSignals, 416)))
                g1_mulAccC(_pVk, IC15x, IC15y, calldataload(add(pubSignals, 448)))
                g1_mulAccC(_pVk, IC16x, IC16y, calldataload(add(pubSignals, 480)))
                g1_mulAccC(_pVk, IC17x, IC17y, calldataload(add(pubSignals, 512)))
                g1_mulAccC(_pVk, IC18x, IC18y, calldataload(add(pubSignals, 544)))
                g1_mulAccC(_pVk, IC19x, IC19y, calldataload(add(pubSignals, 576)))
                g1_mulAccC(_pVk, IC20x, IC20y, calldataload(add(pubSignals, 608)))
                g1_mulAccC(_pVk, IC21x, IC21y, calldataload(add(pubSignals, 640)))
                g1_mulAccC(_pVk, IC22x, IC22y, calldataload(add(pubSignals, 672)))
                g1_mulAccC(_pVk, IC23x, IC23y, calldataload(add(pubSignals, 704)))
                g1_mulAccC(_pVk, IC24x, IC24y, calldataload(add(pubSignals, 736)))
                g1_mulAccC(_pVk, IC25x, IC25y, calldataload(add(pubSignals, 768)))
                g1_mulAccC(_pVk, IC26x, IC26y, calldataload(add(pubSignals, 800)))

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
