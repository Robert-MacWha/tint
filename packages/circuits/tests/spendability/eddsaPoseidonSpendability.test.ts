import { WitnessTester } from "circomkit";
import { describe, before, it } from "mocha";
import { circomkit, randomBigInt } from "../common";
import { buildEddsa } from "circomlibjs";
import { poseidon2 } from "poseidon-lite";


describe("eddsaPoseidonSpendability", () => {
    let circuit: WitnessTester<string[], string[]>;

    before(async function () {
        this.timeout(30_000);
        circuit = await circomkit.WitnessTester("eddsaPoseidonSpendability", {
            file: "spendability/eddsaPoseidon",
            template: "EddsaPoseidonSpendability",
            params: [2],
            pubs: [
                "nullifier", "commitmentsOut", "boundParamsHash",
            ],
        });
    });

    it("should pass for a balanced transaction", async () => {
        const eddsa = await buildEddsa();
        const F = eddsa.F;
        const privKey = Buffer.from(
            "0001020304050607080900010203040506070809000102030405060708090a",
            "hex"
        );
        const pubKey = eddsa.prv2pub(privKey);
        const pubKeyAx = BigInt(F.toObject(pubKey[0]));
        const pubKeyAy = BigInt(F.toObject(pubKey[1]));

        const nullifier = randomBigInt();
        const commitmentsOut = [randomBigInt(), randomBigInt()];
        const boundParamsHash = poseidon2([pubKeyAx, pubKeyAy]);
        const message = F.e(nullifier);
        const signature = eddsa.signPoseidon(privKey, message);
        const signatureR8x = BigInt(F.toObject(signature.R8[0]));
        const signatureR8y = BigInt(F.toObject(signature.R8[1]));
        const signatureS = signature.S;

        const input = {
            nullifier,
            commitmentsOut,
            boundParamsHash,
            pubKeyAx,
            pubKeyAy,
            signatureR8x,
            signatureR8y,
            signatureS,
        };

        await circuit.expectPass(input);
    });
});
