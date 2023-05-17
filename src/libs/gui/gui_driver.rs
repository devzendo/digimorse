use fltk::{
    button::CheckButton, group::Flex, prelude::*, window::*,
};
use std::sync::Arc;
use std::sync::mpsc::SyncSender;
use log::debug;

use super::gui_facades::GUIInputMessage;

const WIDGET_SIZE: i32 = 25;
const WIDGET_PADDING: i32 = 10;

pub struct GuiDriver {
}
impl GuiDriver {
    pub fn new(gui_input: Arc<SyncSender<GUIInputMessage>>, x_position: i32) -> Self {
        debug!("Initialising Window");
        let mut wind = Window::default().with_label("digimorse test");
        wind.set_size(300, 300);
        wind.set_pos(x_position, 0);

        let flex = Flex::default().with_size(300, 300).column().center_of_parent();
        
        let mut toggle_rx_checkbox = CheckButton::default().
            with_label("RX").
            with_size(WIDGET_SIZE, WIDGET_SIZE).
            with_pos(WIDGET_PADDING, WIDGET_PADDING);
        let rx_gui_input = gui_input.clone();
        toggle_rx_checkbox.set_callback(move |wid| {
            debug!("RX checkbox toggled");
            rx_gui_input.send(GUIInputMessage::SetRxIndicator(wid.value())).unwrap();
        });

        let mut toggle_wait_checkbox = CheckButton::default().
            with_label("Wait").
            with_size(WIDGET_SIZE, WIDGET_SIZE).
            with_pos(WIDGET_PADDING, WIDGET_PADDING * 2);
        let wait_gui_input = gui_input.clone();
        toggle_wait_checkbox.set_callback(move |wid| {
            debug!("WAIT checkbox toggled");
            wait_gui_input.send(GUIInputMessage::SetWaitIndicator(wid.value())).unwrap();
        });

        let mut toggle_tx_checkbox = CheckButton::default().
            with_label("TX").
            with_size(WIDGET_SIZE, WIDGET_SIZE).
            with_pos(WIDGET_PADDING, WIDGET_PADDING * 3);
        let tx_gui_input = gui_input.clone();
        toggle_tx_checkbox.set_callback(move |wid| {
            debug!("TX checkbox toggled");
            tx_gui_input.send(GUIInputMessage::SetTxIndicator(wid.value())).unwrap();
        });

        flex.end();
        wind.end();
        wind.show();

        Self {
        }
    }
}


