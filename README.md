# multisig-contract

# Introduction

This project implements a multi-signature smart contract on the Concordium blockchain. It also includes scripts that interact with the blockchain (testnet).

# Functionalities of the Smart Contract

The smart contract supports the following functionalities:

Write functions:

- `initialize()`: To initalize the smart contract and build state using the initial parameters.
- `insert()`: Allows the smart contract to receive ccd tokens.
- `create_transaction()`: Creates a transaction proposal pending approval from the signatories.
- `approve()`: Approves a transaction proposal, can only be called by a signatory.
- `transfer()`: Excutes a transaction if proposal is approved by all signatories.

Read functions:

- `view()`: To check the state of a deployed smart contract, returns a transaction proposal when given an tx_id.
- `get_administrators()`: Returns all the signatory of a Smart contract module.
- `get_approvals_remaining()`: Returns the number of approvals needed for transaction to be excuted.

# Running the code

The code was deployed and initialized and updated with the following command:

```
./concordium-client_6.1.0-1 --grpc-ip node.testnet.concordium.com module deploy concordium-out/module.wasm.v1 --name ccd_multisig --sender mywallet
```

The code ran succesfully and returned a module reference, and an initialization transaction hash

```
Deploying module....
Sent transaction with hash: b9ad7f9eabaedd45b7cdec88a784c9fe53606aff18dced28e9d8e948207dc9a1
Transaction finalized: tx_hash=b9ad7f9eabaedd45b7cdec88a784c9fe53606aff18dced28e9d8e948207dc9a1 module_ref=f211526f3850a523ddf8cdc322bd5cbd37fae67a4791e489bde15947c4b75c08
```

### A screenshot of the contract deployment on the concordium block explorer

![Screenshot of deployment](./img/deploy.png)

Signatories accounts used for this demonstration are:

```
4eG4bkdeJLMX2cCjYqoJnFufKB87dGMiGSBBeN2stvVhqR48ww
4UD4rW7cCvB9ZBs2CQSodtSgEzhvyFeVXymewCFYeQG7xKAnfe
3NV3wrJvKUKCarhG79sj2kTPUhd3bvsZtYWRauTqb8VNG378bF
```
