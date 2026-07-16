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

    uint256 constant IC0x = 6375875324185568716688237208538258495195340518609857990403378328286690417735;
    uint256 constant IC0y = 4118194518687456803726489409179061860297877420141340401324447503630753736812;
    uint256 constant IC1x = 5324494764372280356689257634295947461948683049778194161621348030649067404357;
    uint256 constant IC1y = 12344350012701206117645161186661968696515104235547355534690235562305327799631;
    uint256 constant IC2x = 3346763987755791103684470202771349263597016183137525345983044914832785441696;
    uint256 constant IC2y = 1504664972467914951018950616018314530366500214861165380520250697433373910539;
    uint256 constant IC3x = 6867022038663756612291505574488733468671487019918776065957005550452030415024;
    uint256 constant IC3y = 11171104019068300040063318379817257886637228675720228481703758531426285656578;
    uint256 constant IC4x = 13061879136016168179162673236582641156365740534666543137261917458505161029532;
    uint256 constant IC4y = 45057195729065015344063749210577945501679960396667908286568877691584463735;
    uint256 constant IC5x = 8489426412669329872707368987538116167797843447067451045614968506211347610667;
    uint256 constant IC5y = 19338437049222595082820732076201135516810492837793085089762677046011543124347;
    uint256 constant IC6x = 10618152344472718958177069579653619648886723724887114360588537322769977474815;
    uint256 constant IC6y = 15600057943608929252858915307959658095725960937660524021302488522038771848912;
    uint256 constant IC7x = 6376935976146610299020332869618384278336248764438946388113510885834423302446;
    uint256 constant IC7y = 16449046173547073769643645983832429469886964257567729114081202555055341305097;
    uint256 constant IC8x = 7659348060357004853746066175281120703378753267527070280311718873438814967264;
    uint256 constant IC8y = 2822761620461714820222216165216344274863993378655082310344172692468042664670;
    uint256 constant IC9x = 3468027764725491659476471754455453307310161913076770980347024292825326386449;
    uint256 constant IC9y = 20613464876432176548363966219338617525539705291999325358619703506450393520272;
    uint256 constant IC10x = 20960453109758875001345160483467576449746890826684837924079741357312598757286;
    uint256 constant IC10y = 3339011853377131739079858676655329547337736753730246203075547725168623588144;
    uint256 constant IC11x = 8592901714256960564906990935760959874075686377165328596737322835855283712867;
    uint256 constant IC11y = 2394383112450927010354957492783300375357705344630393504126664738884950183082;
    uint256 constant IC12x = 19293281775836813447803888189531273129491036175788352535487854903612833169840;
    uint256 constant IC12y = 23893118733710560430473990028783590599360807851878316801795091454132495264;
    uint256 constant IC13x = 18793879015561703602566971280655255817093162051945251624438281572293095962885;
    uint256 constant IC13y = 15744680370781671326122594482018658258389176485025126603871814113184550270957;
    uint256 constant IC14x = 6065665115236885437847653465745918972327737756263494307669718699854711530014;
    uint256 constant IC14y = 21225852550110339705348422161724145049317723332804937114452781212799881368293;
    uint256 constant IC15x = 17635274918444674395997589063063450842565532163640403902549893655279836374125;
    uint256 constant IC15y = 7521313897402110184130277004345326080526866389274286356722935078250295047284;
    uint256 constant IC16x = 14529637862201281276354389353606343671245170086089780386554789987772513469481;
    uint256 constant IC16y = 7459347983731683821018476515080948875194420391008710235358944461939298247370;
    uint256 constant IC17x = 15244899154165925380959825198394446567951311728079057599481781695963916142999;
    uint256 constant IC17y = 15784394390187151740711442147339762315126881122530363720587138676863796908003;
    uint256 constant IC18x = 503320274432074392221527107927873887703205464572960950674293768159324934302;
    uint256 constant IC18y = 7911520190953565735546945115216892954628624916738617992185376706288700765780;
    uint256 constant IC19x = 20581185394713460715279780478644438125121973940789147501925101479972753959274;
    uint256 constant IC19y = 14187428163444153472407329127325834145784676025711952888685415722707861452478;
    uint256 constant IC20x = 14387622431515118721224167531714280788798917153918967010837820105479510904388;
    uint256 constant IC20y = 4361665613661287793927655498917296762517883383918256279971269743802355977422;
    uint256 constant IC21x = 17561696164004193712375838622824357530306283873207221835030197411462565006053;
    uint256 constant IC21y = 21514606586949921218073679557714069776231914708052622717483120834497801681861;
    uint256 constant IC22x = 18269267088328860341293295715066036250606893492078351888945610070780484508860;
    uint256 constant IC22y = 20061075585889441494230982373151074350297669219658648001446279450257136003578;
    uint256 constant IC23x = 21270552380850025843420401240266665494254216320049040845520085239280492405770;
    uint256 constant IC23y = 14680640560430012677689343308756810925954325347377289362861442880668132868046;
    uint256 constant IC24x = 605212885348514309902164797801752731957643455048911682388336027040432092019;
    uint256 constant IC24y = 13043004792042087892559011920386025905256120440686052320304235028448520365285;
    uint256 constant IC25x = 16627915093664211060423810973790607492616354316698737552339307805902278153990;
    uint256 constant IC25y = 12616823589374931733091865807584772374442948393242005150740826804699894743903;

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
