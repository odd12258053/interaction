use interaction::Interaction;

fn main() {
    let mut inter = Interaction::from_str(";;>");
    loop {
        let input = inter.line().unwrap();
        println!("input: {:?} {}", input, input.len());
    }
}
