# wallet_balance

Rust based CLI tool to check balance in a bitcoin extended public key in base58 format. It derives addresses, checks balances, calculates the total and prints to the terminal for both mainet and testnet. It does this till it hits the gap-limit (default 10).

Usage:
    
    Wallet_balance <xpub_key> [gap_limit]

The default gap limit in this tool is smaller than the usual 20 to avoid hitting rate limits with free APIs quickly. But this can be changed by passing in whatever value as the second command line argument.

The tool uses blockcypher apis internally. Ideally you would want to configure this from outside via env vars but given the diversity of apis and api patterns it seemed impractical to generalize this part in this context.

