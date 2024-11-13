# wallet_balance

Rust based CLI tool to check balance in a bitcoin extended public key in base58 format. It derives addresses, checks balances, calculates the total and prints to the terminal. It does this till it hits the gap-limit (default 10).

Usage:
    Wallet_connect <xpub_key> [gap_limit]

