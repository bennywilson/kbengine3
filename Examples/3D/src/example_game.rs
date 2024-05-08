use cgmath::InnerSpace;
use instant::Instant;

use kb_engine3::{kb_collision::*, kb_config::*, kb_engine::*, kb_input::*, kb_game_object::*, kb_renderer::*, 
	kb_resource::*, kb_utils::*, log};

use crate::game_actors::*;
use crate::game_actors::GamePlayerState;

pub const CAMERA_MOVE_RATE: f32 = 10.0;
pub const CAMERA_ROTATION_RATE: f32 = 100.0;

pub struct Example3DGame {
	player: Option<GamePlayer>,
	actors: Vec<KbActor>,
	game_objects: Vec<GameObject>,
	game_camera: KbCamera,

	collision_manager: KbCollisionManager,
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
			game_camera,
			player: None,
			collision_manager: KbCollisionManager::new(),
		}
    }

	async fn initialize_world(&mut self, renderer: &mut KbRenderer<'_>, game_config: &KbConfig) {
		log!("GameEngine::initialize_world() caled...");

		let pinky_model = renderer.load_model("game_assets/models/pinky.glb").await;
		let barrel_model = renderer.load_model("game_assets/models/barrel.glb").await;
		let shotgun_model = renderer.load_model("game_assets/models/shotgun.glb").await;
		let floor_model = renderer.load_model("game_assets/models/floor.glb").await;

		// First person set up
		let fp_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::ForegroundCustom, true, "game_assets/shaders/first_person.wgsl").await);
		let fp_outline_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::ForegroundCustom, false, "game_assets/shaders/first_person_outline.wgsl").await);
		let hands_model = renderer.load_model("game_assets/models/fp_hands.glb").await;
		let mut player = GamePlayer::new(&hands_model).await;

		let (hands, hands_outlines) = player.get_actors();
		hands.set_render_group(&KbRenderGroupType::ForegroundCustom, &fp_render_group);
		renderer.add_or_update_actor(&hands);

		for outline in hands_outlines {
			outline.set_render_group(&KbRenderGroupType::ForegroundCustom, &fp_outline_render_group);
			renderer.add_or_update_actor(&outline);
		}
		self.player = Some(player);

		// World objects
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
		actor.set_position(&[9.0, 0.0, -13.0].into());
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

		let particle_params = KbParticleParams {
			texture_file: "/game_assets/fx/smoke_t.png".to_string(),
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
			texture_file: "./game_assets/fx/ember_t.png".to_string(),
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

		// DEBUG
		renderer.add_line(&CgVec3::new(5.0, 2.5, 5.0), &CgVec3::new(10.0, 2.5, 5.0), &CgVec4::new(0.356, 0.807, 0.980, 1.0), 0.25, 35.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 2.0, 5.0), &CgVec3::new(10.0, 2.0, 5.0), &CgVec4::new(0.96, 0.66, 0.72, 1.0), 0.25, 35.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 1.5, 5.0), &CgVec3::new(10.0, 1.5, 5.0), &CgVec4::new(1.0, 1.0, 1.0, 1.0), 0.25, 35.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 1.0, 5.0), &CgVec3::new(10.0, 1.0, 5.0), &CgVec4::new(0.96, 0.66, 0.72, 1.0), 0.25, 35.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 0.5, 5.0), &CgVec3::new(10.0, 0.5, 5.0), &CgVec4::new(0.356, 0.807, 0.980, 1.0), 0.25, 35.0, &game_config);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: CgVec3::new(-8.0, 2.5, 5.0),
			extents: CgVec3::new(2.0, 2.0, 2.0)
		});
		self.collision_manager.add_collision(&collision_box);
    }

	fn get_game_objects(&self) -> &Vec<GameObject> {
		&self.game_objects
	}

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &KbInputManager, game_config: &KbConfig) {
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
		renderer.set_camera(&self.game_camera);

		let player = &mut self.player.as_mut().unwrap();
		let (cur_state, next_state) = player.tick(&input_manager, &self.game_camera, &game_config);
		let (hands, hands_outline) = player.get_actors();
		renderer.add_or_update_actor(&hands);
		for outline in hands_outline {
			renderer.add_or_update_actor(&outline);
		}

		if cur_state != GamePlayerState::Shooting && next_state == GamePlayerState::Shooting {
			let (_, view_dir, right_dir) = self.game_camera.calculate_view_matrix();
			let start = hands.get_position() + view_dir * 1.5 + right_dir * 0.5 + CgVec3::new(0.0, 0.5, 0.0);
			let end = self.game_camera.get_position() + view_dir * 1000.0;

			let (hit, handle) = self.collision_manager.cast_ray(&start, &end);
			let color = if hit { CgVec4::new(1.0, 0.0, 0.0, 1.0) } else { CgVec4::new(0.0, 0.0, 1.0, 1.0) };
			if hit {
				self.collision_manager.remove_collision(&handle.unwrap());
				let box_positions = [CgVec3::new(-12.0, 2.5, 10.0), CgVec3::new(-9.0, 2.5, 7.0), CgVec3::new(-6.0, 2.5, 4.0)];
				let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
					position: box_positions[kb_random_u32(0, 2) as usize],
					extents: CgVec3::new(2.0, 2.0, 2.0)
				});
				self.collision_manager.add_collision(&collision_box);
			}

			renderer.add_line(&start, &end, &color, 0.20, 1.0, &game_config);	
		}

		self.collision_manager.debug_draw(renderer, &game_config);
	}
}