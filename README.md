cx-terminfo
===========
![WTFPL](http://img.shields.io/badge/license-WTFPL-blue.svg)

**cx-terminfo** is a (nearly) pure Rust library to parse terminfo files. No other Rust dependencies required.

Usage
-----
Add this to your 'Cargo.toml':

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

License
-------
[WTFPL](http://www.wtfpl.net/)