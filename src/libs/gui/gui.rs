use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
// use std::sync::Arc;
use fltk::{
    app::*, button::*, draw::*, enums::*, /*menu::*,*/ prelude::*, /*valuator::*,*/ widget::*, window::*,
};
// use fltk::frame::Frame;
use fltk::input::MultilineInput;
//use std::sync::mpsc::{channel, RecvError};
use fltk::output::Output;
use log::{debug, info};
// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::ops::{Deref, DerefMut};
// use std::rc::Rc;
use crate::libs::application::application::Application;
use crate::libs::config_file::config_file::ConfigurationStore;
use crate::libs::gui::message::{KeyingText, Message};
use crate::libs::util::version::VERSION;

const WIDGET_HEIGHT: i32 = 25;
const WIDGET_PADDING: i32 = 10;

const WATERFALL_WIDTH: i32 = 1000;
const WATERFALL_HEIGHT: i32 = 500;

// Central controls column
const CENTRAL_CONTROLS_WIDTH: i32 = 240;

const CODE_SPEED_WIDTH: i32 = 60;
const CODE_SPEED_HEIGHT: i32 = 40;
const CODE_SPEED_BUTTON_DIM: i32 = CODE_SPEED_HEIGHT / 2;

const INDICATORS_CANVAS_HEIGHT: i32 = 40;
const INDICATOR_PADDING: i32 = 6;
const RX_INDICATOR_WIDTH: i32 = 40;
const WAIT_INDICATOR_WIDTH: i32 = 60;
const TX_INDICATOR_WIDTH: i32 = 40;

const TEXT_ENTRY_HEIGHT: i32 = 120;

struct Gui {
    config: Arc<Mutex<ConfigurationStore>>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    waterfall_canvas: Widget,
    status_output: Output,
    code_speed_output: Output,
    code_speed_up_button: Button,
    code_speed_down_button: Button,
    code_speed_label: Widget, // PITA, Frame doesn't align properly
    indicators_canvas: Widget,
    text_entry: Rc<RefCell<MultilineInput>>,
}

pub fn initialise(config: Arc<Mutex<ConfigurationStore>>, _application: &mut Application) -> () {
    debug!("Initialising App");
    let app = App::default().with_scheme(Scheme::Gtk);
    debug!("Initialising Window");
    let mut wind = Window::default().with_label(format!("digimorse v{} de M0CUV", VERSION).as_str());


    let waterfall_canvas_background = Color::from_hex_str("#aab0cb").unwrap();
    let window_background = Color::from_hex_str("#dfe2ff").unwrap();
    let rx_inactive = Color::from_hex_str("#142d59").unwrap();
    let rx_active = Color::from_hex_str("#1f94d9").unwrap();
    let wait_inactive = Color::from_hex_str("#b16e14").unwrap();
    let wait_active = Color::from_hex_str("#f89919").unwrap();
    let tx_inactive = Color::from_hex_str("#6c1c11").unwrap();
    let tx_active = Color::from_hex_str("#da3620").unwrap();

    let (sender, receiver) = channel::<Message>();

    let mut gui = Gui {
        config,
        sender,
        receiver,
        waterfall_canvas: Widget::new(WIDGET_PADDING, WIDGET_PADDING, WATERFALL_WIDTH, WATERFALL_HEIGHT, ""),
        status_output: Output::default()
            .with_size(WATERFALL_WIDTH, WIDGET_HEIGHT)
            .with_pos(WIDGET_PADDING, WIDGET_PADDING + WATERFALL_HEIGHT + WIDGET_PADDING),
        code_speed_output: Output::default()
            .with_size(CODE_SPEED_WIDTH, CODE_SPEED_HEIGHT)
            .with_pos(WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING, WIDGET_PADDING),
        code_speed_up_button: Button::default()
            .with_size(CODE_SPEED_BUTTON_DIM, CODE_SPEED_BUTTON_DIM)
            .with_pos(WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING + CODE_SPEED_WIDTH, WIDGET_PADDING)
            .with_label("▲"),
        code_speed_down_button: Button::default()
            .with_size(CODE_SPEED_BUTTON_DIM, CODE_SPEED_BUTTON_DIM)
            .with_pos(WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING + CODE_SPEED_WIDTH, WIDGET_PADDING + CODE_SPEED_BUTTON_DIM)
            .with_label("▼"),
        code_speed_label: Widget::default()
            .with_size(CENTRAL_CONTROLS_WIDTH - CODE_SPEED_BUTTON_DIM - CODE_SPEED_WIDTH - WIDGET_PADDING, CODE_SPEED_BUTTON_DIM * 2)
            .with_pos(WIDGET_PADDING * 3 + WATERFALL_WIDTH + CODE_SPEED_WIDTH + CODE_SPEED_BUTTON_DIM, WIDGET_PADDING),
        indicators_canvas: Widget::default()
            .with_size(CENTRAL_CONTROLS_WIDTH, INDICATORS_CANVAS_HEIGHT)
            .with_pos(WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING, WIDGET_PADDING + CODE_SPEED_BUTTON_DIM * 2 + WIDGET_PADDING),
        text_entry: Rc::new(RefCell::new(MultilineInput::default()
            .with_size(CENTRAL_CONTROLS_WIDTH, TEXT_ENTRY_HEIGHT)
            .with_pos(WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING, WIDGET_PADDING + CODE_SPEED_BUTTON_DIM * 2 + WIDGET_PADDING + INDICATORS_CANVAS_HEIGHT + WIDGET_PADDING))),
    };

    gui.waterfall_canvas.set_trigger(CallbackTrigger::Release);
    gui.waterfall_canvas.draw(move |wid| {
        push_clip(wid.x(), wid.y(), wid.width(), wid.height());
        draw_rect_fill(wid.x(), wid.y(), wid.width(), wid.height(), waterfall_canvas_background);

        set_draw_color(Color::Black);
        draw_rect(wid.x(), wid.y(), wid.width(), wid.height());
        pop_clip();
    });

    gui.code_speed_label.draw(move |wid| {
        push_clip(wid.x(), wid.y(), wid.width(), wid.height());
        draw_rect_fill(wid.x(), wid.y(), wid.width(), wid.height(), window_background);
        set_draw_color(Color::Black);
        draw_text("Transmit Keyer", wid.x(), 22); // unholy magic co-ordinates
        draw_text("Speed in WPM", wid.x(), 44);
        pop_clip();
    });

    gui.code_speed_up_button.emit(gui.sender.clone(), Message::IncreaseKeyingSpeedRequest);
    gui.code_speed_down_button.emit(gui.sender.clone(), Message::DecreaseKeyingSpeedRequest);

    gui.indicators_canvas.draw(move |wid| {
        push_clip(wid.x(), wid.y(), wid.width(), wid.height());
        draw_rect_fill(wid.x(), wid.y(), wid.width(), wid.height(), window_background);

        draw_rect_fill(wid.x() + INDICATOR_PADDING, wid.y() + INDICATOR_PADDING, RX_INDICATOR_WIDTH, wid.height() - 2 * INDICATOR_PADDING, rx_inactive);
        set_draw_color(rx_active);
        draw_text("RX", wid.x() + INDICATOR_PADDING + 10, wid.y() + INDICATOR_PADDING + 18);

        draw_rect_fill(wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING, wid.y() + INDICATOR_PADDING, WAIT_INDICATOR_WIDTH, wid.height() - 2 * INDICATOR_PADDING, wait_inactive);
        set_draw_color(wait_active);
        draw_text("WAIT", wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING + 12, wid.y() + INDICATOR_PADDING + 18);

        draw_rect_fill(wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING + WAIT_INDICATOR_WIDTH + INDICATOR_PADDING, wid.y() + INDICATOR_PADDING, TX_INDICATOR_WIDTH, wid.height() - 2 * INDICATOR_PADDING, tx_inactive);
        set_draw_color(tx_active);
        draw_text("TX", wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING + WAIT_INDICATOR_WIDTH + INDICATOR_PADDING + 10, wid.y() + INDICATOR_PADDING + 18);

        set_draw_color(Color::Black);
        draw_rect(wid.x(), wid.y(), wid.width(), wid.height());
        pop_clip();
    });

    gui.status_output.set_color(Color::Black);
    gui.status_output.set_text_color(Color::from_hex_str("#f2cc91").unwrap());
    gui.status_output.set_value("status message");

    gui.code_speed_output.set_color(window_background);
    gui.code_speed_output.set_text_color(Color::Black);
    gui.code_speed_output.set_value(gui.config.lock().unwrap().get_wpm().to_string().as_str());
    gui.code_speed_output.set_text_size(36);

    let entry_prompt = "Enter message,\nthen RETURN to send.";
    // TODO set an inner padding?
    gui.text_entry.borrow_mut().set_color(window_background.lighter());
    gui.text_entry.borrow_mut().set_wrap(true);
    gui.text_entry.borrow_mut().set_align(Align::TopLeft);
    gui.text_entry.borrow_mut().set_tooltip(entry_prompt);
    gui.text_entry.borrow_mut().insert(entry_prompt).unwrap();
    gui.text_entry.borrow_mut().set_trigger(CallbackTrigger::EnterKey);
    let cb_text_entry = gui.text_entry.clone();
    gui.text_entry.borrow_mut().handle(move |widget, event| {
        match event {
            Event::Focus => {
                // Clear out the initial prompt text.
                let contents = widget.value();
                if contents == entry_prompt.to_string() {
                    widget.set_value("");
                }
            },
            _ => {
                // nothing
            }
        }
        true
    });
    let text_entry_sender = gui.sender.clone();
    gui.text_entry.borrow_mut().set_callback(move |_f| {
        let contents = cb_text_entry.borrow_mut().value();
        let trimmed_contents = contents.trim();
        if trimmed_contents.len() > 0 {
            text_entry_sender.send(Message::KeyingText { 0: KeyingText {  text: trimmed_contents.to_owned() } });
        }
    });

    wind.set_size(
        WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING + CENTRAL_CONTROLS_WIDTH + WIDGET_PADDING,
        WIDGET_PADDING + WATERFALL_HEIGHT + WIDGET_PADDING + WIDGET_HEIGHT + WIDGET_PADDING,
    );
    wind.set_color(window_background);

    wind.end();
    debug!("Showing main window");
    wind.show();
    debug!("Starting app wait loop");
    while app.wait() {
        debug!("app wait has returned true");
        match gui.receiver.recv() {
            None => {
                // noop
            }
            Some(message) => {
                info!("App message {:?}", message);
                match message {
                    Message::KeyingText(_keying_text) => {}
                    Message::Beep => {}
                    Message::SetKeyingSpeed(_) => {}
                    Message::IncreaseKeyingSpeedRequest => {}
                    Message::DecreaseKeyingSpeedRequest => {}
                }
            }
        }
    }
    debug!("Out of app wait loop");
}
