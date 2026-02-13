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
        let url = "ws://127.0.0.1:3000/ws";

        leptos::logging::log!("[WS] Connecting to {}", url);
        let ws = WebSocket::new(url)?;

        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            leptos::logging::log!("[WS] Successfully connected to server");
        });
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: JsValue| {
            leptos::logging::log!("[WS] Error: {:?}", e);
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::CloseEvent| {
            leptos::logging::log!("[WS] Closed: code={}, reason='{}'", e.code(), e.reason());
        });
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let txt_str = String::from(txt);
                match serde_json::from_str::<ServerMessage>(&txt_str) {
                    Ok(msg) => {
                        leptos::logging::log!("[WS] Received: {:?}", msg);
                        on_message.set(Some(msg));
                    }
                    Err(e) => {
                        leptos::logging::log!(
                            "[WS] Failed to parse ServerMessage: {}, raw: {}",
                            e,
                            txt_str
                        );
                    }
                }
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        Ok(Self { ws })
    }

    pub fn send(&self, msg: &GameMessage) {
        if let Ok(json) = serde_json::to_string(msg) {
            leptos::logging::log!("[WS] Sending: {:?}", msg);
            if let Err(e) = self.ws.send_with_str(&json) {
                leptos::logging::log!("[WS] Failed to send: {:?}", e);
            }
        }
    }
}
