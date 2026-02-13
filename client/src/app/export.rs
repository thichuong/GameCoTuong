use cotuong_core::engine::config::EngineConfig;
use cotuong_core::logic::game::GameState;
use leptos::{document, SignalSet, WriteSignal};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

pub fn handle_file_upload(setter: WriteSignal<EngineConfig>) -> impl Fn(web_sys::Event) {
    move |ev: web_sys::Event| {
        let target_el = ev.target().expect("Event should have target");
        let input = target_el.dyn_into::<web_sys::HtmlInputElement>().expect("Target should be HtmlInputElement");

        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let Ok(reader) = web_sys::FileReader::new() else {
                    return;
                };
                let reader_c = reader.clone();

                let on_load = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                    if let Ok(res) = reader_c.result() {
                        if let Some(text) = res.as_string() {
                            match serde_json::from_str::<EngineConfig>(&text) {
                                Ok(config) => {
                                    web_sys::console::log_1(&"Config loaded successfully".into());
                                    setter.set(config);
                                }
                                Err(e) => {
                                    web_sys::console::log_1(
                                        &format!("Error parsing config: {e:?}").into(),
                                    );
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.alert_with_message(&format!(
                                            "Error parsing JSON: {e}"
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);

                reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
                on_load.forget();

                if let Err(e) = reader.read_as_text(&file) {
                    web_sys::console::log_1(&format!("Error reading file: {e:?}").into());
                }
            }
        }
    }
}

pub fn export_config(config: EngineConfig, filename: &str) {
    if let Ok(json) = serde_json::to_string_pretty(&config) {
        if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&json.into())) {
            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                if let Ok(el) = document().create_element("a") {
                    if let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() {
                        anchor.set_href(&url);
                        anchor.set_download(filename);
                        anchor.click();
                        let _ = web_sys::Url::revoke_object_url(&url);
                    }
                }
            }
        }
    }
}

pub fn export_csv(state: GameState) {
    use std::fmt::Write;

    let mut csv = String::from("Turn,From,To,Piece,Captured,Note\n");
    for (i, record) in state.history.iter().enumerate() {
        let turn = if i % 2 == 0 { "Red" } else { "Black" };
        let from = format!("({},{})", record.from.row, record.from.col);
        let to = format!("({},{})", record.to.row, record.to.col);
        let piece = format!("{:?}", record.piece.piece_type);
        let captured = record
            .captured
            .map(|p| format!("{:?}", p.piece_type))
            .unwrap_or_default();
        let note = record.note.clone().unwrap_or_default();
        let _ = writeln!(csv, "{turn},{from},{to},{piece},{captured},{note}");
    }

    if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&js_sys::Array::of1(&csv.into())) {
        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
            if let Ok(el) = document().create_element("a") {
                if let Ok(anchor) = el.dyn_into::<web_sys::HtmlAnchorElement>() {
                    anchor.set_href(&url);
                    anchor.set_download("xiangqi_game.csv");
                    anchor.click();
                    let _ = web_sys::Url::revoke_object_url(&url);
                }
            }
        }
    }
}
