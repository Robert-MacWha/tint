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
