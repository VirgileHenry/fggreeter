mod args;
mod engine;

fn main() {
    /* Initialize a global tracing subscriber based on the RUST_LOG env var */
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    let args: args::Args = argh::from_env();
    tracing::info!("Running greeter for user {}", args.user);

    let Ok(socket_path) = std::env::var("GREETD_SOCK") else {
        tracing::error!("Failed to get env var GREETD_SOCK");
        std::process::exit(-1);
    };
    let Ok(socket) = std::os::unix::net::UnixStream::connect(&socket_path) else {
        tracing::error!("Failed to connect to greetd socket at {}", socket_path);
        std::process::exit(-1);
    };
    tracing::info!("Connected to greetd via {}", socket_path);

    let mut app = engine::create(args, socket);
    app.run();
}
