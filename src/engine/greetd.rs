use bevy::prelude::*;

struct GreetdChannel {
    pub sender: std::sync::mpsc::Sender<greetd_ipc::Request>,
    pub receiver: std::sync::mpsc::Receiver<greetd_ipc::Response>,
}

fn greetd_socket_thread(
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

pub fn add_plugins(app: &mut App, socket: std::os::unix::net::UnixStream) {
    /* Spin the socket in its own thread so it can block and take its lil time */
    let (sock_sender, engine_receiver) = std::sync::mpsc::channel();
    let (engine_sender, sock_receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || greetd_socket_thread(sock_sender, sock_receiver, socket));

    app.insert_non_send(GreetdChannel {
        sender: engine_sender,
        receiver: engine_receiver,
    });

    app.add_systems(Startup, start_pam_session);
    app.add_systems(Update, handle_greetd_response);
    app.add_systems(OnEnter(crate::engine::state::GreeterState::Windup), send_greetd_response);
    app.add_systems(OnEnter(crate::engine::state::GreeterState::FinisherDone), start_session);
}

/// System to start the pam session in the engine setup
fn start_pam_session(channel: NonSend<GreetdChannel>, input_state: Res<crate::engine::input::InputState>) {
    let request = greetd_ipc::Request::CreateSession {
        username: input_state.user.clone(),
    };
    log_err(channel.sender.send(request));
}

/// System to handle responses from the greetd socket
fn handle_greetd_response(
    channel: NonSend<GreetdChannel>,
    mut input_state: ResMut<crate::engine::input::InputState>,
    mut input_label: Single<&mut Text, With<crate::engine::ui::InputLabel>>,
    mut commands: Commands,
) {
    match channel.receiver.try_recv() {
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            tracing::error!("SockThread -> Engine channel closed, can't receive responses !")
        }
        Ok(greetd_ipc::Response::AuthMessage {
            auth_message_type,
            auth_message,
        }) => {
            input_state.show_response = match auth_message_type {
                greetd_ipc::AuthMessageType::Secret => false,
                greetd_ipc::AuthMessageType::Visible => true,
                /* Fixme: switch to a display err / info state, in that state respond with none and wait, then back to combat */
                greetd_ipc::AuthMessageType::Info => {
                    tracing::info!("Info from greetd: {auth_message}");
                    /* Send a non response to keep the convo with greetd going */
                    let request = greetd_ipc::Request::PostAuthMessageResponse { response: None };
                    log_err(channel.sender.send(request));
                    return; /* Don't do the input cleanup + transition */
                }
                greetd_ipc::AuthMessageType::Error => {
                    tracing::warn!("Error from greetd: {auth_message}");
                    /* Send a non response to keep the convo with greetd going */
                    let request = greetd_ipc::Request::PostAuthMessageResponse { response: None };
                    log_err(channel.sender.send(request));
                    return; /* Don't do the input cleanup + transition */
                }
            };
            input_state.response_buffer.clear();
            input_label.0 = auth_message;
            /* Tell the state manager to go back to combat */
            commands.trigger(crate::engine::state::GreeterTransition::BackToCombat);
        }
        Ok(greetd_ipc::Response::Error { error_type, description }) => match error_type {
            greetd_ipc::ErrorType::AuthError => {
                input_state.response_buffer.clear();
                input_label.0 = format!("Invalid credentials !");
                /* Restart the session */
                let request = greetd_ipc::Request::CreateSession {
                    username: input_state.user.clone(),
                };
                log_err(channel.sender.send(request));
            }
            greetd_ipc::ErrorType::Error => {
                tracing::warn!("Error from greetd: {description}");
                /* Send a non response to keep the convo with greetd going */
                let request = greetd_ipc::Request::PostAuthMessageResponse { response: None };
                log_err(channel.sender.send(request));
                return; /* Don't do the input cleanup + transition */
            }
        },
        Ok(greetd_ipc::Response::Success) => commands.trigger(crate::engine::state::GreeterTransition::LoginSuccess),
    }
}

/// Listen for when we enter the windup state to send the answer to greetd
///
/// This avoids races between bevy events and greetd response
fn send_greetd_response(channel: NonSend<GreetdChannel>, input_state: Res<crate::engine::input::InputState>) {
    let response = Some(input_state.response_buffer.clone());
    let request = greetd_ipc::Request::PostAuthMessageResponse { response };

    match channel.sender.send(request) {
        Ok(_) => { /* All good */ }
        Err(_) => tracing::error!("Engine -> SockThread channel closed, can't send requests !"),
    }
}

/// Listen for when we enter the end state (FinisherDone) to launch the session
fn start_session(channel: NonSend<GreetdChannel>, input_state: Res<crate::engine::input::InputState>) {
    let request = greetd_ipc::Request::StartSession {
        cmd: input_state.command.clone(),
        env: Vec::new(),
    };

    match channel.sender.send(request) {
        Ok(_) => { /* All good */ }
        Err(_) => tracing::error!("Engine -> SockThread channel closed, can't send requests !"),
    }
}

/// Small utility to log errors from sending
/// to channels that are no longer available
fn log_err(err: Result<(), std::sync::mpsc::SendError<greetd_ipc::Request>>) {
    if let Err(req) = err {
        tracing::error!("Engine -> SockThread channel closed, can't send request: {req:?}");
    }
}
