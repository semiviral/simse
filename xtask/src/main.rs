use clap::Parser;
use xshell::cmd;

#[derive(Parser)]
enum Arguments {
    Run,
}

fn main() {
    let sh = xshell::Shell::new().unwrap();

    match Arguments::parse() {
        Arguments::Run => {
            let _dir = sh.push_dir("simse-backend/");
            cmd!(sh, "cargo build").run().unwrap();
            drop(_dir);

            let _dir = sh.push_dir("simse-frontend/");
            cmd!(sh, "cargo build").run().unwrap();
            drop(_dir);

            let mut backend = std::process::Command::new("cargo")
                .arg("run")
                .current_dir("simse-backend/")
                .spawn()
                .expect("could not start backend server");

            let mut frontend = std::process::Command::new("trunk")
                .arg("serve")
                .current_dir("simse-frontend/")
                .spawn()
                .expect("could not start frontend server");

            frontend.wait().unwrap();
            backend.kill().unwrap();
        }
    }
}
