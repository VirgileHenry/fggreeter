use bevy::camera::ScalingMode;
use bevy::prelude::*;

const VIEW_H: f32 = 1080.0;

/// Register the plugins for the background
pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup_camera);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: VIEW_H },
            ..OrthographicProjection::default_2d()
        }),
    ));
}
