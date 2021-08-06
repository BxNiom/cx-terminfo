cx-terminfo
===========
[![crates.io](https://img.shields.io/crates/v/cxterminfo.svg)](https://crates.io/crates/cxterminfo) ![WTFPL](http://img.shields.io/badge/license-WTFPL-blue.svg) [![Crates.io](https://img.shields.io/crates/d/cxterminfo.svg)](https://crates.io/crates/cxterminfo)

**cx-terminfo** is a (nearly) pure Rust library to parse terminfo files. No other Rust dependencies required.

Usage
-----
Add this to your 'Cargo.toml':

```toml
[dependencies]
cxterminfo = "*"
```

or

```toml
[dependencies]
cxterminfo = { git = "https://github.com/bxinom/cx-terminfo" }
```

and this to your crate root:

```rust
extern crate cxterminfo;
```

Examples
--------

### Load default terminfo database

```rust
use cxterminfo::terminfo;

fn main() {
    if Ok(info) = terminfo::from_env() {
        // do whatever you want
    }
}
```

### Standard capabilities

cx-terminfo got three enums for capabilities (each value has documentation):

```
cxterminfo::capabilities::BoolCapability // known bool capabilities
cxterminfo::capabilities::NumberCapability // known number capabilities
cxterminfo::capabilities::StringCapability // known string capabilities
```

Howto get capability values:

```rust
use cxterminfo::terminfo;
use cxterminfo::capabilities::{BoolCapability, NumberCapability, StringCapability};

fn main() {
    if Ok(info) = terminfo::from_env() {
        println!("{:?}", info.get_bool(BoolCapability::AutoLeftMargin));
        println!("{:?}", info.get_number(NumberCapability::MaxColors));
        println!("{:?}", info.get_string(StringCapability::Bell));
    }
}
```

### Extended capabilities

```rust
use cxterminfo::terminfo;

fn main() {
    if Ok(info) = terminfo::from_env() {
        println!("{:?}", info.get_ext_bool("AT"));
        println!("{:?}", info.get_ext_number("IDENT"));
        println!("{:?}", info.get_ext_string("XM"));
    }
}
```

### Parameterized strings

```rust
use cxterminfo::param_string::{evaluate, Param};

fn main() {
    // Move cursor to location 10, 10
    let param_str = "\x1B[%d;%dH";
    if let Ok(move_cursor) = evaluate(param_str, &[Param::Number(10), Param::Number(10)]) {
        println!("{:?}", move_cursor);
    }
}
```

See also [terminfo(4) - Section 1-2](https://man.cx/terminfo(4)) for more information about parameterized strings.

### Terminal responses

To work with responses, use a [sscanf](https://docs.rs/releases/search?query=sscanf) implementation.

License
-------
[WTFPL](http://www.wtfpl.net/)