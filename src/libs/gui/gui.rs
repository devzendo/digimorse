use fltk::{
    app::*, button::*, draw::*, enums::*, menu::*, prelude::*, valuator::*, widget::*, window::*,
};
//use std::sync::mpsc::{channel, RecvError};
use fltk::output::Output;
use log::{debug, info, warn};
// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::ops::{Deref, DerefMut};
// use std::rc::Rc;
use crate::libs::application::application::Application;
use crate::libs::config_file::config_file::ConfigurationStore;
use crate::libs::gui::message::Message;
use crate::libs::util::version::VERSION;

const WIDGET_WIDTH: i32 = 70;
const WIDGET_HEIGHT: i32 = 25;
const WIDGET_PADDING: i32 = 10;

const WATERFALL_WIDTH: i32 = 350;
const WATERFALL_HEIGHT: i32 = 250;

struct Gui {
    waterfallCanvas: Widget,
    statusOutput: Output,
}
pub fn initialise(_config: &mut ConfigurationStore, _application: &mut Application) -> () {
    debug!("Initialising App");
    let app = App::default().with_scheme(Scheme::Gtk);
    debug!("Initialising Window");
    let mut wind = Window::default().with_label(format!("digimorse v{}", VERSION).as_str());

    // move this to the application
    let (sender, receiver) = channel::<Message>();

    let mut gui = Gui {
        waterfallCanvas: Widget::new(WIDGET_PADDING, WIDGET_PADDING, WATERFALL_WIDTH, WATERFALL_HEIGHT, ""),
        statusOutput: Output::default()
            .with_size(WATERFALL_WIDTH, WIDGET_HEIGHT)
            .with_pos(WIDGET_PADDING, WIDGET_PADDING + WATERFALL_HEIGHT + WIDGET_PADDING),
    };
    gui.waterfallCanvas.set_trigger(CallbackTrigger::Release);
    gui.waterfallCanvas.set_color(gui.waterfallCanvas.color().darker());
    gui.statusOutput.set_value("status message");
    wind.set_size(
        WATERFALL_WIDTH + 2*WIDGET_PADDING,
        WATERFALL_HEIGHT + WIDGET_HEIGHT + 3*WIDGET_PADDING,
    );
    wind.end();
    debug!("Showing main window");
    wind.show();
    debug!("Starting app wait loop");
    while app.wait() {
        debug!("app wait has returned true");
        match receiver.recv() {
            // Some(Message::Create) => {
            //     //model.push(formatted_name());
            //     sender.send(Message::Filter);
            // }
            // None => {}
            // Ok(message) => {
            //     info!("App message {:?}", message);
            // }
            // Err(err) => {
            //     warn!("App error {}", err);
            // }
            None => {
                warn!("Got None");
            }
            Some(message) => {
                info!("App message {:?}", message);
            }
        }
    }
    debug!("Out of app wait loop");
}
