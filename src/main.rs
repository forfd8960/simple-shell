use simple_shell::ioloop::run_shell;

fn main() -> anyhow::Result<()> {
    println!("shell started. Type 'exit' to quit.");
    run_shell()
}
