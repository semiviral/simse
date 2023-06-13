use clap::Parser;

#[derive(Parser)]
enum Arguments {
    Run,
}

fn main() {
    match Arguments::parse() {
        Arguments::Run => {
            
        },
    }
}
