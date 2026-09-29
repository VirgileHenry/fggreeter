/// Small utility to log errors from sending
/// to channels that are no longer available
pub fn log_err(err: Result<(), std::sync::mpsc::SendError<greetd_ipc::Request>>) {
    if let Err(req) = err {
        tracing::error!("Engine -> SockThread channel closed, can't send request: {req:?}");
    }
}
