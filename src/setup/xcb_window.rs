use core::panic;
use std::{thread::sleep, time::Duration};

use log::{debug, SetLoggerError};
use xcb::{
    randr::GetMonitorsReply,
    x::{self, Atom, Cw, MapWindow},
    Connection, Extension,
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
        value_list: &[],
    };

    conn.send_request_checked(&our_window);

    let monitor_cookie = conn.send_request(&xcb::randr::GetMonitors {
        window: window_id,
        get_active: true,
    });

    if let Ok(reply) = conn.wait_for_reply(monitor_cookie) {
        for monitor in reply.monitors() {
            debug!("Monitor name: {:?}", monitor.name());
            debug!("Primary? {}", monitor.primary());
            debug!("Automatic? {}", monitor.automatic());
            debug!(
                "Width x height (px): {} x {}",
                monitor.width(),
                monitor.height()
            );
            debug!(
                "Width x height (mm): {} x {}",
                monitor.width_in_millimeters(),
                monitor.height_in_millimeters()
            );
            debug!(
                "What's this x and y? x: {}, y: {}",
                monitor.x(),
                monitor.y()
            )
        }
    }

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
    conn.flush();

    sleep(Duration::from_secs(10));

    conn.send_request(&x::DestroyWindow { window: window_id });
}
