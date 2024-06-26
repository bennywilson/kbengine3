use kb_engine3::kb_config::KbConfig;

mod example_game;
mod game_actors;
mod game_vfx;

use example_game::Example3DGame;

fn main() {
    let config_file_text = include_str!("game_config.txt");
    let game_config = KbConfig::new(config_file_text);

    let run_game = kb_engine3::run_game::<Example3DGame>(game_config);

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(run_game);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run_game);
    }
}
