use cgmath::{InnerSpace, Vector3};
use instant::Instant;

use kb_engine3::{log, kb_config::KbConfig, kb_engine::KbGameEngine, kb_input::KbInputManager, kb_game_object::{GameObject, GameObjectState, GameObjectType}, kb_renderer::KbRenderer};
use kb_engine3::kb_utils::*;

const SKY_Z:f32 = 0.0;
const SUN_Z:f32 = 15.0;
const CLOUD_Z:f32 = 20.0;
const HILL_Z:f32 = 30.0;
const BUILDING_Z:f32 = 50.0;
const CHARACTER_Z:f32 = 100.0;

pub struct Example2DGame {
	pub game_objects: Vec<GameObject>,
	game_start_time:  Instant,
	current_frame_time:  Instant,
	next_enemy_spawn_time: f32,
	num_enemies: u32,

	// data
	max_game_objects: usize, 
	enemy_spawn_delay: f32,
	enemy_speed: f32,
}

impl Example2DGame {
	fn update_enemies(&mut self) {
		if self.game_objects.len() >= self.max_game_objects {
			return
		}

		let game_time = self.game_start_time.elapsed().as_secs_f32();
		if game_time > self.next_enemy_spawn_time {
			self.next_enemy_spawn_time  = game_time + self.enemy_spawn_delay;
			self.num_enemies = self.num_enemies + 1;
			
			let mut start_x = 2.0;
			let mut vel_x = -self.enemy_speed;

			let randnum = kb_random_u32(1, 2);
		    if randnum == 2 {
				start_x = start_x * -1.0;
				vel_x = vel_x * -1.0;
			}
			let y_pos: f32 = kb_random_f32(0.0, 0.75) - 0.35;

			// Create Enemy
			self.game_objects.push(GameObject { 
				position: (start_x, y_pos, CHARACTER_Z).into(),
				scale: (0.1, 0.15, 0.15).into(),
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (vel_x, 0.0, 0.0).into(),
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
		}
	}

	fn update_projectiles(&mut self) {
		let mut i = 0;
		while i < self.game_objects.len() {
			if !matches!(self.game_objects[i].object_type, GameObjectType::Projectile) {
				i = i + 1;
				continue;
			}

			let mut j = 0;
			while j < self.game_objects.len() {

				// Don't hit other projectiles
				if i == j || matches!(self.game_objects[j].object_type, GameObjectType::Projectile) {
					j = j + 1;
					continue;
				}

				// Allegiance test
				if self.game_objects[i].is_enemy == self.game_objects[j].is_enemy {
					j = j + 1;
					continue;
				}

				let dist = cgmath::Vector2::<f32>::new(self.game_objects[i].position.x - self.game_objects[j].position.x, self.game_objects[i].position.y - self.game_objects[j].position.y).magnitude2();
				if dist < 0.05 {
					if i > j {
						self.game_objects.remove(i);
						self.game_objects.remove(j);
					} else {
						self.game_objects.remove(j);
						self.game_objects.remove(i);
					}
					break;
				}
				j = j + 1;

			}

			i = i + 1;
		}
	}
}

impl KbGameEngine for Example2DGame {
	fn new(game_config: &KbConfig) -> Self {
		log!("GameEngine::new() caled...");

		let cur_time = Instant::now();

		Self {
			game_objects: Vec::<GameObject>::new(),
			game_start_time:  cur_time,
			current_frame_time : cur_time,
			next_enemy_spawn_time: cur_time.elapsed().as_secs_f32() + game_config.enemy_spawn_delay,
			num_enemies: 0,

			max_game_objects: game_config.max_render_instances as usize,
			enemy_spawn_delay: game_config.enemy_spawn_delay,
			enemy_speed: game_config.enemy_move_speed
		}
    }

	async fn initialize_world(&mut self, _renderer: &mut KbRenderer<'_>, _game_config: &mut KbConfig ) {
		log!("GameEngine::initialize_world() caled...");

		// Create Player
		self.game_objects.push(GameObject { 
			position: (0.0, -0.35, CHARACTER_Z).into(),
			scale: (0.15, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Character,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 0,
			sprite_index: 0,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 3.1,
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});
	
		// Sky
		self.game_objects.push(GameObject { 
			position: (0.0, -0.35, SKY_Z).into(),
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

		// Sun
		self.game_objects.push(GameObject { 
			position: (-0.5, 1.0 - 0.35, SUN_Z).into(),
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
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});

		// Clouds
		let mut i = 0;
		while i < 10 {

			let rand_x = kb_random_f32(-2.0, 2.0);
			let rand_y = kb_random_f32(0.8, 1.1) - 0.35;
			let scale_x = kb_random_f32(0.17, 0.3);
			let scale_y = kb_random_f32(0.13, 0.17);
			let x_speed = kb_random_f32(0.03, 0.05);

			// Cloud
			self.game_objects.push(GameObject { 
				position: (rand_x, rand_y, CLOUD_Z).into(),
				scale: (scale_x, scale_y, 0.15).into(),
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (0.0, 0.0, 0.0).into(),
				object_type: GameObjectType::Cloud,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 1,
				sprite_index: 18 + kb_random_u32(0, 1) as i32,
				anim_frame: 0,
				life_start_time: Instant::now(),
				state_start_time: Instant::now(),
				gravity_scale: 0.0,
				random_val: kb_random_f32(0.0, 1000.0),
				is_enemy: false
			});

			match self.game_objects.last_mut() {
				Some(game_obj) => {
					game_obj.set_velocity(Vector3::<f32>::new(x_speed, 0.0, 0.0));
				}

				None => ()
			
			}
			i = i + 1;
		}

		// Hills
		self.game_objects.push(GameObject { 
			position: (0.0, 0.75 - 0.35, HILL_Z).into(),
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
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});

		let mut x = -1.9;
		while x < 1.9 {
			let building_width = kb_random_f32(0.12, 0.20);
			let building_height = kb_random_f32(0.25, 0.5);
			self.game_objects.push(GameObject { 
				position: (x, building_height - 0.35, BUILDING_Z - (building_height * 3.0)).into(),
				scale: (building_width, building_height, 1.0).into(),
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (0.0, 0.0, 0.0).into(),
				object_type: GameObjectType::Background,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 1,
				sprite_index: kb_random_u32(32, 39) as i32,
				anim_frame: 0,
				life_start_time: Instant::now(),
				state_start_time: Instant::now(),
				gravity_scale: 0.0,
				random_val: kb_random_f32(0.0, 1000.0),
				is_enemy: false
			});
			x = x + kb_random_f32(0.2, 0.3);
		}

		// Trees
		let mut x = -1.9;
		while x < 1.9 {
			let tree_width = kb_random_f32(0.08, 0.15);
			let tree_height = kb_random_f32(0.16, 0.2);
			self.game_objects.push(GameObject { 
				position: (x, tree_height - 0.35, BUILDING_Z + 1.0).into(),
				scale: (tree_width, tree_height, 1.0).into(),
				direction: (1.0, 0.0, 0.0).into(),
				velocity: (0.0, 0.0, 0.0).into(),
				object_type: GameObjectType::Background,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 1,
				sprite_index: kb_random_u32(23, 24) as i32,
				anim_frame: 0,
				life_start_time: Instant::now(),
				state_start_time: Instant::now(),
				gravity_scale: 0.0,
				random_val: kb_random_f32(0.0, 1000.0),
				is_enemy: false
			});
			x = x + kb_random_f32(0.12, 0.19);
		}
		// Roads
		self.game_objects.push(GameObject { 
			position: (1.0, -0.5 - 0.35, BUILDING_Z + 2.0).into(),
			scale: (1.0, 0.5, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 22,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 0.0,
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});
		self.game_objects.push(GameObject { 
			position: (-1.0, -0.5 - 0.35, BUILDING_Z + 2.0).into(),
			scale: (1.0, 0.5, 1.0).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Background,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 1,
			sprite_index: 22,
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

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &KbInputManager, _game_config: &KbConfig) {
		let _delta_time_secs = self.current_frame_time.elapsed().as_secs_f32();
        self.current_frame_time = Instant::now();

		// Player Movement
        let mut move_vec:cgmath::Vector3<f32> = (0.0, 0.0, 0.0).into();

        if input_manager.left_pressed() {
            move_vec = Vector3::new(-1.0, 0.0, 0.0);
			self.game_objects[0].direction.x = -1.0;
        }

        if input_manager.right_pressed() {
           move_vec = Vector3::new(1.0, 0.0, 0.0);
		   self.game_objects[0].direction.x = 1.0;
		}

        if input_manager.up_pressed {
            move_vec.y = 1.0;
        }

        self.game_objects[0].set_velocity(move_vec);

		self.update_enemies();
		self.update_projectiles();

		// Player Action
		if input_manager.fire_pressed() && self.game_objects[0].start_attack() {
			let direction = self.game_objects[0].direction;
			let velocity = if direction.x > 0.0 { (5.0, 0.0, 0.0).into() } else { (-5.0, 0.0, 0.0).into() };
			let new_projectile = GameObject { 
				position: self.game_objects[0].position + direction * 0.1,
				scale: (0.035, 0.05, 0.05).into(),
				direction,
				velocity,
				object_type: GameObjectType::Projectile,
				object_state: GameObjectState::Idle,
				next_attack_time: 0.0,
				texture_index: 0,
				sprite_index: 5,
				anim_frame: 0,
				life_start_time: Instant::now(),
				state_start_time: Instant::now(),
				gravity_scale: 0.0,
				random_val: kb_random_f32(0.0, 1000.0),
				is_enemy: false
			};

			self.game_objects.push(new_projectile);
		}

		// Update game objects
		let game_object_iter = self.game_objects.iter_mut();
		for game_object in game_object_iter {
			game_object.update(_delta_time_secs);
		}

		let debug_msg = format!("Move: [W][A][S][D]    Shoot: [Space]\nToggle VSync: [V]");
        renderer.set_debug_game_msg(&debug_msg);
	}
}