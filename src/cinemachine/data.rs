use bevy::{
    prelude::{Vec2, warn},
    reflect::{TypePath, TypeUuid, List},
};

#[derive(Clone, Debug)]
pub struct TargetPath {
    pub move_to_target: Vec2,
    pub move_speed: f32,
}

#[derive(Clone, Debug)]
pub enum CutsceneGoal{
    MOVEMENT{
        pathing: TargetPath
    },
    TIMED{
        waitInSeconds: f32
    }
}

impl CutsceneGoal {
    pub fn subtract_time(&mut self, time: f32){
        if let CutsceneGoal::TIMED(ref mut content) = *self{
            content.push(time);
        } else {
            unreachable!();
        }
        /*match &self {
            CutsceneGoal::MOVEMENT { pathing } => {},
            CutsceneGoal::TIMED { mut waitInSeconds } => { 
                waitInSeconds -= time;
                warn!("{}", waitInSeconds.to_string());
            },
        }*/
    }
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub image_path: Option<String>,
    pub foreground_elements: Option<Vec<Clip>>,
    pub start_coordinates: Vec2,
    pub layer_index: f32,
    pub snd: Option<String>,
    pub goal: CutsceneGoal
}

#[derive(TypeUuid, TypePath, Clone, Debug)]
#[uuid = "8962be51-bbd5-42b4-95a9-269294ddf17a"]
pub struct CinemachineData { 
    pub name: String,
    pub start_coordinates: Vec2,
    pub clips: Vec<Clip>,
}