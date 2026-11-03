## What and why

## Checks

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --locked --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] Checked affected interface behavior in the Development profile, or noted why not

## Data safety

- [ ] No lost or double-counted time; a schema change backs up first and runs in a transaction
- [ ] No personal projects, paths, histories or exports are included
