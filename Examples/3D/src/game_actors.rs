use cgmath::{InnerSpace, Rotation, SquareMatrix};

use instant::Instant;

use kb_engine3::{kb_assets::*, kb_collision::*, kb_config::*, kb_game_object::*, kb_input::*, kb_renderer::*, kb_resource::*, 
	kb_utils::*, log};

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GamePlayerState {
	None,
	Idle,
	Shooting,
	Reloading,
}

pub struct GamePlayer {
	current_state: GamePlayerState,
	current_state_time: Instant,

	hand_model: KbModelHandle,
	hands_actor: KbActor,
	outline_actors: Vec<KbActor>,
	hand_bone_offset: CgVec3,

	has_shotgun: bool,
	ammo_count: u32,
}

const PISTOL_AMMO_MAX: u32 = 8;
const SHOTGUN_AMMO_MAX: u32 = 4;

impl GamePlayer {
	pub async fn new(hand_model: &KbModelHandle) -> Self {
		log!("Creating Player");
		let current_state = GamePlayerState::Idle;
		let current_state_time = Instant::now();
		let mut hands_actor = KbActor::new();
		hands_actor.set_position(&[5.0, 1.0, 3.0].into());
		hands_actor.set_scale(&[1.0, 1.0, 1.0].into());
		hands_actor.set_model(&hand_model);
		hands_actor.set_render_group(&KbRenderGroupType::Foreground, &None);

		let mut outline_actors = Vec::<KbActor>::new();

		let mut push = 0.00075;
		let num_steps = 10;
		for i in 0..num_steps + 1 {
			let mut outline_actor = KbActor::new();
			outline_actor.set_position(&[5.0, 1.0, 3.0].into());
			outline_actor.set_scale(&CG_VEC3_ONE);
			let alpha = 1.0 - (i as f32 / num_steps as f32);
			let alpha = (alpha).clamp(0.0, 1.0);
			outline_actor.set_color(CgVec4::new(0.2, 0.2, 0.2, alpha));
			outline_actor.set_custom_data_1(CgVec4::new(push, 0.0, 0.0, 0.0)); 
			outline_actor.set_model(&hand_model);
			outline_actor.set_render_group(&KbRenderGroupType::Foreground, &None);
			outline_actors.push(outline_actor);
			push += 0.00075;
		}

		GamePlayer {
			current_state,
			current_state_time,
			hands_actor,
			outline_actors,
			has_shotgun: false,
			ammo_count: PISTOL_AMMO_MAX,
			hand_model: hand_model.clone(),
			hand_bone_offset: CG_VEC3_ZERO,
		}
	}

	pub fn get_actors(&mut self) ->(&mut KbActor, &mut Vec<KbActor>) {
		(&mut self.hands_actor, &mut self.outline_actors)
	}
	
	pub fn set_state(&mut self, new_state: GamePlayerState) {
		self.current_state = new_state.clone();
		self.current_state_time = Instant::now();
	}

	pub fn give_shotgun(&mut self, model_handle: &KbModelHandle) {
		self.hands_actor.set_model(&model_handle);

		for i in 0..11 {
			self.outline_actors[i].set_model(&model_handle);
		}
		self.ammo_count = SHOTGUN_AMMO_MAX;
		self.has_shotgun = true;
	}

	pub fn give_pistol(&mut self) {
		self.hands_actor.set_model(&self.hand_model);

		for i in 0..11 {
			self.outline_actors[i].set_model(&self.hand_model);
		}
		self.ammo_count = PISTOL_AMMO_MAX;
		self.has_shotgun = false;
	}

	pub fn has_shotgun(&self) -> bool {
		self.has_shotgun
	}

	pub fn get_ammo_count(&self) -> u32 {
		self.ammo_count
	}

	pub fn tick(&mut self, input_manager: &KbInputManager, game_camera: &KbCamera, _game_config: &KbConfig) -> (GamePlayerState, GamePlayerState) {
		let mut recoil_rad = cgmath::Rad::from(cgmath::Deg(0.0));

		let ret_val: (GamePlayerState, GamePlayerState);
		match self.current_state {
			GamePlayerState::Idle => {
				ret_val = (GamePlayerState::Idle, self.tick_idle(&input_manager));
			}
			GamePlayerState::Shooting => {
				recoil_rad = cgmath::Rad::from(cgmath::Deg(5.0));
				ret_val = (GamePlayerState::Shooting, self.tick_shooting(&game_camera));
			}
			GamePlayerState::Reloading => {
				ret_val = (GamePlayerState::Reloading, self.tick_reloading(&game_camera));
			}
			_ => { panic!("GamePlayer::tick() - GamePlayerState::None is an invalid state") }
		}


		let (view_matrix, view_dir, right_dir) = game_camera.calculate_view_matrix();
		let up_dir = view_dir.cross(right_dir).normalize();
		let hand_mat3 = cgmat4_to_cgmat3(&view_matrix).invert().unwrap();

		let mut hand_pos;
		let hand_rot;

		if self.has_shotgun == false {
			hand_pos = game_camera.get_position() + (view_dir * 0.9) + (up_dir * 0.7) + (right_dir * 0.6);
			let hand_fix_rad = cgmath::Rad::from(cgmath::Deg(85.0) ); 
			hand_rot = cgmath::Quaternion::from(hand_mat3 * CgMat3::from_angle_x(recoil_rad) * CgMat3::from_angle_y(hand_fix_rad)); 
		} else {
			hand_pos = game_camera.get_position() + (view_dir * 0.5) + (up_dir * 1.0) + (right_dir * 0.4);
			hand_rot = cgmath::Quaternion::from(hand_mat3 * CgMat3::from_angle_x(recoil_rad)); 

		}
		hand_pos += self.hand_bone_offset;

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
			self.ammo_count = self.ammo_count - 1;
			return GamePlayerState::Shooting;
		}
		GamePlayerState::Idle
	}

	fn tick_shooting(&mut self, _game_camera: &KbCamera) -> GamePlayerState {
		if self.current_state_time.elapsed().as_secs_f32() > 0.3  {
			if self.ammo_count == 0 {
				self.set_state(GamePlayerState::Reloading);
				return GamePlayerState::Reloading;
			}

			self.set_state(GamePlayerState::Idle);
			return GamePlayerState::Idle;
		}
		GamePlayerState::Shooting
	}

	fn tick_reloading(&mut self, _game_camera: &KbCamera) -> GamePlayerState {
		let reload_duration = 0.85;
		let one_over_duration = 1.0 / reload_duration;
		let half_duration = reload_duration * 0.5;
		let bottom_y = -3.0;

		let cur_state_time = self.current_state_time.elapsed().as_secs_f32();
		if cur_state_time < half_duration {
			self.hand_bone_offset.y = (bottom_y * cur_state_time * one_over_duration).clamp(bottom_y, 0.0);
		} else {
			self.give_pistol();
			self.hand_bone_offset.y = (bottom_y * (reload_duration - cur_state_time) * one_over_duration).clamp(bottom_y, 0.0);
		}

		if cur_state_time > reload_duration {
			self.give_pistol();
			self.set_state(GamePlayerState::Idle);
			GamePlayerState::Idle
		} else {
			GamePlayerState::Reloading
		}
	}
}

#[allow(dead_code)]
enum GameMobState {
	Idle,
	Chasing,
	Dying,
	Dead	
}

#[allow(dead_code)]
pub struct GameMob {
	monster_actor: KbActor,
	collision_handle: KbCollisionHandle,

	current_state: GameMobState,
	current_state_time: Instant
}

#[allow(dead_code)]
impl GameMob {
	pub fn new(position: &CgVec3, model_handle: &KbModelHandle, collision_manager: &mut KbCollisionManager) -> Self {
		let mut monster_actor = KbActor::new();
		monster_actor.set_position(&position);
		monster_actor.set_scale(&[3.0, 3.0, 3.0].into());
		monster_actor.set_model(&model_handle);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: monster_actor.get_position().clone(),
			extents: CgVec3::new(2.0, 2.0, 2.0)
		});
		let collision_handle = collision_manager.add_collision(&collision_box);

		let current_state = GameMobState::Idle;
		let current_state_time = Instant::now();

		GameMob {
			monster_actor,
			collision_handle,
			current_state,
			current_state_time
		}
	}

	pub fn get_actor(&mut self) -> &mut KbActor {
		&mut self.monster_actor
	}

	pub fn get_collision_handle(&self) -> &KbCollisionHandle {
		&self.collision_handle
	}

	pub fn take_damage(&mut self, collision_manager: &mut KbCollisionManager, renderer: &mut KbRenderer) -> bool {
		collision_manager.remove_collision(&self.collision_handle);
		renderer.remove_actor(&self.monster_actor);
		true
	}

	pub fn tick(&mut self, player_pos: CgVec3, collision_manager: &mut KbCollisionManager, game_config: &KbConfig) {
		let vec_to_player = player_pos - self.monster_actor.get_position();
		let dist_to_player = vec_to_player.magnitude();
		let vec_to_player = vec_to_player.normalize();

		let monster_actor = &mut self.monster_actor;
		if dist_to_player > 5.0 {
			let new_pos = monster_actor.get_position() + vec_to_player * game_config.delta_time * 5.0;
			monster_actor.set_position(&new_pos);
		}
		monster_actor.set_rotation(&CgQuat::look_at(vec_to_player, -CG_VEC3_UP));

		collision_manager.update_collision_position(&self.collision_handle, &monster_actor.get_position());
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
	actor: KbActor,
	collision_handle: KbCollisionHandle,
	prop_type: GamePropType,
	particle_handles: [KbParticleHandle; 2],
	start_time: Instant
}

impl GameProp {
	pub fn new(prop_type: &GamePropType, position: &CgVec3, model_handle: &KbModelHandle, collision_manager: &mut KbCollisionManager, particle_handles: [KbParticleHandle; 2]) -> Self {
		let mut actor = KbActor::new();
		actor.set_position(&position);
		actor.set_model(&model_handle);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: actor.get_position().clone(),
			extents: CgVec3::new(2.0, 2.0, 2.0)
		});

		let collision_handle = collision_manager.add_collision(&collision_box);
		let start_time = Instant::now();

		GameProp {
			actor,
			collision_handle,
			prop_type: *prop_type,
			particle_handles,
			start_time
		}
	}

	pub fn take_damage(&mut self, collision_manager: &mut KbCollisionManager, renderer: &mut KbRenderer) -> bool {
		collision_manager.remove_collision(&self.collision_handle);
		renderer.remove_actor(&self.actor);

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
	pub fn get_actor(&mut self) -> &mut KbActor {
		&mut self.actor
	}
}