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

    uint256 constant IC0x = 560657354372294286558653307408278814781279344761111694694932834191626552183;
    uint256 constant IC0y = 20889321169542680073013600142929689922061755004291746261402192509904289898459;
    uint256 constant IC1x = 1558734369049746697952640104253921254898764018200848457720826224009056999005;
    uint256 constant IC1y = 18127697603900707728247442431488961884820628396130258459294351676883845945002;
    uint256 constant IC2x = 3152875765638908417500159434817472800653734834353892863784934422295330996181;
    uint256 constant IC2y = 7013635744716248621129666769981761041937685480938571490789882476974945856680;
    uint256 constant IC3x = 8332826272566681864342892124822289826173019028426561407009704470851623608001;
    uint256 constant IC3y = 11632762743116936049935022608672439285097047087194653417694042142402445056434;
    uint256 constant IC4x = 12276903449191025714451464174332145690615691234618249440205189164818515313382;
    uint256 constant IC4y = 4650125818030075823659100454665793344463449071559427983024989714504727434790;
    uint256 constant IC5x = 21204675508163310882547576445804029459156399479901468250744214209954347523277;
    uint256 constant IC5y = 10477294313986548841196069360554036450351314404914059244039864269396706960570;
    uint256 constant IC6x = 14944556307776262817591069174239778320690105263725503219108660006243607133244;
    uint256 constant IC6y = 13867001595904989024043726647064011659196335236060628576705032534105702419906;
    uint256 constant IC7x = 10614927444555195776761498209164123907234689327507818025061547013152085736789;
    uint256 constant IC7y = 20348377614642680917194521993430145338085115451381997589162557032897783027434;
    uint256 constant IC8x = 7982507473454873494181329964374306494472671216507232996538836858138220155811;
    uint256 constant IC8y = 16763771406583008625554030973964995079796911435328657133449855497419182403262;
    uint256 constant IC9x = 9636884253791843754867678693713270815646208722892911370770576555602210265704;
    uint256 constant IC9y = 3328651983408165353683933242105885242742449427434055288007532435598900589532;
    uint256 constant IC10x = 11168158961604874035936075490124823774434193077679674067127558480412572747563;
    uint256 constant IC10y = 9600689536602038679707565776445342385217009482204389061799352346197588368918;
    uint256 constant IC11x = 14374148611733578177517150378262787578372440893516562486085694431835454758426;
    uint256 constant IC11y = 9299059664014871663709864852585833457668983245170923909672736833652782617274;
    uint256 constant IC12x = 18015811751624466142463831286820379682462072317701069805210701105367393897835;
    uint256 constant IC12y = 3904394875918036011842166734323892256481171858211408768420359086497075315574;
    uint256 constant IC13x = 14670484012199552978075949155023553215337275493629461693908510837918965790487;
    uint256 constant IC13y = 14095145594489153349996054319706213869307918706656948127756269691913866500161;
    uint256 constant IC14x = 887728868720748786467341299050533658842618801644836818140575847876905152271;
    uint256 constant IC14y = 17370272822478040934710502167752172746620069511459533635776938403594255063696;
    uint256 constant IC15x = 4045427270853195144371204809080028371175545226481341114472307233587155169534;
    uint256 constant IC15y = 15962751523939316997999576647166012403436190969788665878750239644499233279679;
    uint256 constant IC16x = 12570559810670219115656319385461469119622028476276620561683804066622280065549;
    uint256 constant IC16y = 2747891594027751485345111137727202739013121713068790204385191624006963938506;
    uint256 constant IC17x = 140995889985912184117991634572976522025728261723575299465154251999392373035;
    uint256 constant IC17y = 2103379252880190396155406073295320319481709049025288917270807006263430526444;
    uint256 constant IC18x = 18975919452873405402873854095197230491529594853496930469863480581366354738514;
    uint256 constant IC18y = 10984785770711983990914380594543917035405011577288824614563301883779449796492;
    uint256 constant IC19x = 13660810169145599409652802072461280367369359278320280176396821544538580106274;
    uint256 constant IC19y = 13220456966926414929208628666654465173713601541812529388364605065489394379305;

    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    function verifyProof(uint[2] calldata _pA, uint[2][2] calldata _pB, uint[2] calldata _pC, uint[19] calldata _pubSignals) public view returns (bool) {
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

            // Validate all evaluations
            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)

            mstore(0, isValid)
             return(0, 0x20)
         }
     }
 }
