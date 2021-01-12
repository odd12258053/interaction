use interaction::Interaction;

fn main() {
    let mut inter = Interaction::from_str(";;>");
    inter.set_completion(|_input, completions| {
        completions.push(b"foo");
        completions.push(b"bar");
    });
    loop {
        let input = inter.line().unwrap();
        println!("input: {:?} {}", input, input.len());
    }
}
