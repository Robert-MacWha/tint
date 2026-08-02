#ifndef TINT_H
#define TINT_H

#include <stdint.h>

enum
{
	N_INPUTS = 5,
	N_OUTPUTS = 5,
	N_WITHDRAWALS = 2,
	N_SIGNERS = 3,
};

typedef struct
{
	uint8_t data[32];
} TintBytes32;

typedef struct
{
	TintBytes32 asset_fr;
	TintBytes32 amount_fr;
	TintBytes32 spendability_hash;
	TintBytes32 random_fr;
	TintBytes32 nullifier_pub_key;
} TintBaseCommitment;

typedef struct
{
	TintBaseCommitment inner;
	TintBytes32 spendability_address;
	TintBytes32 spendability_witness;
} TintSpendableCommitment;

typedef struct
{
	TintBytes32 asset_fr;
	TintBytes32 amount_fr;
} TintWithdrawal;

typedef struct
{
	TintSpendableCommitment inputs[N_INPUTS];
	TintBaseCommitment output_commitments[N_OUTPUTS];
	TintWithdrawal output_withdrawals[N_WITHDRAWALS];
} TintOperation;

typedef struct
{
	TintBytes32 x;
	TintBytes32 y;
} TintPubKeyXY;

typedef struct
{
	TintBytes32 r;
	TintBytes32 s;
} TintSignatureRS;

typedef struct
{
	TintBytes32 spendability_address;
	TintOperation operation;
	TintPubKeyXY pub_keys[N_SIGNERS];
	TintSignatureRS signatures[N_SIGNERS];
} TintProveInput;

#endif
