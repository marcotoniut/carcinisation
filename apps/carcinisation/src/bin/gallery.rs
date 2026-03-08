#[cfg(feature = "gallery")]
mod inner {
    use bevy::prelude::*;
    use carcinisation::app::{AppLaunchOptions, StartFlow, build_app};
    use carcinisation::gallery::messages::GalleryStartupEvent;

    struct GalleryBootstrapPlugin;

    impl Plugin for GalleryBootstrapPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, trigger_gallery_startup);
        }
    }

    fn trigger_gallery_startup(mut commands: Commands) {
        info!("Launching character gallery");
        commands.trigger(GalleryStartupEvent);
    }

    pub fn run() {
        let mut app = build_app(AppLaunchOptions {
            start_flow: StartFlow::Gallery,
            ..Default::default()
        });
        app.add_plugins(GalleryBootstrapPlugin);
        app.run();
    }
}

fn main() {
    #[cfg(feature = "gallery")]
    inner::run();
}
