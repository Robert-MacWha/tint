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

    uint256 constant IC0x = 15024929870310595464602957810811898876359279699650534008245459619513202533397;
    uint256 constant IC0y = 1625094540734393942882927347322646275685005208280277896102603258795266875104;
    uint256 constant IC1x = 6212295951994718878750432446633406430440320935755864461128537969058709030527;
    uint256 constant IC1y = 3249427127765822766344504234694995382257642259106683288813236493474615025756;
    uint256 constant IC2x = 7467801853896506674100217785009951312680636156901909872350839402889961509560;
    uint256 constant IC2y = 13273883595998116566172527667053119378618886646411889716077595805997910122985;
    uint256 constant IC3x = 7346997161699532777227176845581602785405816459850811098405258398143603683516;
    uint256 constant IC3y = 13818511829401300780939561566799762697563961911972358308050509938062669351463;
    uint256 constant IC4x = 19823532200926284951060209505540170169863117134339294034125195888644319278820;
    uint256 constant IC4y = 9839672488122498613415092434814133456080017145982352126052732732584111507135;
    uint256 constant IC5x = 2987916806195919118637521835345987065998558390933330311843685079019462012107;
    uint256 constant IC5y = 14350057038782193381007259308775684202852329099574824084822404663895123317592;
    uint256 constant IC6x = 15192529450790675069921796469640820046206602912213952497146070238024717729367;
    uint256 constant IC6y = 10453870714415698730827940700856345470136460380605290435105599716969257245081;
    uint256 constant IC7x = 10550366200063898402207920625045551422043295433242390003530162431304115722041;
    uint256 constant IC7y = 12444689993477347139025012549034613601245607280121676588506730317230479737168;
    uint256 constant IC8x = 7226814985696157822660998969692813107808823854670119100351292443937525356089;
    uint256 constant IC8y = 9812590246566822212181556984259047587813966795842509453751294720347955007153;
    uint256 constant IC9x = 899706208714955145733523126856971080536150394800453291416595089752538799029;
    uint256 constant IC9y = 19561634855023248363336498750647292566830227828794115840069257667470460989349;
    uint256 constant IC10x = 7591646937498799312366649510424737169941216455891626178887765387278785726653;
    uint256 constant IC10y = 6982808997991326765219130026406780170837960050856750398980444430075384528602;
    uint256 constant IC11x = 15780901940282505227737353998922787179781163721497411270111885493367582932631;
    uint256 constant IC11y = 6741492297631048526757878172717694306317638563650788190765745189095061839606;
    uint256 constant IC12x = 15628077940588645700069780895322924937236661760911857930158702095249480541060;
    uint256 constant IC12y = 252661738110723407526038563974163472522641628619952585014159492081012124867;
    uint256 constant IC13x = 5786288645695394260943111159170673350029395726319739227080551024759973449204;
    uint256 constant IC13y = 19439852451683075697891172177580272503770755371985246263289350592186744742249;
    uint256 constant IC14x = 18057900012226958376782339029478084442133654097032468633306640624882849800621;
    uint256 constant IC14y = 20336572862740336225872810949991511194540367740866041891787147196609121022059;
    uint256 constant IC15x = 8102200805977450511184923117339252412666559627623748878392020386957100608632;
    uint256 constant IC15y = 4039712260044879392026099377140563345844140230307911780519013335445096555322;
    uint256 constant IC16x = 5213375087358392487203930369279575881983493140590240742477353539270448512046;
    uint256 constant IC16y = 7310371617069557255626706087948836370462492416532824723778403628157756269037;
    uint256 constant IC17x = 20466236586673638355205527099049379354293510232016205982065887123442957619149;
    uint256 constant IC17y = 7888628895103708003042232710633627313997855419354882060600320576457363069705;
    uint256 constant IC18x = 18983928195824546233554946612829439777867198731560143915603739219426415421676;
    uint256 constant IC18y = 12037164360190919704098160374655230716708107120854227596832323823689098425228;
    uint256 constant IC19x = 18994328561440212065482224302129517148173701550797160398081219816811326190223;
    uint256 constant IC19y = 12088148137366493188777078067058556507573189052053148839837597122012865832921;
    uint256 constant IC20x = 19873682754041136732036386399463643596786099288552649218640650146759734302733;
    uint256 constant IC20y = 529491433499278035557315248072321009884338726123654984730947043488339981746;

    // Memory data
    uint16 constant pVk = 0;
    uint16 constant pPairing = 128;

    uint16 constant pLastMem = 896;

    function verifyProof(uint[2] calldata _pA, uint[2][2] calldata _pB, uint[2] calldata _pC, uint[20] calldata _pubSignals) public view returns (bool) {
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

            // Validate all evaluations
            let isValid := checkPairing(_pA, _pB, _pC, _pubSignals, pMem)

            mstore(0, isValid)
             return(0, 0x20)
         }
     }
 }
