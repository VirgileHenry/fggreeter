use bevy::prelude::*;

#[derive(States)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum GreeterState {
    /// Very first state, mostly here to properly enter combat
    #[default]
    Init,
    /// Both fighters in stance, each keystroke = an attack.
    Combat,
    /// Answer sent, charging up until greetd replies.
    Windup,
    /// 0.2s final hit, B/W flash, fade to black, then exit into the session.
    Finisher,
    /// State for when the finisher animation is done and we need to start the session
    FinisherDone,
    /// Parried and kicked out, while the session is recreated in the background.
    Countered,
}

/// Event to advance the state machine.
///
/// All transition may not be valid in all states,
/// but having a single centralized enum allow to keep a simple observer + match
/// and allows to easily catch and log invalid transations rather than
/// keeping them lost by the bevy schedule system.
#[derive(Event)]
#[derive(Debug, Clone)]
pub enum GreeterTransition {
    /// Answer a greetd question.
    /// - Combat -> Windup.
    SubmitAnswer,
    /// Return to the combat state.
    /// - Windup -> Combat,
    /// - Init -> Combat,
    GreetdAskedQuestion,
    /// Greetd returned a success
    /// - Windup -> Finisher
    /// - Countered -> Init
    /// - FinisherDone -> FinisherDone (end trap)
    GreetdOk,
    /// Greetd kicked us out
    /// - Windup -> Countered
    LoginFailure,
    /// The finisher animation is done
    /// - Finisher -> FinisherDone
    FinisherDone,
}

pub fn add_plugins(app: &mut App) {
    app.init_state::<GreeterState>();
    app.add_observer(advance);
}

/// System param to get both the state and next state
#[derive(bevy::ecs::system::SystemParam)]
pub struct StateParam<'w> {
    state: Res<'w, State<GreeterState>>,
    next_state: ResMut<'w, NextState<GreeterState>>,
}

fn advance(transition: On<GreeterTransition>, mut state: StateParam, mut commands: Commands) {
    match (state.state.get(), &*transition) {
        (GreeterState::Init, GreeterTransition::GreetdAskedQuestion) => {
            tracing::debug!("Transition GreetdAskedQuestion: Init -> Combat");
            /* Move from Init to Combat */
            state.next_state.set(GreeterState::Combat);
        }
        (GreeterState::Combat, GreeterTransition::SubmitAnswer) => {
            tracing::debug!("Transition SubmitAnswer: Combat -> Windup");
            /* Move from Combat to Windup, submitting an answer */
            commands.trigger(crate::engine::animation::AnimationEvent::Windup);
            state.next_state.set(GreeterState::Windup);
        }
        (GreeterState::Windup, GreeterTransition::GreetdAskedQuestion) => {
            tracing::debug!("Transition GreetdAskedQuestion: Windup -> Combat");
            /* Good answer, but greetd is asking for more */
            commands.trigger(crate::engine::animation::AnimationEvent::Reset);
            state.next_state.set(GreeterState::Combat);
        }
        (GreeterState::Countered, GreeterTransition::GreetdOk) => {
            tracing::debug!("Transition GreetdOk: Countered -> Init");
            /* Wrong auth, back to combat my guy */
            commands.trigger(crate::engine::animation::AnimationEvent::Reset);
            state.next_state.set(GreeterState::Init);
        }
        (GreeterState::Windup, GreeterTransition::GreetdOk) => {
            tracing::debug!("Transition GreetdOk: Windup -> Finisher");
            /* We're all done, finish this */
            commands.trigger(crate::engine::animation::AnimationEvent::Finisher);
            state.next_state.set(GreeterState::Finisher);
        }
        (GreeterState::Finisher, GreeterTransition::FinisherDone) => {
            tracing::debug!("Transition FinisherDone: Finisher -> FinisherDone");
            /* End state, start the session */
            state.next_state.set(GreeterState::FinisherDone);
        }
        (GreeterState::Windup, GreeterTransition::LoginFailure) => {
            tracing::debug!("Transition LoginFailure: Windup -> Countered");
            /* Invalid auth, go to countered */
            commands.trigger(crate::engine::animation::AnimationEvent::Counter);
            state.next_state.set(GreeterState::Countered);
        }
        (GreeterState::FinisherDone, GreeterTransition::GreetdOk) => {
            tracing::debug!("Transition GreetdOk: FinisherDone -> FinisherDone");
            /* Greetd answering to the start session correctly, we're all good */
            tracing::info!("Session started, all good and good to go!");
        }
        (state, transition) => tracing::warn!("Unhandled transition: {state:?} -> {transition:?}"),
    }
}
