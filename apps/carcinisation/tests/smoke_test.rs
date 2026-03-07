use carcinisation::app::{AppLaunchOptions, StartFlow, build_app};

#[test]
fn app_boots_headless_stage_only() {
    let mut app = build_app(AppLaunchOptions {
        start_flow: StartFlow::StageOnly,
        headless: true,
    });
    for _ in 0..5 {
        app.update();
    }
}
