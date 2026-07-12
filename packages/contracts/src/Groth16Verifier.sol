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

    uint256 constant IC0x = 21840577951386351595014345414874747099568653737823442064887631112900790677644;
    uint256 constant IC0y = 7278216698333667586263214993252224496940100377322259422560575259019657927799;
    uint256 constant IC1x = 3037355583491766906522321356652447743186606791940602532793550863683570608062;
    uint256 constant IC1y = 13570263109191596460931401158941515791092886982564446112648458813499432375083;
    uint256 constant IC2x = 17754258059081959687218943087378153071583251369216621312118853891576467620101;
    uint256 constant IC2y = 6839756861496315913444473271170619216780428151945348283099043852123221341162;
    uint256 constant IC3x = 11081334287939250486219044399586745769759697107587353830931163338227788667336;
    uint256 constant IC3y = 13562440949891881236944495596061487632230699955951951436468875792463825457729;
    uint256 constant IC4x = 10007720450602260618650147181328951764170016606189687353740353149524970151103;
    uint256 constant IC4y = 17205787852480259412380161103001568966626486676197599198327202263784093528603;
    uint256 constant IC5x = 6708148104886989335232726914181325676744627068896369575450452034769286225139;
    uint256 constant IC5y = 5176514888329872016907108671766610574957650606164068954805616992188968417659;
    uint256 constant IC6x = 14215736899077379127871562075486307523787404166086363948005603377171193402004;
    uint256 constant IC6y = 17770332491665133480050028719845806104204187399815257993173508293925090007201;
    uint256 constant IC7x = 20083319201717437270113627895578339578484828866711194584703267682932718647095;
    uint256 constant IC7y = 132978267158559641034071800695916817499351583188818331452921610760642872556;
    uint256 constant IC8x = 4443092248015256612028199606796087062121824999816776150405172042838210656861;
    uint256 constant IC8y = 9234636271885652937391961078261748335966654672086098107014717915351971504529;
    uint256 constant IC9x = 13022511635440094514658399948529008745406468388917975530194493052199356802228;
    uint256 constant IC9y = 15044359442961107731864324894460365018171340240407132747466252340941915063075;
    uint256 constant IC10x = 16915100367445478759617728139707806623355015635020393478764523556857268261829;
    uint256 constant IC10y = 21609657032539945715286715929474684330453712181406595446374699081775915022779;
    uint256 constant IC11x = 10691936205115469372592330639171264149009645006429949206036072242943103579940;
    uint256 constant IC11y = 2799499183015488238829141957739570243815495663315925042669421097691651552204;
    uint256 constant IC12x = 20840883239455019349155367650487247914613503427851329653387804432125959241305;
    uint256 constant IC12y = 10544296463692420909131921700479357559286782832389647721814722947595330724743;
    uint256 constant IC13x = 6019797200913303255931901631494791565146498919605341277912838147728169664605;
    uint256 constant IC13y = 9952250595357157821981296019230792484741044991100384661812022290205819544475;
    uint256 constant IC14x = 4445662048604615088928216509616364136574486382486667315417155154273237932848;
    uint256 constant IC14y = 11656608937104134650679737465724931914854743522696221171116044999746101033937;
    uint256 constant IC15x = 1968005745130587071005846804866221848808342347719846536770940068928707360510;
    uint256 constant IC15y = 20182557295488276184659279170936533744715312561684457468431528549787142619451;
    uint256 constant IC16x = 6014074394477918812587944776227301390765789872836437795349915892693267352523;
    uint256 constant IC16y = 3759934849213609933629798750804365562533195149514261540166262021694426988267;
    uint256 constant IC17x = 12691505093246615664798678314357485801705216415714158727764098003423651818840;
    uint256 constant IC17y = 15144164866129439419129051874451769592838563283454925796613969584866071492972;
    uint256 constant IC18x = 18106744524950631161223396276012495658580282048633112020969706356840059593874;
    uint256 constant IC18y = 3372526250880491284817140030558374620851913076800426039956770732851521486240;
    uint256 constant IC19x = 9122928174668890530150787884394874837029135084615582089711878462853753557213;
    uint256 constant IC19y = 3863431962091352340874690773307828505384695004799067408886392763755621958524;
    uint256 constant IC20x = 12317338247287474128398380827787452269142686211255891007346733191874517546360;
    uint256 constant IC20y = 10384991303647466408350302632244109408006291876658877165743430681763771766278;
    uint256 constant IC21x = 14650243771324542099266407282216376747355614086512188039993710765555792603141;
    uint256 constant IC21y = 17004074512787864015164786258666260213290359951560499824222546025590785324938;
    uint256 constant IC22x = 19382584125651174975793161863771140835387474079014388935811769093417441036885;
    uint256 constant IC22y = 20983466884306401517081426656912343797040736986103648941901126829842093935119;
    uint256 constant IC23x = 9470691357323267837677130116873124471935813896816167039926644559166892042903;
    uint256 constant IC23y = 16599756794993788153081705405734672369736680972362213958666336691002146345151;
    uint256 constant IC24x = 5598184175941601179519289342458777424380703148858300523561662260540008981383;
    uint256 constant IC24y = 4067708885841652984956135282711689940068912537651622769930802875982580258571;
    uint256 constant IC25x = 13822099562188414164010111404824338925284574092329885213967256771243620404478;
    uint256 constant IC25y = 18252786786346419948315070560356796812577407505225845205983926553103914352504;
    uint256 constant IC26x = 1720889255993247830061079009078533180323809786573090444923023492493937899451;
    uint256 constant IC26y = 13251254437802616254878455710027650665419891911502021590241037258393822436128;

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
