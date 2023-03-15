
// use fltk::{
//     app::*, button::*, draw::*, enums::*, menu::*, prelude::*, valuator::*, widget::*, window::*,
// };
// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::ops::{Deref, DerefMut};
// use std::rc::Rc;
use crate::libs::application::application::Application;
use crate::libs::config_file::config_file::ConfigurationStore;

// const WIDGET_WIDTH: i32 = 70;
// const WIDGET_HEIGHT: i32 = 25;
// const WIDGET_PADDING: i32 = 10;
//
// const CANVAS_WIDTH: i32 = 350;
// const CANVAS_HEIGHT: i32 = 250;

pub fn initialise(_config: &mut ConfigurationStore, _application: &mut Application) -> () {

}

#[cfg(test)]
#[path = "./gui_spec.rs"]
mod gui_spec;
