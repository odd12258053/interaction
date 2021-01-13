# Interaction
![Crates.io](https://img.shields.io/crates/l/interaction)
[![Crates.io](https://img.shields.io/crates/v/interaction.svg)](https://crates.io/crates/interaction)

Interaction is a minimal and a simple readline library for Rust.

* [x] Single line editing mode
* [x] Multi line editing mode
* [x] Key bindings
* [ ] History
* [x] Completion


# Usage 
Add this in your `Cargo.toml`:

```toml
[dependencies]
interaction = "0.2.0"
```

Or, if you installed [cargo-edit](https://github.com/killercup/cargo-edit), you run this command:

```sh
$ cargo add interaction
```

import `interaction::Interaction`.

```rust
use interaction::Interaction;

fn main() {
    let mut inter = Interaction::from_str(";;>");
    inter.set_completion(|_input, completions| {
        completions.push(b"foo");
        completions.push(b"bar");
    });
    loop {
        let input = inter.line().unwrap();
        // write any code.
    }
}
```

