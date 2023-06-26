use chargrid_web::{Context, Size};
use synth_app::{self, args, music};
use wasm_bindgen::prelude::*;

fn run_synth() {
    let context = Context::new(Size::new(100, 60), "content");
    let args = args::Args {
        volume_scale: 0.1,
        start_note: music::note(music::NoteName::C, 2),
        downsample: 2,
    };
    context.run(synth_app::app(args).unwrap());
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    console_error_panic_hook::set_once();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let button = document.create_element("button")?;
    button.set_inner_html("Press to start!");
    let button = button.unchecked_into::<web_sys::HtmlElement>();
    let style = button.style();
    style.set_property("font-size", "24pt")?;
    style.set_property("font-family", "monospace")?;
    style.set_property("position", "fixed")?;
    style.set_property("top", "50%")?;
    style.set_property("left", "50%")?;
    style.set_property("transform", "translate(-50%, -50%)")?;
    let body = document.body().unwrap();
    body.append_child(&button)?;
    let handle_keydown = {
        let button_for_closure = button.clone();
        Closure::wrap(Box::new(move |_event: JsValue| {
            button_for_closure.remove();
            run_synth();
        }) as Box<dyn FnMut(JsValue)>)
    };
    button
        .add_event_listener_with_callback("click", handle_keydown.as_ref().unchecked_ref())
        .unwrap();
    handle_keydown.forget();
    Ok(())
}
