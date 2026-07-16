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

    uint256 constant IC0x = 6959051936446084520644965669691188859985847200245841367414206240248330590197;
    uint256 constant IC0y = 3615105212129535646185736197418482286746581379067700368688343690169971650268;
    uint256 constant IC1x = 16558647796224829687649331833786877438996278726846408233616999907547531495458;
    uint256 constant IC1y = 8526648944819334522862945611137350466038563735182901299468716867918944013684;
    uint256 constant IC2x = 7034075316205505069935293782078514208372865753739859247115042522313916015336;
    uint256 constant IC2y = 10305396225007036505218098722743966378864570712659211363725198753612601353143;
    uint256 constant IC3x = 9058584611845061116248618187244553134456582839229628405559619586579447450765;
    uint256 constant IC3y = 8364224082505546227652577798763020291990427170099496214298874982015562307126;
    uint256 constant IC4x = 21410314009284418247945667432411342465908891236175685788679356367347139594427;
    uint256 constant IC4y = 6468967246996770181393394622019019438670212232948823189138626819162115811767;
    uint256 constant IC5x = 17925043957367913464884676781766713094122004303594920176040425655841366262054;
    uint256 constant IC5y = 12621717537632121602217695870452362901893001202427435549110962734130342806285;
    uint256 constant IC6x = 13503463573949256531080879270937836647973416359317614974398417536359213809016;
    uint256 constant IC6y = 16434380273063071279230144309762407267319005803519687435541159285360809703517;
    uint256 constant IC7x = 20720266483401392418251501321430437049002733573231500722691339602308063145559;
    uint256 constant IC7y = 20426396338253546883190060146434662196496228679483252311226384187963662161294;
    uint256 constant IC8x = 20137106902939302838214756657096726275399611419178574833230920787337264646659;
    uint256 constant IC8y = 14451549676471699982331420884440343015642608415735761833698843077611058672429;
    uint256 constant IC9x = 18528359041878674084559335020156664666184900603493666930594352275332747030545;
    uint256 constant IC9y = 9085515330049306458095216375919219553299109753311965472230109721080820670672;
    uint256 constant IC10x = 12247986758613091440400321833328469626175253762555575455555242895483028175054;
    uint256 constant IC10y = 12945332554348562531988253499528347535124729670612092825123054590956713417946;
    uint256 constant IC11x = 16607784615110301796305488760502207417682974357538126543175128827604370162830;
    uint256 constant IC11y = 17178703268424373356383498791406087173140506843138784799142947049650331659996;
    uint256 constant IC12x = 9288532173727115210587170012598183464637010845788075019836966135861428417172;
    uint256 constant IC12y = 21791809245683703850541200368740479476050464253678796368335190536921947354177;
    uint256 constant IC13x = 19559683146607084805160074663692454645954200845505962792362416421676822314874;
    uint256 constant IC13y = 8365806241308215188057163192055566904317810624690207025119765219490028353403;
    uint256 constant IC14x = 20332265699163479953215105278561792566929907650575027463824767079136699805925;
    uint256 constant IC14y = 14336187399588946400757532949804580550789991844943690266863526403395319002649;
    uint256 constant IC15x = 5914717837695892540643055047927706161507334728347533571037103586536033453987;
    uint256 constant IC15y = 4576479383202645444855813214746457255712585455134333396138539062158018576119;
    uint256 constant IC16x = 10020024975161193136669363614851294954980304900724426945828037230604868153340;
    uint256 constant IC16y = 8579609330437230738118267002534767351212004202985327170786819375501059069253;
    uint256 constant IC17x = 16307017212075572509787251801443743815623259194753985421887193251289724299024;
    uint256 constant IC17y = 1919720457424301207458126562845694901736563909562018576641281555329151394214;
    uint256 constant IC18x = 6345764836803079193346479185640373607392185718822175949640929771657157684779;
    uint256 constant IC18y = 5987317145873285925563185301085592436312401653542335567459041954838481117435;
    uint256 constant IC19x = 10893886724242835967139318650002579152246020502026538442259673955350797439659;
    uint256 constant IC19y = 5840643393492194253576160410811006130618260638124707422607518671813061669399;
    uint256 constant IC20x = 7816433285853652512275377928811022518052024408155988801951989034278993454025;
    uint256 constant IC20y = 12540549764370486132735088052273809010872217249920829088933003235890394936365;
    uint256 constant IC21x = 12673319099046591201247657142077637729578155071860481368945027760802230727141;
    uint256 constant IC21y = 12597786684395920488847765734160622449820822862659273032743125982802885017681;
    uint256 constant IC22x = 12946221333299214791819379132627732651005840536097020349761718303173358510797;
    uint256 constant IC22y = 269963696470251146419006800009617265808281609845185136164552376278668282104;
    uint256 constant IC23x = 13912488495055750671054937949160707925570677872300686662322958968135079799304;
    uint256 constant IC23y = 709214637220038566797563582061452307768809000747480869183534265182025432939;
    uint256 constant IC24x = 2458135008269895248553445880217777386568991231639480677075449879292575198876;
    uint256 constant IC24y = 21830750586985278869283440722258401719967114929633836846597187232774713267880;
    uint256 constant IC25x = 2056093359768570991433958544930983408905513682555137108715861798847148001569;
    uint256 constant IC25y = 7572948506111777110215474911789977191815339553450806476122777461578982619235;

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
