use crate::app::export::{export_config, handle_file_upload};
use cotuong_core::engine::config::EngineConfig;
use leptos::{
    component, event_target_value, view, IntoView, ReadSignal, SignalGet, SignalSet, WriteSignal,
};

#[component]
pub fn ConfigPanel(
    show_config: ReadSignal<bool>,
    red_config: ReadSignal<EngineConfig>,
    set_red_config: WriteSignal<EngineConfig>,
    black_config: ReadSignal<EngineConfig>,
    set_black_config: WriteSignal<EngineConfig>,
) -> impl IntoView {
    view! {
        <div style=move || if show_config.get() { "display: block;" } else { "display: none;" }>
            <div class="config-panel">
                <div class="config-column">
                    <div class="config-title" style="color: #ff6b6b;">"Cấu hình Đỏ (Red)"</div>
                    <div style="margin-bottom: 15px; text-align: center;">
                        <label style="display: block; margin-bottom: 5px; color: #ccc; font-size: 0.9em;">"Load JSON Config"</label>
                        <input type="file" accept=".json" on:change=handle_file_upload(set_red_config) style="color: #ccc;" />
                        <button style="margin-top: 5px; font-size: 0.8em;" on:click=move |_| export_config(red_config.get(), "red_config.json")>"Export JSON"</button>
                    </div>
                    {
                        move || {
                            let config = red_config.get();
                            view! {
                                <div>
                                    <Slider label="Tốt (Pawn)" val=config.val_pawn min=0 max=200 step=1 on_input=move |v| { let mut c = red_config.get(); c.val_pawn = v; set_red_config.set(c); } />
                                    <Slider label="Sĩ (Advisor)" val=config.val_advisor min=0 max=400 step=1 on_input=move |v| { let mut c = red_config.get(); c.val_advisor = v; set_red_config.set(c); } />
                                    <Slider label="Tượng (Elephant)" val=config.val_elephant min=0 max=400 step=1 on_input=move |v| { let mut c = red_config.get(); c.val_elephant = v; set_red_config.set(c); } />
                                    <Slider label="Mã (Horse)" val=config.val_horse min=0 max=800 step=1 on_input=move |v| { let mut c = red_config.get(); c.val_horse = v; set_red_config.set(c); } />
                                    <Slider label="Pháo (Cannon)" val=config.val_cannon min=0 max=900 step=1 on_input=move |v| { let mut c = red_config.get(); c.val_cannon = v; set_red_config.set(c); } />
                                    <Slider label="Xe (Rook)" val=config.val_rook min=0 max=1800 step=1 on_input=move |v| { let mut c = red_config.get(); c.val_rook = v; set_red_config.set(c); } />
                                    <Slider label="Tướng (King)" val=config.val_king min=5000 max=20000 step=100 on_input=move |v| { let mut c = red_config.get(); c.val_king = v; set_red_config.set(c); } />
                                    <hr style="border-color: #444; margin: 10px 0;"/>
                                    <Slider label="Hash Move" val=config.score_hash_move min=0 max=5_000_000 step=100_000 on_input=move |v| { let mut c = red_config.get(); c.score_hash_move = v; set_red_config.set(c); } />
                                    <Slider label="Capture Base" val=config.score_capture_base min=0 max=2_000_000 step=100_000 on_input=move |v| { let mut c = red_config.get(); c.score_capture_base = v; set_red_config.set(c); } />
                                    <Slider label="Killer Move" val=config.score_killer_move min=0 max=2_000_000 step=100_000 on_input=move |v| { let mut c = red_config.get(); c.score_killer_move = v; set_red_config.set(c); } />
                                    <Slider label="History Max" val=config.score_history_max min=0 max=2_000_000 step=100_000 on_input=move |v| { let mut c = red_config.get(); c.score_history_max = v; set_red_config.set(c); } />
                                    <Dropdown label="Pruning Method" val=config.pruning_method options=vec![
                                        (0, "Dynamic Limiting"),
                                        (1, "Late Move Reductions (LMR)"),
                                        (2, "Both (Aggressive)"),
                                    ] on_set=move |v| { let mut c = red_config.get(); c.pruning_method = v; set_red_config.set(c); } />
                                    <FloatSlider label="Multiplier" val=config.pruning_multiplier min=0.1 max=2.0 step=0.1 on_input=move |v| { let mut c = red_config.get(); c.pruning_multiplier = v; set_red_config.set(c); } />
                                    <hr style="border-color: #444; margin: 10px 0;"/>
                                    <Slider label="Mate Score" val=config.mate_score min=10000 max=50000 step=1000 on_input=move |v| { let mut c = red_config.get(); c.mate_score = v; set_red_config.set(c); } />

                                </div>
                            }
                        }
                    }
                </div>
                <div class="config-column">
                    <div class="config-title" style="color: #a8e6cf;">"Cấu hình Đen (Black)"</div>
                    <div style="margin-bottom: 15px; text-align: center;">
                        <label style="display: block; margin-bottom: 5px; color: #ccc; font-size: 0.9em;">"Load JSON Config"</label>
                        <input type="file" accept=".json" on:change=handle_file_upload(set_black_config) style="color: #ccc;" />
                        <button style="margin-top: 5px; font-size: 0.8em;" on:click=move |_| export_config(black_config.get(), "black_config.json")>"Export JSON"</button>
                    </div>
                    {
                        move || {
                            let config = black_config.get();
                            view! {
                                <div>
                                    <Slider label="Tốt (Pawn)" val=config.val_pawn min=0 max=200 step=1 on_input=move |v| { let mut c = black_config.get(); c.val_pawn = v; set_black_config.set(c); } />
                                    <Slider label="Sĩ (Advisor)" val=config.val_advisor min=0 max=400 step=1 on_input=move |v| { let mut c = black_config.get(); c.val_advisor = v; set_black_config.set(c); } />
                                    <Slider label="Tượng (Elephant)" val=config.val_elephant min=0 max=400 step=1 on_input=move |v| { let mut c = black_config.get(); c.val_elephant = v; set_black_config.set(c); } />
                                    <Slider label="Mã (Horse)" val=config.val_horse min=0 max=800 step=1 on_input=move |v| { let mut c = black_config.get(); c.val_horse = v; set_black_config.set(c); } />
                                    <Slider label="Pháo (Cannon)" val=config.val_cannon min=0 max=900 step=1 on_input=move |v| { let mut c = black_config.get(); c.val_cannon = v; set_black_config.set(c); } />
                                    <Slider label="Xe (Rook)" val=config.val_rook min=0 max=1800 step=1 on_input=move |v| { let mut c = black_config.get(); c.val_rook = v; set_black_config.set(c); } />
                                    <Slider label="Tướng (King)" val=config.val_king min=5000 max=20000 step=100 on_input=move |v| { let mut c = black_config.get(); c.val_king = v; set_black_config.set(c); } />
                                    <hr style="border-color: #444; margin: 10px 0;"/>
                                    <Slider label="Hash Move" val=config.score_hash_move min=0 max=5_000_000 step=100_000 on_input=move |v| { let mut c = black_config.get(); c.score_hash_move = v; set_black_config.set(c); } />
                                    <Slider label="Capture Base" val=config.score_capture_base min=0 max=2_000_000 step=100_000 on_input=move |v| { let mut c = black_config.get(); c.score_capture_base = v; set_black_config.set(c); } />
                                    <Slider label="Killer Move" val=config.score_killer_move min=0 max=2_000_000 step=100_000 on_input=move |v| { let mut c = black_config.get(); c.score_killer_move = v; set_black_config.set(c); } />
                                    <Slider label="History Max" val=config.score_history_max min=0 max=2_000_000 step=100_000 on_input=move |v| { let mut c = black_config.get(); c.score_history_max = v; set_black_config.set(c); } />
                                    <Dropdown label="Pruning Method" val=config.pruning_method options=vec![
                                        (0, "Dynamic Limiting"),
                                        (1, "Late Move Reductions (LMR)"),
                                        (2, "Both (Aggressive)"),
                                    ] on_set=move |v| { let mut c = black_config.get(); c.pruning_method = v; set_black_config.set(c); } />
                                    <FloatSlider label="Multiplier" val=config.pruning_multiplier min=0.1 max=2.0 step=0.1 on_input=move |v| { let mut c = black_config.get(); c.pruning_multiplier = v; set_black_config.set(c); } />
                                    <hr style="border-color: #444; margin: 10px 0;"/>
                                    <Slider label="Mate Score" val=config.mate_score min=10000 max=50000 step=1000 on_input=move |v| { let mut c = black_config.get(); c.mate_score = v; set_black_config.set(c); } />

                                </div>
                            }
                        }
                    }
                </div>
            </div>
        </div>
    }
}

#[component]
fn Slider<F>(
    label: &'static str,
    val: i32,
    min: i32,
    max: i32,
    step: i32,
    on_input: F,
) -> impl IntoView
where
    F: Fn(i32) + 'static,
{
    view! {
        <div style="margin-bottom: 8px;">
            <div style="display: flex; justify-content: space-between; font-size: 0.9em; color: #ccc;">
                <span>{label}</span>
                <span>{val}</span>
            </div>
            <input
                type="range"
                min=min
                max=max
                step=step
                value=val
                prop:value=val
                style="width: 100%; accent-color: #a8e6cf;"
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                        on_input(v);
                    }
                }
            />
        </div>
    }
}

#[component]
fn Dropdown<F>(
    label: &'static str,
    val: i32,
    options: Vec<(i32, &'static str)>,
    on_set: F,
) -> impl IntoView
where
    F: Fn(i32) + 'static,
{
    view! {
        <div style="margin-bottom: 8px;">
            <div style="display: flex; justify-content: space-between; font-size: 0.9em; color: #ccc;">
                <span>{label}</span>
            </div>
            <select
                style="width: 100%; padding: 4px; background: #444; color: #eee; border: 1px solid #555; border-radius: 4px;"
                on:change=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                        on_set(v);
                    }
                }
                prop:value=val
            >
                {options.into_iter().map(|(v, txt)| {
                    view! {
                        <option value=v selected={v == val}>{txt}</option>
                    }
                }).collect::<Vec<_>>()}
            </select>
        </div>
    }
}

#[component]
fn FloatSlider<F>(
    label: &'static str,
    val: f32,
    min: f32,
    max: f32,
    step: f32,
    on_input: F,
) -> impl IntoView
where
    F: Fn(f32) + 'static,
{
    view! {
        <div style="margin-bottom: 8px;">
            <div style="display: flex; justify-content: space-between; font-size: 0.9em; color: #ccc;">
                <span>{label}</span>
                <span>{format!("{val:.1}")}</span>
            </div>
            <input
                type="range"
                min=min
                max=max
                step=step
                value=val
                prop:value=val
                style="width: 100%; accent-color: #a8e6cf;"
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                        on_input(v);
                    }
                }
            />
        </div>
    }
}
