mod events;

pub use bevy::prelude::*;
pub use events::AnimationEvent;

#[derive(Resource)]
pub struct FinisherTimer(Timer);

/// Register the plugins for the animation part
pub fn add_plugins(app: &mut App) {
    app.add_systems(OnEnter(crate::engine::state::GreeterState::Finisher), start_finisher_timer);
    app.add_systems(Update, tick_finisher_timer);
}

/// When we enter the finisher part, lets the animation play
fn start_finisher_timer(mut commands: Commands) {
    commands.insert_resource(FinisherTimer(Timer::from_seconds(0.2, TimerMode::Once)));
}

/// Tick the finisher animation timer, and trigger the finisher done transition when it has reached the end.
fn tick_finisher_timer(time: Res<Time>, mut timer: If<ResMut<FinisherTimer>>, mut commands: Commands) {
    timer.0.0.tick(time.delta());
    if timer.0.0.just_finished() {
        commands.trigger(crate::engine::state::GreeterTransition::FinisherDone);
    }
}
