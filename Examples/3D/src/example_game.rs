use instant::Instant;

use cgmath::Rotation3;

use kb_engine3::{kb_config::KbConfig, kb_engine::KbGameEngine, kb_input::InputManager, kb_game_object::*, kb_renderer::KbRenderer};
use kb_engine3::kb_utils::*;
use kb_engine3::{game_random_f32, log};

pub const CAMERA_MOVE_RATE: f32 = 10.0;
pub const CAMERA_ROTATION_RATE: f32 = 50.0;

pub struct Example3DGame {
	actors: Vec<KbActor>,
	game_objects: Vec<GameObject>,
	game_camera: KbCamera,
}

impl Example3DGame { }

impl KbGameEngine for Example3DGame {
	fn new(_game_config: &KbConfig) -> Self {
		log!("GameEngine::new() caled...");
		let mut game_objects = Vec::<GameObject>::new();
		game_objects.push(GameObject { 
			position: (-1.0, -0.33, 55.0).into(),
			scale: (0.1, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.3, 0.0, 0.0).into(),
			object_type: GameObjectType::Robot,
			object_state: GameObjectState::Running,
			next_attack_time: 0.0,
			texture_index: 0,
			sprite_index: 8,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 0.0,
			random_val: game_random_f32!(0.0, 1000.0),
			is_enemy: true
		});

		let mut game_camera = KbCamera::new();
		game_camera.set_look_at(&CgVec3::new(0.0, 2.0, 5.0), &CgVec3::new(0.0, 2.0, -5.0));
	
		Self {
			actors: Vec::<KbActor>::new(),
			game_objects,
			game_camera
		}
    }

	fn initialize_world(&mut self, renderer: &mut KbRenderer) {
		log!("GameEngine::initialize_world() caled...");

		let pinky_model = renderer.load_model("game_assets/pinky.gltf");
		let barrel_model = renderer.load_model("game_assets/barrel.gltf");
		let shotgun_model = renderer.load_model("game_assets/shotgun.gltf");
		let floor_model = renderer.load_model("game_assets/floor.gltf");

		let mut actor = KbActor::new();
		actor.set_position(&[3.0, 0.0, 3.0].into());
		actor.set_scale(&[1.0, 1.0, 1.0].into());
		actor.set_model(&pinky_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[0]);

		let mut actor = KbActor::new();
		actor.set_position(&[0.0, 0.0, 0.0].into());
		actor.set_scale(&[1.0, 1.0, 1.0].into());
		actor.set_model(&barrel_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[1]);
	
		let mut actor = KbActor::new();
		actor.set_position(&[-4.0, 0.0, -5.0].into());
		actor.set_scale(&[2.0, 2.0, 2.0].into());
		actor.set_model(&shotgun_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[2]);

		let mut actor = KbActor::new();
		actor.set_position(&[0.0, 0.0, 0.0].into());
		actor.set_scale(&[10.0, 19.0, 10.0].into());
		actor.set_model(&floor_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[3]);

		// Sky
		self.game_objects.push(GameObject { 
			position: (0.0, 0.0, 0.0).into(),
			scale: (2.0, 2.0, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Skybox,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 25,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 0.0,
			random_val: game_random_f32!(0.0, 1000.0),
			is_enemy: false
		});

		// Sun
	/*	self.game_objects.push(GameObject { 
			position: (-0.5, 1.0, 1.0).into(),
			scale: (0.15, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Skybox,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 27,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 0.0,
			random_val: game_random_f32!(0.0, 1000.0),
			is_enemy: false
		});

		// Hills
		self.game_objects.push(GameObject { 
			position: (0.0, 0.75, 2.0).into(),
			scale: (2.0, 1.6, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 21,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 0.0,
			random_val: game_random_f32!(0.0, 1000.0),
			is_enemy: false
		});*/
    }

	fn get_game_objects(&self) -> &Vec<GameObject> {
		&self.game_objects
	}

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &InputManager, game_config: &KbConfig) {
		for game_object in &mut self.game_objects {
			game_object.update(game_config.delta_time);
		}
		let delta_time = game_config.delta_time;
		let (_s, view_dir, right_dir) = self.game_camera.get_view_matrix();
		let mut camera_pos = self.game_camera.get_position();
		let mut camera_rot = self.game_camera.get_rotation();

		if input_manager.up_pressed {
			camera_pos = camera_pos + view_dir * delta_time * CAMERA_MOVE_RATE;
		}

		if input_manager.down_pressed {
			camera_pos = camera_pos - view_dir * delta_time * CAMERA_MOVE_RATE;
		}

		if input_manager.right_pressed {
			camera_pos = camera_pos + right_dir * delta_time * CAMERA_MOVE_RATE;
		}

		if input_manager.left_pressed {
			camera_pos = camera_pos - right_dir * delta_time * CAMERA_MOVE_RATE;
		}

		let radians = cgmath::Rad::from(cgmath::Deg(delta_time * CAMERA_ROTATION_RATE));
		if input_manager.left_arrow_pressed {
			let rot_quat = CgQuat::from_angle_y(radians);
			camera_rot = camera_rot * rot_quat;
		}
		if input_manager.right_arrow_pressed {
			let rot_quat = CgQuat::from_angle_y(-radians);
			camera_rot = camera_rot * rot_quat;
		}

		self.game_camera.set_position(&camera_pos);
		self.game_camera.set_rotation(&camera_rot);
		renderer.set_camera(&self.game_camera);
	}

}