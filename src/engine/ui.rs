use bevy::prelude::*;

#[derive(Component)]
pub struct InputField;

#[derive(Component)]
pub struct InputLabel;

const ACCENT: Color = Color::srgb(0.95, 0.22, 0.28);
const PANEL: Color = Color::srgba(0.08, 0.08, 0.11, 0.92);
const FIELD: Color = Color::srgb(0.03, 0.03, 0.05);
const DIM: Color = Color::srgb(0.6, 0.6, 0.66);

/// Register the plugins for the UI
pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup_ui);
    app.add_systems(
        Update,
        refresh_ui.run_if(resource_changed::<crate::engine::input::InputState>),
    );
}

/// Instantiate the UI components in the scene
fn setup_ui(mut commands: Commands, login_state: Res<crate::engine::input::InputState>) {
    let font = |size: f32| TextFont {
        font_size: FontSize::Px(size),
        ..default()
    };

    commands.spawn((
        /* Full screen node to place the UI card */
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::top(percent(8)),
            ..default()
        },
        children![(
            /* UI card */
            Node {
                width: px(440),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                padding: UiRect::all(px(24)),
                border: UiRect {
                    left: px(6),
                    ..UiRect::all(px(1))
                },
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(ACCENT),
            children![
                /* Title for the fighting game theme thingy */
                (Text::new("PLAYER 1"), font(14.0), TextColor(ACCENT)),
                /* Current user login name */
                (
                    Text::new(login_state.user.to_uppercase()),
                    font(36.0),
                    TextColor(Color::WHITE)
                ),
                /* Current required field */
                (InputLabel, Text::new("Starting Round..."), font(16.0), TextColor(DIM)),
                /* Password input field */
                (
                    Node {
                        padding: UiRect::axes(px(14), px(10)),
                        border: UiRect::bottom(px(2)),
                        ..default()
                    },
                    BackgroundColor(FIELD),
                    BorderColor::all(ACCENT),
                    children![(InputField, Text::new("_"), font(26.0), TextColor(Color::WHITE))],
                ),
            ],
        )],
    ));
}

/// Update the Display UI whenever the user input got updated
fn refresh_ui(input_state: Res<crate::engine::input::InputState>, mut password_display: Single<&mut Text, With<InputField>>) {
    if input_state.show_response {
        password_display.0 = format!("{}_", input_state.response_buffer.clone())
    } else {
        let password_length = input_state.response_buffer.chars().count();
        password_display.0 = format!("{}_", "*".repeat(password_length))
    };
}
