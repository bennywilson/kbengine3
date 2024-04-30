use crate::{kb_config::KbConfig, kb_game_object::*, kb_input::InputManager, kb_renderer::KbRenderer};

#[allow(dead_code)] 
trait KbAsset {
    fn asset_name(&self) -> &String;
}

struct KbTexture {
	name: String,
}

impl KbAsset for KbTexture {
     fn asset_name(&self) -> &String {
		 return &self.name;
	 }
}

#[allow(dead_code)] 
#[derive(Default)]
pub struct KbAssetManager {
	resources: Vec<Box<dyn KbAsset>>,
}

#[allow(dead_code)] 
impl KbAssetManager {
	pub fn new() -> Self {
		Self {
			..Default::default()
		}
	}
	fn load_asset(_asset_name: String) {

	}
}

pub trait KbGameEngine {
	fn new(game_config: &KbConfig) -> Self;

	#[allow(async_fn_in_trait)]
	async fn initialize_world<'a>(&mut self, renderer: &'a mut KbRenderer<'_>);

	fn get_game_objects(&self) -> &Vec<GameObject>;

	// Do not override tick_frame().  Put custom code in tick_frame_internal()
	fn tick_frame(&mut self, renderer: &mut KbRenderer, input_manager: &InputManager, game_config: &mut KbConfig) {
		game_config.update_frame_times();
		self.tick_frame_internal(renderer, input_manager, game_config);
	}

	fn tick_frame_internal(&mut self, renderer: &mut KbRenderer, input_manager: &InputManager, game_config: &KbConfig);
}