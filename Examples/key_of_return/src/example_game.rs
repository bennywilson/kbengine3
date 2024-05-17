use instant::Instant;

use kb_engine3::{log, kb_config::*, kb_engine::*, kb_input::*, kb_game_object::*, kb_renderer::*, kb_utils::*};

#[allow(dead_code)]
pub struct KeyOfReturn {
	pub game_objects: Vec<GameObject>,
	game_start_time:  Instant,
	current_frame_time:  Instant,

    timeline_index: i32,
}

impl KeyOfReturn {

}

impl KbGameEngine for KeyOfReturn {
	fn new(_game_config: &KbConfig) -> Self {
		log!("GameEngine::new() caled...");

		let cur_time = Instant::now();

		Self {
			game_objects: Vec::<GameObject>::new(),
			game_start_time:  cur_time,
			current_frame_time : cur_time,
            timeline_index: 0,
		}
    }

	async fn initialize_world(&mut self, renderer: &mut KbRenderer<'_>, game_config: &mut KbConfig ) {
		log!("GameEngine::initialize_world() caled...");

		game_config.clear_color = CgVec4::new(0.0, 0.0, 0.0, 1.0);

		// Create Player
		self.game_objects.push(GameObject { 
			position: (-0.5, 0.5, 1.0).into(),
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

		self.game_objects.push(GameObject { 
			position: (0.5, 0.5, 1.0).into(),
			scale: (0.15, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Character,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 0,
			sprite_index: 1,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 3.1,
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});

        self.game_objects.push(GameObject { 
			position: (-1.0, -0.5, 1.0).into(),
			scale: (0.15, 0.15, 0.15).into(),
			direction: (1.0, 0.0, 0.0).into(),
			velocity: (0.0, 0.0, 0.0).into(),
			object_type: GameObjectType::Character,
			object_state: GameObjectState::Idle,
			next_attack_time: 0.0,
			texture_index: 0,
			sprite_index: 2,
			anim_frame: 0,
			life_start_time: Instant::now(),
			state_start_time: Instant::now(),
			gravity_scale: 3.1,
			random_val: kb_random_f32(0.0, 1000.0),
			is_enemy: false
		});

		let particle_smoke_params = KbParticleParams {
            texture_file: "/game_assets/fx/smoke_t.png".to_string(),
            blend_mode: KbParticleBlendMode::AlphaBlend,

            min_burst_count: 0,
            max_burst_count: 0,

            min_particle_life: 3.0,
            max_particle_life: 5.0,

            _min_actor_life: -1.0,
            _max_actor_life: -1.0,

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
        let particle_transform = KbActorTransform::from_position(CgVec3::new(10.0, 0.0, 15.0));
        let _ = renderer
            .add_particle_actor(&particle_transform, &particle_smoke_params, true)
            .await;

        let particle_ember_params = KbParticleParams {
            texture_file: "./game_assets/fx/ember_t.png".to_string(),
            blend_mode: KbParticleBlendMode::Additive,

            min_burst_count: 0,
            max_burst_count: 0,

            min_particle_life: 1.5,
            max_particle_life: 2.5,

            _min_actor_life: -1.0,
            _max_actor_life: -1.0,

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
        let particle_transform = KbActorTransform::from_position(CgVec3::new(10.0, -2.0, 15.0));
        let _ = renderer
            .add_particle_actor(&particle_transform, &particle_ember_params, true)
            .await;
	}

	fn get_game_objects(&self) -> &Vec<GameObject> {
		&self.game_objects
	}

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &KbInputManager, game_config: &KbConfig) {
		let _delta_time_secs = self.current_frame_time.elapsed().as_secs_f32();
        self.current_frame_time = Instant::now();

		renderer.add_line(
            &CgVec3::new(1.0, -0.5, 1.0),
            &CgVec3::new(-1.0, -0.5, 1.0),
            &CgVec4::new(1.0, 1.0, 1.0, 1.0),
            0.01,
            0.01,
            game_config,
        );
		let debug_msg = "Debug Info can go here!".to_string();
        renderer.set_debug_game_msg(&debug_msg);

        if input_manager.key_arrow_right() == KbButtonState::JustPressed {
            self.timeline_index += 1;
            if self.timeline_index > 2 {
                self.timeline_index = 0;
            }
        }
        if input_manager.key_arrow_left() == KbButtonState::JustPressed {
            self.timeline_index -= 1;
            if self.timeline_index < 0 {
                self.timeline_index = 2;
            }
        }

        match self.timeline_index {
            0 => {
                self.game_objects[2].sprite_index = 2;
                self.game_objects[2].position.x = -1.0;
            }

            1 => {
                self.game_objects[2].sprite_index = 3;
                self.game_objects[2].position.x = 0.0;
            }

            2 => {
                self.game_objects[2].sprite_index = 4;
                self.game_objects[2].position.x = 1.0;
            }

            _ => {}
        }
	}
}