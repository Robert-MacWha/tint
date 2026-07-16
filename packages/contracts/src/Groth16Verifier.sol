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

    uint256 constant IC0x = 3037585713491141682237940450824400849786895203942018909302715247186383963917;
    uint256 constant IC0y = 20485377844997448323971131918776744029920197923461414269669130593432294818892;
    uint256 constant IC1x = 9525776122317034805618683459458170057376439983681183768187885795296865986977;
    uint256 constant IC1y = 2670265261734340980352046307149413059502337443009352048013980969963395115474;
    uint256 constant IC2x = 9264583184158511598183838273280654413639328142463903677853334565625954239446;
    uint256 constant IC2y = 10471895060584864918130992303859963565503412617550506108063034404400395092370;
    uint256 constant IC3x = 9976333884301941490582465418795837064088141117224316974861816643579055810029;
    uint256 constant IC3y = 2183018732658921738145804169937612087741067018038639477578951039208025329153;
    uint256 constant IC4x = 21690835004513147291183404727873838117725885792192651526504891298629166715488;
    uint256 constant IC4y = 10297859513209773390333092903613364206012600455853440321283255601988759078460;
    uint256 constant IC5x = 5892493104125668411721820230266534328234585365244740643391339785193864776471;
    uint256 constant IC5y = 8423212568424237786669578890166753781320370708340530587368868755154038232220;
    uint256 constant IC6x = 14603720207739882323223223001127902932262006072757869068028961594236460383821;
    uint256 constant IC6y = 3666476351409284307420276474481893689928139635362458843908875381906807890480;
    uint256 constant IC7x = 8731334567390900544816494622618362892111780519514974843730597692638635574462;
    uint256 constant IC7y = 15709622585459307766123039019665735419394161430564297058018028875927134607577;
    uint256 constant IC8x = 20338726783757359867376933555423600794493545090306372745290152031788960676462;
    uint256 constant IC8y = 14955116688102985723240353626199046904125546201049541801771861171679201221592;
    uint256 constant IC9x = 9737180763479898444535208226572971699466859931920239682452109076819931186536;
    uint256 constant IC9y = 7031809342782971702475648109139332241925930161746156985626601002167055753359;
    uint256 constant IC10x = 6636233654931550797340479476319354681479458364756586892351534686412243157669;
    uint256 constant IC10y = 16569337050230112092077259373048024736504009500635017602530713449131215190112;
    uint256 constant IC11x = 9863059357542999696728270726507357457200692492913078703837020903419471200663;
    uint256 constant IC11y = 5038138362753492742486550002957040591379843290208721961592134497779839966885;
    uint256 constant IC12x = 4109236029073175522059424247343855126337972573183197557454370125643834571677;
    uint256 constant IC12y = 1690990098604522983087525712657878379496423527541427439131637420764102562853;
    uint256 constant IC13x = 17843183410421975276316311159167388674441459516532206637522846671729560378785;
    uint256 constant IC13y = 20961466913775155952675136659923512581411927930105047436756992216442255095653;
    uint256 constant IC14x = 16874173855203249397751914923839482930602875349581586691825943200492802259709;
    uint256 constant IC14y = 8116341174453757826216831387691758338101363062491093209088787344014561842447;
    uint256 constant IC15x = 10162414185330628464015761961291662684647428298375089819118652422903258610055;
    uint256 constant IC15y = 10688094626098954944206156198849444187527937964967374408462322285559783069027;
    uint256 constant IC16x = 15883936975108063553906017210718920732618163734335828299020161590834355910843;
    uint256 constant IC16y = 19871535524727872457101638319924114157564311623261238459016523384969911105640;
    uint256 constant IC17x = 4447427440922694688693967562124836057874291863017941644481186546659640104605;
    uint256 constant IC17y = 9603950587800975300795252060219817493742403217167135073766479577741742082734;
    uint256 constant IC18x = 4312552232773722888715579917983468487572819577968860831249835747644184662745;
    uint256 constant IC18y = 3754517579857472362814318234256776344515113418425096590991656503947183381262;
    uint256 constant IC19x = 5436289948197358701671744399381546680569518439775459394368368123932995484802;
    uint256 constant IC19y = 14521526887444598597283588000108650613882171636613725912440339823008789261364;
    uint256 constant IC20x = 19589439645813748739220435496942708753845006601885139730943643989197726019044;
    uint256 constant IC20y = 4567842154456977545123140870653001140596948216558765289342761843921591385572;
    uint256 constant IC21x = 16711231988931434469370045697632885454184162699358167367280091523823952108655;
    uint256 constant IC21y = 11630240943226476422441947806440869886543976224117967536863641591352998326002;
    uint256 constant IC22x = 11064441331415340984145726397738222549337289453142493761390795331723222644067;
    uint256 constant IC22y = 14228936555704409251124900594688884940602577577792401736359928784762676652903;
    uint256 constant IC23x = 18233554726019015316560534769269468047224089061282716098959444719853463729058;
    uint256 constant IC23y = 9535004086223592609303627968340231006483380158356183088029888554079064152680;
    uint256 constant IC24x = 16846642397404317485653743532900831179042298221820756908659535299363401720400;
    uint256 constant IC24y = 7454475130150014990079342604926437966205968319389815009964930040998387369386;
    uint256 constant IC25x = 12830896968898324736353858270042673106259189013601696562442526553634828546566;
    uint256 constant IC25y = 3210460522347670669738143778836256394837916566811781607522865273376282875931;
    uint256 constant IC26x = 16886116234301858727781208458698099958851182097628581009320489102703607164140;
    uint256 constant IC26y = 3851777225598842911154381332966023534990144018578867020570275695791374106132;

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
