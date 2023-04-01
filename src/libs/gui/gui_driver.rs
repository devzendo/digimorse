use fltk::{
    app::*, button::{*, self}, draw::*, enums::*, group::*, /*menu::*,*/ prelude::*, /*valuator::*,*/ widget::*, window::*,
};
use fltk::{prelude::*, *};

use std::sync::{Arc, Mutex};
use log::debug;
use crate::libs::gui::gui_facades::GUIInput;

const WIDGET_SIZE: i32 = 25;
const WIDGET_PADDING: i32 = 10;

enum Message {
    ToggleRx(bool),
    ToggleWait(bool),
    ToggleTx(bool),
}

pub struct GuiDriver {
    gui_input: Arc<Mutex<dyn GUIInput>>,
}
impl GuiDriver {
    pub fn new(gui_input: Arc<Mutex<dyn GUIInput>>, x_position: i32) -> Self {
        debug!("Initialising Window");
        let mut wind = Window::default().with_label("digimorse test");
        wind.set_size(300, 300);
        wind.set_pos(x_position, 0);
        //let (sender, receiver) = channel::<Message>();

        let flex = group::Flex::default().with_size(300, 300).column().center_of_parent();

        
        let mut toggle_rx_checkbox = button::CheckButton::default().
            with_label("RX").
            with_size(WIDGET_SIZE, WIDGET_SIZE).
            with_pos(WIDGET_PADDING, WIDGET_PADDING);
        let rx_gui_input = gui_input.clone();
        toggle_rx_checkbox.set_callback(move |wid| {
            debug!("RX checkbox toggled");
            rx_gui_input.lock().unwrap().set_rx_indicator(wid.value());
        });

        let mut toggle_wait_checkbox = button::CheckButton::default().
            with_label("Wait").
            with_size(WIDGET_SIZE, WIDGET_SIZE).
            with_pos(WIDGET_PADDING, WIDGET_PADDING * 2);
        let wait_gui_input = gui_input.clone();
        toggle_wait_checkbox.set_callback(move |wid| {
            debug!("WAIT checkbox toggled");
            wait_gui_input.lock().unwrap().set_wait_indicator(wid.value());
        });

        let mut toggle_tx_checkbox = button::CheckButton::default().
            with_label("TX").
            with_size(WIDGET_SIZE, WIDGET_SIZE).
            with_pos(WIDGET_PADDING, WIDGET_PADDING * 3);
        let tx_gui_input = gui_input.clone();
        toggle_tx_checkbox.set_callback(move |wid| {
            debug!("TX checkbox toggled");
            tx_gui_input.lock().unwrap().set_tx_indicator(wid.value());
        });

        flex.end();
        wind.end();
        wind.show();

        Self {
            gui_input
        }
    }
}


