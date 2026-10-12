mod animation;
mod background;
mod camera;
mod greetd;
mod input;
mod ui;

use bevy::prelude::*;

/// Where the assets are installed by distro packages
const SYSTEM_ASSET_DIR: &str = "/usr/share/fggreeter/assets";

/// Folder to load the assets from.
///
/// Bevy looks for an `assets` folder in BEVY_ASSET_ROOT, then CARGO_MANIFEST_DIR, then next to the executable.
/// When none of the env vars are set and the assets are installed system wide, use those instead.
fn asset_dir() -> String {
    let default_dir = AssetPlugin::default().file_path;
    let root_from_env = ["BEVY_ASSET_ROOT", "CARGO_MANIFEST_DIR"]
        .iter()
        .any(|var| std::env::var_os(var).is_some());

    if !root_from_env && std::path::Path::new(SYSTEM_ASSET_DIR).is_dir() {
        tracing::info!("Loading assets from {SYSTEM_ASSET_DIR}");
        SYSTEM_ASSET_DIR.to_string()
    } else {
        default_dir
    }
}

pub fn create(args: crate::args::Args, socket: std::os::unix::net::UnixStream) -> App {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    decorations: false,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_dir(),
                ..default()
            })
            .build()
            .disable::<bevy::log::LogPlugin>(),
    );

    animation::add_plugins(&mut app);
    background::add_plugins(&mut app);
    camera::add_plugins(&mut app);
    greetd::add_plugins(&mut app, socket);
    input::add_plugins(&mut app, args);
    ui::add_plugins(&mut app);

    app
}
