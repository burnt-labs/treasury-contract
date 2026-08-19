# XION Treasury Contract

The governance-deployed treasury contract for the XION network's fee grant
infrastructure. It issues bounded fee grants so applications can sponsor gas
for their users, and manages the grant configurations and admin controls that
scope what each grant may spend.

This repository was split out of
[`burnt-labs/contracts`](https://github.com/burnt-labs/contracts) with its
history preserved; the contract previously lived at `contracts/treasury`.

## Building

```sh
cargo wasm    # optimized wasm build (alias for build --release --lib --target wasm32-unknown-unknown)
cargo schema  # generate JSON schema
cargo test
```

Reproducible release artifacts are built with
[`cosmwasm/optimizer`](https://github.com/CosmWasm/optimizer):

```sh
docker run --rm -v "$(pwd)":/code cosmwasm/optimizer:0.17.0
```

## Security

This contract is an asset in the
[Core Protocol Contracts bug bounty program](https://github.com/burnt-labs/bug-bounty/blob/main/programs/contracts.md).
See [SECURITY.md](SECURITY.md) for how to report a vulnerability.

## License

[MIT](LICENSE)
