use core::panic;
use std::{str::FromStr, thread::sleep, time::Duration, u32};

type Point = (i32, i32);
type Rect = (u32, u32);

use log::debug;
use xcb::{
    randr::GetMonitorsReply,
    x::{self, Atom, ConfigWindow, Cw, EventMask, MapWindow},
    Connection, Error, Extension,
};

pub fn connect() -> (Connection, i32) {
    let ext = [Extension::Dri2, Extension::Dri3, Extension::RandR];
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

pub fn interrogate_randr(conn: &Connection, display_num: i32) {
    let (parent_win, parent_vis, parent_depth) = if let Some(root) = conn
        .get_setup()
        .roots()
        .nth(display_num.unsigned_abs() as usize)
    {
        (root.root(), root.root_visual(), root.root_depth())
    } else {
        panic!("Unable to capture the parent window!");
    };

    let window_id: x::Window = conn.generate_id();

    let our_window = x::CreateWindow {
        depth: parent_depth,
        wid: window_id,
        parent: parent_win,
        x: 0,
        y: 0,
        width: 600,
        height: 200,
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

    if let Ok((upper_left, window_rect)) = find_best_region(conn, window_id) {
        debug!(
            "find_best_region has given us a bounding box of ({}, {})-({},{})",
            upper_left.0, upper_left.1, window_rect.0, window_rect.1
        );
        conn.send_request(&x::ConfigureWindow {
            window: window_id,
            value_list: &[
                ConfigWindow::X(upper_left.0),
                ConfigWindow::Y(upper_left.1),
                ConfigWindow::Width(window_rect.0),
                ConfigWindow::Height(window_rect.1),
            ],
        });
        match conn.flush() {
            Ok(_) => {
                debug!("flush was successful")
            }
            Err(msg) => {
                debug!("Flush failed?  {:?}", msg)
            }
        }
    }

    debug!("Pausing to see if window manager will process the resize for us...");
    sleep(Duration::from_secs(5));
    debug!("Paused, and now we can check.");

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

    conn.send_request_checked(&MapWindow { window: window_id });

    let geom_cookie = conn.send_request(&x::GetGeometry {
        drawable: x::Drawable::Window(window_id),
    });

    conn.flush();

    if let Ok(geom_response) = conn.wait_for_reply(geom_cookie) {
        debug!("Status of fullscreen window: ");
        debug!(
            "(x, y)/(width, height): ({}, {})/({}, {})",
            geom_response.x(),
            geom_response.y(),
            geom_response.width(),
            geom_response.height()
        );
    }

    sleep(Duration::from_secs(10));

    conn.send_request(&x::DestroyWindow { window: window_id });
}

fn find_best_region(conn: &Connection, window_id: x::Window) -> Result<(Point, Rect), String> {
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
                Ok((
                    (primary_monitor.x() as i32, primary_monitor.y() as i32),
                    (
                        primary_monitor.width() as u32,
                        primary_monitor.height() as u32,
                    ),
                ))
            } else {
                Err(String::from("No monitor flagged as primary."))
            }
        }
        Err(msg) => Err(format!(
            "Unable to retrieve monitor data for analysis: {:?}",
            msg
        )),
    }
}

fn event_loop(conn: Connection, channel: std::sync::mpsc::Sender) {}
