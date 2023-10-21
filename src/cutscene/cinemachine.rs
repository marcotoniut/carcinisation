use bevy::prelude::*;

#[derive(Component)]
///cinemachine: self
pub struct CinemachineModule;

#[derive(Component)]
///cinemachine: BG
pub struct UIBackground;

#[derive(Component)]
pub struct ClipBundle;

#[derive(Resource, Default)]
pub struct CurrentClipInfo {
    ///index
    pub index: u8,
    ///is Renderered
    pub is_rendered: bool,
    pub has_finished: bool,
}

#[derive(Resource)]
pub struct CinemachineTimer {
    pub timer: Timer,
}

impl Default for CinemachineTimer {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0., TimerMode::Once);
        timer.pause();
        CinemachineTimer { timer }
    }
}

impl CurrentClipInfo {
    pub fn reset(&mut self) {
        self.index = 0;
        self.is_rendered = false;
        self.has_finished = false;
    }

    pub fn started_render(&mut self) {
        self.is_rendered = true;
    }

    pub fn eof(&mut self) {
        self.has_finished = true;
    }

    pub fn inc(&mut self) -> bool {
        if self.has_finished {
            self.index += 1;
            self.is_rendered = false;
            self.has_finished = false;
            return true;
        } else {
            return false;
        }
    }
}
