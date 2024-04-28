use instant::Instant;
use cgmath::InnerSpace;

use crate::{kb_renderer::{KbModelHandle, INVALID_MODEL_HANDLE}, kb_utils::*, game_random_f32};

static mut NEXT_ACTOR_ID: u32 = 0;

#[derive(Clone)]
pub struct KbActor {
    pub id: u32,
    position: CgVec3,
    scale: CgVec3,

    model_handle: KbModelHandle,
}

impl KbActor {
    pub fn new() -> Self {
        unsafe {
            NEXT_ACTOR_ID = NEXT_ACTOR_ID + 1;
            KbActor {
                id: NEXT_ACTOR_ID,
                position: (0.0, 0.0, 0.0).into(),
                scale: (0.0, 0.0, 0.0).into(),
                model_handle: KbModelHandle { index: INVALID_MODEL_HANDLE } 
            }
        }
    }

    pub fn set_position(&mut self, position: &CgVec3) {
        self.position = position.clone();
    }

    pub fn get_position(&self) -> CgVec3 {
        self.position
    }

    pub fn set_scale(&mut self, scale: &CgVec3) {
        self.scale = scale.clone();
    }
 
    pub fn get_scale(&self) -> CgVec3 {
        self.scale
    }

    pub fn set_model(&mut self, new_model: &KbModelHandle) {
        self.model_handle = new_model.clone();
    }

    pub fn get_model(&self) -> KbModelHandle {
        self.model_handle.clone()
    }
}

#[derive(Clone)]
pub struct KbCamera {
    position: CgVec3,
    rotation: CgQuat,
}

impl KbCamera {
    pub fn new() -> Self {
        KbCamera {
            position: CG_VEC_ZERO,
            rotation: CG_QUAT_IDENT
        }
    }

    pub fn set_look_at(&mut self, new_pos: &CgVec3, target_pos: &CgVec3) {
        self.set_position(new_pos);
        self.set_rotation(&cgmath::Matrix3::look_to_rh((new_pos - target_pos).normalize(), CG_VEC_UP).into());
    }

    pub fn set_position(&mut self, new_pos: &CgVec3) {
        self.position = new_pos.clone();
    }

    pub fn get_position(&self) -> CgVec3 {
        self.position.clone()
    }

    pub fn set_rotation(&mut self, new_rot: &CgQuat) {
        self.rotation = new_rot.clone();
    }

    pub fn get_rotation(&self) -> CgQuat {
        self.rotation.clone()
    }

    pub fn get_view_matrix(&self) -> (CgMat, CgVec3, CgVec3) {
        let cam_pos = self.get_position();
        let eye: CgPoint = CgPoint::new(cam_pos.x, cam_pos.y, cam_pos.z);
        let view_mat = cgmath::Matrix4::from(self.get_rotation());
        let right_dir = -CgVec3::new(view_mat.x.x, view_mat.x.y, view_mat.x.z);
        let view_dir = CgVec3::new(view_mat.z.x, view_mat.z.y, view_mat.z.z);
        let target = eye + view_dir;
        let up = cgmath::Vector3::unit_y();
        (cgmath::Matrix4::look_at_rh(eye, target, up), view_dir, right_dir)
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
    pub position: CgVec3,
    pub direction: CgVec3,
    pub scale: CgVec3,
    pub velocity: CgVec3,
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
    pub fn new(object_type: GameObjectType, sprite_index: i32, position: CgVec3, direction: CgVec3, scale: CgVec3) -> Self {

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

    fn update_movement(&mut self, delta_time: f32) {
        
        self.position = self.position + self.velocity * delta_time;

        // Apply Gravity
        if f32::abs(self.gravity_scale) > 0.001 {
            if self.position.y > 0.0 {
                self.velocity.y -= delta_time * self.gravity_scale;
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

    pub fn set_velocity(&mut self, move_vec: CgVec3) {
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
