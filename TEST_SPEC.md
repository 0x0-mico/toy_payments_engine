# Test plan

Three levels: unit tests inside each module, directory cases driven through
the compiled binary, and two extra integration binaries for the exit-status
contract and the heavy stress dataset.

## 1. Layout

```
tests/
  cli.rs                    harness, one #[test] per registered case
  exit_codes.rs             exit-status contract: 0 success, 1 fatal IO, 2 usage
  stress.rs                 ignored heavy case over ~1.36M generated rows
  cases/
    <case_name>/
      input.csv             the only argument passed to the binary
      expected_output.csv   exact stdout
      expected_errors.txt   exact stderr, empty file when the case logs nothing
```

`tests/cli.rs` discovers `tests/cases/*/`, runs the binary once per case with
`input.csv` as the only argument, and compares both streams against the
expected files. Every case must exit 0: a rejected row is logged, never
fatal. The exit-status contract itself (0 on success, 1 on fatal IO, 0 when
every row is rejected, 2 on usage errors) lives in `tests/exit_codes.rs`.

Comparison rules:

- bytes are decoded as UTF-8, `\r\n` is folded to `\n`, trailing whitespace
  on each line and the final newline are ignored;
- `stdout` is compared against `expected_output.csv` in full;
- `stderr` is compared against `expected_errors.txt` in full, line order
  included;
- exit status is asserted to be 0 in every case.

Expected files are written from the specification and reviewed by hand, not
regenerated from the binary's own output. Where the task statement declares
a difference irrelevant, such as the printed scale of round decimals, one
form is chosen once and pinned. A failing comparison is resolved by deciding
which side is wrong.

## 2. Unit tests

One `#[cfg(test)] mod tests` per module, exercising the public surface.

| Module | Cases |
| --- | --- |
| `client` | new account starts with the passed amount; credit; debit; debit of the exact balance; debit beyond balance; credit and debit refused on a locked account; `put_to_held` moves available to held; `put_to_held` with less available than the amount leaves available negative and total unchanged; `resolve_held` reverses it; chargeback removes held and locks; chargeback with insufficient held; overflow on credit; `ClientRow` total, locked flag and overflow when `available + held` exceeds `Decimal::MAX` |
| `transaction` | amount for a deposit and for a withdrawal; open dispute on a deposit; open dispute on a withdrawal rejected; open dispute by a foreign client; second open dispute; open dispute after chargeback; resolve an open dispute; resolve without a dispute; resolve after chargeback; chargeback an open dispute; second chargeback; chargeback without a dispute; `reset_dispute` restores a previous state |
| `storage` | `init_client` on a new client; `init_client` leaves an existing account untouched; `get_client_mut` on an unknown client; `add_tx` then `get_tx_mut`; `get_tx_mut` unknown; `check_used_tx_id` free and taken; `sorted_clients` orders by client ID |
| `event` | deposit, withdrawal, dispute, resolve and chargeback rows; dispute row with three fields; missing amount on deposit and on withdrawal; zero amount; negative amount; five decimals; six decimals with trailing zeros accepted; four decimals accepted |
| `engine` | deposit to a new client; deposit to an existing client; deposit on a locked client; withdrawal within funds; withdrawal beyond funds; withdrawal from an unknown client creating a zeroed account; dispute moves the amount to held; resolve returns it; chargeback clears it and locks; dispute by a foreign client; dispute of a withdrawal; duplicate transaction ID; dispute of an unknown ID |

## 3. Integration cases

39 case folders, each proving the property named in the `asserts` column.

| Case | Input | Asserts |
| --- | --- | --- |
| `pdf_sample` | the example from the task statement | published balances, one rejected row, exit 0 |
| `deposit_single` | one deposit | account creation, total |
| `withdrawal_within_funds` | deposit then withdrawal | debit |
| `withdrawal_exact_balance` | withdrawal of the full balance | `available` reaches 0 |
| `withdrawal_insufficient` | withdrawal larger than the balance | rejection, balance unchanged |
| `withdrawal_unknown_client` | withdrawal by a client never seen | zeroed account appears in output |
| `multiple_clients_interleaved` | three clients, interleaved rows | independent accounts |
| `dispute_resolve` | deposit, dispute, resolve | funds return to available |
| `dispute_chargeback_locks` | deposit, dispute, chargeback | held and total released, account locked |
| `dispute_reopen_after_resolve` | dispute, resolve, dispute, chargeback | a resolved dispute can be reopened |
| `chargeback_after_resolve_rejected` | resolve then chargeback | rejection, account untouched |
| `double_chargeback_rejected` | two chargebacks | second rejected, account still locked |
| `resolve_without_dispute` | resolve on a cleared transaction | rejection |
| `dispute_unknown_tx` | dispute of an unused ID | rejection |
| `dispute_foreign_client` | dispute by a client that does not own the transaction | rejection, no account created |
| `dispute_withdrawal_rejected` | dispute of a withdrawal | rejection |
| `dispute_after_chargeback_rejected` | second dispute after chargeback | rejection, state terminal |
| `locked_blocks_deposit` | deposit after a chargeback | rejection, locked flag stays |
| `locked_blocks_withdrawal` | withdrawal after a chargeback | rejection |
| `negative_available_on_dispute` | dispute of funds already withdrawn | `available` negative, `total` unchanged |
| `negative_total_on_chargeback` | the same then chargeback | `total` negative, locked |
| `duplicate_tx_id` | the same transaction ID twice | rejection |
| `max_ids` | client 65535, transaction 4294967295 | full-width IDs |
| `min_values` | client 0, transaction 0, amount 0.0001 | zero identifiers, smallest amount |
| `precision_four_places` | 1.2345 then 0.0001 | sums stay exact |
| `precision_trailing_zeros` | 1.000000 | accepted, printed with its own scale |
| `precision_too_precise` | 1.00005 | rejection |
| `amount_negative_rejected` | -1 | rejection |
| `amount_zero_rejected` | 0 | rejection |
| `amount_missing_on_deposit` | three-field deposit row | rejection |
| `unknown_type_skipped` | unknown token, then a valid row | row skipped, stream continues |
| `amount_non_numeric` | `abc` as amount | row skipped |
| `header_only` | header row and nothing else | header on stdout, no rows |
| `empty_file` | zero bytes | header on stdout, no rows |
| `crlf_line_endings` | CRLF file | same balances as LF |
| `whitespace_padded` | spaces around header and fields | trimmed and accepted |
| `corrupt_row_mid_file` | valid, corrupt, valid | both valid rows applied |
| `amount_max_overflow` | deposit of `Decimal::MAX`, then 1 | overflow rejected, first balance intact |
| `big_dataset` | 1000 generated rows over 20 clients | every balance equals the closed form |

## 4. Exit statuses

`tests/exit_codes.rs` drives the binary directly: a valid stream exits 0, a
stream whose rows are all rejected exits 0 with a header-only output, an
empty file exits 0, and a missing input file exits 1 with a fatal message on
stderr. Usage errors are outside the 0/1 processing contract: missing or
extra arguments make the argument parser print a usage message and exit 2,
and `--help` exits 0.

## 5. Stress dataset

`scripts/generate_stress.py` writes a dataset of repeating, cycle-invariant
68-row blocks (default 20000 cycles = 1,360,004 rows, 29.8 MB) plus a
closed-form `expected_output.csv`. Every cycle embeds all failure modes of
the smaller cases: every amount failure (zero, negative, non-numeric,
missing, five decimals, `Decimal::MAX` overflow), every tx failure
(duplicate ID same and cross client, unknown ID), every dispute failure
(withdrawal dispute, double dispute, resolve without dispute, chargeback
without dispute, chargeback after resolve, all post-chargeback probes),
foreign-client events, locked-account deposits and withdrawals, the
zeroed-account withdrawal, negative `available` and negative `total`
accounting, unknown type, non-numeric client and tx fields, a two-field row,
and the id boundaries (client 0 and 65535, tx 0 and `u32::MAX`). Balance-
dependent probes (exact and insufficient withdrawals) live on dedicated
clients so the block stays cycle-invariant.

`tests/stress.rs` regenerates the dataset, runs the engine, and compares the
output numerically against the closed form (the specification declares the
printed scale of round decimals irrelevant). The test is `#[ignore]`d so the
default suite stays fast:

```sh
cargo test --test stress -- --ignored --nocapture   # needs python3
```

Measured on this machine: generation 0.3 s; engine (release) 2.8 s with a
peak RSS of 24.6 MiB; engine (debug) 6.3 s with 25.7 MiB. The result was
additionally cross-checked against an independent re-implementation of the
specification: engine, closed form and simulation agree on all 26 clients.

## 6. Out of scope

- stdin input and argument reordering;
- concurrency. The engine is single threaded by decision.
