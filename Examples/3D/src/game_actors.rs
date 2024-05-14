use cgmath::{InnerSpace, Rotation, SquareMatrix};
use instant::Instant;

use kb_engine3::{
    kb_assets::*, kb_collision::*, kb_config::*, kb_game_object::*, kb_input::*, kb_renderer::*,
    kb_resource::*, kb_utils::*,
};

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GamePlayerState {
    None,
    Idle,
    Shooting,
    StartReloading,
    FinishReloading,
}

pub struct GamePlayer {
    current_state: GamePlayerState,
    current_state_time: Instant,

    hands_model: KbModelHandle,
    hands_actor: KbActor,
    outline_actors: Vec<KbActor>,
    hand_bone_offset: CgVec3,

    recoil_radians: cgmath::Rad<f32>,
    recoil_offset: f32,

    next_weapon_model: KbModelHandle,
    has_shotgun: bool,
    ammo_count: u32,
}

const PISTOL_AMMO_MAX: u32 = 8;
const SHOTGUN_AMMO_MAX: u32 = 4;
pub const GLOBAL_SCALE: CgVec3 = CgVec3::new(0.3, 0.3, 0.3);

impl GamePlayer {
    pub async fn new(hands_model: &KbModelHandle) -> Self {
        let current_state = GamePlayerState::Idle;
        let current_state_time = Instant::now();

        let mut hands_actor = KbActor::new();
        hands_actor.set_position(&CgVec3::new(5.0, 1.0, 3.0));
        hands_actor.set_scale(&GLOBAL_SCALE);
        hands_actor.set_model(hands_model);
        hands_actor.set_render_group(&KbRenderGroupType::Foreground, &None);

        let mut outline_actors = Vec::<KbActor>::new();

        let mut push = 0.0035;
        let num_steps = 10;
        for i in 0..num_steps + 1 {
            let mut outline_actor = KbActor::new();
            outline_actor.set_position(&[5.0, 1.0, 3.0].into());
            outline_actor.set_scale(&GLOBAL_SCALE);
            let alpha = 1.0 - (i as f32 / num_steps as f32);
            let alpha = (alpha).clamp(0.0, 1.0);
            outline_actor.set_color(CgVec4::new(0.1, 0.1, 0.1, alpha));
            outline_actor.set_custom_data_1(CgVec4::new(push, 0.75, 0.75, 0.75));
            outline_actor.set_model(hands_model);
            outline_actor.set_render_group(&KbRenderGroupType::Foreground, &None);
            outline_actors.push(outline_actor);
            push += 0.0035;
        }

        GamePlayer {
            current_state,
            current_state_time,
            hands_actor,
            outline_actors,
            has_shotgun: false,
            next_weapon_model: hands_model.clone(),
            ammo_count: PISTOL_AMMO_MAX,
            hands_model: hands_model.clone(),
            hand_bone_offset: CG_VEC3_ZERO,
            recoil_offset: 0.0,
            recoil_radians: cgmath::Rad::from(cgmath::Deg(0.0)),
        }
    }

    pub fn get_actors(&mut self) -> (&mut KbActor, &mut Vec<KbActor>) {
        (&mut self.hands_actor, &mut self.outline_actors)
    }

    pub fn set_state(&mut self, new_state: GamePlayerState) {
        self.current_state = new_state.clone();
        self.current_state_time = Instant::now();
    }

    pub fn give_shotgun(&mut self, model_handle: &KbModelHandle) {
        self.set_state(GamePlayerState::StartReloading);
        self.next_weapon_model = model_handle.clone();
    }

    pub fn has_shotgun(&self) -> bool {
        self.has_shotgun
    }

    pub fn get_ammo_count(&self) -> u32 {
        self.ammo_count
    }

    pub fn tick(
        &mut self,
        input_manager: &KbInputManager,
        game_camera: &KbCamera,
        _game_config: &KbConfig,
    ) -> (GamePlayerState, GamePlayerState) {
        let ret_val: (GamePlayerState, GamePlayerState);
        match self.current_state {
            GamePlayerState::Idle => {
                ret_val = (GamePlayerState::Idle, self.tick_idle(input_manager));
            }
            GamePlayerState::Shooting => {
                ret_val = (GamePlayerState::Shooting, self.tick_shooting(game_camera));
            }
            GamePlayerState::StartReloading => {
                ret_val = (
                    GamePlayerState::StartReloading,
                    self.tick_start_reloading(game_camera),
                );
            }
            GamePlayerState::FinishReloading => {
                ret_val = (
                    GamePlayerState::FinishReloading,
                    self.tick_finish_reloading(game_camera),
                );
            }
            _ => {
                panic!("GamePlayer::tick() - GamePlayerState::None is an invalid state")
            }
        }

        let (view_matrix, view_dir, right_dir) = game_camera.calculate_view_matrix();
        let up_dir = view_dir.cross(right_dir).normalize();
        let hand_mat3 = cgmat4_to_cgmat3(&view_matrix).invert().unwrap();

        let mut hand_pos;
        let hand_rot;

        if !self.has_shotgun {
            hand_pos = game_camera.get_position()
                + (view_dir * 0.9)
                + (up_dir * 0.75)
                + (right_dir * 0.5)
                + (view_dir * self.recoil_offset);
            let hand_fix_rad = cgmath::Rad::from(cgmath::Deg(85.0));
            hand_rot = cgmath::Quaternion::from(
                hand_mat3
                    * CgMat3::from_angle_x(self.recoil_radians)
                    * CgMat3::from_angle_y(hand_fix_rad),
            );
        } else {
            hand_pos =
                game_camera.get_position() + (view_dir * 0.5) + (up_dir * 1.0) + (right_dir * 0.4);
            hand_rot =
                cgmath::Quaternion::from(hand_mat3 * CgMat3::from_angle_x(self.recoil_radians));
        }
        hand_pos += -up_dir * self.hand_bone_offset.y;

        self.hands_actor.set_position(&(hand_pos));
        self.hands_actor.set_rotation(&hand_rot);

        let outline_iter = self.outline_actors.iter_mut();
        for outline in outline_iter {
            outline.set_position(&hand_pos);
            outline.set_rotation(&hand_rot);
        }

        ret_val
    }

    // Returns a state change if any.
    fn tick_idle(&mut self, input_manager: &KbInputManager) -> GamePlayerState {
        if self.current_state_time.elapsed().as_secs_f32() > 0.1 && input_manager.fire_pressed {
            self.set_state(GamePlayerState::Shooting);
            self.ammo_count -= 1;
            return GamePlayerState::Shooting;
        }
        GamePlayerState::Idle
    }

    fn tick_shooting(&mut self, _game_camera: &KbCamera) -> GamePlayerState {
        let shoot_state_length = 0.3;
        let recoil_time = 0.016;

        let elasped_state_time = self.current_state_time.elapsed().as_secs_f32();
        if elasped_state_time <= recoil_time {
            let t = elasped_state_time / recoil_time;
            self.recoil_radians = cgmath::Rad::from(cgmath::Deg(5.0 * t));
            self.recoil_offset = t * -0.1;
        } else {
            let t = 1.0 - (elasped_state_time - recoil_time) / (shoot_state_length - recoil_time);
            self.recoil_radians = cgmath::Rad::from(cgmath::Deg(5.0 * t));
            self.recoil_offset = t * -0.1;
        }

        if self.current_state_time.elapsed().as_secs_f32() > 0.3 {
            if self.ammo_count == 0 {
                self.set_state(GamePlayerState::StartReloading);
                return GamePlayerState::StartReloading;
            }

            self.set_state(GamePlayerState::Idle);
            return GamePlayerState::Idle;
        }
        GamePlayerState::Shooting
    }

    fn tick_start_reloading(&mut self, _game_camera: &KbCamera) -> GamePlayerState {
        let reload_duration = 0.85;
        let one_over_duration = 1.0 / reload_duration;
        let half_duration = reload_duration * 0.5;
        let bottom_y = -3.0;

        let cur_state_time = self.current_state_time.elapsed().as_secs_f32();
        if cur_state_time < half_duration {
            self.hand_bone_offset.y =
                (bottom_y * cur_state_time * one_over_duration).clamp(bottom_y, 0.0);
        } else {
            self.hand_bone_offset.y = bottom_y;

            self.hands_actor.set_model(&self.next_weapon_model);
            for i in 0..11 {
                self.outline_actors[i].set_model(&self.next_weapon_model);
            }

            if self.next_weapon_model != self.hands_model {
                self.ammo_count = SHOTGUN_AMMO_MAX;
                self.has_shotgun = true;
            } else {
                self.ammo_count = PISTOL_AMMO_MAX;
                self.has_shotgun = false;
            }

            let start_push = if self.has_shotgun { 0.01 } else { 0.0035 };
            let mut push = start_push;
            self.hands_actor
                .set_custom_data_1(CgVec4::new(push, 0.75, 0.75, 0.75));
            for outline_actor in &mut self.outline_actors {
                outline_actor.set_custom_data_1(CgVec4::new(push, 0.75, 0.75, 0.75));
                push += 0.0035;
            }

            self.next_weapon_model = self.hands_model.clone();
            self.set_state(GamePlayerState::FinishReloading);
            return GamePlayerState::FinishReloading;
        }

        GamePlayerState::StartReloading
    }

    fn tick_finish_reloading(&mut self, _game_camera: &KbCamera) -> GamePlayerState {
        let reload_duration = 0.85;
        let one_over_duration = 1.0 / reload_duration;
        let half_duration = reload_duration * 0.5;
        let bottom_y = -3.0;

        let cur_state_time = self.current_state_time.elapsed().as_secs_f32();
        if cur_state_time < half_duration {
            self.hand_bone_offset.y =
                (bottom_y * (half_duration - cur_state_time) * one_over_duration)
                    .clamp(bottom_y, 0.0);
        } else {
            self.hand_bone_offset.y = 0.0;
            self.set_state(GamePlayerState::Idle);
            return GamePlayerState::Idle;
        }

        GamePlayerState::FinishReloading
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameMobState {
    Idle,
    Chasing,
    Attacking,
    Dying,
    Dead,
}

#[allow(dead_code)]
pub struct GameMob {
    monster_actors: Vec<KbActor>,
    collision_handle: KbCollisionHandle,

    current_state: GameMobState,
    current_state_time: Instant,
}

#[allow(dead_code)]
impl GameMob {
    pub fn new(
        position: &CgVec3,
        model_handle: &KbModelHandle,
        render_group: usize,
        outline_render_group: usize,
        collision_manager: &mut KbCollisionManager,
    ) -> Self {
        let mut monster_actor = KbActor::new();
        monster_actor.set_position(position);
        monster_actor.set_render_group(&KbRenderGroupType::WorldCustom, &Some(render_group));
        monster_actor.set_scale(&(CgVec3::new(3.0, 3.0, 3.0) * GLOBAL_SCALE.x));
        monster_actor.set_model(model_handle);
        let mut monster_actors = Vec::<KbActor>::new();

        let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
            position: monster_actor.get_position(),
            extents: CgVec3::new(2.0, 2.0, 2.0),
            block: true,
        });
        let collision_handle = collision_manager.add_collision(&collision_box);

        let current_state = GameMobState::Idle;
        let current_state_time = Instant::now();

        monster_actors.push(monster_actor);
        let mut monster_outline = KbActor::new();
        monster_outline.set_position(position);
        monster_outline
            .set_render_group(&KbRenderGroupType::WorldCustom, &Some(outline_render_group));
        monster_outline.set_scale(&(CgVec3::new(3.0, 3.0, 3.0) * GLOBAL_SCALE.x));
        monster_outline.set_model(model_handle);

        #[cfg(not(target_arch = "wasm32"))]
        monster_outline.set_custom_data_1(CgVec4::new(0.045, 3.0, 3.0, 3.0));

        #[cfg(target_arch = "wasm32")]
        monster_outline.set_custom_data_1(CgVec4::new(0.045, 7.0, 7.0, 7.0));

        monster_actors.push(monster_outline);

        GameMob {
            monster_actors,
            collision_handle,
            current_state,
            current_state_time,
        }
    }

    pub fn get_actors(&mut self) -> &mut Vec<KbActor> {
        &mut self.monster_actors
    }

    pub fn get_state(&self) -> GameMobState {
        self.current_state.clone()
    }
    pub fn get_collision_handle(&self) -> &KbCollisionHandle {
        &self.collision_handle
    }

    pub fn take_damage(
        &mut self,
        collision_manager: &mut KbCollisionManager,
        renderer: &mut KbRenderer,
    ) -> bool {
        collision_manager.remove_collision(&self.collision_handle);
        renderer.remove_actor(&self.monster_actors[0]);
        renderer.remove_actor(&self.monster_actors[1]);
        true
    }

    pub fn tick(
        &mut self,
        player_pos: CgVec3,
        speed_multiplier: f32,
        collision_manager: &mut KbCollisionManager,
        game_config: &KbConfig,
    ) {
        let vec_to_player = player_pos - self.monster_actors[0].get_position();
        let dist_to_player = vec_to_player.magnitude();
        let vec_to_player = vec_to_player.normalize();

        {
            let monster_actor = &mut self.monster_actors[0];
            if dist_to_player > 5.0 {
                collision_manager.remove_collision(&self.collision_handle); // hack. Don't collide with self
                let move_vec = vec_to_player * game_config.delta_time * speed_multiplier;
                let (t, _, _, blocks) =
                    collision_manager.cast_ray(&monster_actor.get_position(), &move_vec);

                let block = blocks.unwrap_or(true);
                if !(0.0..1.0).contains(&t) || !block {
                    let new_pos = monster_actor.get_position() + move_vec;
                    monster_actor.set_position(&new_pos);
                }
                let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
                    position: monster_actor.get_position(),
                    extents: CgVec3::new(2.0, 2.0, 2.0),
                    block: true,
                });
                self.collision_handle = collision_manager.add_collision(&collision_box);
                self.current_state = GameMobState::Chasing;
            } else {
                self.current_state = GameMobState::Attacking;
            }

            let vec_to_player = CgVec3::new(vec_to_player.x, 0.0, vec_to_player.z).normalize();
            monster_actor.set_rotation(&CgQuat::look_at(vec_to_player, -CG_VEC3_UP));
        }
        let monster_pos = self.monster_actors[0].get_position();
        let monster_rot = self.monster_actors[0].get_rotation();
        self.monster_actors[1].set_position(&monster_pos);
        self.monster_actors[1].set_rotation(&monster_rot);

        collision_manager.update_collision_position(
            &self.collision_handle,
            &self.monster_actors[0].get_position(),
        );
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum GamePropType {
    Shotgun,
    Barrel,
}

#[allow(dead_code)]
pub struct GameProp {
    actors: Vec<KbActor>,
    collision_handle: KbCollisionHandle,
    prop_type: GamePropType,
    particle_handles: [KbParticleHandle; 2],
    start_time: Instant,
}

impl GameProp {
    pub fn new(
        prop_type: &GamePropType,
        position: &CgVec3,
        model_handle: &KbModelHandle,
        outline_render_group: usize,
        collision_manager: &mut KbCollisionManager,
        particle_handles: [KbParticleHandle; 2],
    ) -> Self {
        let extents = {
            match prop_type {
                GamePropType::Shotgun => CgVec3::new(1.5, 1.5, 1.5),
                GamePropType::Barrel => CgVec3::new(1.1, 4.0, 1.1),
            }
        };

        let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
            position: *position,
            extents,
            block: false,
        });

        let collision_handle = collision_manager.add_collision(&collision_box);
        let start_time = Instant::now();

        let mut actors = Vec::<KbActor>::new();
        let mut actor = KbActor::new();
        actor.set_position(position);
        actor.set_model(model_handle);
        actor.set_scale(&GLOBAL_SCALE);
        actors.push(actor);

        // Outline
        let mut actor = KbActor::new();
        actor.set_position(position);
        actor.set_model(model_handle);
        actor.set_scale(&GLOBAL_SCALE);
        actor.set_render_group(&KbRenderGroupType::WorldCustom, &Some(outline_render_group));

        let push = {
            match prop_type {
                GamePropType::Shotgun => 0.21,
                GamePropType::Barrel => 0.21,
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        actor.set_custom_data_1(CgVec4::new(push, 0.05, 0.05, 0.05));

        #[cfg(target_arch = "wasm32")]
        actor.set_custom_data_1(CgVec4::new(push, 0.17, 0.17, 0.17));

        actors.push(actor);

        GameProp {
            actors,
            collision_handle,
            prop_type: *prop_type,
            particle_handles,
            start_time,
        }
    }

    pub fn take_damage(
        &mut self,
        collision_manager: &mut KbCollisionManager,
        renderer: &mut KbRenderer,
    ) -> bool {
        collision_manager.remove_collision(&self.collision_handle);
        for actor in &mut self.actors {
            renderer.remove_actor(actor);
        }

        if self.particle_handles[0] != INVALID_PARTICLE_HANDLE {
            renderer.enable_particle_actor(&self.particle_handles[0], false);
        }

        if self.particle_handles[1] != INVALID_PARTICLE_HANDLE {
            renderer.enable_particle_actor(&self.particle_handles[1], false);
        }

        true
    }

    pub fn get_collision_handle(&self) -> KbCollisionHandle {
        self.collision_handle.clone()
    }

    pub fn get_prop_type(&self) -> GamePropType {
        self.prop_type
    }
    pub fn get_actors(&mut self) -> &mut Vec<KbActor> {
        &mut self.actors
    }
}

pub struct GameDecal {
    pub actor: KbActor,
    pub start_time: f32,
}
