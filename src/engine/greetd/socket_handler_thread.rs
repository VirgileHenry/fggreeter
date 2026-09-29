/// Thread that handle the greetd socket.
///
/// Since the socket reading is blocking, it is done in a separate thread to not freeze the engine.
/// This thread will await for a greetd request from the engine, send it on the socket,
/// listen for the socket response, send it back to the engine, and start all over again.
pub fn greetd_socket_thread(
    sender: std::sync::mpsc::Sender<greetd_ipc::Response>,
    receiver: std::sync::mpsc::Receiver<greetd_ipc::Request>,
    mut socket: std::os::unix::net::UnixStream,
) {
    loop {
        /* Start by waiting a message on the receiver */
        let Ok(message) = receiver.recv() else {
            tracing::info!("Engine -> SockThread channel closed, stopping the socket thread");
            break;
        };
        /* Send it out to the socket */
        use greetd_ipc::codec::SyncCodec;
        if let Err(e) = message.write_to(&mut socket) {
            tracing::error!("Failed to write to the greetd socket: {e}");
            continue;
        }
        /* Expect a response from the socket */
        let response = match greetd_ipc::Response::read_from(&mut socket) {
            Ok(response) => response,
            Err(e) => {
                tracing::error!("Failed to read response from the greetd socket, closing: {e}");
                break;
            }
        };
        /* Send the response to the engine */
        if let Err(_) = sender.send(response) {
            tracing::info!("SockThread -> Engine channel closed, stopping the socket thread");
            break;
        }
    }
}
