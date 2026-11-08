# Engine Specification

## Overview

The job of the engine is to keep robust and correct accounting for clients.
The engine is streamed transaction events that add new accounts and modify their balances.
When the stream of events finishes the program outputs the state of accounts.

## Event types

### Deposit

Deposit represents depositing of funds `amount` into `client_id` account.

#### Flow:

1. We check validity of the transaction event. 
2. If there is no account yet for associated client, we create a new account and credit it and write the transaction to storage.
3. If there is already an account under associated client:
  - and is not locked: we credit the account and record the transaction.
  - but is locked: we deny the transaction and log the reason to stderr

### Withdraw

Withdraw represents withdrawal of funds `amount` out of `client_id` account.

#### Flow:

1. We check validity of the transaction event.
2. If there is no account yet for associated client, we create a new account, with zero values and deny the transaction logging the reason to stderr.
3. If there is already an account undter associated client:
  - and is not locked: we debit the account and record the transaction.
  - but is locked: we deny the transaction and log the reason to stderr

### Dispute

Dispute represents a claim that a particular `tx_id` was erroneous.
A transaction can be disputed again after being resolved.
Locked accounts can still have disputed Deposits.

Only Deposit transactions can be disputed.

(The spec clearly outlines how the funds move in a dispute / resolve and chargeback procedures. The flow of the funds in the dispute process is clearly outlined only for disputing the deposits, not withdrawals. I can see the value in disputing withdrawals when protecting clients whose fund were withdrawn to a wrong or malicious address. Nevertheless i opted for Deposit only as disputing withdrawals would clash with spec too much.)

#### Flow:

1. We check validity of the transaction event.
2. We mark the transaction as disputed and change accounting by decreasing the `available` and increasing the `held` funds by same amount in client's account.

### Resolve

Represents the release of the `held` funds on a transaction marking it as successfully resolved.

#### Flow:

1. We check validity of the transaction event.
2. We mark the transaction as having no dispute anymore and change accounting by increasing the `available` and decreasing the `held` funds by same amount in client's account.

### Chargeback

Represents the release of the `held` funds back to the client. This locks the client's account.

#### Flow:

1. We check validity of the transaction event.
2. We mark the transaction as having been charged back and decrease the `held` and `total` balances.
3. Lock the account.

## Decisions

- No tracing crate & no fancy logging.
- No multithreading. No transaction event actually crosses clients.

## Assumptions

- Transactions streamed are ordered chronologically

## Input

a csv file with rows that are defined as:

```rust
enum TxEventType {
  Deposit,
  Withdrawal,
  Dispute,
  Resolve,
  ChargeBack
}

struct TxRow {
  type: TxEventType,
  client: u16,
  tx: u32,
  amount: Option<Decimal>
}
```

the rows represent different events during the accounting.

## Output

The program outputs the state of the client accounts in a form of rows to STDOUT.

```rust
struct Client {
  client: u16,
  available: Decimal,
  held: Decimal,
  total: Decimal,
  locked: bool
}
```

Any invalid transaction is reported to STDERR referencing line number and reason for rejection.

Program terminates with either status code 0 (success) or 1 (io fatal )

## Entities

pub type TxId = u32;
pub type ClientId = u16;

pub enum Event {
   Deposit    { client: ClientId, tx: TxId, amount: Decimal },
   Withdrawal { client: ClientId, tx: TxId, amount: Decimal },
   Dispute    { client: ClientId, tx: TxId },
   Resolve    { client: ClientId, tx: TxId },
   ChargeBack { client: ClientId, tx: TxId },
}

pub enum MoneyTx {
  Deposit(Decimal),
  Withdrawal(Decimal)
}

pub enum Dispute {
  Open,
  ChargedBack
}

pub struct Tx {
  client: ClientId,
  kind: MoneyTx,
  dispute: Option<Dispute>
}

pub struct Client {
  available: Decimal,
  held: Decimal,
  is_locked: bool
}

impl Serialize for Client {
  ...
  total() = held + available
}

pub struct Storage {
  accounts: HashMap<ClientId, Client>,
  transactions: HashMap<TxId, Tx>,
}

impl Storage {
  pub fn create_account;
  pub fn get_accounts;
  pub fn create_tx;
  pub fn get_tx;
}

struct Engine;
impl Engine {
  pub apply_event(ev: Event, &mut Storage) -> Result<(), EngineError>
}

## Errors

pub enum EngineError {

  ZeroAmount
  NegativeAmount
  AmountOutOfBounds
  
  TxIdAlreadyExists  
  TxIdDoesNotExist

  ClientDoesNotExist
  ClientIsLocked
  NotEnoughFunds
  BalanceOverflow

  DisputeDoesNotExist
  DisputeAlreadyExists
  DisputeAlreadyChargedBack
  DisputeByWrongClient

  TooPrecise // when amount is precise more than 4 digits
  
}

