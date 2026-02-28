# Privacy Providers

A collection of privacy providers for EVM chains, built in rust, wasm-compatible, with JS-compatible wasm bindings. Build with the goal of being native, JS, and wasm32-unknown-unknown compatible.

## Secret Management

Development secret management is handled via [SOPS](https://github.com/getsops/sops).

To edit secrets: 
- `sops secrets/secrets.yaml`

To load secrets:
- `export $(sops -d secrets/secrets.yaml | xargs)`
- Or load them automatically with direnv

To add a new contributor: 
- Have them run `age-keygen -o ~/.config/sops/age/keys.txt` and share the public key.
- Add the public key to `.sops.yaml`.
- Run `sops updatekeys secrets/secrets.yaml` to update the encrypted secrets file for the new key.

## Docs
- ./docs/compatibility.md - Compatibility considerations with official railgun js-SDK
