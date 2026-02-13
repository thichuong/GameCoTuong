use leptos::{SignalSet, WriteSignal};
use shared::{GameMessage, ServerMessage};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

#[derive(Clone)]
pub struct NetworkClient {
    ws: WebSocket,
}

impl NetworkClient {
    pub fn new(on_message: WriteSignal<Option<ServerMessage>>) -> Result<Self, JsValue> {
        let location = web_sys::window().unwrap().location();
        let host = location.host()?; // e.g. "localhost:3000" or "example.com"
        let protocol = if location.protocol()? == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        let url = format!("{}//{}/ws", protocol, host);

        let ws = WebSocket::new(&url)?;

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                if let Ok(msg) = serde_json::from_str::<ServerMessage>(&String::from(txt)) {
                    on_message.set(Some(msg));
                }
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        Ok(Self { ws })
    }

    pub fn send(&self, msg: &GameMessage) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.ws.send_with_str(&json);
        }
    }
}
