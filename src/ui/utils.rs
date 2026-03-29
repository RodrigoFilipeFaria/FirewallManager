use adw::prelude::*;
use gtk::prelude::*;

pub fn show_toast(overlay: &adw::ToastOverlay, message: &str) {
    let toast = adw::Toast::builder()
        .title(message)
        .timeout(3)
        .build();
    overlay.add_toast(toast);
}

pub fn clear_listbox(listbox: &gtk::ListBox) {
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }
}

pub fn parse_ports(input: &str) -> Vec<(String, String)> {
    input
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            let mut parts = s.splitn(2, '/');
            let port = parts.next()?.trim().to_string();
            let proto = parts.next().unwrap_or("tcp").trim().to_string();
            if port.is_empty() { None } else { Some((port, proto)) }
        })
        .collect()
}
