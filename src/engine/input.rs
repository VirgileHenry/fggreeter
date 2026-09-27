use bevy::ecs::message::MessageReader;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

#[derive(Resource)]
pub struct InputState {
    pub user: String,
    pub command: Vec<String>,
    pub response_buffer: String,
    pub show_response: bool,
}

/// Register the plugins for the input handling
pub fn add_plugins(app: &mut App, args: crate::args::Args) {
    app.insert_resource(InputState {
        user: args.user,
        command: args.command,
        response_buffer: String::new(),
        show_response: true,
    });

    app.add_systems(Update, handle_keystrokes);
}

/// Read the user input and update the input field accordingly
fn handle_keystrokes(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut input_state: ResMut<InputState>,
    mut commands: Commands,
) {
    for keystroke in keyboard_input.read().filter(|ev| ev.state == ButtonState::Pressed) {
        match &keystroke.logical_key {
            Key::Backspace => {
                input_state.response_buffer.pop();
                commands.trigger(crate::engine::animation::AnimationEvent::Block);
            }
            Key::Escape => {
                input_state.response_buffer.clear();
                commands.trigger(crate::engine::animation::AnimationEvent::Reset);
            }
            Key::Enter => {
                commands.trigger(crate::engine::state::GreeterTransition::SubmitAnswer);
                commands.trigger(crate::engine::animation::AnimationEvent::Windup);
            }
            _ => {
                if let Some(text) = &keystroke.text {
                    let chars = text.chars().filter(|c| !c.is_control());
                    input_state.response_buffer.extend(chars);
                    commands.trigger(crate::engine::animation::AnimationEvent::Hit);
                }
            }
        }
    }
}
