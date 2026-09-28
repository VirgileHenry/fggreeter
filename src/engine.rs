mod animation;
mod background;
mod camera;
mod greetd;
mod input;
mod state;
mod ui;

use bevy::prelude::*;

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
            .build()
            .disable::<bevy::log::LogPlugin>(),
    );

    animation::add_plugins(&mut app);
    background::add_plugins(&mut app);
    camera::add_plugins(&mut app);
    greetd::add_plugins(&mut app, socket);
    input::add_plugins(&mut app, args);
    state::add_plugins(&mut app);
    ui::add_plugins(&mut app);

    app
}
