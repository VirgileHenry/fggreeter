#[derive(argh::FromArgs)]
/// Reach new heights.
pub struct Args {
    /// user for which to open a session.
    #[argh(option)]
    pub user: String,
    /// command to execute after the greeter started.
    #[argh(option)]
    pub command: Vec<String>,
}
