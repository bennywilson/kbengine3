 use cgmath::{InnerSpace, SquareMatrix};
use instant::Instant;

use kb_engine3::{kb_config::*, kb_engine::*, kb_input::*, kb_game_object::*, kb_renderer::*, kb_utils::*, log};

pub const CAMERA_MOVE_RATE: f32 = 10.0;
pub const CAMERA_ROTATION_RATE: f32 = 100.0;

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
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: true
		});

		let mut game_camera = KbCamera::new();
		game_camera.set_position(&CgVec3::new(0.0, 2.0, -5.0));
	
		Self {
			actors: Vec::<KbActor>::new(),
			game_objects,
			game_camera
		}
    }

	async fn initialize_world(&mut self, renderer: &mut KbRenderer<'_>) {
		log!("GameEngine::initialize_world() caled...");
		let pinky_model = renderer.load_model("game_assets/pinky.glb").await;
		let barrel_model = renderer.load_model("game_assets/barrel.glb").await;
		let shotgun_model = renderer.load_model("game_assets/shotgun.glb").await;
		let floor_model = renderer.load_model("game_assets/floor.glb").await;
		let hands_model = renderer.load_model("game_assets/fp_hands.glb").await;
		
		let mut actor = KbActor::new();
		actor.set_position(&[5.0, 1.0, 3.0].into());
		actor.set_scale(&[1.0, 1.0, 1.2].into());
		actor.set_model(&hands_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[0]);

		let mut actor = KbActor::new();
		actor.set_position(&[3.0, 0.0, 3.0].into());
		actor.set_scale(&[1.0, 1.0, 1.0].into());
		actor.set_model(&pinky_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[1]);

		let mut actor = KbActor::new();
		actor.set_position(&[0.0, 0.0, 0.0].into());
		actor.set_scale(&[1.0, 1.0, 1.0].into());
		actor.set_model(&barrel_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[2]);

		let mut actor = KbActor::new();
		actor.set_position(&[-4.0, 0.0, -5.0].into());
		actor.set_scale(&[2.0, 2.0, 2.0].into());
		actor.set_model(&shotgun_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[3]);

		let mut actor = KbActor::new();
		actor.set_position(&[0.0, 0.0, 0.0].into());
		actor.set_scale(&[10.0, 19.0, 10.0].into());
		actor.set_model(&floor_model);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[4]);

		let particle_params = KbParticleParams {
			texture_file: "/game_assets/smoke_t.png".to_string(),
			blend_mode: KbParticleBlendMode::AlphaBlend,

			min_particle_life: 3.0,
			max_particle_life: 5.0,

			_min_actor_life: 5.1,
			_max_actor_life: 5.1,

			min_start_spawn_rate: 0.06,
			max_start_spawn_rate: 0.06,

			min_start_pos: CgVec3::new(-0.5, -0.2, -0.2),
			max_start_pos: CgVec3::new(0.5, 0.2, 0.2),

			min_start_scale: CgVec3::new(0.5, 0.5, 0.5),
			max_start_scale: CgVec3::new(0.8, 0.8, 0.8),

			min_end_scale: CgVec3::new(2.1, 2.1, 2.1),
			max_end_scale: CgVec3::new(3.0, 3.0, 3.0),

			min_start_velocity: CgVec3::new(-0.2, 1.0, -0.2),
			max_start_velocity: CgVec3::new(0.2, 1.0, 0.2),

			min_start_rotation_rate: -0.5,
			max_start_rotation_rate: 0.5,

			min_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(0.4, 0.04, 0.0, 1.0),
			start_color_1: CgVec4::new(0.4, 0.07, 0.0, 1.0),

			end_color_0: CgVec4::new(-0.5, -0.5, -0.5, 0.0),
			_end_color1: CgVec4::new(-0.5, -0.5, -0.5, 1.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(0.0, 3.5, 0.0));
		let _ = renderer.add_particle_actor(&particle_transform, &particle_params).await;

		let particle_params = KbParticleParams {
			texture_file: "./game_assets/ember_t.png".to_string(),
			blend_mode: KbParticleBlendMode::Additive,

			min_particle_life: 1.5,
			max_particle_life: 2.5,

			_min_actor_life: 5.1,
			_max_actor_life: 5.1,

			min_start_spawn_rate: 0.3,
			max_start_spawn_rate: 0.3,

			min_start_pos: CgVec3::new(-0.75, -0.2, -0.75),
			max_start_pos: CgVec3::new(0.75, 0.2, 0.75),
    
			min_start_scale: CgVec3::new(0.3, 0.3, 0.3),
			max_start_scale: CgVec3::new(0.5, 0.5, 0.5),

			min_end_scale: CgVec3::new(0.0, 0.0, 0.0),
			max_end_scale: CgVec3::new(0.05, 0.05, 0.05),

			min_start_velocity: CgVec3::new(-0.2, 3.0, -0.2),
			max_start_velocity: CgVec3::new(0.2, 3.0, 0.2),

			min_start_rotation_rate: -15.5,
			max_start_rotation_rate: 15.5,

			min_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),
			max_start_acceleration: CgVec3::new(0.0, -0.1, 0.0),

			min_end_velocity: CgVec3::new(0.0, 0.0, 0.0),
			max_end_velocity: CgVec3::new(0.0, 0.0, 0.0),

			start_color_0: CgVec4::new(2.0, 1.0, 0.2, 1.0),
			start_color_1: CgVec4::new(2.0, 1.0, 0.2, 1.0),

			end_color_0: CgVec4::new(1.0, 0.8, -0.1, 0.0),
			_end_color1: CgVec4::new(1.0, 0.8, -0.1, 1.0),
		};
		let particle_transform = KbActorTransform::from_position(CgVec3::new(0.0, 3.5, 0.0));
		let _ = renderer.add_particle_actor(&particle_transform, &particle_params).await;

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
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});
    }

	fn get_game_objects(&self) -> &Vec<GameObject> {
		&self.game_objects
	}

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &InputManager, game_config: &KbConfig) {
		for game_object in &mut self.game_objects {
			game_object.update(game_config.delta_time);
		}
		let delta_time = game_config.delta_time;
		let (_s, view_dir, right_dir) = self.game_camera.calculate_view_matrix();
		let forward_dir = CgVec3::new(view_dir.x, 0.0, view_dir.z).normalize();
		let mut camera_pos = self.game_camera.get_position();
		let mut camera_rot = self.game_camera.get_rotation();

		if input_manager.up_pressed {
			camera_pos = camera_pos + forward_dir * delta_time * CAMERA_MOVE_RATE;
		}

		if input_manager.down_pressed {
			camera_pos = camera_pos - forward_dir * delta_time * CAMERA_MOVE_RATE;
		}

		if input_manager.right_pressed {
			camera_pos = camera_pos + right_dir * delta_time * CAMERA_MOVE_RATE;
		}

		if input_manager.left_pressed {
			camera_pos = camera_pos - right_dir * delta_time * CAMERA_MOVE_RATE;
		}

		let radians = delta_time * CAMERA_ROTATION_RATE;
		if input_manager.left_arrow_pressed {
			camera_rot.x += radians;
		}
		if input_manager.right_arrow_pressed {
			camera_rot.x -= radians;
		}

		if input_manager.up_arrow_pressed {
			camera_rot.y -= radians;
		}
		if input_manager.down_arrow_pressed {
			camera_rot.y += radians
		}

		self.game_camera.set_position(&camera_pos);
		self.game_camera.set_rotation(&camera_rot);
		let (view_matrix, view_dir, right_dir) = self.game_camera.calculate_view_matrix();
		let up_dir = view_dir.cross(right_dir).normalize();
		let gun_pos = camera_pos + (view_dir * 1.0) + (up_dir * 1.0) + (right_dir * 0.5);
		let view3 = CgMat3::new(view_matrix.x.x, view_matrix.x.y, view_matrix.x.z, view_matrix.y.x, view_matrix.y.y, view_matrix.y.z, view_matrix.z.x, view_matrix.z.y, view_matrix.z.z);
		let view3 = view3.invert().unwrap();

		
        let gun_fix_rad = cgmath::Rad::from(cgmath::Deg(90.0));
		let gun_fix_mat3 = CgMat3::from_angle_y(gun_fix_rad);
		let gun_rot: CgQuat = cgmath::Quaternion::from(view3 * gun_fix_mat3); 
		self.actors[0].set_position(&gun_pos);
		self.actors[0].set_rotation(&gun_rot);

		renderer.add_or_update_actor(&self.actors[0]);

		renderer.set_camera(&self.game_camera);
	}

}