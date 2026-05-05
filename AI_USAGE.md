# AI Usage Disclosure

## Tools Used

- GitHub Copilot: scaffolded Rust module boilerplate and endpoint skeletons and get help to fix md files.
- ChatGPT: reviewed idempotency and state-machine edge cases and review.
- Claude: I collaborated with Claude to design the invoice payment API flow — refining my initial approach, resolving gaps in the logic, and arriving at a clean, production-ready architecture.


## Three Decisions Made Independently

1. Added `request_hash` to `payment_attempts` for idempotency-key payload conflict detection.
2. Chose `202 Accepted` on timeout ambiguity instead of collapsing into hard failure.
3. Used partial index for webhook retry queue rows only.
4. Implement neccessary api like customers api for invoice requirements.

## Thing AI Suggested Incorrectly

An early draft used decimal/floating money columns. This was corrected to integer minor units (`BIGINT cents`) throughout the schema and API payload expectations.

It is do not follow propper code convention so I have to fix this to ensure code consistancy and follow dry principals.


cron job 