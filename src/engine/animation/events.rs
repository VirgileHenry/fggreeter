/// The animation event is an event sent by the state
/// machine that drives the fighting game animation.
#[derive(bevy::prelude::Event)]
pub enum AnimationEvent {
    Hit,
    Block,
    Reset,
    Windup,
    Finisher,
    Counter,
}
