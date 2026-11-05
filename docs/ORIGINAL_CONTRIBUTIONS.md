# Source provenance and attribution

The repository LICENSE remains unchanged. This inventory is source attribution,
not a legal opinion or a grant of additional rights.

| Component | Source and revision | Status |
| --- | --- | --- |
| Router | [Creit-Tech Stellar-Router-Contract](https://github.com/Creit-Tech/Stellar-Router-Contract/tree/04975c434dec362584aa99458ae2c25d803ec570), `contracts/router-v0/src/lib.rs` | Adapted caller-authenticated invocation loop; local resource limits and TTL changes. No license file found in that revision or the inspected upstream checkout. Obtain upstream permission/license clarification before redistribution/release. The local MIT file cannot establish rights to upstream code. |
| Airdrop | [OpenZeppelin example](https://github.com/OpenZeppelin/stellar-contracts/blob/857bd364a88c4e50faae8b2c160d91b108f1c53d/examples/fungible-merkle-airdrop/src/contract.rs) | Adapted Merkle distributor; local immutable deadline and recovery policy. |
| NFT, token test fixtures, Merkle and ownership helpers | [OpenZeppelin Stellar Contracts v0.3.0](https://github.com/OpenZeppelin/stellar-contracts/tree/659fcabdc0e4712254dfebe4f45efd54f49826e9) | MIT, Copyright (c) 2024 OpenZeppelin. Notice preserved in [LICENSES/OpenZeppelin-MIT.txt](../LICENSES/OpenZeppelin-MIT.txt); include it in source and binary distributions. NFT adds local data/indexing/TTL helpers around upstream Base. |
| Merkle proof fixtures | [philipliu/soroban-merkle-airdrop](https://github.com/philipliu/soroban-merkle-airdrop), cited by the airdrop test header | Matching root/recipient/proof found in upstream commit `c01829c0b2d3e441bd4d9da431d06ea42c675d31`; complete four-recipient inputs in `b611ea8d017a7fe1a9c6e4386e8491c16c821fce`. New local regression reconstructs the root/proof. Actual original generation execution/revision and redistribution rights remain unverified; see the follow-up evidence. |
| Account, shared verifier, subsidy | Introduced in local commit `110f837`, then modified locally | Repository-authored implementation according to local history; no exhaustive originality claim. Uses Soroban host cryptography and serde JSON parsing. |

The OpenZeppelin git dependencies resolve to commit
`659fcabdc0e4712254dfebe4f45efd54f49826e9`; their SDK requirement is 22.x.
The workspace pins SDK 22.0.11 to keep Env, Address and token types identical.
A major SDK migration requires a compatible OpenZeppelin migration and a fresh
review; independently bumping the workspace SDK to 25.x breaks this integration.

## Follow-up evidence

See [dependency and provenance follow-up](DEPENDENCY_PROVENANCE_FOLLOWUP.md)
for exact upstream revisions/blob IDs, package constraints and unresolved rights.
The OpenZeppelin manifest now uses the full `rev`, not the mutable v0.3.0 tag;
all eight resolved upstream packages still use the same commit and versions.
Technical reproducibility establishes consistency of the fixture data, not
copyright permission. No existing attribution, copyright or license notice was
removed or relicensed.
