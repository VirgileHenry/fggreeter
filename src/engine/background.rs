use bevy::pbr::Material;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

#[derive(ShaderType, Clone, Debug)]
pub struct LandscapeParams {
    pub sky_top: LinearRgba,
    pub sky_horizon: LinearRgba,
    pub sun: LinearRgba,
    pub cloud: LinearRgba,
    pub mountain: LinearRgba,
    pub snow: LinearRgba,
    pub hill_far: LinearRgba,
    pub hill_near: LinearRgba,
    pub ground: LinearRgba,
    pub ground_dark: LinearRgba,
    pub accent: LinearRgba,
    pub petal: LinearRgba,
    pub aspect: f32,
    pub seed: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct LandscapeMaterial {
    #[uniform(0)]
    pub params: LandscapeParams,
}

impl Material for LandscapeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/landscape.wgsl".into()
    }
}

pub fn add_plugins(app: &mut App) {
    app.add_plugins(MaterialPlugin::<LandscapeMaterial>::default())
        .add_systems(Startup, spawn_landscape);
}

fn spawn_landscape(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<LandscapeMaterial>>) {
    let h = crate::engine::camera::SCENE_HEIGHT;
    let w = h * 2.4; // covers screens up to 2.4:1
    let quad_pos = crate::engine::camera::SCENE_CENTER + vec3(0.0, 0.0, -4.0);
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(w, h))),
        MeshMaterial3d(materials.add(LandscapeMaterial {
            params: LandscapeParams {
                sky_top: Color::srgb(0.30, 0.22, 0.45).to_linear(),     // deep violet
                sky_horizon: Color::srgb(0.98, 0.62, 0.42).to_linear(), // warm orange
                sun: Color::srgb(1.00, 0.80, 0.58).to_linear(),
                cloud: Color::srgb(0.98, 0.80, 0.76).to_linear(),
                mountain: Color::srgb(0.36, 0.30, 0.48).to_linear(),
                snow: Color::srgb(0.97, 0.94, 0.98).to_linear(),
                hill_far: Color::srgb(0.45, 0.36, 0.55).to_linear(),
                hill_near: Color::srgb(0.28, 0.22, 0.38).to_linear(),
                ground: Color::srgb(0.66, 0.56, 0.50).to_linear(), // warm sand
                ground_dark: Color::srgb(0.44, 0.36, 0.38).to_linear(),
                accent: Color::srgb(0.80, 0.18, 0.16).to_linear(), // torii red
                petal: Color::srgb(0.98, 0.74, 0.82).to_linear(),
                aspect: w / h,
                seed: 4.2,
            },
        })),
        Transform::from_translation(quad_pos),
    ));
}
