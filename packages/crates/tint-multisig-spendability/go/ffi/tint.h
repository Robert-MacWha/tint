#ifndef TINT_H
#define TINT_H

#include <stdint.h>

enum
{
	N_INPUTS = 5,
	N_OUTPUTS = 5,
	N_WITHDRAWALS = 2,
	N_SIGNERS = 3,
	THRESHOLD = 2,
};

typedef struct
{
	uint8_t data[32];
} Bytes32;

typedef struct
{
	Bytes32 asset_fr;
	Bytes32 amount_fr;
	Bytes32 spendability_hash;
	Bytes32 random_fr;
	Bytes32 nullifier_pub_key;
} TintBaseCommitment;

typedef struct
{
	TintBaseCommitment inner;
	Bytes32 spendability_address;
	Bytes32 spendability_witness;
} TintSpendableCommitment;

typedef struct
{
	Bytes32 asset_fr;
	Bytes32 amount_fr;
} TintWithdrawal;

typedef struct
{
	TintSpendableCommitment inputs[N_INPUTS];
	TintBaseCommitment output_commitments[N_OUTPUTS];
	TintWithdrawal output_withdrawals[N_WITHDRAWALS];
} TintOperation;

typedef struct
{
	Bytes32 x;
	Bytes32 y;
} TintPubKeyXY;

typedef struct
{
	Bytes32 r;
	Bytes32 s;
} TintSignatureRS;

typedef struct
{
	Bytes32 spendability_address;
	TintOperation operation;
	TintPubKeyXY pub_keys[N_SIGNERS];
	TintSignatureRS signatures[N_SIGNERS];
} TintProveInput;

#endif
