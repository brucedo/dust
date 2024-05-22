use core::panic;
use std::{
    thread::{self, sleep},
    u32,
};

type Point = (i32, i32);
type Rect = (u32, u32);

use log::{debug, warn};
use xcb::{
    x::{self, Atom, ConfigWindow, Cw, EventMask, GetKeyboardMapping, MapWindow, Window},
    xinput::{ListDeviceProperties, ListInputDevices},
    xkb::{GetDeviceInfo, XiFeature},
    Connection, Error, Extension,
};

pub fn connect() -> (Connection, i32) {
    let ext = [
        Extension::Dri2,
        Extension::Dri3,
        Extension::RandR,
        Extension::Xkb,
        Extension::Input,
    ];
    match xcb::Connection::connect_with_extensions(None, &ext, &[]) {
        Ok((conn, screen_num)) => {
            debug!("Connected to screen number {}", screen_num);
            (conn, screen_num)
        }
        Err(_) => {
            panic!("Connection attempt failed.  oooh noeeees");
        }
    }
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
        x: 0,
        y: 0,
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

pub fn event_loop(conn: Connection) {
    loop {
        match conn.wait_for_event() {
            Ok(event) => {
                debug!("Event received: {:?}", event)
            }
            Err(msg) => {
                debug!("woops there it is: {:?}", msg);
                break;
            }
        }
    }
}

pub fn interrogate_keymaps(conn: &Connection) {
    let list_device_req = ListInputDevices {};

    let list_cookie = conn.send_request(&list_device_req);

    let min_keycode = conn.get_setup().min_keycode();
    let count = conn.get_setup().max_keycode() - min_keycode + 1;

    let keycode_cookie = conn.send_request(&GetKeyboardMapping {
        count,
        first_keycode: min_keycode,
    });

    match conn.wait_for_reply(keycode_cookie) {
        Ok(sym_list) => {
            debug!("Keysyms per keycode: {}", sym_list.keysyms_per_keycode());
            for index in 0..count {
                let keycode = index + min_keycode;
                let keysyms = sym_list.keysyms
            }
        }
        Err(msg) => {
            panic!("Unable to retrieve keycode->keysym conversion list");
        }
    }

    match conn.wait_for_reply(list_cookie) {
        Ok(devices) => {
            debug!("List of captured devices: ");
            for device in devices.devices() {
                debug!("├ {:?}", device);
                let prop_cookie = conn.send_request(&ListDeviceProperties {
                    device_id: device.device_id(),
                });
                match conn.wait_for_reply(prop_cookie) {
                    Ok(props) => {
                        debug!("| ├ Properties for device {:?}", device.device_id());
                        debug!("| ├ XiReplyType: {:?}", props.xi_reply_type());
                        debug!("| └ Properties: {:?}", props.atoms());
                        debug!("| ");
                    }
                    Err(msg) => {
                        panic!("properties error: {:?}", msg);
                    }
                }
                // GetDeviceInfo{device_spec: device.device_id, wanted: XiFeature::all(), all_buttons: true, };
            }
        }
        Err(msg) => {
            panic!("{:?}", msg)
        }
    }
}
