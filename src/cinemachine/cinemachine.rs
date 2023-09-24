use bevy::prelude::*;

use super::data::CinemachineData;

#[derive(Component)]
///cinemachine: self
pub struct CinemachineModule;

#[derive(Component)]
///cinemachine: BG
pub struct UIBackground;

#[derive(Component)]
pub struct ClipBundle;

#[derive(Resource, Default)]
pub struct CinemachineScene(pub Option<CinemachineData>);

#[derive(Resource, Default)]
pub struct CurrentClipInfo{
    ///index
    pub index: usize,
    ///is Renderered
    pub isRendered: bool,
    pub hasFinished: bool
}

impl CurrentClipInfo {

    pub fn reset(&mut self) {
        self.index = 0;
        self.isRendered = false;
        self.hasFinished = false;
    }

    pub fn startedRender(&mut self) {
        self.isRendered = true;
    }

    pub fn eof (&mut self){
        self.hasFinished = true;
    }

    pub fn inc(&mut self) -> bool {
        if self.hasFinished {
            self.index += 1;
            self.isRendered = false;
            self.hasFinished = false;
            return true;
        } else {
            return false;
        }
    }
}