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
                println!("Input: {:?} {}", input, input.len());
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                inter.save_history(history_file).unwrap();
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}
