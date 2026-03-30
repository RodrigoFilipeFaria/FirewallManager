use adw::prelude::*;

use gtk::glib;
use std::convert::TryFrom;
use crate::backend::FirewallClient;
use crate::ui::utils::{show_toast, clear_listbox};

pub fn setup_zones_list(
    client: FirewallClient,
    toast_overlay: adw::ToastOverlay,
    zones_listbox: gtk::ListBox,
    main_stack: gtk::Stack,
    settings_listbox: gtk::ListBox,
    details_page: adw::StatusPage,
) {
    glib::spawn_future_local(async move {
        clear_listbox(&zones_listbox);

        match client.fetch_zones().await {
            Ok(zones) => {
                for zone_name in zones {
                    let row = build_zone_row(
                        &client,
                        &zone_name,
                        &main_stack,
                        &settings_listbox,
                        &details_page,
                        &toast_overlay,
                    );
                    zones_listbox.append(&row);
                }
            }
            Err(e) => {
                let error_row = adw::ActionRow::builder()
                    .title(&format!("Error fetching zones: {}", e))
                    .build();
                zones_listbox.append(&error_row);
            }
        }
    });
}

pub fn build_zone_row(
    client: &FirewallClient,
    zone_name: &str,
    main_stack: &gtk::Stack,
    settings_listbox: &gtk::ListBox,
    details_page: &adw::StatusPage,
    toast_overlay: &adw::ToastOverlay,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(zone_name)
        .activatable(true)
        .build();

    let arrow = gtk::Image::from_icon_name("go-next-symbolic");
    row.add_suffix(&arrow);

    let zone_name = zone_name.to_string();
    let stack = main_stack.clone();
    let settings = settings_listbox.clone();
    let details = details_page.clone();
    let client = client.clone();
    let overlay = toast_overlay.clone();

    row.connect_activated(move |_| {
        let zone = zone_name.clone();
        let stack = stack.clone();
        let settings = settings.clone();
        let details = details.clone();
        let client_clone = client.clone();
        let overlay_clone = overlay.clone();

        glib::spawn_future_local(async move {
            load_zone_details(&client_clone, &zone, &stack, &settings, &details, &overlay_clone).await;
        });
    });

    row
}

pub async fn load_zone_details(
    client: &FirewallClient,
    zone_name: &str,
    stack: &gtk::Stack,
    settings_listbox: &gtk::ListBox,
    details_page: &adw::StatusPage,
    toast_overlay: &adw::ToastOverlay,
) {
    clear_listbox(settings_listbox);
    details_page.set_title(&format!("Zone: {}", zone_name));

    match client.fetch_zone_settings(zone_name).await {
        Ok(settings) => {
            for (key, variant) in settings {
                if let Ok(val) = String::try_from(variant.clone()) {
                    let row = adw::ActionRow::builder()
                        .title(&key)
                        .subtitle(&val)
                        .build();
                    settings_listbox.append(&row);
                } else if let Ok(items) = Vec::<String>::try_from(variant) {
                    let expander = adw::ExpanderRow::builder()
                        .title(&key)
                        .subtitle(&format!("{} items", items.len()))
                        .build();

                    for item in items {
                        let sub = adw::ActionRow::builder().title(&item).activatable(false).build();

                        if key == "services" {
                            let remove_btn = gtk::Button::builder()
                                .icon_name("user-trash-symbolic")
                                .valign(gtk::Align::Center)
                                .tooltip_text("Remove service from zone")
                                .css_classes(vec!["flat".to_string(), "destructive-action".to_string()])
                                .build();

                            let z_name = zone_name.to_string();
                            let s_name = item.clone();
                            let listbox_clone = settings_listbox.clone();
                            let stack_clone = stack.clone();
                            let details_clone = details_page.clone();
                            let client_rm = client.clone();
                            let overlay_rm = toast_overlay.clone();

                            remove_btn.connect_clicked(move |_| {
                                let z = z_name.clone();
                                let s = s_name.clone();
                                let listbox = listbox_clone.clone();
                                let stack = stack_clone.clone();
                                let details = details_clone.clone();
                                let c = client_rm.clone();
                                let overlay = overlay_rm.clone();

                                glib::spawn_future_local(async move {
                                    let dialog = adw::AlertDialog::builder()
                                        .heading(&format!("Remove '{}'?", s))
                                        .body(&format!("Are you sure you want to remove the service '{}' from the '{}' zone?", s, z))
                                        .build();
                                    
                                    dialog.add_response("cancel", "Cancel");
                                    dialog.add_response("remove", "Remove");
                                    dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
                                    
                                    if dialog.choose_future(&listbox).await == "remove" {
                                        match c.remove_service_to_zone(&z, &s).await {
                                            Ok(_) => {
                                                load_zone_details(&c, &z, &stack, &listbox, &details, &overlay).await;
                                            }
                                            Err(e) => show_toast(&overlay, &format!("Error removing service from zone: {}", e)),
                                        }
                                    }
                                });
                            });

                            sub.add_suffix(&remove_btn);
                        }

                        expander.add_row(&sub);
                    }

                    if key == "services" {
                        let add_service_row = adw::ActionRow::builder()
                            .title("Add Service")
                            .activatable(true)
                            .build();

                        let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
                        add_service_row.add_prefix(&add_icon);

                        let z_name_clone = zone_name.to_string();
                        let listbox_clone = settings_listbox.clone();
                        let stack_clone = stack.clone();
                        let details_clone = details_page.clone();
                        let client_add = client.clone();
                        let overlay_add = toast_overlay.clone();

                        add_service_row.connect_activated(move |_| {
                            show_add_service_to_zone_dialog(
                                client_add.clone(),
                                z_name_clone.clone(),
                                listbox_clone.clone(),
                                stack_clone.clone(),
                                details_clone.clone(),
                                overlay_add.clone(),
                            );
                        });

                        expander.add_row(&add_service_row);
                    }

                    settings_listbox.append(&expander);
                }
            }
        }
        Err(e) => {
            show_toast(toast_overlay, &format!("Error fetching details: {}", e));
        }
    }

    stack.set_visible_child_name("zone_details");
}

pub fn show_add_service_to_zone_dialog(
    client: FirewallClient,
    zone: String,
    listbox: gtk::ListBox,
    stack: gtk::Stack,
    details_page: adw::StatusPage,
    toast_overlay: adw::ToastOverlay,
) {
    glib::spawn_future_local(async move {
        let services = client.fetch_services().await.unwrap_or_default();

        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Add Service to {}", zone))
            .build();

        let svc_strs: Vec<&str> = services.iter().map(|s| s.as_str()).collect();
        let combo = gtk::DropDown::from_strings(&svc_strs);

        let timeout_entry = adw::EntryRow::builder()
            .title("Timeout (seconds, 0 for permanent)")
            .text("0")
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();

        let label = gtk::Label::builder()
            .label("Select a service:")
            .halign(gtk::Align::Start)
            .margin_bottom(8)
            .build();

        content_box.append(&label);
        content_box.append(&combo);
        content_box.append(&timeout_entry);
        dialog.set_extra_child(Some(&content_box));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("add", "Add");
        dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("add"));
        dialog.set_close_response("cancel");

        let response = dialog.choose_future(&listbox).await;
        if response == "add" {
            let selected_idx = combo.selected() as usize;
            let timeout = timeout_entry.text().parse::<i32>().unwrap_or(0);
            if let Some(new_svc) = services.get(selected_idx) {
                match client.add_service_to_zone(&zone, new_svc, timeout).await {
                    Ok(_) => {
                        load_zone_details(&client, &zone, &stack, &listbox, &details_page, &toast_overlay).await;
                    }
                    Err(e) => show_toast(&toast_overlay, &format!("Failed to add service: {}", e)),
                }
            }
        }
    });
}
