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
    uint256 constant alphax  = 10314683402145919335415264089338013869151872735661243528435829273762895412475;
    uint256 constant alphay  = 15311410802386913807174485311770598542990692773058369166097347676916826427424;
    uint256 constant betax1  = 8742476040979126669529512667270167370972734897784767815803986702043271844587;
    uint256 constant betax2  = 21546977338313367449764778081974431327514198880463503495700342085596869133184;
    uint256 constant betay1  = 9289809140465907199790286310535739942219483053026485452107542051245760617369;
    uint256 constant betay2  = 10645240371221413259645920038475973272479052146228783657329287031421083633145;
    uint256 constant gammax1 = 19786488175694835941529082486176010093671788452701050737945948286615958246569;
    uint256 constant gammax2 = 20330634461338860209244322586166193708999379153762374339780730602203574324967;
    uint256 constant gammay1 = 5928072313096986966179943778308363878908435360419431578510684278352903202909;
    uint256 constant gammay2 = 2797341508826357269065116312229379096387363396018049448422156015279464026096;
    uint256 constant deltax1 = 15931872898476652233416341120558294461206602664690427226190761090136039762594;
    uint256 constant deltax2 = 486543185197210868010625674188217166207603296241821390928288273180909083262;
    uint256 constant deltay1 = 8810824702403597757041960647441338188287452861929357829520560151016180636116;
    uint256 constant deltay2 = 20423055301853288990931997049485455849638502515255059322630657921443113246087;

    uint256 constant IC0x = 11263252873419856337204863760481005655813817338598520724696285230456908535153;
    uint256 constant IC0y = 18109083087927441076066962322411997225856994029584819540001836663521823670448;
    uint256 constant IC1x = 4754476616176428783842811353811457729487987878040041631876120017475880429829;
    uint256 constant IC1y = 13155685366623567927753860637793825272774246221078857369447003820505420451306;
    uint256 constant IC2x = 11976189917022779959506856663801210448820242249648257523290934935458081766802;
    uint256 constant IC2y = 10874585761398061770324942138681348198148506600683348763303623950812006148408;
    uint256 constant IC3x = 6058770321235454637716536647975527269469098378300835135572430156896390321346;
    uint256 constant IC3y = 20660200845921959789272065994199205210031072293628067497328911180030230627148;
    uint256 constant IC4x = 9629949504407922567621236886616993786833617694246728727259111710323482342775;
    uint256 constant IC4y = 12328511404336270084171293601532085948285971130442506524225594170607754334129;
    uint256 constant IC5x = 11550291025345114395873331098968068961038180572772149155449580773049256376967;
    uint256 constant IC5y = 2969496224371993490689458145675788501152725232510837210400557822964183220142;
    uint256 constant IC6x = 13282337280939867097460629351204907204909330694521768099999483758353024726214;
    uint256 constant IC6y = 7116611933090514341665480793088953518266110728888226581954027613557691044654;
    uint256 constant IC7x = 20955790042719180819269920536995811339322840773754707507365843558607908563941;
    uint256 constant IC7y = 12118477086218824947839593657800431692616856745370132803414085499681171669612;
    uint256 constant IC8x = 11380139986391764031088961033903936468040887868915503594101577287689415306869;
    uint256 constant IC8y = 9542453016029544280722227147423432087902479582227668981668117959364207357611;
    uint256 constant IC9x = 9759657077131353476411847475168665849915457697253552438898451399286794821603;
    uint256 constant IC9y = 4925424312828951456313563702892612085327385877951925388431941769893385728617;
    uint256 constant IC10x = 11562349793785675172823692576101702884216690640486449748297538167391015750043;
    uint256 constant IC10y = 19137003166540683452093775681890395255177452866317744927244298940620750377319;
    uint256 constant IC11x = 6163914864552419039004275397595654773158570276541886389830872817433283461772;
    uint256 constant IC11y = 18398860483059302684124416788343917818331583139497172959837734020538868901774;
    uint256 constant IC12x = 128951514774579990578486110226273213454280900153451011463853860507946633139;
    uint256 constant IC12y = 13225746180538408376749409274389494557590308232582252670773282337241398116167;
    uint256 constant IC13x = 3515994430860324842082520783842232311016156042849017483994994219914913651238;
    uint256 constant IC13y = 3305422729560834449468221240540229804044393557034594492934670313727279564489;
    uint256 constant IC14x = 5625247566763341632405792617035100847298552670444250630265927555082668042783;
    uint256 constant IC14y = 7772875690292306834706135027972072201263872659804355017296962660122057108798;
    uint256 constant IC15x = 12776366677213737408047569322873131188845341567286355791914377149428140631814;
    uint256 constant IC15y = 5134197144141145663339657370254664461056483111162289313937308502568410008146;
    uint256 constant IC16x = 3859417738888629261181374446009820660768631990777431979208083132304868394150;
    uint256 constant IC16y = 19131517538841845660129652683453205480296602183213185040064228122705776084563;
    uint256 constant IC17x = 19148972704874828532586839560510768452701760584317447883765444156649816993978;
    uint256 constant IC17y = 19308370836460548490826262109929099251843779278881434510707975234823962359396;
    uint256 constant IC18x = 18667717443976760634781067006034397713278565460130457691888531849833396842768;
    uint256 constant IC18y = 10882641825031058730386636683646856544629532755259764180374276126260383735098;
    uint256 constant IC19x = 10206217416353189516746036811950232510766473729519096225510927288869438490513;
    uint256 constant IC19y = 19407645233013484264106095777574800795689923793505782089067675152475388605099;
    uint256 constant IC20x = 18549364613742770680989629858543796469877214715927493056101741500357600843400;
    uint256 constant IC20y = 2809104209107489355195008160486327966146217339375281306837770297357357697779;
    uint256 constant IC21x = 13879102812826337459727074401854190377246423682364636361403433765023690154150;
    uint256 constant IC21y = 7281258638484632065900286591646779462608866062580139128254685033300084189883;
    uint256 constant IC22x = 541793203268339821207659779801567230657504256872929332239224631995741126753;
    uint256 constant IC22y = 1774714980710192859437624973970160091764423433183854308176502369431923015589;
    uint256 constant IC23x = 13042858288680650889376015808667288137631267865346649294126154697722741326706;
    uint256 constant IC23y = 12994959209625680297212476748001633780609989928930192108403051462913091998318;
    uint256 constant IC24x = 15069597573991253985443104784525957379584052940293267615080387403220276930357;
    uint256 constant IC24y = 19075941320350730582502562238275887266502478451178641357950742850383303345278;
    uint256 constant IC25x = 13105810492083587335174712865735496075296156958632581614283426421475943519677;
    uint256 constant IC25y = 13022260030093549656857480317652810993476446030527602300419670380798960223133;

    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    function verifyProof(uint[2] calldata _pA, uint[2][2] calldata _pB, uint[2] calldata _pC, uint[25] calldata _pubSignals) public view returns (bool) {
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

            // Validate all evaluations
            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)

            mstore(0, isValid)
             return(0, 0x20)
         }
     }
 }
