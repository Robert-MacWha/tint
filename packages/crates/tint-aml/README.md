# Anti money laundering spec

New contract where each user will publish a zk proof:
```
public signals:
- Institution public key
- Operation JoinSplit circuit public signals
- Encrypted operation data
    - List of input commitments
    - Any other relevant AML data

private signals:
- Operation JoinSplit circuit private signals

circuit:
- Verify that the encrypted data is the encryption of the relevant operation data
- Verify that the operation private signals match the public signals where relevant
```

Smart contract
```
1. Verifies that the operation has been executed by the privacy protocol
2. Store the public signals & proof for the AML proof
```

## Insight

Table describing what every participant in the system can see and do:

| Situation                                                 | Asset   | Amount  | Counterparty Institution | Counterparty |
| --------------------------------------------------------- | ------- | ------- | ------------------------ | ------------ |
| Deposit (entering tint)                                   | Public  | Public  | N/A [^1]                 | N/A [^1]     |
| Internal transfer (to same-institution counterparty)      | Private | Private | Private                  | Private      |
| External transfer (to different-institution counterparty) | Private | Private | Private                  | N/A [^2]     |
| Withdrawal (exiting tint)                                 | Public  | Public  | Private                  | Public       |

[^1]: Consider whether these should also be public.  Currently, users don't take any action on depositing since a later AML proof will be uploaded that binds spending the deposited notes to the deposit.  If we did make these public, it would enable associating users depositing into tint with the institutions they work with, which may be undesirable. It also means that people could contact those institutions directly though, which may be desirable.
[^2]: Institutions can only see signals from their own users, so they won't know who a counterparty is if they are from a different institution.

## Considering requiring AML proofs at a protocol level

Fold the AML circuit into the JoinSplit circuit.  Therefor, any operation will require:
1. An encrypted blob of AML data, which can be decrypted by the user's institution
2. The user to hold a key signed by their institution.

### Censorship option 1

For a user to perform an operation they must be associated with an institution.  If no institutions are willing
to work with a user, they will be unable to perform any operations.
- Solution: add a "null institution" that any user can join, and where the encryption keys are public.
