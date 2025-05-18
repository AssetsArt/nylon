fn main() {
    // Initialize the logger.
    tracing_subscriber::fmt::init();

    // command
    let args = nylon_command::parse();
    println!("{:?}", args);
}
