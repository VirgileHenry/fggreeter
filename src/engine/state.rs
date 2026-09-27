use bevy::prelude::*;

#[derive(States)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum GreeterState {
    /// Both fighters in stance, each keystroke = an attack.
    #[default]
    Combat,
    /// Answer sent, charging up until greetd replies.
    Windup,
    /// 0.2s final hit, B/W flash, fade to black, then exit into the session.
    Finisher,
    /// State for when the finisher animation is done and we need to start the session
    FinisherDone,
    /// Parried and kicked out, while the session is recreated in the background.
    Countered,
    /// Dropping back in (typing allowed).
    Respawn,
    /// greetd gone: show it, then exit with an error so greetd relaunches us.
    Disconnected,
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
    /// - Countered -> Combat,
    BackToCombat,
    /// Greetd let us in
    /// - Windup -> Finisher
    LoginSuccess,
    /// Greetd let us in
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
        (GreeterState::Combat, GreeterTransition::SubmitAnswer) => {
            /* Move from Combat to Windup, submitting an answer */
            commands.trigger(crate::engine::animation::AnimationEvent::Windup);
            state.next_state.set(GreeterState::Windup);
        }
        (GreeterState::Windup, GreeterTransition::BackToCombat) => {
            /* Good answer, but greetd is asking for more */
            commands.trigger(crate::engine::animation::AnimationEvent::Reset);
            state.next_state.set(GreeterState::Combat);
        }
        (GreeterState::Countered, GreeterTransition::BackToCombat) => {
            /* Wrong auth, back to combat my guy */
            commands.trigger(crate::engine::animation::AnimationEvent::Reset);
            state.next_state.set(GreeterState::Combat);
        }
        (GreeterState::Windup, GreeterTransition::LoginSuccess) => {
            /* We're all done, finish this */
            commands.trigger(crate::engine::animation::AnimationEvent::Finisher);
            state.next_state.set(GreeterState::Finisher);
        }
        (GreeterState::Finisher, GreeterTransition::FinisherDone) => {
            /* End state, start the session */
            state.next_state.set(GreeterState::FinisherDone);
        }
        (GreeterState::Windup, GreeterTransition::LoginFailure) => {
            /* Invalid auth, go to countered */
            commands.trigger(crate::engine::animation::AnimationEvent::Counter);
            state.next_state.set(GreeterState::Countered);
        }
        (GreeterState::Combat, GreeterTransition::BackToCombat) => { /* Happens at startup, is fine */ }
        (GreeterState::FinisherDone, GreeterTransition::LoginSuccess) => { /* Happens at end, is fine */ }
        (state, transition) => tracing::warn!("Unhandled transition: {state:?} -> {transition:?}"),
    }
}
