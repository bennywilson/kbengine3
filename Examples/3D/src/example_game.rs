﻿use cgmath::InnerSpace;
use instant::Instant;

use kb_engine3::{kb_assets::*, kb_collision::*, kb_config::*, kb_engine::*, kb_input::*, kb_game_object::*, 
	kb_renderer::*, kb_resource::*, kb_utils::*, log};

use crate::game_actors::*;
use crate::game_actors::GamePlayerState;
use crate::game_vfx::*;

pub const CAMERA_MOVE_RATE: f32 = 10.0;
pub const CAMERA_ROTATION_RATE: f32 = 150.0;
pub const CROSSHAIR_ERROR_RATE: f32 = 10.0;

pub struct Example3DGame {
	player: Option<GamePlayer>,
	mobs: Vec<GameMob>,
	world_actors: Vec<KbActor>,
	props: Vec<GameProp>,

	game_objects: Vec<GameObject>,
	game_camera: KbCamera,

	collision_manager: KbCollisionManager,

	vfx_manager: GameVfxManager,

	barrel_model: Option<KbModelHandle>,
	shotgun_model: Option<KbModelHandle>,
	monster_model: Option<KbModelHandle>,

	monster_render_group: usize,
	monster_spawn_timer: Instant,

	barrel_spawn_timer: Instant,
	shotgun_spawn_timer: Instant,

	outline_render_group: usize,
	decal_render_group: usize,

	crosshair_error: f32,

	invert_y: bool,
	debug_collision: bool,
	pause_monsters: bool,

	score: i32,
	high_score: i32,
	next_harm_time: f32,
}

impl Example3DGame {
	fn spawn_monster(&mut self, renderer: &mut KbRenderer<'_>) {
		if self.pause_monsters {
			return;
		}

		if self.mobs.len() > 10 {
			return;
		}

		let pos = [
			CgVec3::new(10.0, 2.0, 10.0),
			CgVec3::new(-10.0, 2.0, 10.0),
			CgVec3::new(-10.0, 2.0, -10.0),
			CgVec3::new(10.0, 2.0, -10.0),
		];

		let monster_pos = pos[kb_random_u32(0, 3) as usize];
		let mut monster = GameMob::new(&monster_pos, &mut self.monster_model.as_ref().unwrap(), &mut self.collision_manager);
		let monster_actors = monster.get_actors();
		monster_actors[0].set_render_group(&KbRenderGroupType::WorldCustom, &Some(self.monster_render_group));
		renderer.add_or_update_actor(&monster_actors[0]);

		monster_actors[1].set_render_group(&KbRenderGroupType::WorldCustom, &Some(self.outline_render_group));

		#[cfg(not(target_arch = "wasm32"))]
		monster_actors[1].set_custom_data_1(CgVec4::new(0.01, 3.0, 3.0, 3.0));

		#[cfg(target_arch = "wasm32")]
		monster_actors[1].set_custom_data_1(CgVec4::new(0.01, 7.0, 7.0, 7.0));
		
		renderer.add_or_update_actor(&monster_actors[1]);

		self.mobs.push(monster);
	}

	fn spawn_barrel(&mut self, renderer: &mut KbRenderer<'_>) {
		let pos = [
			CgVec3::new(0.0, 0.0, 0.0),
		];
		let barrel_pos = pos[0];//kb_random_u32(0, 3) as usize];
		let smoke_pos = barrel_pos + CgVec3::new(0.0, 3.5, 0.0);

		// Smoke
		let (smoke_handle_1, smoke_handle_2) = self.vfx_manager.spawn_barrel_smoke(&smoke_pos, renderer);

		let mut barrel = GameProp::new(&GamePropType::Barrel, &barrel_pos, self.barrel_model.as_ref().unwrap(), &mut self.collision_manager, [smoke_handle_1, smoke_handle_2]);
		let barrel_actors = barrel.get_actors();
		barrel_actors[1].set_render_group(&KbRenderGroupType::WorldCustom, &Some(self.outline_render_group));

		#[cfg(not(target_arch = "wasm32"))]
		barrel_actors[1].set_custom_data_1(CgVec4::new(0.07, 0.1, 0.1, 0.1));

		#[cfg(target_arch = "wasm32")]
		barrel_actors[1].set_custom_data_1(CgVec4::new(0.07, 0.351, 00.351, 0.351));

		for actor in barrel_actors {
			renderer.add_or_update_actor(&actor);
		}
		self.props.push(barrel);
	}

	fn spawn_shotgun(&mut self, renderer: &mut KbRenderer<'_>) {
		let pos = [
			CgVec3::new(9.0, 0.0, -4.0),
		];
		let shotgun_pos = pos[0];//kb_random_u32(0, 3) as usize];

		let mut shotgun = GameProp::new(&GamePropType::Shotgun, &shotgun_pos, self.shotgun_model.as_ref().unwrap(), &mut self.collision_manager, [INVALID_PARTICLE_HANDLE, INVALID_PARTICLE_HANDLE]);
		let shotgun_actors = shotgun.get_actors();
		shotgun_actors[1].set_render_group(&KbRenderGroupType::WorldCustom, &Some(self.outline_render_group));

		#[cfg(not(target_arch = "wasm32"))]
		shotgun_actors[1].set_custom_data_1(CgVec4::new(0.07, 0.1, 0.1, 0.1)); 
		
		#[cfg(target_arch = "wasm32")]
		shotgun_actors[1].set_custom_data_1(CgVec4::new(0.07, 0.351, 0.351, 0.351));

		for actor in shotgun_actors {
			renderer.add_or_update_actor(&actor);
		}
		self.props.push(shotgun);
	}
}

impl KbGameEngine for Example3DGame {
	fn new(_game_config: &KbConfig) -> Self {
		log!("GameEngine::new() caled...");
		let game_objects = Vec::<GameObject>::new();

		let mut game_camera = KbCamera::new();
		game_camera.set_position(&CgVec3::new(0.0, 3.5, -5.0));
	
		Self {
			world_actors: Vec::<KbActor>::new(),
			mobs: Vec::<GameMob>::new(),
			props: Vec::<GameProp>::new(),
			game_objects,
			game_camera,
			vfx_manager: GameVfxManager::new(),
			barrel_model: None,
			shotgun_model: None,
			monster_model: None,
			monster_render_group: usize::MAX,
			monster_spawn_timer: Instant::now(),
			shotgun_spawn_timer: Instant::now(),
			barrel_spawn_timer: Instant::now(),
			outline_render_group: usize::MAX,
			decal_render_group: usize::MAX,
			player: None,
			crosshair_error: 0.0,
			collision_manager: KbCollisionManager::new(),
			debug_collision: false,
			invert_y: false,
			pause_monsters: false,
			score: 0,
			high_score: 0,
			next_harm_time: -1.0,
		}
    }

	async fn initialize_world(&mut self, renderer: &mut KbRenderer<'_>, game_config: &mut KbConfig) {
		log!("GameEngine::initialize_world() caled...");

		#[cfg(not(target_arch = "wasm32"))]
		{
			game_config.clear_color = CgVec4::new(0.87, 0.58, 0.24, 0.0);
			game_config.sun_color = CgVec4::new(0.8 * 0.8, 0.58 * 0.58, 0.24 * 0.24, 0.0);
		}

		#[cfg(target_arch = "wasm32")]
		{
			use cgmath::num_traits::Pow;
			let color_fix: f32 = 1.0 / 2.2;
			game_config.clear_color = CgVec4::new(0.87_f32.pow(color_fix), 0.58_f32.pow(color_fix), 0.24_f32.pow(color_fix), 0.0);
			game_config.sun_color = CgVec4::new(0.8, 0.58, 0.24, 0.0);		
		}

		// self.game_objects order is hard-coded.  Indexes 0-3 contain the cross hair
		let positions = [
			CgVec3::new(0.0, 0.5, 0.0),
			CgVec3::new(0.0, 0.3, 0.0),
			CgVec3::new(0.1, 0.4, 0.0),
			CgVec3::new(-0.1, 0.4, 0.0)
		];
		let sprites = [40, 40, 41, 41];
		let scale = CgVec3::new(0.035, 0.035, 1.0);
		for i in 0..4 {
			self.game_objects.push(GameObject { 
				position: positions[i],
				scale,
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (0.0, 0.0, 0.0).into(),
				object_type: GameObjectType::Background,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 1,
				sprite_index: sprites[i],
				anim_frame: 0,
				life_start_time: Instant::now(),
				state_start_time: Instant::now(),
				gravity_scale: 0.0,
				random_val: kb_random_f32(0.0, 1000.0),
				is_enemy: false
			});
		}

		renderer.set_debug_game_msg("Move: [W][A][S][D]   Look: [Arrow Keys]   Shoot: [Space]     Invert Y: [Y]   Toggle collision: [i]   Pause monsters: [M] ");
		renderer.set_debug_font_color(&CgVec4::new(1.0, 0.0, 0.0, 1.0));

		self.barrel_model = Some(renderer.load_model("game_assets/models/barrel.glb").await);
		self.shotgun_model = Some(renderer.load_model("game_assets/models/shotgun.glb").await);

		self.decal_render_group = renderer.add_custom_render_group(&KbRenderGroupType::WorldCustom, &KbBlendMode::Additive, "engine_assets/shaders/decal.wgsl").await;

		// First person set up
		let fp_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::ForegroundCustom, &KbBlendMode::None, "game_assets/shaders/first_person.wgsl").await);
		let fp_outline_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::ForegroundCustom, &KbBlendMode::Alpha, "game_assets/shaders/first_person_outline.wgsl").await);
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

		// Monster
		let monster_model = renderer.load_model("game_assets/models/monster.glb").await;
		let monster_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::WorldCustom, &KbBlendMode::Additive, "game_assets/shaders/monster.wgsl").await);
		self.monster_render_group = monster_render_group.unwrap();
		self.monster_model = Some(monster_model);

		// World objects
		let level_model = renderer.load_model("game_assets/models/level.glb").await;
		let mut actor = KbActor::new();
		actor.set_position(&[0.0, 0.0, 0.0].into());
		actor.set_scale(&[10.0, 19.0, 10.0].into());
		actor.set_model(&level_model);
		renderer.add_or_update_actor(&actor);
		self.world_actors.push(actor);

		let sky_model = renderer.load_model("game_assets/models/sky_dome.glb").await;
		{
			let sky_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::WorldCustom, &KbBlendMode::Alpha, "engine_assets/shaders/sky_dome_occlude.wgsl").await);
			let mut actor = KbActor::new();
			actor.set_position(&[0.0, 0.0, 0.0].into());
			actor.set_scale(&[30.0, 30.0, 30.0].into());
			actor.set_model(&sky_model);
			actor.set_render_group(&KbRenderGroupType::WorldCustom, &sky_render_group);
			renderer.add_or_update_actor(&actor);
			self.world_actors.push(actor);
		}
		{
			let sky_render_group = Some(renderer.add_custom_render_group(&KbRenderGroupType::WorldCustom, &KbBlendMode::Alpha, "engine_assets/shaders/sky_dome_draw.wgsl").await);
			let mut actor = KbActor::new();
			actor.set_position(&[0.0, 0.0, 0.0].into());
			actor.set_scale(&[30.0, 30.0, 30.0].into());
			actor.set_model(&sky_model);
			actor.set_render_group(&KbRenderGroupType::WorldCustom, &sky_render_group);
			renderer.add_or_update_actor(&actor);
			self.world_actors.push(actor);
		}

		self.outline_render_group = renderer.add_custom_render_group(&KbRenderGroupType::WorldCustom, &KbBlendMode::Alpha, "game_assets/shaders/first_person_outline.wgsl").await;
		let pinky_model = renderer.load_model("game_assets/models/pinky.glb").await;
		let mut actor = KbActor::new();
		actor.set_position(&[16.5, 0.5, 6.0].into());
		let pinky_rot_x = cgmath::Rad::from(cgmath::Deg(90.0)); 
		let pinky_rot_z = cgmath::Rad::from(cgmath::Deg(115.0)); 
		let pinky_rot = cgmath::Quaternion::from(CgMat3::from_angle_z(pinky_rot_z) * CgMat3::from_angle_x(pinky_rot_x));
		actor.set_rotation(&pinky_rot);
		actor.set_scale(&[1.0, 1.0, 1.0].into());
		actor.set_model(&pinky_model);
		renderer.add_or_update_actor(&actor);
		self.world_actors.push(actor);

		let mut actor = KbActor::new();
		actor.set_position(&[16.5, 0.5, 6.0].into());
		actor.set_rotation(&pinky_rot);
		actor.set_scale(&[1.0, 1.0, 1.0].into());
		actor.set_model(&pinky_model);
		 
		#[cfg(not(target_arch = "wasm32"))]
		actor.set_custom_data_1(CgVec4::new(0.05, 0.1, 0.1, 0.1));

		#[cfg(target_arch = "wasm32")]
		actor.set_custom_data_1(CgVec4::new(0.05, 0.351, 00.351, 0.351));

		actor.set_render_group(&KbRenderGroupType::WorldCustom, &Some(self.outline_render_group));
		renderer.add_or_update_actor(&actor);
		self.world_actors.push(actor);

		// World Collision
		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: CgVec3::new(0.0, 2.4, 20.0),
			extents: CgVec3::new(20.0, 10.0, 2.0),
			block: true,
		});
		let _ = self.collision_manager.add_collision(&collision_box);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: CgVec3::new(0.0, 2.4, -20.0),
			extents: CgVec3::new(-20.0, 10.0, 2.0),
			block: true,
		});
		let _ = self.collision_manager.add_collision(&collision_box);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: CgVec3::new(20.0, 2.4, 0.0),
			extents: CgVec3::new(2.0, 10.0, 20.0),
			block: true,
		});
		let _ = self.collision_manager.add_collision(&collision_box);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: CgVec3::new(-20.0, 2.4, 0.0),
			extents: CgVec3::new(2.0, 10.0, 20.0),
			block: true,
		});
		let _ = self.collision_manager.add_collision(&collision_box);

		let collision_box = KbCollisionShape::AABB(KbCollisionAABB {
			position: CgVec3::new(0.0, -0.5, 0.0),
			extents: CgVec3::new(20.0, 0.0, 20.0),
			block: true,
		});
		let _ = self.collision_manager.add_collision(&collision_box);

		// Trans Flag
		let sun_color = game_config.sun_color;
		let trans_colors = [
			CgVec4::new(0.356 * sun_color.x, 0.807 * sun_color.y, 0.980 * sun_color.z, 1.0),
			CgVec4::new(0.96 * sun_color.x, 0.66 * sun_color.y, 0.72 * sun_color.z, 1.0),
			CgVec4::new(1.0 * sun_color.x, 1.0 * sun_color.y, 1.0 * sun_color.z, 1.0),
		];

		renderer.add_line(&CgVec3::new(5.0, 6.5, 17.4), &CgVec3::new(10.0, 6.5, 17.4), &trans_colors[0], 0.25, 5535.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 6.0, 17.4), &CgVec3::new(10.0, 6.0, 17.4), &trans_colors[1], 0.25, 5535.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 5.5, 17.4), &CgVec3::new(10.0, 5.5, 17.4), &trans_colors[2], 0.25, 5535.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 5.0, 17.4), &CgVec3::new(10.0, 5.0, 17.4), &trans_colors[1], 0.25, 5535.0, &game_config);
		renderer.add_line(&CgVec3::new(5.0, 4.5, 17.4), &CgVec3::new(10.0, 4.5, 17.4), &trans_colors[0], 0.25, 5535.0, &game_config);

		self.vfx_manager.init(renderer).await;

		self.spawn_shotgun(renderer);
		self.spawn_barrel(renderer);
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
		let camera_pos = self.game_camera.get_position();
		let mut camera_rot = self.game_camera.get_rotation();

		// Movement
		let mut move_vec = CG_VEC3_ZERO;
		if input_manager.up_pressed {
			move_vec += forward_dir
		}

		if input_manager.down_pressed {
			move_vec += -forward_dir;
		}

		if input_manager.right_pressed {
			move_vec += right_dir;
		}

		if input_manager.left_pressed {
			move_vec += -right_dir;
		}

		move_vec = move_vec.normalize() * delta_time * CAMERA_MOVE_RATE;
		if move_vec.magnitude2() > 0.001 {
			let trace_start = CgVec3::new(camera_pos.x, 0.25, camera_pos.z);
			let (t, handle, _, _) = self.collision_manager.cast_ray(&trace_start, &move_vec);
			if t >= 0.0 && t < 1.0 {
				self.props.retain_mut(|prop| {
					if prop.get_prop_type() == GamePropType::Shotgun && prop.get_collision_handle() == *handle.as_ref().unwrap() {
						prop.take_damage(&mut self.collision_manager, renderer);
						self.player.as_mut().unwrap().give_shotgun(&self.shotgun_model.as_ref().unwrap());
						return false;
					}
					true
				});
			}
			let mut final_pos = camera_pos + move_vec;
			final_pos.x = final_pos.x.clamp(-17.0, 17.0);
			final_pos.z = final_pos.z.clamp(-17.0, 17.0);

			self.game_camera.set_position(&final_pos);

			self.crosshair_error = (self.crosshair_error + delta_time * CROSSHAIR_ERROR_RATE).clamp(0.0, 1.0);
		}
		else {
			self.crosshair_error = (self.crosshair_error - delta_time * CROSSHAIR_ERROR_RATE).clamp(0.0, 1.0);
		}

		let x_radians = delta_time * CAMERA_ROTATION_RATE;
		let y_radians = if self.invert_y { -delta_time * CAMERA_ROTATION_RATE } else { delta_time * CAMERA_ROTATION_RATE };
		if input_manager.left_arrow_pressed {
			camera_rot.x += x_radians;
		}
		if input_manager.right_arrow_pressed {
			camera_rot.x -= x_radians;
		}

		if input_manager.up_arrow_pressed {
			camera_rot.y -= y_radians;
		}
		if input_manager.down_arrow_pressed {
			camera_rot.y += y_radians
		}

		self.game_camera.set_rotation(&camera_rot);
		renderer.set_camera(&self.game_camera);

		let player = &mut self.player.as_mut().unwrap();
		let has_shotgun = player.has_shotgun();
		let (cur_state, next_state) = player.tick(&input_manager, &self.game_camera, &game_config);
		let (hands, hands_outline) = player.get_actors();
		renderer.add_or_update_actor(&hands);
		for outline in hands_outline {
			renderer.add_or_update_actor(&outline);
		}

		let (_, view_dir, right_dir) = self.game_camera.calculate_view_matrix();
		let start = {
			if has_shotgun {
				hands.get_position() + view_dir * 3.0 + right_dir * 0.5 + CgVec3::new(0.0, 0.75, 0.0)
			} else {
				hands.get_position() + view_dir * 1.5 + right_dir * 0.5 + CgVec3::new(0.0, 0.5, 0.0)
			}
		};

		self.vfx_manager.tick(&start, renderer, game_config);

		if cur_state != GamePlayerState::Shooting && next_state == GamePlayerState::Shooting {
			let (_, view_dir, right_dir) = self.game_camera.calculate_view_matrix();
			let start = hands.get_position() + view_dir * 1.5 + right_dir * 0.5 + CgVec3::new(0.0, 0.5, 0.0);
			let num_shots = if self.player.as_ref().unwrap().has_shotgun() == true { 8 } else { 1 };

			// Muzzle Flash
			let scale = if has_shotgun { CgVec3::new(2.0, 2.0, 2.0) } else { CgVec3::new(1.0, 1.0, 1.0) };
			self.vfx_manager.spawn_muzzle_flash(&start, &scale, renderer);

			for i in 0..num_shots {
				let mut end = self.game_camera.get_position() + view_dir * 1000.0;
				if i > 0 {
					end += kb_random_vec3(CgVec3::new(-1.0, -1.0, -1.0,), CgVec3::new(1.0, 1.0, 1.0));
				}

				let (hit_t, handle, hit_loc, _) = self.collision_manager.cast_ray(&start, &end);
				let found_hit = hit_t >= 0.0 && hit_t < 1.0;
				let mut mob_killed = false;

				let color = if found_hit { CgVec4::new(1.0, 0.0, 0.0, 1.0) } else { CgVec4::new(0.0, 0.0, 1.0, 1.0) };
				if found_hit {
					let hit_loc = hit_loc.unwrap();
					self.mobs.retain_mut(|mob| {
						if *mob.get_collision_handle() == *handle.as_ref().unwrap() {
							mob_killed = mob.take_damage(&mut self.collision_manager, renderer);
							self.score = self.score + 1;

							let mob_pos = mob.get_actors()[0].get_position();
							self.vfx_manager.spawn_mob_death_fx(&mob_pos, &view_dir, renderer, &mut self.collision_manager, game_config);

							!mob_killed
						} else {
							true
						}
					});

					let mut barrel_exploded = false;
					let mut explode_pos = CG_VEC3_ZERO;
					if mob_killed == false {
						self.props.retain_mut(|prop| {
							if prop.get_prop_type() == GamePropType::Barrel && prop.get_collision_handle() == *handle.as_ref().unwrap() {
								explode_pos = prop.get_actors()[0].get_position();

								prop.take_damage(&mut self.collision_manager, renderer);

								// Barrel explosion
								self.vfx_manager.spawn_explosion(&explode_pos, renderer);
								barrel_exploded = true;
								return false
							}
							return true;
						});
					};

					// Radius Damage
					if barrel_exploded {
						explode_pos.y = 0.0;
						self.mobs.retain_mut(|mob| {
							let mut mob_pos = mob.get_actors()[0].get_position();
							mob_pos.y = 0.0;
							let magnitude = (mob_pos - explode_pos).magnitude();
							if magnitude < 15.0 {
								mob.take_damage(&mut self.collision_manager, renderer);
								self.score = self.score + 1;

								self.vfx_manager.spawn_mob_death_fx(&mob_pos, &view_dir, renderer, &mut self.collision_manager, game_config);
								return false
							}

							return true
						});
					}

					if mob_killed == false {
						// Hit a wall, spawn impact
						self.vfx_manager.spawn_impact(&hit_loc, renderer);
					}
				}

				if self.debug_collision {
					renderer.add_line(&start, &end, &color, 0.05, 0.33, &game_config);
				}
			}
		}

		// Tick monster
		if self.pause_monsters == false {
			let monster_iter = self.mobs.iter_mut();
			for monster in monster_iter {
				monster.tick(camera_pos, &mut self.collision_manager, &game_config);
				renderer.add_or_update_actor(&monster.get_actors()[0]);
				renderer.add_or_update_actor(&monster.get_actors()[1]);
			}
		}

		if self.monster_spawn_timer.elapsed().as_secs_f32() > 2.0 {
			self.monster_spawn_timer = Instant::now();
			self.spawn_monster(renderer);
		}
		
		if self.shotgun_spawn_timer.elapsed().as_secs_f32() > 20.0 {
			if self.props.iter().filter(|&p| p.get_prop_type() == GamePropType::Shotgun).count() == 0 {
				self.shotgun_spawn_timer = Instant::now();
				self.spawn_shotgun(renderer);
			}
		}

		if self.barrel_spawn_timer.elapsed().as_secs_f32() > 20.0 {
			if self.props.iter().filter(|&p| p.get_prop_type() == GamePropType::Barrel).count() == 0 {
				self.barrel_spawn_timer = Instant::now();
				self.spawn_barrel(renderer);
			}
		}

		let mut num_attacking = 0;
		for monster in &self.mobs {
			if monster.get_state() == GameMobState::Attacking {
				num_attacking = num_attacking + 1;
			}
		}

		if num_attacking > 0 {
			let elapsed_time = game_config.start_time.elapsed().as_secs_f32();
			if self.next_harm_time < 0.0 {
				self.next_harm_time = elapsed_time + 1.0;
			} else {
				renderer.set_postprocess_mode(&KbPostProcessMode::ScanLines);
				if elapsed_time > self.next_harm_time {
					self.next_harm_time = elapsed_time + 1.0;
					self.score = (self.score - 1).max(0);
				}
			}
		} else {
			renderer.set_postprocess_mode(&KbPostProcessMode::Passthrough);
		}



		// UI
		{
			self.high_score = self.high_score.max(self.score);
			let hud_msg = format!("Score: {}  High Score: {}", self.score, self.high_score);
			renderer.set_hud_msg(&hud_msg);
			let player = self.player.as_ref().unwrap();
			let (positions, sprites, scale) = {
				if player.has_shotgun() == false {
					([
						CgVec3::new(0.0, 0.5, 0.0),
						CgVec3::new(0.0, 0.3, 0.0),
						CgVec3::new(0.1, 0.4, 0.0),
						CgVec3::new(-0.1, 0.4, 0.0)
					],
					[40, 40, 41, 41],
					CgVec3::new(0.035, 0.035, 1.0))
				} else {
					([
						CgVec3::new(-0.11, 0.55, 0.0),
						CgVec3::new(0.11, 0.55, 0.0),
						CgVec3::new(-0.11, 0.35, 0.0),
						CgVec3::new(0.11, 0.35, 0.0)
					],
					[48, 49, 56, 57],
					CgVec3::new(0.065, 0.065, 0.065))
				}
			};

			let center = (positions[0] + positions[1] + positions[2] + positions[3]) * 0.25;
			for i in 0..4 {
				self.game_objects[i].sprite_index = sprites[i];
				self.game_objects[i].position = positions[i] + (positions[i] - center).normalize() * self.crosshair_error * 0.1;
				self.game_objects[i].scale = scale;
			}
			self.game_objects.truncate(4);

			let ammo_count = player.get_ammo_count();
			let mut position = CgVec3::new(-1.7, -0.45, 0.0);
			let scale = CgVec3::new(0.1, 0.1, 0.1);
			let sprite_index = if player.has_shotgun() { 50 } else { 42 };

			for _ in 0..ammo_count {
				self.game_objects.push(
					GameObject{
						position,
						scale,
						direction: (1.0, 0.0, 0.0).into(),
						velocity: (0.0, 0.0, 0.0).into(),
						object_type: GameObjectType::Background,
						object_state: GameObjectState::Idle,
						next_attack_time: 0.0,
						texture_index: 1,
						sprite_index,
						anim_frame: 0,
						life_start_time: Instant::now(),
						state_start_time: Instant::now(),
						gravity_scale: 0.0,
						random_val: kb_random_f32(0.0, 1000.0),
						is_enemy: false
					}
				);
				position.x += 0.08;
			}
		}

		// Debug
		if input_manager.key_i() == KbButtonState::JustPressed {
			self.debug_collision = !self.debug_collision;
		}
		
		if input_manager.key_y() == KbButtonState::JustPressed {
			self.invert_y = !self.invert_y;
		}
		  
		if input_manager.key_m() == KbButtonState::JustPressed {
			self.pause_monsters = !self.pause_monsters;
		}

		if self.debug_collision {
			self.collision_manager.debug_draw(renderer, &game_config);
		}
	}
}