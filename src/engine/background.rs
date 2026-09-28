use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;
use bevy::sprite_render::Material2d;
use bevy::sprite_render::Material2dPlugin;

#[derive(ShaderType, Clone, Debug)]
pub struct LandscapeParams {
    pub mountain_lit: LinearRgba,
    pub mountain_shadow: LinearRgba,
    pub ridge_top: LinearRgba,
    pub ridge_bottom: LinearRgba,
    pub ground_lit: LinearRgba,
    pub ground_shadow: LinearRgba,
    pub sky_top: LinearRgba,
    pub sky_horizon: LinearRgba,
    pub aspect: f32,
    pub seed: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct LandscapeMaterial {
    #[uniform(0)]
    pub params: LandscapeParams,
}

impl Material2d for LandscapeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/landscape.wgsl".into()
    }
}

pub fn add_plugins(app: &mut App) {
    app.add_plugins(Material2dPlugin::<LandscapeMaterial>::default())
        .add_systems(Startup, spawn_landscape);
}

fn spawn_landscape(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<LandscapeMaterial>>) {
    let (w, h) = (2400.0, 1080.0); // covers up to ~2.2:1 screens with the FixedVertical camera
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(w, h))),
        MeshMaterial2d(materials.add(LandscapeMaterial {
            params: LandscapeParams {
                mountain_lit: Color::srgb(0.99, 0.99, 1.00).to_linear(),
                mountain_shadow: Color::srgb(0.78, 0.86, 0.93).to_linear(),
                ridge_top: Color::srgb(0.478, 0.506, 0.6).to_linear(),
                ridge_bottom: Color::srgb(0.427, 0.471, 0.62).to_linear(),
                ground_lit: Color::srgb(1.0, 0.631, 0.078).to_linear(),
                ground_shadow: Color::srgb(1.0, 0.631, 0.078).to_linear(),
                sky_top: Color::srgb(0.96, 0.72, 0.78).to_linear(),
                sky_horizon: Color::srgb(1.00, 0.84, 0.66).to_linear(),
                aspect: w / h,
                seed: 4.2,
            },
        })),
        Transform::from_xyz(0.0, 0.0, -10.0),
    ));
}
