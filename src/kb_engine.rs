use crate::{kb_config::*, kb_game_object::*, kb_input::*, kb_renderer::*};

pub trait KbGameEngine {
	fn new(game_config: &KbConfig) -> Self;

	#[allow(async_fn_in_trait)]
	async fn initialize_world<'a>(&mut self, renderer: &'a mut KbRenderer<'_>, game_config: &KbConfig);

	fn get_game_objects(&self) -> &Vec<GameObject>;

	// Do not override tick_frame().  Put custom code in tick_frame_internal()
	fn tick_frame(&mut self, renderer: &mut KbRenderer, input_manager: &KbInputManager, game_config: &mut KbConfig) {
		game_config.update_frame_times();
		self.tick_frame_internal(renderer, input_manager, game_config);
	}

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &KbInputManager, game_config: &KbConfig);
}