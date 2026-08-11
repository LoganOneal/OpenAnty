# Contributing to OpenAnty

Thanks for helping build an open, agent-first antidetect platform.

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo run -p openanty-cli -- init
```

## Commit style

Use clear, conventional commits:

- `feat(fp): ...`
- `fix(session): ...`
- `docs: ...`
- `chore: ...`
- `test: ...`
- `build(packaging): ...`

Prefer small, focused commits that each leave the tree buildable.

## Design

Architecture decisions live in [DESIGN.md](DESIGN.md). Please update design docs when changing public MCP/REST contracts.

## Responsible use

Do not contribute CAPTCHA solvers, fraud tooling, or docs that promote illegal multi-accounting. See [RESPONSIBLE_USE.md](RESPONSIBLE_USE.md).

## License

By contributing, you agree your contributions are licensed under Apache-2.0.
