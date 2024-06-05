use core::panic;
use std::sync::mpsc::SyncSender;
use std::{time::Duration, u32};

type Point = (i32, i32);
type Rect = (u32, u32);

use log::debug;
use xcb::{
    x::{self, ConfigWindow, Cw, Event, EventMask, Keycode, MapWindow, Window},
    xinput::KeyCode,
    xkb::{GetDeviceInfo, UseExtension, XiFeature},
    Connection, Extension,
};
use xkbcommon::xkb::{self, Keymap, Keysym};

use crate::input::input::KeyStroke;

pub fn connect() -> (Connection, i32) {
    let ext = [
        Extension::Dri2,
        Extension::Dri3,
        Extension::RandR,
        Extension::Xkb,
        Extension::Input,
    ];
    let conn = match xcb::Connection::connect_with_extensions(None, &ext, &[]) {
        Ok((conn, screen_num)) => {
            debug!("Connected to screen number {}", screen_num);
            (conn, screen_num)
        }
        Err(_) => {
            panic!("Connection attempt failed.  oooh noeeees");
        }
    };

    let xkb_cookie = conn.0.send_request(&UseExtension {
        wanted_major: xkb::x11::MIN_MAJOR_XKB_VERSION,
        wanted_minor: xkb::x11::MIN_MINOR_XKB_VERSION,
    });

    match conn.0.wait_for_reply(xkb_cookie) {
        Ok(xkb_support) => {
            debug!("XKB supported? {}", xkb_support.supported());
        }
        Err(msg) => {
            panic!("XKB not supported: {}", msg);
        }
    }

    conn
}

pub fn extension_data(conn: &Connection) {
    debug!("Checking loaded extensions");
    for ext in conn.active_extensions() {
        debug!("{:?}", ext);
    }

    for root in conn.get_setup().roots() {
        debug!("Screen statistics");
        debug!(
            "Screen width x height (px):   {} x {}",
            root.width_in_pixels(),
            root.height_in_pixels()
        );
        debug!(
            "Screen width x height (mm)    {} x {}",
            root.width_in_millimeters(),
            root.height_in_millimeters()
        );
        debug!("Allowed depths: ");
        root.allowed_depths()
            .for_each(|depth| debug!("\t{:?}", depth));
    }
}

pub fn resize_window(conn: &Connection, window_id: Window, upper_left: Point, dim: Rect) {
    let net_wm_win_state_cookie = conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: b"_NET_WM_STATE",
    });
    let net_wm_win_type = conn.wait_for_reply(net_wm_win_state_cookie).unwrap().atom();
    let net_wm_win_state_fs_cookie = conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: b"_NET_WM_STATE_FULLSCREEN",
    });
    let net_wm_win_state_fs = conn
        .wait_for_reply(net_wm_win_state_fs_cookie)
        .unwrap()
        .atom();

    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: window_id,
        property: net_wm_win_type,
        r#type: x::ATOM_ATOM,
        data: &[net_wm_win_state_fs],
    });

    conn.send_request(&x::ConfigureWindow {
        window: window_id,
        value_list: &[
            ConfigWindow::X(upper_left.0),
            ConfigWindow::Y(upper_left.1),
            ConfigWindow::Width(dim.0),
            ConfigWindow::Height(dim.1),
        ],
    });

    conn.send_request_checked(&MapWindow { window: window_id });

    match conn.flush() {
        Ok(_) => {
            debug!("flush was successful")
        }
        Err(msg) => {
            debug!("Flush failed?  {:?}", msg)
        }
    }

    let confirm_change = x::GetGeometry {
        drawable: x::Drawable::Window(window_id),
    };

    debug!("Attempting to confirm resize command was successful.");
    debug!(
        "Checking against           ({}, {}), {}x{}",
        upper_left.0, upper_left.1, dim.0, dim.1
    );
    loop {
        let cookie = conn.send_request(&confirm_change);

        match conn.wait_for_reply(cookie) {
            Ok(geom_reply) => {
                let (x, y, width, height) = (
                    geom_reply.x(),
                    geom_reply.y(),
                    geom_reply.width(),
                    geom_reply.height(),
                );
                debug!(
                    "Window is currently     ({}, {}), {}x{}",
                    x, y, width, height
                );
                // The x and y coordinates do not behave as expected on a multi-monitor display.
                // The reasons for this are, as yet, unclear.  What I do know is that the
                // update command _does_ change the x and y axis - they will show initially as
                // (10, 10) as per the create_window func, but then change immediately to (0, 0).
                // The window manager is clearly doing some piddling about with the window
                // upper left coordinatges - possibly because it is fullscreen?
                // Regardless, we will remove checking for the upper left coordiantes.
                // if x as i32 == upper_left.0 &&
                // y as i32 == upper_left.1 &&
                if width as u32 == dim.0 && height as u32 == dim.1 {
                    break;
                } else {
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
            Err(msg) => {
                panic!("Trying to check the update status of the geometry failed.")
            }
        }
    }

    debug!("Window resized.");
}

fn deconstruct_parent(conn: &Connection, display_num: &i32) -> (Window, u32, u8) {
    if let Some(root) = conn
        .get_setup()
        .roots()
        .nth(display_num.unsigned_abs() as usize)
    {
        (root.root(), root.root_visual(), root.root_depth())
    } else {
        panic!("Unable to capture parent window data");
    }
}

pub fn create_window(conn: &Connection, display_num: i32) -> Window {
    let (parent_win, parent_vis, parent_depth) = deconstruct_parent(conn, &display_num);
    let window_id: x::Window = conn.generate_id();

    let our_window = x::CreateWindow {
        depth: parent_depth,
        wid: window_id,
        parent: parent_win,
        x: 10,
        y: 10,
        width: 1,
        height: 1,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: parent_vis,
        value_list: &[
            Cw::BackPixel(0x00555555),
            Cw::EventMask(
                EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::POINTER_MOTION,
            ),
        ],
    };

    conn.send_request_checked(&our_window);
    conn.flush();

    window_id
}

pub fn interrogate_randr(conn: &Connection, window_id: Window) -> (Point, Rect) {
    let monitor_cookie = conn.send_request(&xcb::randr::GetMonitors {
        window: window_id,
        get_active: true,
    });

    match conn.wait_for_reply(monitor_cookie) {
        Ok(reply) => {
            if let Some(primary_monitor) = reply.monitors().find(|monitor| monitor.primary()) {
                debug!("Monitor name: {:?}", primary_monitor.name());
                debug!("Primary? {}", primary_monitor.primary());
                debug!("Automatic? {}", primary_monitor.automatic());
                debug!(
                    "Width x height (px): {} x {}",
                    primary_monitor.width(),
                    primary_monitor.height()
                );
                debug!(
                    "Width x height (mm): {} x {}",
                    primary_monitor.width_in_millimeters(),
                    primary_monitor.height_in_millimeters()
                );
                debug!(
                    "What's this x and y? x: {}, y: {}",
                    primary_monitor.x(),
                    primary_monitor.y()
                );
                (
                    (primary_monitor.x() as i32, primary_monitor.y() as i32),
                    (
                        primary_monitor.width() as u32,
                        primary_monitor.height() as u32,
                    ),
                )
            } else {
                panic!("No monitor flagged as primary.");
            }
        }
        Err(msg) => {
            panic!("Unable to retrieve monitor data for analysis: {:?}", msg);
        }
    }
}

pub fn event_loop(conn: Connection, sender: SyncSender<KeyStroke>) {
    let keymap = interrogate_keymaps(&conn);
    let state = xkb::State::new(&keymap);
    loop {
        match conn.wait_for_event() {
            Ok(event) => {
                match event {
                    xcb::Event::X(Event::KeyPress(key)) => {
                        debug!(
                            "Single key: {:?}",
                            state.key_get_one_sym(xkb::Keycode::new(key.detail() as u32))
                        );
                        match keymap.key_get_name(xkb::Keycode::new(key.detail() as u32)) {
                            Some(sym) => {
                                debug!("Key pressed: {}", sym);
                            }
                            None => {
                                debug!("Key pressed with no corresponding symbol name in the map.");
                            }
                        }
                    }
                    _ => {
                        debug!("Event received: {:?}", event);
                    }
                };
            }
            Err(msg) => {
                debug!("woops there it is: {:?}", msg);
                break;
            }
        }
    }
}

pub fn interrogate_keymaps(conn: &Connection) -> Keymap {
    let xkb_ctxt = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let core_kb_id = xkb::x11::get_core_keyboard_device_id(conn);

    debug!("core keyboard id: {}", core_kb_id);

    xkb::x11::keymap_new_from_device(&xkb_ctxt, conn, 3, xkb::KEYMAP_COMPILE_NO_FLAGS)

    // let min_keycode = conn.get_setup().min_keycode();
    // let count = conn.get_setup().max_keycode() - min_keycode + 1;
    //
    // let keycode_cookie = conn.send_request(&GetKeyboardMapping {
    //     count,
    //     first_keycode: min_keycode,
    // });
    //
    // match conn.wait_for_reply(keycode_cookie) {
    //     Ok(sym_list) => {
    //         let keysyms = sym_list.keysyms();
    //         debug!("Keysyms per keycode: {}", sym_list.keysyms_per_keycode());
    //         debug!("Length of keysyms: {}", keysyms.len());
    //         for index in 0..count {
    //             let keycode = index + min_keycode;
    //             debug!("Keycode: {}", keycode);
    //             let sym_slice: &[u32] =
    //                 &keysyms[((index as usize) * 7)..=((index as usize) * 7 + 6)];
    //             for sym_set in sym_slice {
    //                 debug!("├ {:#034b}", sym_set);
    //             }
    //         }
    //     }
    //     Err(msg) => {
    //         panic!("Unable to retrieve keycode->keysym conversion list");
    //     }
    // }
}

// def translate_keysym(symbol: u32) =>  {
//
// }
