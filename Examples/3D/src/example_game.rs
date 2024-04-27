use instant::Instant;

use kb_engine3::{kb_config::KbConfig, kb_engine::KbGameEngine, kb_input::InputManager, kb_game_object::{KbActor, GameObject, GameObjectState, GameObjectType}, kb_renderer::KbRenderer};
use kb_engine3::{game_random_f32, log};

pub struct Example3DGame {
	actors: Vec<KbActor>,
	game_objects: Vec<GameObject>,
}

impl Example3DGame {
}

impl KbGameEngine for Example3DGame {

	fn new(_game_config: &KbConfig) -> Self {
		log!("GameEngine::new() caled...");

		Self {
			actors: Vec::<KbActor>::new(),
			game_objects: Vec::<GameObject>::new(),
		}
    }

	fn get_game_objects(&self) -> &Vec<GameObject> {
		&self.game_objects
	}

	fn tick_frame(&mut self, _renderer: &mut KbRenderer, _input_manager: &InputManager) {
	}

	fn initialize_world(&mut self, renderer: &mut KbRenderer)
	{
		log!("GameEngine::initialize_world() caled...");

		renderer.load_model("game_assets/pinky.gltf");
		renderer.load_model("game_assets/ELP_Barrel.gltf");
		renderer.load_model("game_assets/Shotgun.gltf");

		let mut actor = KbActor::new();
		actor.set_position([0.0, 0.0, 0.0].into());
		actor.set_scale([0.0, 0.0, 0.0].into());
		actor.set_model_id(0);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[0]);

		let mut actor = KbActor::new();
		actor.set_position([5.0, 0.0, 0.0].into());
		actor.set_scale([0.0, 0.0, 0.0].into());
		actor.set_model_id(1);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[1]);
	
		let mut actor = KbActor::new();
		actor.set_position([5.0, 0.0, 0.0].into());
		actor.set_scale([0.0, 0.0, 0.0].into());
		actor.set_model_id(2);
		self.actors.push(actor);
		renderer.add_or_update_actor(&self.actors[2]);

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
		self.game_objects.push(GameObject { 
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
		});
    }
}