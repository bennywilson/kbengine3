use cgmath::{InnerSpace, SquareMatrix};

use instant::Instant;

use kb_engine3::{kb_assets::*, kb_config::*, kb_game_object::*, kb_input::*, kb_renderer::*, kb_utils::*, log};

#[derive(Clone, Debug)]
enum GamePlayerState {
	Idle,
	Shooting
}

pub struct GamePlayer {
	transform: KbActorTransform,
	current_state: GamePlayerState,
	current_state_time: Instant,

	actor: KbActor,
	model_handle: KbModelHandle,
}

impl GamePlayer {
	pub async fn new(model_handle: &KbModelHandle) -> Self {
		log!("Creating Player");

		let transform = KbActorTransform {
			position: CG_VEC3_ZERO,
			rotation: CG_QUAT_IDENT,
			scale: CG_VEC3_ONE
		};
		let current_state = GamePlayerState::Idle;
		let current_state_time = Instant::now();
		let mut actor = KbActor::new();
		actor.set_position(&[5.0, 1.0, 3.0].into());
		actor.set_scale(&[1.0, 1.0, 1.2].into());
		actor.set_model(&model_handle);

		GamePlayer {
			transform,
			current_state,
			current_state_time,
			model_handle: model_handle.clone(),
			actor
		}
	}

	pub fn get_actor(&self) -> &KbActor {
		&self.actor
	}

	
	pub fn set_state(&mut self, new_state: GamePlayerState) {
		self.current_state = new_state.clone();
		self.current_state_time = Instant::now();
		log!("Changing state to {:?}", new_state);
	}

	pub fn tick(&mut self, input_manager: &KbInputManager, game_camera: &KbCamera, _game_config: &KbConfig) {
		let (view_matrix, view_dir, right_dir) = game_camera.calculate_view_matrix();
		let up_dir = view_dir.cross(right_dir).normalize();
		let gun_pos = game_camera.get_position() + (view_dir * 1.0) + (up_dir * 1.0) + (right_dir * 0.5);
		self.actor.set_position(&gun_pos);

        let gun_fix_rad = cgmath::Rad::from(cgmath::Deg(90.0));
		let gun_mat3 = cgmat4_to_cgmat3(&view_matrix).invert().unwrap();
		let gun_rot: CgQuat = cgmath::Quaternion::from(gun_mat3 * CgMat3::from_angle_y(gun_fix_rad)); 
		self.actor.set_rotation(&gun_rot);

		match self.current_state {
			GamePlayerState::Idle => {
				self.tick_idle(&input_manager);
			}
			GamePlayerState::Shooting => {
				self.tick_shooting(&game_camera);
			}
		}
	}

	fn tick_idle(&mut self, input_manager: &KbInputManager) {
		if self.current_state_time.elapsed().as_secs_f32() > 1.0 && input_manager.fire_pressed {
			self.set_state(GamePlayerState::Shooting);
		}
	}

	fn tick_shooting(&mut self, _game_camera: &KbCamera) {
		if self.current_state_time.elapsed().as_secs_f32() > 1.0  {
			self.set_state(GamePlayerState::Idle);
		}
	}
}

enum GameMobState {
	Idle,
	Chasing,
	Dying,
	Dead
}

pub struct GameMob {
	transform: KbActorTransform,
	current_state: GameMobState,
	current_state_time: Instant
}

impl GameMob {
	pub fn new() -> Self {
		let transform = KbActorTransform {
			position: CG_VEC3_ZERO,
			rotation: CG_QUAT_IDENT,
			scale: CG_VEC3_ONE
		};
		let current_state = GameMobState::Idle;
		let current_state_time = Instant::now();

		GameMob {
			transform,
			current_state,
			current_state_time
		}
	}
}