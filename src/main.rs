
fn main() {

    let main_future = kb_engine3::run_game();

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(main_future);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(main_future);
    }
}