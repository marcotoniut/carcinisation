use crate::stage::enemy::data::{AnimationData, PATH_SPRITES_ENEMIES};

pub struct MosquitoAnimations {
    pub death: Vec<AnimationData>,
    pub fly: Vec<AnimationData>,
    pub idle: Vec<AnimationData>,
    pub melee_attack: Vec<AnimationData>,
}

// Animation fragments
const FRAGMENT_IDLE: &str = "idle";
const FRAGMENT_FLY: &str = "fly";
const FRAGMENT_DEATH: &str = "death";
const FRAGMENT_MELEE_ATTACK: &str = "melee_attack";

// Enemy
const FRAGMENT_MOSQUITO: &str = "mosquito";

fn concat_strings_and_number(s1: &str, s2: &str, s3: &str, index: u32) -> String {
    format!("{}{}_{}_{}.png", s1, s2, s3, index)
}

lazy_static! {
    static ref MOSQUITO_ANIMATIONS: MosquitoAnimations = {
        let idle_frames = 3;
        let idle_speed = 500;

        // TODO
        let fly_frames = 3;
        let fly_speed = 90;

        let death_frames = 19;
        let death_speed = 780;

        let attack_frames = 8;
        let melee_attack_speed = 130;

        MosquitoAnimations {
            death: vec![
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_DEATH, FRAGMENT_MOSQUITO, 0),
                    frames: death_frames,
                    speed: death_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_DEATH, FRAGMENT_MOSQUITO, 1),
                    frames: death_frames,
                    speed: death_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_DEATH, FRAGMENT_MOSQUITO, 2),
                    frames: death_frames,
                    speed: death_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_DEATH, FRAGMENT_MOSQUITO, 3),
                    frames: death_frames,
                    speed: death_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
            ],
            fly: vec![
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_FLY, FRAGMENT_MOSQUITO, 0),
                    frames: fly_frames,
                    speed: fly_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_FLY, FRAGMENT_MOSQUITO, 1),
                    frames: fly_frames,
                    speed: fly_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_FLY, FRAGMENT_MOSQUITO, 2),
                    frames: fly_frames,
                    speed: fly_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_FLY, FRAGMENT_MOSQUITO, 3),
                    frames: fly_frames,
                    speed: fly_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
            ],
            idle: vec![
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_IDLE, FRAGMENT_MOSQUITO, 0),
                    frames: idle_frames,
                    speed: idle_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_IDLE, FRAGMENT_MOSQUITO, 1),
                    frames: idle_frames,
                    speed: idle_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_IDLE, FRAGMENT_MOSQUITO, 2),
                    frames: idle_frames,
                    speed: idle_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_IDLE, FRAGMENT_MOSQUITO, 3),
                    frames: idle_frames,
                    speed: idle_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
            ],
            melee_attack: vec![
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MELEE_ATTACK, FRAGMENT_MOSQUITO, 0),
                    frames: attack_frames,
                    speed: melee_attack_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MELEE_ATTACK, FRAGMENT_MOSQUITO, 1),
                    frames: attack_frames,
                    speed: melee_attack_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MELEE_ATTACK, FRAGMENT_MOSQUITO, 2),
                    frames: attack_frames,
                    speed: melee_attack_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
                AnimationData {
                    sprite_path: concat_strings_and_number(PATH_SPRITES_ENEMIES, FRAGMENT_MELEE_ATTACK, FRAGMENT_MOSQUITO, 3),
                    frames: attack_frames,
                    speed: melee_attack_speed,
                    ..Default::default()
                    // collision: CollisionBox::new(),
                },
            ]
        }
    };
}
