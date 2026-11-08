#!/usr/bin/env python3
"""Generate the stress dataset for the toy payments engine.

Emits a CSV of `cycles` repeating 68-row blocks (default 20000 cycles =
1,360,004 rows) plus `expected_output.csv` computed from a closed form.

The block is cycle-invariant: every cycle leaves the normal clients with the
same net change and the same dispute state, so the final balances are a
closed form of the cycle count. Every failure mode of the integration and
unit suites appears in every cycle:

  money paths        deposit to new/existing client, withdrawal within funds,
                     withdrawal of the exact balance, dispute -> resolve,
                     dispute reopen after resolve, dispute with an amount
                     field (ignored), resolve with an amount field (ignored),
                     extra field on a row (ignored)
  amount failures    zero, negative, non-numeric, missing, 5th decimal place,
                     overflow of Decimal::MAX, trailing-zero scale accepted
  tx failures        duplicate tx id (same client), duplicate tx id (foreign
                     client), unknown tx id on dispute/resolve/chargeback
  dispute failures   dispute of a withdrawal, double dispute, resolve without
                     dispute, chargeback without dispute, chargeback after
                     resolve, dispute/resolve/chargeback after chargeback
  client failures    dispute/resolve/chargeback by a foreign client,
                     withdrawal for an unknown client (creates a zeroed
                     account), withdrawal beyond available, deposit and
                     withdrawal on a locked account
  terminal states    chargeback locks the account; a locked account can still
                     dispute, resolve and re-dispute a deposit
  negative accounting available driven negative by a dispute of withdrawn
                     funds; chargeback driving total negative
  parse failures     unknown type, non-numeric client, non-numeric tx,
                     two-field row
  id boundaries      client 0 and 65535, tx 0, 4294967294, 4294967295

Dedicated probe clients keep the balance-dependent probes cycle-invariant:
client 25 repeats deposit-then-exact-withdrawal-then-insufficient-withdrawal
every cycle; clients 20-24 carry fixed terminal or recurring states.

The closed form (derived from the pattern, not from the engine), K = cycles/20:

  clients 1..19   available 35 * K, held 0, unlocked
  client 0        the same plus 0.0001 from the prologue
  client 20       0/0/0 locked (charged back in cycle 0)
  client 21       0/0/0 unlocked (only a rejected withdrawal)
  client 22       Decimal::MAX (one deposit; every later deposit overflows)
  client 23       -10/5/-5 locked (negative available, open dispute)
  client 24       0/0/0 unlocked (dispute/resolve cycle nets to zero)
  client 25       0/0/0 unlocked (exact and insufficient withdrawals)
  client 65535    2 + cycles (prologue deposits plus one per cycle)
"""

import argparse
from decimal import Decimal
from pathlib import Path

MAX = "79228162514264337593543950335"
UNKNOWN_TX = 999_999_999
PROLOGUE_TX_BASE = 1_000_000


def cycle_rows(cycle: int) -> list[str]:
    c = cycle % 20          # active normal client
    f = (c + 1) % 20        # foreign normal client
    b = PROLOGUE_TX_BASE + cycle * 100
    return [
        # --- normal client c: money paths and their failures -------------
        f"deposit,{c},{b + 1},10.0000",
        f"deposit,{c},{b + 2},1.000000",      # trailing zeros, scale 6
        f"withdrawal,{c},{b + 3},1.0000",
        f"deposit,{c},{b + 4},20",
        f"dispute,{c},{b + 4}",
        f"dispute,{c},{b + 4},5",             # double dispute, amount ignored
        f"resolve,{c},{b + 4}",
        f"resolve,{c},{b + 4}",               # resolve without dispute
        f"dispute,{c},{b + 4}",               # reopen after resolve
        f"resolve,{c},{b + 4}",
        f"withdrawal,{c},{b + 5},1",
        f"dispute,{c},{b + 5}",               # dispute of a withdrawal
        f"dispute,{c},{UNKNOWN_TX}",          # unknown tx
        f"dispute,{f},{b + 4}",               # foreign client
        f"chargeback,{c},{b + 4}",            # chargeback without dispute
        f"chargeback,{c},{b + 5}",            # chargeback without dispute
        f"deposit,{c},{b + 6},5",
        f"deposit,{c},{b + 1},5",             # duplicate tx id, same client
        f"deposit,{f},{b + 1},5",             # duplicate tx id, foreign client
        f"deposit,{c},{b + 7},1.00005",       # 5th decimal place
        f"deposit,{c},{b + 8},0",             # zero amount
        f"deposit,{c},{b + 9},-1",            # negative amount
        f"deposit,{c},{b + 10},abc",          # non-numeric amount
        f"deposit,{c},{b + 11}",              # missing amount
        f"bogus,{c},{b + 12},1",              # unknown type
        f"deposit,x,{b + 13},1",              # non-numeric client
        f"deposit,{c},{b + 14},1,EXTRA",      # extra field, still applied
        f"deposit,{c},{b + 15},{MAX}",        # overflow on top of balance
        f"withdrawal,{c},{b + 16}",           # missing amount
        f"dispute,{c},{b + 6},7",             # amount ignored, applied
        f"resolve,{c},{b + 6},9",             # amount ignored, applied
        f"deposit,{c},xyz,1",                 # non-numeric tx
        f"resolve,{c},{UNKNOWN_TX}",          # unknown tx
        f"chargeback,{c},{UNKNOWN_TX}",       # unknown tx
        f"deposit,{c}",                       # two-field row
        # --- client 25: balance-independent withdrawal probes -------------
        f"deposit,25,{b + 17},10",
        f"withdrawal,25,{b + 18},10",         # exact balance
        f"withdrawal,25,{b + 19},11",         # insufficient
        # --- client 20: locked by chargeback in cycle 0 -------------------
        f"deposit,20,{b + 20},100",           # applied in cycle 0 only
        f"dispute,20,{b + 20}",               # applied in cycle 0 only
        f"chargeback,20,{b + 20}",            # applied in cycle 0, locks
        f"withdrawal,20,{b + 21},10",         # locked: always rejected
        f"resolve,20,{b + 20}",               # charged back: always rejected
        # --- client 21: zeroed account from an unknown-client withdrawal ---
        f"withdrawal,21,{b + 22},5",          # cycle 0: creates zeroed account
        f"dispute,21,{b + 22}",               # tx never recorded
        f"withdrawal,21,{b + 23},1",          # zero balance
        # --- client 22: balance pinned at Decimal::MAX ---------------------
        f"deposit,22,{b + 24},{MAX}",         # applied in cycle 0 only
        f"deposit,22,{b + 25},1",             # overflow, always rejected
        # --- client 23: negative available, then locked --------------------
        f"deposit,23,{b + 26},10",            # cycle 0 only (locked later)
        f"deposit,23,{b + 32},5",             # cycle 0 only
        f"withdrawal,23,{b + 27},10",         # cycle 0 only
        f"dispute,23,{b + 26}",               # cycle 0: available goes negative
        f"chargeback,23,{b + 26}",            # cycle 0: locks, total negative
        f"dispute,23,{b + 32}",               # dispute on a locked account
        f"resolve,23,{b + 32}",               # resolve on a locked account
        f"dispute,23,{b + 32}",               # reopen on a locked account
        f"dispute,23,{b + 26}",               # charged back: rejected
        f"resolve,23,{b + 26}",               # charged back: rejected
        f"chargeback,23,{b + 27}",            # withdrawal, never disputed
        # --- client 24: recurring dispute/resolve, nets to zero ------------
        f"deposit,24,{b + 28},10",
        f"withdrawal,24,{b + 29},10",
        f"dispute,24,{b + 28}",
        f"dispute,24,{b + 28},3",             # double dispute, amount ignored
        f"resolve,24,{b + 28}",
        f"dispute,{f},{b + 28}",              # foreign client
        f"chargeback,{f},{b + 28}",           # foreign client
        f"resolve,{f},{b + 28}",              # foreign client
        # --- id boundaries --------------------------------------------------
        f"deposit,65535,{b + 30},1",
    ]


def prologue() -> list[str]:
    return [
        "deposit,65535,0,1",
        "deposit,65535,4294967295,1",
        "deposit,0,4294967294,0.0001",
    ]


def expected_output(cycles: int) -> str:
    k = cycles // 20
    rows = ["client,available,held,total,locked"]
    for c in range(20):
        avail = Decimal(35) * k
        if c == 0:
            avail += Decimal("0.0001")
        rows.append(f"{c},{avail},0,{avail},false")
    rows.append("20,0,0,0,true")            # locked sacrificial client
    rows.append("21,0,0,0,false")           # zeroed account
    rows.append(f"22,{MAX},0,{MAX},false")  # balance pinned at Decimal::MAX
    rows.append("23,-10,5,-5,true")         # negative available, locked
    rows.append("24,0,0,0,false")
    rows.append("25,0,0,0,false")
    rows.append(f"65535,{2 + cycles},0,{2 + cycles},false")
    return "\n".join(rows) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cycles", type=int, default=20000,
                        help="number of 68-row cycles (default 20000)")
    parser.add_argument("--out-dir", type=Path, default=None,
                        help="output directory (default: <repo>/target/stress)")
    args = parser.parse_args()

    if args.cycles % 20 != 0:
        parser.error("--cycles must be a multiple of 20 so every normal "
                     "client receives the same number of cycles")

    out_dir = args.out_dir or Path(__file__).resolve().parent.parent / "target" / "stress"
    out_dir.mkdir(parents=True, exist_ok=True)

    input_path = out_dir / "input.csv"
    with input_path.open("w") as f:
        f.write("type,client,tx,amount\n")
        f.write("\n".join(prologue()))
        f.write("\n")
        chunk = []
        for cycle in range(args.cycles):
            chunk.extend(cycle_rows(cycle))
            if len(chunk) >= 100_000:
                f.write("\n".join(chunk))
                f.write("\n")
                chunk.clear()
        if chunk:
            f.write("\n".join(chunk))
            f.write("\n")

    (out_dir / "expected_output.csv").write_text(expected_output(args.cycles))

    total = 1 + len(prologue()) + args.cycles * len(cycle_rows(0))
    print(f"cycles:        {args.cycles}")
    print(f"rows:          {total}")
    print(f"input:         {input_path} ({input_path.stat().st_size / 1e6:.1f} MB)")
    print(f"expected:      {out_dir / 'expected_output.csv'}")


if __name__ == "__main__":
    main()
