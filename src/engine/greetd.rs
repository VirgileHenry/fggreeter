mod socket_handler_thread;
mod utils;

use bevy::prelude::*;

#[derive(Resource)]
pub struct FinisherTimer(Timer);

/// Main state of the greeter.
#[derive(States)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GreeterState {
    /// Initialization state, start a greetd session when entering it.
    ///
    /// This state can be re-entered if something went wrong with greetd
    /// and the state machine decided to start over with a new session.
    #[default]
    Init,
    /// State for when the user is currently answering a greetd auth message.
    Input,
    /// State for when an answer have been submitted to greetd and we are
    /// waiting for validation.
    Submitting,
    /// State for when greetd accepted the response and we can start the session.
    /// This state is also used to deliver the enter animation of the greeter.
    Accepted,
    /// Dummy end state that signals we are all good and the session has been launched.
    /// If the greeter ever enters this state, it shall be killed pretty soon.
    SessionLaunched,
    /// State for when the answer provided to greetd is invalid.
    /// This will cause a session cancel and re-entered to be able to try auth again.
    Rejected,
    /// An info has been received from greetd.
    /// This will cause to automatically send a None response to keep going.
    Info,
    /// An error has been received from greetd.
    /// This will cause to automatically move to the Init state to open a new session.
    Error,
}

impl std::fmt::Display for GreeterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init => write!(f, "Init"),
            Self::Input => write!(f, "Input"),
            Self::Submitting => write!(f, "Submitting"),
            Self::Accepted => write!(f, "Accepted"),
            Self::SessionLaunched => write!(f, "SessionLaunched"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Info => write!(f, "Info"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// System param to get both the greeter state and next state.
#[derive(bevy::ecs::system::SystemParam)]
pub struct StateParam<'w> {
    pub state: Res<'w, State<GreeterState>>,
    pub next_state: ResMut<'w, NextState<GreeterState>>,
}

/// Resource that holds the channel to the thread handling the greeter socket and com.
struct GreetdChannel {
    pub sender: std::sync::mpsc::Sender<greetd_ipc::Request>,
    pub receiver: std::sync::mpsc::Receiver<greetd_ipc::Response>,
}

/// Register all the systems and resources for the greeter state manager.
pub fn add_plugins(app: &mut App, socket: std::os::unix::net::UnixStream) {
    /* Spin the socket in its own thread so it can block and take its lil time */
    let (sock_sender, engine_receiver) = std::sync::mpsc::channel();
    let (engine_sender, sock_receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || socket_handler_thread::greetd_socket_thread(sock_sender, sock_receiver, socket));

    app.insert_non_send(GreetdChannel {
        sender: engine_sender,
        receiver: engine_receiver,
    });
    app.init_state::<GreeterState>();

    app.add_systems(Update, handle_greetd_response);
    app.add_systems(Update, tick_finisher_timer);
    app.add_systems(OnEnter(GreeterState::Init), start_greetd_session);
    app.add_systems(OnEnter(GreeterState::Submitting), send_greetd_response);
    app.add_systems(OnEnter(GreeterState::Accepted), start_finisher_timer);
    app.add_systems(OnEnter(GreeterState::Rejected), restart_greetd_session);
    app.add_systems(OnEnter(GreeterState::Error), restart_greetd_session);
    app.add_observer(on_input_submitted);
}

/// System to start the greetd session.
///
/// This is called whenever we enter the Init state.
fn start_greetd_session(channel: NonSend<GreetdChannel>, input_state: Res<crate::engine::input::InputState>) {
    tracing::debug!("Entered Init state, creating a new greetd session");
    let username = input_state.user.clone();
    let request = greetd_ipc::Request::CreateSession { username };

    utils::log_err(channel.sender.send(request));
}

/// Listen for when we enter the windup state to send the answer to greetd
///
/// This avoids races between bevy events and greetd response
fn send_greetd_response(channel: NonSend<GreetdChannel>, input_state: Res<crate::engine::input::InputState>) {
    tracing::debug!("Entered Submitting state, sending the input buffer to greetd");
    let response = Some(input_state.response_buffer.clone());
    let request = greetd_ipc::Request::PostAuthMessageResponse { response };

    utils::log_err(channel.sender.send(request));
}

/// When we enter the accepted part, spin up a small timer to show
/// the animation before asking to enter the user session.
fn start_finisher_timer(mut commands: Commands) {
    tracing::debug!("Entered Accepted state, starting finisher timer");
    commands.insert_resource(FinisherTimer(Timer::from_seconds(0.2, TimerMode::Once)));
}

/// Tick the finisher animation timer, and ask to start the user session when the timer ran out.
fn tick_finisher_timer(
    time: Res<Time>,
    channel: NonSend<GreetdChannel>,
    input_state: Res<crate::engine::input::InputState>,
    mut timer: If<ResMut<FinisherTimer>>,
) {
    timer.0.0.tick(time.delta());
    if timer.0.0.just_finished() {
        tracing::debug!("Finisher timer timed out, sending a start user session request");
        let request = greetd_ipc::Request::StartSession {
            cmd: input_state.command.clone(),
            env: Vec::new(),
        };
        utils::log_err(channel.sender.send(request));
    }
}

/// Listen for when we enter the countered state (Countered) to restart the greetd session
fn restart_greetd_session(channel: NonSend<GreetdChannel>) {
    tracing::debug!("Entered Rejected/Error state, asking to cancel the session");
    let request = greetd_ipc::Request::CancelSession {};
    utils::log_err(channel.sender.send(request));
}

/// System to handle responses from the greetd socket
fn handle_greetd_response(
    channel: NonSend<GreetdChannel>,
    mut input_state: ResMut<crate::engine::input::InputState>,
    mut input_label: Single<&mut Text, With<crate::engine::ui::InputLabel>>,
    mut state: StateParam,
    mut commands: Commands,
) {
    match channel.receiver.try_recv() {
        Err(std::sync::mpsc::TryRecvError::Empty) => return,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            tracing::error!("SockThread -> Engine channel closed, can't receive responses!")
        }
        Ok(response) => advance(&mut state, response, &mut *input_state, &mut *input_label, &mut commands),
    }
}

/// Update the greeter internal state machine based on the current state and a greetd response.
///
/// This is where all the greeter logic go, and almost all of the state transitions are expressed here.
/// This function also does not send anything to greetd. All messages are sent on the state transition.
fn advance(
    state: &mut StateParam,
    response: greetd_ipc::Response,
    input_state: &mut crate::engine::input::InputState,
    input_label: &mut Text,
    commands: &mut Commands,
) {
    let current_state: GreeterState = **state.state;
    match (current_state, response) {
        /* ========= Normal flow for authentication ========= */
        /* When greetd asks questions, we go to input state */
        (
            GreeterState::Init | GreeterState::Submitting | GreeterState::Info,
            greetd_ipc::Response::AuthMessage {
                auth_message_type,
                auth_message,
            },
        ) => match auth_message_type {
            greetd_ipc::AuthMessageType::Visible => {
                tracing::debug!("Greetd asked for visible question, going to Input state");
                input_state.response_buffer.clear();
                input_state.show_response = true;
                input_label.0 = auth_message;
                state.next_state.set(GreeterState::Input);
            }
            greetd_ipc::AuthMessageType::Secret => {
                tracing::debug!("Greetd asked for secret question, going to Input state");
                input_state.response_buffer.clear();
                input_state.show_response = false;
                input_label.0 = auth_message;
                state.next_state.set(GreeterState::Input);
            }
            greetd_ipc::AuthMessageType::Info => {
                tracing::debug!("Greetd sent info: {auth_message}, going to Info state");
                state.next_state.set(GreeterState::Info)
            }
            greetd_ipc::AuthMessageType::Error => {
                tracing::debug!("Greetd sent error: {auth_message}, going to Error state");
                state.next_state.set(GreeterState::Error)
            }
        },
        /* When the authentication is a sucess, go to accepted state */
        (GreeterState::Submitting, greetd_ipc::Response::Success) => {
            tracing::debug!("Greetd accepted our response to the auth message, going to Accepted state");
            commands.trigger(crate::engine::animation::AnimationEvent::Finisher);
            state.next_state.set(GreeterState::Accepted);
        }
        /* In accepted state, when receiving a sucess, it means greetd launched the session */
        (GreeterState::Accepted, greetd_ipc::Response::Success) => {
            tracing::debug!("Greetd started the user session, going to SessionLaunched state");
            state.next_state.set(GreeterState::SessionLaunched);
        }

        /* ========= Authentication Error ========= */
        /* When we get an auth error while submitting, our answer was invalid */
        (
            GreeterState::Submitting,
            greetd_ipc::Response::Error {
                error_type: greetd_ipc::ErrorType::AuthError,
                description,
            },
        ) => {
            tracing::debug!("Greetd rejected our response to the auth message, going to Rejected state");
            input_label.0 = description;
            commands.trigger(crate::engine::animation::AnimationEvent::Counter);
            state.next_state.set(GreeterState::Rejected);
        }

        /* ========= Session restart cycle ========= */
        /* A success while in the Error state means we successefuly closed the session */
        (GreeterState::Error, greetd_ipc::Response::Success) => {
            tracing::debug!("Session successefuly canceled, going back to Init state");
            state.next_state.set(GreeterState::Init);
        }
        /* A success while in the Rejected state means we successefuly closed the session */
        (GreeterState::Rejected, greetd_ipc::Response::Success) => {
            tracing::debug!("Session successefuly canceled, going back to Init state");
            state.next_state.set(GreeterState::Init);
        }

        /* ========= Generic error handling ========= */
        /* When we get an error in the Error state, go straight to Init to try to restart the session */
        (GreeterState::Error, greetd_ipc::Response::Error { description, .. }) => {
            tracing::warn!("Received an greetd error while already in Error state: {description}, going to Init");
            state.next_state.set(GreeterState::Init);
        }
        /* When we receive an auth message while we are not expecting one, something went wrong */
        (current, message) => {
            tracing::warn!("Unhandled message {message:?} while in {current} state, going to the Error state");
            state.next_state.set(GreeterState::Error);
        }
    }
}

/// Observer that listen for the SubmitInputEvent to send the input to greetd.
fn on_input_submitted(_event: On<crate::engine::input::SubmitInputEvent>, mut state: StateParam) {
    let current_state: GreeterState = **state.state;
    match current_state {
        GreeterState::Input => {
            /* We can't submit the input right away, that would create a race between bevy state
             * system and greetd. If greetd respond before bevy changed the state to submitting,
             * we would get a greetd response in the invalid state.
             *
             * Instead, we switch to the submitting state, and a system will trigger when entering
             * this state to send the input. We are thus sure that we are in the correct state. */
            tracing::debug!("Submit input event received: going to Submitting state");
            state.next_state.set(GreeterState::Submitting);
        }
        other => {
            tracing::warn!("Unable to submit input from state {other}, switching to Error state");
            state.next_state.set(GreeterState::Error);
        }
    }
}
