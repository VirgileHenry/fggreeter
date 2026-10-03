use bevy::camera::ScalingMode;
use bevy::prelude::*;

/// Height of viewed part of the scene
pub const SCENE_HEIGHT: f32 = 5.0;
/// Point in the scene we want the camera to look at
pub const SCENE_CENTER: Vec3 = vec3(0.0, 1.5, 0.0);
/// Offset of the camera relative to the scene center
pub const CAMERA_OFFSET: Vec3 = vec3(0.0, 0.0, 2.0);

/// Register the plugins for the background
pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup_camera);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: SCENE_HEIGHT,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(SCENE_CENTER + CAMERA_OFFSET).looking_at(SCENE_CENTER, Vec3::Y),
    ));
}
