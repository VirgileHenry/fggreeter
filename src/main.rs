mod args;
mod engine;

fn main() {
    let args: args::Args = argh::from_env();

    init_logging(args.log_file.as_ref());

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

fn init_logging(log_file: Option<&std::path::PathBuf>) {
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    #[cfg(debug_assertions)]
    let fggreeter_log_level = "debug";
    #[cfg(not(debug_assertions))]
    let fggreeter_log_level = "info";

    let bevy_log_filters = bevy::log::DEFAULT_FILTER;

    /* Initialize a global tracing subscriber based on the RUST_LOG env var */
    let filter = format!("info,fggreeter={fggreeter_log_level},{bevy_log_filters}");
    let builder = tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::new(filter));

    match log_file.and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok()) {
        Some(file) => builder.with_writer(std::io::stderr.and(std::sync::Mutex::new(file))).init(),
        None => builder.init(),
    }
}
