use cotuong_core::worker::GameWorker;
use gloo_worker::Registrable;

fn main() {
    console_error_panic_hook::set_once();
    GameWorker::registrar().register();
}
