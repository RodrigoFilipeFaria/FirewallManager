use adw::prelude::*;

use gtk::glib;
use crate::backend::FirewallClient;
use crate::ui::utils::{show_toast, clear_listbox, parse_ports};

pub fn init_services(
    client: FirewallClient,
    toast_overlay: adw::ToastOverlay,
    services_listbox: gtk::ListBox,
    add_service_button: gtk::Button,
    services_search_entry: gtk::SearchEntry,
) {
    let client_for_dialog = client.clone();
    let overlay_dialog = toast_overlay.clone();

    let search_entry_clone = services_search_entry.clone();
    services_listbox.set_filter_func(move |row| {
        if let Some(action_row) = row.downcast_ref::<adw::ActionRow>() {
            let title = action_row.title().to_string().to_lowercase();
            let query = search_entry_clone.text().to_string().to_lowercase();
            title.contains(&query)
        } else {
            true
        }
    });

    let listbox_search = services_listbox.clone();
    services_search_entry.connect_search_changed(move |_| {
        listbox_search.invalidate_filter();
    });

    add_service_button.connect_clicked(glib::clone!(
        #[strong] services_listbox,
        move |_| show_add_service_dialog(client_for_dialog.clone(), services_listbox.clone(), overlay_dialog.clone())
    ));
}

pub async fn reload_services(client: &FirewallClient, listbox: &gtk::ListBox, toast_overlay: &adw::ToastOverlay) {
    clear_listbox(listbox);
    match client.fetch_services().await {
        Ok(services) => {
            for svc in services {
                let row = build_service_row(client, &svc, listbox, toast_overlay);
                listbox.append(&row);
            }
        }
        Err(e) => {
            show_toast(toast_overlay, &format!("Error loading services: {}", e));
        }
    }
}

pub fn build_service_row(
    client: &FirewallClient,
    service_name: &str,
    services_listbox: &gtk::ListBox,
    toast_overlay: &adw::ToastOverlay,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(service_name)
        .activatable(false)
        .build();

    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text("Edit service")
        .css_classes(vec!["flat".to_string()])
        .build();

    let svc_name_edit = service_name.to_string();
    let list_edit = services_listbox.clone();
    let client_edit = client.clone();
    let overlay_edit = toast_overlay.clone();

    edit_btn.connect_clicked(move |_| {
        let sn = svc_name_edit.clone();
        let list = list_edit.clone();
        let c = client_edit.clone();
        let overlay = overlay_edit.clone();

        glib::spawn_future_local(async move {
            match c.fetch_service_settings(&sn).await {
                Ok((_ver, _name, desc, ports, _mods, _dests, _includes, _src_ports)) => {
                    show_edit_service_dialog(c.clone(), sn, desc, ports, list, overlay);
                }
                Err(e) => show_toast(&overlay, &format!("Error fetching service settings: {e}")),
            }
        });
    });

    let remove_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text("Remove service")
        .css_classes(vec!["flat".to_string(), "destructive-action".to_string()])
        .build();

    let svc_name_rm = service_name.to_string();
    let list_rm = services_listbox.clone();
    let client_rm = client.clone();
    let overlay_rm = toast_overlay.clone();

    remove_btn.connect_clicked(move |_| {
        let sn = svc_name_rm.clone();
        let list = list_rm.clone();
        let c = client_rm.clone();
        let overlay = overlay_rm.clone();

        let dialog = adw::AlertDialog::builder()
            .heading("Remove service?")
            .body(&format!("Are you sure you want to permanently remove \"{}\"?", sn))
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("remove", "Remove");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        glib::spawn_future_local(async move {
            let response = dialog.choose_future(&list).await;
            if response == "remove" {
                match c.remove_service(&sn).await {
                    Ok(_) => reload_services(&c, &list, &overlay).await,
                    Err(e) => show_toast(&overlay, &format!("Failed to remove service: {e}")),
                }
            }
        });
    });

    let is_perm = client.is_permanent_mode();
    edit_btn.set_sensitive(is_perm);
    remove_btn.set_sensitive(is_perm);

    if !is_perm {
        edit_btn.set_tooltip_text(Some("Switch to Permanent mode to edit"));
        remove_btn.set_tooltip_text(Some("Switch to Permanent mode to remove"));
    }

    row.add_suffix(&edit_btn);
    row.add_suffix(&remove_btn);
    row
}

pub fn show_add_service_dialog(client: FirewallClient, listbox: gtk::ListBox, toast_overlay: adw::ToastOverlay) {
    let dialog = adw::AlertDialog::builder()
        .heading("Add Service")
        .build();

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("add", "Add");
    dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("add"));
    dialog.set_close_response("cancel");

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let name_entry = adw::EntryRow::builder().title("Service name").build();
    let desc_entry = adw::EntryRow::builder().title("Description").build();
    let ports_entry = adw::EntryRow::builder()
        .title("Ports (e.g. 80/tcp, 443/tcp)")
        .build();

    content_box.append(&name_entry);
    content_box.append(&desc_entry);
    content_box.append(&ports_entry);
    dialog.set_extra_child(Some(&content_box));

    glib::spawn_future_local(async move {
        let response = dialog.choose_future(&listbox).await;
        if response == "add" {
            let name = name_entry.text().to_string();
            let desc = desc_entry.text().to_string();
            let ports = parse_ports(&ports_entry.text());
            match client.add_service(&name, &desc, ports).await {
                Ok(_) => reload_services(&client, &listbox, &toast_overlay).await,
                Err(e) => show_toast(&toast_overlay, &format!("Failed to add service: {e}")),
            }
        }
    });
}

pub fn show_edit_service_dialog(
    client: FirewallClient,
    service_name: String,
    current_desc: String,
    current_ports: Vec<(String, String)>,
    listbox: gtk::ListBox,
    toast_overlay: adw::ToastOverlay,
) {
    let dialog = adw::AlertDialog::builder()
        .heading(&format!("Edit \"{}\"", service_name))
        .build();

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("save", "Save");
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("cancel");

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let desc_entry = adw::EntryRow::builder().title("Description").build();
    desc_entry.set_text(&current_desc);

    let ports_str = current_ports
        .iter()
        .map(|(p, pr)| format!("{}/{}", p, pr))
        .collect::<Vec<_>>()
        .join(", ");
    let ports_entry = adw::EntryRow::builder()
        .title("Ports (e.g. 80/tcp, 443/tcp)")
        .build();
    ports_entry.set_text(&ports_str);

    content_box.append(&desc_entry);
    content_box.append(&ports_entry);
    dialog.set_extra_child(Some(&content_box));

    glib::spawn_future_local(async move {
        let response = dialog.choose_future(&listbox).await;
        if response == "save" {
            let new_desc = desc_entry.text().to_string();
            let new_ports = parse_ports(&ports_entry.text());
            match client.edit_service(&service_name, &new_desc, new_ports).await {
                Ok(_) => reload_services(&client, &listbox, &toast_overlay).await,
                Err(e) => show_toast(&toast_overlay, &format!("Failed to edit service: {e}")),
            }
        }
    });
}
