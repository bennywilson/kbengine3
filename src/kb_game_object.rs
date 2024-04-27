use cgmath::Vector3;
use instant::Instant;

use crate::{game_random_f32};

static mut NEXT_ACTOR_ID: u32 = 0;

#[derive(Clone)]
pub struct KbActor {
    pub id: u32,
    position: Vector3<f32>,
    scale: Vector3<f32>,

    model_id: i32,
}

impl KbActor {
    pub fn new() -> Self {
        unsafe {
            NEXT_ACTOR_ID = NEXT_ACTOR_ID + 1;
            KbActor {
                id: NEXT_ACTOR_ID,
                position: (0.0, 0.0, 0.0).into(),
                scale: (0.0, 0.0, 0.0).into(),
                model_id: -1
            }
        }
    }

    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    pub fn set_scale(&mut self, scale: Vector3<f32>) {
        self.scale = scale;
    }
    
    pub fn set_model_id(&mut self, new_id: i32) {
        self.model_id = new_id;
    }

    pub fn get_model_id(&self) -> i32 {
        self.model_id
    }
}

// todo: deprecate the below

#[derive(Clone)]
pub enum GameObjectType {
    Character,
    Robot,
    Projectile,
    Background,
    Skybox,
    Cloud,
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum GameObjectState {
    Idle,
    Jumping,
    Running
}

#[allow(dead_code)] 
#[derive(Clone)]
pub struct GameObject {
    pub position: Vector3<f32>,
    pub direction: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub object_type: GameObjectType,
    pub object_state: GameObjectState,
    pub next_attack_time: f32,
    pub texture_index: i32,
    pub sprite_index: i32,
    pub anim_frame: i32,
    pub life_start_time: Instant,
    pub state_start_time: Instant,
    pub gravity_scale: f32,
    pub is_enemy: bool,
    pub random_val: f32,
}

#[allow(dead_code)] 
impl GameObject {
    pub fn new(object_type: GameObjectType, sprite_index: i32, position: Vector3<f32>, direction: Vector3<f32>, scale: Vector3<f32>) -> Self {

        GameObject {
            position,
            direction,
            scale,
            velocity: (0.0, 0.0, 0.0).into(),
            object_type,
            object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 0,
			sprite_index,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 3.1,
			random_val: game_random_f32!(0.0, 1000.0),
			is_enemy: false
        }
    }

    fn set_state(&mut self, next_state: GameObjectState) {
        self.object_state = next_state;
        self.state_start_time = Instant::now();
    }

    fn update_movement(&mut self, frame_time: f32) {
        
        self.position = self.position + self.velocity * frame_time;

        // Apply Gravity
        if f32::abs(self.gravity_scale) > 0.001 {
            if self.position.y > 0.0 {
                self.velocity.y -= frame_time * self.gravity_scale;
            } else if self.position.y < 0.0 {
                self.velocity.y = 0.0;
                self.position.y = 0.0;
                self.set_state(GameObjectState::Idle);
            }
        }

        match self.object_state {
            GameObjectState::Running => {
                let duration = self.state_start_time.elapsed().as_secs_f32() * 5.0;
                self.anim_frame = 1 + (duration as i32) % 4;
            }

            _ => {
                self.anim_frame = 0;
            }
        }     
        
        match self.object_type {
            GameObjectType::Projectile => {
                let duration = self.state_start_time.elapsed().as_secs_f32() * 15.0;
                self.anim_frame = (duration as i32) % 3;
            }

            GameObjectType::Skybox => {
                let duration = self.state_start_time.elapsed().as_secs_f32() * 1.2;
                self.anim_frame = (duration as i32) % 2;
            }

            GameObjectType::Cloud => {
                if self.velocity.x > 0.0 {
                    if self.position.x > 2.1 {
                        self.position.x = -2.1;
                    }
                } else {
                    if self.position.x < -2.1 {
                        self.position.x = 2.1;
                    }
                }
            }

            GameObjectType::Robot => {
                if self.velocity.x > 0.0 {
                    if self.position.x > 1.9 {
                        self.velocity.x *= -1.0;
                    }
                } else {
                    if self.position.x < -1.9 {
                        self.velocity.x *= -1.0;
                    }
                }
            }

            GameObjectType::Character => {
                if self.position.x > 1.9 {
                    self.position.x = 1.9;
                } else if self.position.x < -1.9 {
                    self.position.x = -1.9;
                }
            }
            _ => ()
        }
    }

    pub fn update(&mut self, frame_time: f32) {

        self.update_movement(frame_time);
    }

    pub fn set_velocity(&mut self, move_vec: Vector3<f32>) {
        self.velocity.x = move_vec.x;

        if matches!(self.object_type, GameObjectType::Character) == false {
            return;
        }

        let is_jumping = matches!(self.object_state, GameObjectState::Jumping);
        if f32::abs(move_vec.x) < 0.0001 {
            if is_jumping == false {
              self.set_state(GameObjectState::Idle);
            }
        } else if matches!(self.object_state,  GameObjectState::Running) == false {
            if is_jumping == false {
                self.set_state(GameObjectState::Running);
            }
        }

        if move_vec.y > 0.0 && matches!(self.object_state, GameObjectState::Jumping) == false {
            self.velocity.y = 2.1;
            self.set_state(GameObjectState::Jumping);
        }
    }

    pub fn start_attack(&mut self) -> bool {
        let cur_time = self.life_start_time.elapsed().as_secs_f32();
        if self.next_attack_time > cur_time {
            return false;
        }

        self.next_attack_time = cur_time + 0.1;
        return true;
    }
}
