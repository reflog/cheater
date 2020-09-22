pub(crate) fn caseless_replace(haystack: &str, needle: &str, replacement: &str) -> String {
    let r = format!("(?i){}", needle);
    let re = regex::Regex::new(&r).unwrap();
    return re.replace_all(haystack, replacement).to_string();
}

pub(crate) fn clean_i3_shortcut(s: String) -> String {
    let mut ss = caseless_replace(&s, "Super", "<Super>");
    ss = caseless_replace(&ss, "Shift", "<Shift>");
    ss = caseless_replace(&ss, "Space", "space");
    ss = caseless_replace(&ss, "Enter", "Return");
    return ss;
}

pub(crate) fn get_proc_and_focused_window_pid() -> Result<(String, String), String> {
    use byteorder::{LittleEndian, ReadBytesExt};

    let (conn, screen_num) = xcb::Connection::connect(None)
        .map_err(|error| format!("Unable to open X11 connection: {}.", error))?;

    let active_window_atom_cookie = xcb::intern_atom(&conn, false, "_NET_ACTIVE_WINDOW");
    let pid_atom_cookie = xcb::intern_atom(&conn, false, "_NET_WM_PID");

    //get window
    let root = conn
        .get_setup()
        .roots()
        .nth(screen_num as usize)
        .ok_or_else(|| "Unable to select current screen.".to_string())?
        .root();

    let active_window_atom = active_window_atom_cookie
        .get_reply()
        .map_err(|error| format!("Unable to retrieve _NET_ACTIVE_WINDOW atom: {}.", error))?
        .atom();

    let reply = xcb::get_property(
        &conn,
        false,
        root,
        active_window_atom,
        xcb::ATOM_WINDOW,
        0,
        1,
    )
    .get_reply()
    .map_err(|error| {
        format!(
            "Unable to retrieve _NET_ACTIVE_WINDOW property from root: {}.",
            error
        )
    })?;
    if reply.value_len() == 0 {
        return Err("Unable to retrieve _NET_ACTIVE_WINDOW property from root.".to_string());
    }
    assert_eq!(reply.value_len(), 1);
    let mut raw = reply.value();
    assert_eq!(
        raw.len(),
        4,
        "_NET_ACTIVE_WINDOW property is expected to be at least 4 bytes."
    );
    let window = raw.read_u32::<LittleEndian>().unwrap() as xcb::Window;
    if window == xcb::WINDOW_NONE {
        return Err("No window is focused".to_string());
    }

    let mut reply = xcb::get_property(
        &conn,
        false,
        window,
        xcb::ATOM_WM_CLASS,
        xcb::ATOM_STRING,
        0,
        64,
    )
    .get_reply()
    .unwrap_or_else(|error| {
        panic!(
            "Unable to retrieve WM_CLASS from focused window {}: {}",
            window, error
        )
    });
    let class = String::from_utf8(
        reply
            .value()
            .iter()
            .cloned()
            .take_while(|c| *c != 0u8)
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|error| panic!("Unable to decode {:#?}: {}", reply.value() as &[u8], error));
    let pid_atom = pid_atom_cookie
        .get_reply()
        .map_err(|error| format!("Unable to retrieve _NET_WM_PID: {}.", error))?
        .atom();

    let ureply =
        xcb::get_property(&conn, false, window, pid_atom, xcb::ATOM_CARDINAL, 0, 1).get_reply();
    if let Err(e) = ureply {
        return Err(e.to_string());
    }
    reply = ureply.unwrap();
    assert_eq!(reply.value_len(), 1);
    raw = reply.value();
    //open proc
    let pid = raw.read_u32::<LittleEndian>().unwrap();
    let proc = std::fs::read_to_string(format!("/proc/{}/comm", pid)).map_err(|e| e.to_string())?;

    Ok((proc.trim().to_string(), class.to_string()))
}
