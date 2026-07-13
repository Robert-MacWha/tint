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

    uint256 constant IC0x = 3954570438884715927770373369589470457184861925505527177363726943711580530739;
    uint256 constant IC0y = 1613864831531537161866763026698651728366378226952701023692867816735701653511;
    uint256 constant IC1x = 1998602063850080571284658611307594575846413058322262642877096619613384784048;
    uint256 constant IC1y = 17529336312844333159465134239794265516875925289013366073976383990924570891978;
    uint256 constant IC2x = 19968431406806188204070004347272894301230774460541331771576860126931860566365;
    uint256 constant IC2y = 16873059793121822476700443071254721582615847391380965515141179940012426390010;
    uint256 constant IC3x = 1612723764303107947181501755428014575912686593764903122204689301962770663228;
    uint256 constant IC3y = 15246438717022315380120386376403480425870387712426000183833643073588276176167;
    uint256 constant IC4x = 164292047031735642685842371352886043028666627628300915540574249953074960924;
    uint256 constant IC4y = 2858024140283784060776525466338304549331846322042424500688872398328686704221;
    uint256 constant IC5x = 8221716060145383132782105019988875176213039168283123127604013928101709822495;
    uint256 constant IC5y = 1149026803276706996240252033185047666021402499828252181087429472281491652366;
    uint256 constant IC6x = 14474140874994511616833812038219130612523198323379723560944954983781369974604;
    uint256 constant IC6y = 4576721612004097911339390126733857070962984035050235679213046209490886156851;
    uint256 constant IC7x = 7757450716640374407743806972391512410709795367851316698027439410918789342091;
    uint256 constant IC7y = 11795696350506196347770587069528705380049994908612213547410539969257408590086;
    uint256 constant IC8x = 8867276987520033889912927571544205938086213789654171025792505408095663691900;
    uint256 constant IC8y = 11593516324042959193363297848389937091311157481267346533592630558944908864185;
    uint256 constant IC9x = 2272747782493987205697861023620158123553753721755167643080521649271121600467;
    uint256 constant IC9y = 7243395024959640075295334465032639291304437764626440649233734005032699931620;
    uint256 constant IC10x = 21822489104267489104071403703335465622071311646647291041869206425578806552748;
    uint256 constant IC10y = 560143743233890438042828867644097628562091121621241576651296503992985118331;
    uint256 constant IC11x = 9082587059493743199288890852031949422814124714223088436663742448018252395233;
    uint256 constant IC11y = 15752483505590419388943561290161103353139120817482291808324912619416318143104;
    uint256 constant IC12x = 5768099161824268991225915478179243466543447011524437507587231956667225346748;
    uint256 constant IC12y = 11615856094265839920680123207397041693926568503302894089884534522077232985987;
    uint256 constant IC13x = 8304739314486124632403927131219419207232681891938898677163102543235255086007;
    uint256 constant IC13y = 11897788835428392325740573586978001629359795755640712823482858184632581728461;
    uint256 constant IC14x = 2256929578605351450209325076737649465514576733265104486487103225069217620015;
    uint256 constant IC14y = 21854157273578459482149522017132261681445845673653698581123784806605647830372;
    uint256 constant IC15x = 15849979310288654357573385661157123756858679676539082530090461264177048193543;
    uint256 constant IC15y = 8466869176616658173082711983071551614771515441738115590126875867123017184480;
    uint256 constant IC16x = 19830814952173106112556564598132921540438404008442551588334478951739104973198;
    uint256 constant IC16y = 3537682889884391192551528403575117158097642124422488331074516071525064141918;
    uint256 constant IC17x = 9762165583806125737799806447117378416306809049967654761849926465314521829656;
    uint256 constant IC17y = 17491167186841362876739564005687769870356882910068192348695879627273250116990;
    uint256 constant IC18x = 3854082283124612390162332333036967505560042773203690825017472782757448686354;
    uint256 constant IC18y = 8289674476068370719255712117389266752931861047209400681753046550038367963253;
    uint256 constant IC19x = 3978447732365864936394746816197089511799760946038292559092303744706262128347;
    uint256 constant IC19y = 4996348732191861655130728065734257188869318249129374847020950317972530010341;
    uint256 constant IC20x = 14690821929146077940688915388725440882004393982277345443013195823016660461778;
    uint256 constant IC20y = 18081009765973401239306602346512081446992138486417277240370390832185196675047;
    uint256 constant IC21x = 20469216930489849436519032145366881261928351044135880543370249329329188894900;
    uint256 constant IC21y = 14402068628952435484150770804113341218875654264864446262839387152586062604222;
    uint256 constant IC22x = 2609290632502722139116212815314686492338120941550342947771579492597277568512;
    uint256 constant IC22y = 10455602358802182511914178002821286123299782633592934901228109685884522616180;
    uint256 constant IC23x = 12279858699617226848969707743345021260413826569111271665385705752212648556555;
    uint256 constant IC23y = 16484480311601026450872346772468662404525370863555370321503760481588034319427;
    uint256 constant IC24x = 17770026218222634000409998250945071705489194694289019610946690463782522311666;
    uint256 constant IC24y = 1364885711260881141765496288587761119960793878670608545978624553072769068095;
    uint256 constant IC25x = 5820128001396433898278991558018918124113897540060829011215254901911948645533;
    uint256 constant IC25y = 4554186811226590286714693490157890443507092287591659386076051796412017147916;
    uint256 constant IC26x = 12191876411384166985541533492502572646962837342697666313708855156870219168947;
    uint256 constant IC26y = 15558916399719254968540671110565336777245638680943889801778536842531845634758;

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
