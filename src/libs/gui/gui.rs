use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::sync::mpsc::sync_channel;
use std::thread::{JoinHandle, self};
use std::time::Duration;
use fltk::{
    app::*, button::*, draw::*, enums::*, /*menu::*,*/ prelude::*, /*valuator::*,*/ widget::*, window::*,
};
use fltk::input::MultilineInput;
use fltk::output::Output;
use log::{debug, info};
use crate::libs::config_file::config_file::ConfigurationStore;
use crate::libs::gui::message::{KeyingText, Message};
use crate::libs::gui::gui_facades::GUIOutput;
use crate::libs::keyer_io::keyer_io::{MAX_KEYER_SPEED, MIN_KEYER_SPEED};
use crate::libs::util::version::VERSION;

use super::gui_facades::GUIInputMessage;

pub const WIDGET_PADDING: i32 = 10;

const WIDGET_HEIGHT: i32 = 25;

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

pub struct Gui {
    config: Arc<Mutex<ConfigurationStore>>,
    gui_output: Arc<Mutex<dyn GUIOutput>>,
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
    window_width: i32,
    window_height: i32,
    rx_indicator: Arc<RefCell<bool>>,
    wait_indicator: Arc<RefCell<bool>>,
    tx_indicator: Arc<RefCell<bool>>,
    gui_input_tx: Arc<mpsc::SyncSender<GUIInputMessage>>,
    thread_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Gui {
    pub fn new(config: Arc<Mutex<ConfigurationStore>>, gui_output: Arc<Mutex<dyn GUIOutput>>, terminate: Arc<AtomicBool>) -> Self {
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

        let rx_indicator = Arc::new(RefCell::new(false));
        let wait_indicator = Arc::new(RefCell::new(false));
        let tx_indicator = Arc::new(RefCell::new(false));
 
        let thread_terminate = terminate.clone();

        let (gui_input_tx, gui_input_rx) = sync_channel::<GUIInputMessage>(16);

        let mut gui = Gui {
            config,
            gui_output,
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
            window_width: WIDGET_PADDING + WATERFALL_WIDTH + WIDGET_PADDING + CENTRAL_CONTROLS_WIDTH + WIDGET_PADDING,
            window_height: WIDGET_PADDING + WATERFALL_HEIGHT + WIDGET_PADDING + WIDGET_HEIGHT + WIDGET_PADDING,
            rx_indicator,
            wait_indicator,
            tx_indicator,
            gui_input_tx: Arc::new(gui_input_tx),
            thread_handle: Mutex::new(None),
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

        let canvas_rx = gui.rx_indicator.clone();
        let canvas_wait = gui.wait_indicator.clone();
        let canvas_tx = gui.tx_indicator.clone();
        gui.indicators_canvas.draw(move |wid| {
            push_clip(wid.x(), wid.y(), wid.width(), wid.height());
            draw_rect_fill(wid.x(), wid.y(), wid.width(), wid.height(), window_background);

            draw_rect_fill(wid.x() + INDICATOR_PADDING, wid.y() + INDICATOR_PADDING, RX_INDICATOR_WIDTH, wid.height() - 2 * INDICATOR_PADDING, 
                           if *canvas_rx.borrow() { rx_active } else { rx_inactive });
            set_draw_color(if *canvas_rx.borrow() { rx_inactive } else { rx_active });
            draw_text("RX", wid.x() + INDICATOR_PADDING + 10, wid.y() + INDICATOR_PADDING + 18);

            draw_rect_fill(wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING, wid.y() + INDICATOR_PADDING, WAIT_INDICATOR_WIDTH, wid.height() - 2 * INDICATOR_PADDING, 
                           if *canvas_wait.borrow() { wait_active } else { wait_inactive });
            set_draw_color(if *canvas_wait.borrow() { wait_inactive } else { wait_active });
            draw_text("WAIT", wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING + 12, wid.y() + INDICATOR_PADDING + 18);

            draw_rect_fill(wid.x() + INDICATOR_PADDING + RX_INDICATOR_WIDTH + INDICATOR_PADDING + WAIT_INDICATOR_WIDTH + INDICATOR_PADDING, wid.y() + INDICATOR_PADDING, TX_INDICATOR_WIDTH, wid.height() - 2 * INDICATOR_PADDING, 
                           if *canvas_tx.borrow() { tx_active } else { tx_inactive });
            set_draw_color(if *canvas_tx.borrow() { tx_inactive } else { tx_active });
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
        gui.code_speed_output.set_readonly(true);

        let entry_prompt = "Enter message,\nthen RETURN to send.";
        // TODO set an inner padding?
        gui.text_entry.borrow_mut().set_color(window_background.lighter());
        gui.text_entry.borrow_mut().set_wrap(true);
        gui.text_entry.borrow_mut().set_align(Align::TopLeft);
        gui.text_entry.borrow_mut().set_tooltip(entry_prompt);
        gui.text_entry.borrow_mut().insert(entry_prompt).unwrap();
        gui.text_entry.borrow_mut().set_trigger(CallbackTrigger::EnterKey);
        gui.text_entry.borrow_mut().handle(move |widget, event| {
            if event == Event::Focus {
                // Clear out the initial prompt text.
                let contents = widget.value();
                if contents == *entry_prompt {
                    widget.set_value("");
                }
            }
            true
        });
        let text_entry_sender = gui.sender.clone();
        gui.text_entry.borrow_mut().set_callback(move |text_entry| {
            let contents = text_entry.value();
            let trimmed_contents = contents.trim();
            text_entry.set_value("");
            if !trimmed_contents.is_empty() {
                text_entry_sender.send(Message::KeyingText ( KeyingText { text: trimmed_contents.to_owned() } ));
            }
        });

        wind.set_size(gui.window_width, gui.window_height);
        wind.set_color(window_background);

        // Functions called on the GUI by the rest of the system...
        let thread_gui_sender = gui.sender.clone();
        let thread_handle = thread::spawn(move || {
            loop {
                if thread_terminate.load(Ordering::SeqCst) {
                    info!("Terminating GUI input thread");
                    break;
                }

                if let Ok(gui_input_message) = gui_input_rx.recv_timeout(Duration::from_millis(250)) {
                    match gui_input_message {
                        GUIInputMessage::SetRxIndicator(state) => {
                            thread_gui_sender.send(Message::SetRxIndicator(state));
                        }
                        GUIInputMessage::SetWaitIndicator(state) => {
                            thread_gui_sender.send(Message::SetWaitIndicator(state));
                        }
                        GUIInputMessage::SetTxIndicator(state) => {
                            thread_gui_sender.send(Message::SetTxIndicator(state));
                        }
                    }
                }
             }
        });

        *gui.thread_handle.lock().unwrap() = Some(thread_handle);

        wind.end();
        debug!("Showing main window");
        wind.show();
        debug!("Starting app wait loop");
        gui
    }

    pub fn main_window_dimensions(&self) -> (i32, i32) {
        (self.window_width, self.window_height)
    }

    pub fn message_handle(&mut self) {
        match self.receiver.recv() {
            None => {
                // noop
            }
            Some(message) => {
                info!("App message {:?}", message);
                match message {
                    Message::KeyingText(keying_text) => {
                        info!("Sending the text [{}]", keying_text.text);
                        self.gui_output.lock().unwrap().encode_and_send_text(keying_text.text);
                        info!("Text sent");
                    }

                    Message::Beep => {}

                    Message::SetKeyingSpeed(_) => {}

                    Message::IncreaseKeyingSpeedRequest => {
                        let new_keyer_speed = self.gui_output.lock().unwrap().get_keyer_speed();
                        info!("Initial speed is {}", new_keyer_speed);
                        if new_keyer_speed < MAX_KEYER_SPEED {
                            self.set_keyer_speed(new_keyer_speed + 1);
                        } else {
                            self.gui_output.lock().unwrap().warning_beep();
                        }
                    }

                    Message::DecreaseKeyingSpeedRequest => {
                        let new_keyer_speed = self.gui_output.lock().unwrap().get_keyer_speed();
                        info!("Initial speed is {}", new_keyer_speed);
                        if new_keyer_speed > MIN_KEYER_SPEED {
                            self.set_keyer_speed(new_keyer_speed - 1);
                        } else {
                            self.gui_output.lock().unwrap().warning_beep();
                        }
                    }

                    Message::RedrawIndicatorsCanvas => {
                        self.indicators_canvas.redraw();
                    }

                    Message::SetRxIndicator(state) => {
                        info!("Will set RX indicator to {}", state);
                        *self.rx_indicator.borrow_mut() = state;
                        self.indicators_canvas.redraw();
                    }

                    Message::SetWaitIndicator(state) => {
                        info!("Will set WAIT indicator to {}", state);
                        *self.wait_indicator.borrow_mut() = state;
                        self.indicators_canvas.redraw();
                    }

                    Message::SetTxIndicator(state) => {
                        info!("Will set TX indicator to {}", state);
                        *self.tx_indicator.borrow_mut() = state;
                        self.indicators_canvas.redraw();
                    }

                }
            }
        }
    }

    pub fn gui_input_sender(&self) -> Arc<mpsc::SyncSender<GUIInputMessage>> {
        self.gui_input_tx.clone()
    }

    fn set_keyer_speed(&mut self, new_keyer_speed: u8) {
        info!("Setting keyer speed to {}", new_keyer_speed);
        self.gui_output.lock().unwrap().set_keyer_speed(new_keyer_speed);
        self.config.lock().unwrap().set_wpm(new_keyer_speed as usize).unwrap();
        self.code_speed_output.set_value(new_keyer_speed.to_string().as_str());
    }
}

