# Interaction
![Crates.io](https://img.shields.io/crates/l/interaction)
[![Crates.io](https://img.shields.io/crates/v/interaction.svg)](https://crates.io/crates/interaction)

Interaction is a minimal and a simple readline library for Rust.

* [x] Single line editing mode
* [x] Multi line editing mode
* [x] Key bindings
* [x] History
* [x] Completion


# Usage
Add this in your `Cargo.toml`:

```toml
[dependencies]
interaction = "0.3.2"
```

Or, if you installed [cargo-edit](https://github.com/killercup/cargo-edit), you run this command:

```sh
$ cargo add interaction
```

# Example

```rust
use interaction::InteractionBuilder;
use std::io;

fn main() {
    let history_file = "./.example_history";
    let mut inter = InteractionBuilder::new()
        .prompt_str(";;>")
        .history_limit(5)
        .completion(|_input, completions| {
            completions.push(b"foo");
            completions.push(b"bar");
        })
        .load_history(history_file)
        .unwrap()
        .build();
    loop {
        match inter.line() {
            Ok(input) => {
                // write any code.
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                inter.save_history(history_file).unwrap();
                break;
            }
            Err(_) => {
                break;
            }
        }
    }
}
```

