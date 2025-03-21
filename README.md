# Cargo-Wrap

![Crates.io Version](https://img.shields.io/crates/v/cargo_wrap?link=https%3A%2F%2Fcrates.io%2Fcrates%2Fcargo_wrap)

This crate is basically just Rust bindings for `cargo`. For now supports the following flags and features:

## Features

* Verbose logging (`--verbose`)
* Release or Debug build modes (`--release`)
* Custom job counts (`--jobs N`)
* Custom target output directories (`CARGO_TARGET_DIR`)
* Specify build targets (`--target X`)
* Feature listing and activation (`--features X`, `--no-default-features`)
* Binary/Library build selection (`--bin X`, `--lib X`)
* Extra `rustc` flags (`RUSTFLAGS`)

## Installation

```shell
cargo add cargo_wrap
```

```toml
cargo_wrap = "0.1.1"
```

## Examples

### Basic Build

```rust
use cargo_wrap::{Builder, ProjectSettings};
use std::io;

fn main() -> io::Result<()> {
    let mut settings = ProjectSettings::new("/path/to/project", None, None, false);
    settings.set_release();

    let builder = Builder::new(settings, 0, Some("output.log"))?;
    builder.build()?;

    Ok(())
}
```

### Enable Features

```rust
use cargo_wrap::{Builder, ProjectSettings};
use std::io;

fn main() -> io::Result<()> {
    let mut settings = ProjectSettings::new("/path/to/project", None, None, false);
    settings.add_feature("my_feature".to_string());

    let builder = Builder::new(settings, 0, None)?;
    builder.build()?;

    Ok(())
}
```

## Changelog
### 0.1.0
* Initial commit

### 0.1.1
* Added additional `rustc` flag support

## License

MIT
