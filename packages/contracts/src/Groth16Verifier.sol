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

    uint256 constant IC0x = 17158156676106371811183808038436390809826440561440243803763842982670818921300;
    uint256 constant IC0y = 5736064145189755371842181837364337392462019911977568658404457851850959230634;
    uint256 constant IC1x = 4141591942682167128749071966095891839511939028763658251649380357257776960531;
    uint256 constant IC1y = 19073196396418876641873628946379545104580910843803232098935840747085434053144;
    uint256 constant IC2x = 15245902790366555264199369468447644762070043908970859494376215898372386259905;
    uint256 constant IC2y = 11949508512537332025836715402125546388523728427142669142974494051341889874489;
    uint256 constant IC3x = 9305524157095400798528137543855926410973738130475199649019892051373786398949;
    uint256 constant IC3y = 2238919025488776905946142417936307261858489902343738892042075331505795169341;
    uint256 constant IC4x = 12843025944771610057571035671884792275544370148494642084187583586346792698363;
    uint256 constant IC4y = 12254143583813756375948550346100832588277941931109192627520995545564307814282;
    uint256 constant IC5x = 2265500373152088510955421605863076532378858883721226673444795103962196465040;
    uint256 constant IC5y = 15468393288831098079964414436778468565975568442096937489469401631872997366082;
    uint256 constant IC6x = 9401057312849474606006025037222541449021155592069847634390367120330208166453;
    uint256 constant IC6y = 1175200510514283866656850773146441671910481684270303193879353670715147825258;
    uint256 constant IC7x = 8141633768123163107248581986083706036417426590659546187610992616046862452904;
    uint256 constant IC7y = 2820900250525611268805425964518673480327831064530288479579993008076885101854;
    uint256 constant IC8x = 19647005044689719894593613988277147797984210086327712318294804600052984643315;
    uint256 constant IC8y = 21469587048514156499464964670439632365984275679940295217879410064746839614878;
    uint256 constant IC9x = 2404174708093482458678812302062824480689293185647235655731625523169268874949;
    uint256 constant IC9y = 8753021120633285895977833304359129947037871241219417600591163216978210419749;
    uint256 constant IC10x = 20621521714621546640172554569075626393035557683195572994664821999647070055952;
    uint256 constant IC10y = 5467317816707932483017412121122809122587766260621097715945577555752420209472;
    uint256 constant IC11x = 17271971021772525888258279623380891808227364787435846550806457390412867951588;
    uint256 constant IC11y = 13823100293578235882347352406642104824505403709881270860909322378570498269143;
    uint256 constant IC12x = 9056955992246797839779778494163102189602281604581660602460061891504237235494;
    uint256 constant IC12y = 1502860054052324247924987593545308880907407007168376414018297103723138262686;
    uint256 constant IC13x = 10880891608351582912594852720384040529768629081166904255426280245584096997088;
    uint256 constant IC13y = 6609667596023242206237368617944617665285359366827511049538140873467284169545;
    uint256 constant IC14x = 5310828331409989831475458036981216022527103278506584825778592372989107039782;
    uint256 constant IC14y = 12162463198196277331244762342193858119410592169202129196782957104397554020168;
    uint256 constant IC15x = 17255105319254035918333227974569478758556012203198390735137706631494538490498;
    uint256 constant IC15y = 9515071117810231975988427440794011575904951632718870670335422935625894545210;
    uint256 constant IC16x = 12199608677283706728348206946791129809828388110407775221625173762966653338038;
    uint256 constant IC16y = 5272544681437563681607730716625492010067320708214888681652770286616867938076;
    uint256 constant IC17x = 4198740313847395468495744171207645955557877565502224065548107023083984889739;
    uint256 constant IC17y = 16371831351414530816656645401358709971129769754638780581485581907184673635371;
    uint256 constant IC18x = 3576338113321007899287045044703567069023510576370070866471523824293193802444;
    uint256 constant IC18y = 19519740789935262565054229975043755680005973214354089232245795717842511605315;
    uint256 constant IC19x = 7506164344768770821644403128693161929679866940993821034781799099405654308425;
    uint256 constant IC19y = 19557075846449915824336275940370888066361919541852803692444424911291474033107;
    uint256 constant IC20x = 12598750056307494094632086675366262691686151499566918762196577866148514908840;
    uint256 constant IC20y = 19095168501416836450381052983364348584310529496845670348161961057166515861136;
    uint256 constant IC21x = 8749723849288149407104526819725292095793994427891700160608520998775283490867;
    uint256 constant IC21y = 3849053378887454693144996588179620351369091152141524605316425153563001050367;
    uint256 constant IC22x = 2151170652193914642850872398042377449365529911667131256813146695612804097575;
    uint256 constant IC22y = 8242783047426520997687759427491887914555843753199827814330854714661023579589;
    uint256 constant IC23x = 17167648836003406987681081353014483312864754395521829060643747844275418215491;
    uint256 constant IC23y = 15164215964810647935716626876298181679907443466003792319435514237852938456448;
    uint256 constant IC24x = 4802647466886996601806291333515365949819125515963918472995825201449939498994;
    uint256 constant IC24y = 884235143017355503832183719502722921926204126378604295539900558431392473012;
    uint256 constant IC25x = 16741787587001000818445346577298941215115071716808090347856117575748975741799;
    uint256 constant IC25y = 2092388378409950927145307233923486214286817462775677514577030692376125569340;
    uint256 constant IC26x = 9811117281592239728866110548299694691295654707479472338573342221685434451312;
    uint256 constant IC26y = 13208146191379586009055769282763689761016330579082154955476980233129276460069;

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
