use crate::{kb_object::*,kb_input::InputManager, kb_config::KbConfig};

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

	fn initialize_world(&mut self);

	fn get_game_objects(&self) -> &Vec<GameObject>;

	fn tick_frame(&mut self, input_manager: &InputManager);
}