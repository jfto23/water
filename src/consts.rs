pub const RES_HEIGHT: u32 = 1080;
pub const RES_WIDTH: u32 = 1920;

pub const DEFAULT_RENDER_LAYER: usize = 0;
pub const VIEW_MODEL_RENDER_LAYER: usize = 1;

pub const SHOOT_COOLDOWN: f32 = 0.5;

// meters per sec
pub const ROCKET_SPEED: f32 = 20.0;
pub const ROCKET_EXPLOSION_RADIUS: f32 = 4.0;
pub const ROCKET_EXPLOSION_FORCE: f32 = 20.0;
pub const MAX_ROCKET_DAMAGE: usize = 50;

// used for air strafing calculations. Not the actual max air speed
pub const PSEUDO_MAX_AIR_SPEED: f32 = 7.0;

pub const PLAYER_HEALTH: usize = 128;
// in seconds
pub const PLAYER_DEATH_TIMER: f32 = 1.0;

pub const SERVER_CAMERA_SPEED: f32 = 32.0;

pub const CHARACTER_MODEL_PATH: &str = "models/character.glb";
