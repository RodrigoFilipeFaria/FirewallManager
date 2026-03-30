use adw::prelude::*;

use gtk::glib;
use crate::backend::FirewallClient;
use crate::ui::utils::{show_toast, clear_listbox};

pub fn setup_firewall_state(
    client: FirewallClient,
    toast_overlay: adw::ToastOverlay,
    state_label: gtk::Label,
    load_button: gtk::Button,
    status_page: adw::StatusPage,
    interfaces_listbox: gtk::ListBox,
) {
    glib::spawn_future_local(async move {
        match client.fetch_state().await {
            Ok(state) => {
                state_label.set_label(&format!("Service state: {}", state));
                update_firewall_button(&load_button, state.trim().to_lowercase() == "running");
            }
            Err(_) => {
                state_label.set_label("Service state: stopped");
                update_firewall_button(&load_button, false);
            }
        }

        match client.fetch_default_zone().await {
            Ok(zone) => status_page.set_description(Some(&format!("Fallback Zone: {}", zone))),
            Err(e) => status_page.set_description(Some(&format!("Error reading zone: {}", e))),
        }

        reload_interfaces(&client, &interfaces_listbox, &toast_overlay).await;
    });
}

pub fn update_firewall_button(button: &gtk::Button, is_running: bool) {
    button.set_sensitive(true);
    if is_running {
        button.set_label("Disable Firewall");
        button.remove_css_class("suggested-action");
        button.add_css_class("destructive-action");
    } else {
        button.set_label("Enable Firewall");
        button.remove_css_class("destructive-action");
        button.add_css_class("suggested-action");
    }
}

pub async fn reload_interfaces(client: &FirewallClient, listbox: &gtk::ListBox, toast_overlay: &adw::ToastOverlay) {
    clear_listbox(listbox);

    match client.fetch_interfaces().await {
        Ok(interfaces) => {
            if interfaces.is_empty() {
                let empty_row = adw::ActionRow::builder()
                    .title("No interfaces found.")
                    .build();
                listbox.append(&empty_row);
                return;
            }

            for iface in interfaces {
                let current_zone = client.fetch_zone_of_interface(&iface)
                    .await
                    .unwrap_or_else(|e| {
                        show_toast(toast_overlay, &format!("Error fetching zone for {iface}: {e}"));
                        "Unknown".to_string()
                    });

                let row = adw::ActionRow::builder()
                    .title(&iface)
                    .subtitle(&format!("Current Zone: {}", current_zone))
                    .activatable(false)
                    .build();

                let change_btn = gtk::Button::builder()
                    .label("Change Zone")
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();

                let list_clone = listbox.clone();
                let iface_clone = iface.clone();
                let zone_clone = current_zone.clone();
                let client_clone = client.clone();
                let overlay_clone = toast_overlay.clone();

                change_btn.connect_clicked(move |_| {
                    show_change_zone_dialog(client_clone.clone(), iface_clone.clone(), zone_clone.clone(), list_clone.clone(), overlay_clone.clone());
                });

                row.add_suffix(&change_btn);
                listbox.append(&row);
            }
        }
        Err(e) => {
            show_toast(toast_overlay, &format!("Error fetching interfaces: {}", e));
        }
    }
}

pub fn show_change_zone_dialog(client: FirewallClient, interface: String, current_zone: String, listbox: gtk::ListBox, toast_overlay: adw::ToastOverlay) {
    glib::spawn_future_local(async move {
        let zones = client.fetch_zones().await.unwrap_or_default();

        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Change Zone for {}", interface))
            .build();

        let zone_strs: Vec<&str> = zones.iter().map(|s| s.as_str()).collect();
        let combo = gtk::DropDown::from_strings(&zone_strs);

        if let Some(pos) = zones.iter().position(|z| z == &current_zone) {
            combo.set_selected(u32::try_from(pos).unwrap_or(0));
        }

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8).margin_bottom(8).margin_start(8).margin_end(8)
            .build();

        let label = gtk::Label::builder()
            .label("Select a new zone:")
            .halign(gtk::Align::Start)
            .margin_bottom(8)
            .build();

        content_box.append(&label);
        content_box.append(&combo);
        dialog.set_extra_child(Some(&content_box));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("save", "Save");
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");

        let response = dialog.choose_future(&listbox).await;
        if response == "save" {
            let selected_idx = combo.selected() as usize;
            if let Some(new_zone) = zones.get(selected_idx) {
                match client.change_zone_interface(new_zone, &interface).await {
                    Ok(_) => reload_interfaces(&client, &listbox, &toast_overlay).await,
                    Err(e) => show_toast(&toast_overlay, &format!("Failed to change zone: {}", e)),
                }
            }
        }
    });
}
