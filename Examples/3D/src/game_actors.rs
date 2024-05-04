use cgmath::{InnerSpace, SquareMatrix};

use instant::Instant;

use kb_engine3::{kb_assets::*, kb_config::*, kb_game_object::*, kb_input::*, kb_renderer::*, kb_resource::*, kb_utils::*, log};

#[derive(Clone, Debug)]
enum GamePlayerState {
	Idle,
	Shooting
}

pub struct GamePlayer {
	transform: KbActorTransform,
	current_state: GamePlayerState,
	current_state_time: Instant,

	hands_actor: KbActor,
	outline_actor: KbActor,

	hand_model_handle: KbModelHandle,
	hand_outline_model_handle: KbModelHandle,
}

impl GamePlayer {
	pub async fn new(hand_handle: &KbModelHandle, hand_outline: &KbModelHandle) -> Self {
		log!("Creating Player");

		let transform = KbActorTransform {
			position: CG_VEC3_ZERO,
			rotation: CG_QUAT_IDENT,
			scale: CG_VEC3_ONE
		};
		let current_state = GamePlayerState::Idle;
		let current_state_time = Instant::now();
		let mut hands_actor = KbActor::new();
		hands_actor.set_position(&[5.0, 1.0, 3.0].into());
		hands_actor.set_scale(&[1.0, 1.0, 1.0].into());
		hands_actor.set_model(&hand_handle);
		hands_actor.set_render_group(&KbRenderGroup::Foreground);

		let mut outline_actor = KbActor::new();
		outline_actor.set_position(&[5.0, 1.0, 3.0].into());
		outline_actor.set_scale(&[0.99, 0.99, 0.99].into());
		outline_actor.set_model(&hand_outline);
		outline_actor.set_render_group(&KbRenderGroup::Foreground);

		GamePlayer {
			transform,
			current_state,
			current_state_time,
			hand_model_handle: hand_handle.clone(),
			hand_outline_model_handle: hand_outline.clone(),
			hands_actor,
			outline_actor,
		}
	}

	pub fn get_actors(&self) ->(&KbActor, &KbActor) {
		(&self.hands_actor, &self.outline_actor)
	}
	
	pub fn set_state(&mut self, new_state: GamePlayerState) {
		self.current_state = new_state.clone();
		self.current_state_time = Instant::now();
		log!("Changing state to {:?}", new_state);
	}

	pub fn tick(&mut self, input_manager: &KbInputManager, game_camera: &KbCamera, _game_config: &KbConfig) {
		let (view_matrix, view_dir, right_dir) = game_camera.calculate_view_matrix();
		let up_dir = view_dir.cross(right_dir).normalize();
		let hand_pos = game_camera.get_position() + (view_dir * 0.9) + (up_dir * 0.7) + (right_dir * 0.6);
		self.hands_actor.set_position(&hand_pos);
		self.outline_actor.set_position(&(hand_pos + view_dir * (0.02) + right_dir * 0.000 + up_dir * 0.000));

        let hand_fix_rad = cgmath::Rad::from(cgmath::Deg(85.0));
		let hand_mat3 = cgmat4_to_cgmat3(&view_matrix).invert().unwrap();
		let hand_rot: CgQuat = cgmath::Quaternion::from(hand_mat3 * CgMat3::from_angle_y(hand_fix_rad)); 
		self.hands_actor.set_rotation(&hand_rot);
		self.outline_actor.set_rotation(&hand_rot);

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