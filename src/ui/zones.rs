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
    let client_clone = client.clone();
    let overlay_clone = toast_overlay.clone();

    row.connect_activated(move |_| {
        let zone = zone_name.clone();
        let stack = stack.clone();
        let settings = settings.clone();
        let details = details.clone();
        let client_clone = client_clone.clone();
        let overlay_clone = overlay_clone.clone();

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
            let key_order = [
                "short", "description", "target", "services", "ports", "sources", "forward-ports", "interfaces", "masquerade", "forward"
            ];

            for key in key_order.iter() {
                // Not all keys are guaranteed to be in the HashMap (empty lists are sometimes omitted)
                let variant_opt = settings.get(*key).cloned();
                
                if key == &"services" || key == &"interfaces" || key == &"sources" {
                    let mut items = Vec::new();
                    if let Some(variant) = variant_opt {
                        if let Ok(arr) = Vec::<String>::try_from(variant) {
                            items = arr;
                        }
                    }

                    let expander = adw::ExpanderRow::builder()
                        .title(key.replace("-", " ").to_uppercase())
                        .subtitle(&format!("{} items", items.len()))
                        .build();

                    for item in items {
                        let sub = adw::ActionRow::builder().title(&item).activatable(false).build();

                        if key == &"services" {
                            let remove_btn = build_trash_button("Remove service from zone");
                            let z_name = zone_name.to_string();
                            let s_name = item.clone();
                            let listb = settings_listbox.clone();
                            let st = stack.clone();
                            let det = details_page.clone();
                            let cl = client.clone();
                            let ol = toast_overlay.clone();

                            remove_btn.connect_clicked(move |_| {
                                let z = z_name.clone();
                                let s = s_name.clone();
                                let listbox = listb.clone();
                                let stack = st.clone();
                                let details = det.clone();
                                let c = cl.clone();
                                let overlay = ol.clone();

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
                                            Ok(_) => load_zone_details(&c, &z, &stack, &listbox, &details, &overlay).await,
                                            Err(e) => show_toast(&overlay, &format!("Error removing service from zone: {}", e)),
                                        }
                                    }
                                });
                            });
                            sub.add_suffix(&remove_btn);
                        } else if key == &"sources" {
                            let remove_btn = build_trash_button("Remove source from zone");
                            let z_name = zone_name.to_string();
                            let s_name = item.clone();
                            let listb = settings_listbox.clone();
                            let st = stack.clone();
                            let det = details_page.clone();
                            let cl = client.clone();
                            let ol = toast_overlay.clone();

                            remove_btn.connect_clicked(move |_| {
                                let z = z_name.clone();
                                let s = s_name.clone();
                                let listbox = listb.clone();
                                let stack = st.clone();
                                let details = det.clone();
                                let c = cl.clone();
                                let overlay = ol.clone();

                                glib::spawn_future_local(async move {
                                    let dialog = adw::AlertDialog::builder()
                                        .heading(&format!("Remove Source '{}'?", s))
                                        .body(&format!("Are you sure you want to remove the source '{}' from the '{}' zone?", s, z))
                                        .build();
                                    
                                    dialog.add_response("cancel", "Cancel");
                                    dialog.add_response("remove", "Remove");
                                    dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
                                    
                                    if dialog.choose_future(&listbox).await == "remove" {
                                        match c.remove_source_from_zone(&z, &s).await {
                                            Ok(_) => load_zone_details(&c, &z, &stack, &listbox, &details, &overlay).await,
                                            Err(e) => show_toast(&overlay, &format!("Error removing source from zone: {}", e)),
                                        }
                                    }
                                });
                            });
                            sub.add_suffix(&remove_btn);
                        }
                        expander.add_row(&sub);
                    }

                    if key == &"services" {
                        let add_row = build_add_row("Add Service");
                        let z_n = zone_name.to_string();
                        let l_box = settings_listbox.clone();
                        let stk = stack.clone();
                        let detail = details_page.clone();
                        let c_clone = client.clone();
                        let o_clone = toast_overlay.clone();
                        add_row.connect_activated(move |_| {
                            show_add_service_to_zone_dialog(c_clone.clone(), z_n.clone(), l_box.clone(), stk.clone(), detail.clone(), o_clone.clone());
                        });
                        expander.add_row(&add_row);
                    } else if key == &"sources" {
                        let add_row = build_add_row("Add Source");
                        let z_n = zone_name.to_string();
                        let l_box = settings_listbox.clone();
                        let stk = stack.clone();
                        let detail = details_page.clone();
                        let c_clone = client.clone();
                        let o_clone = toast_overlay.clone();
                        add_row.connect_activated(move |_| {
                            show_add_source_to_zone_dialog(c_clone.clone(), z_n.clone(), l_box.clone(), stk.clone(), detail.clone(), o_clone.clone());
                        });
                        expander.add_row(&add_row);
                    }

                    settings_listbox.append(&expander);
                } else if key == &"ports" {
                    let mut items = Vec::new();
                    if let Some(variant) = variant_opt {
                        if let Ok(arr) = Vec::<(String, String)>::try_from(variant) {
                            items = arr;
                        }
                    }

                    let expander = adw::ExpanderRow::builder()
                        .title("PORTS")
                        .subtitle(&format!("{} items", items.len()))
                        .build();

                    for (port, protocol) in items {
                        let sub = adw::ActionRow::builder()
                            .title(&format!("{}/{}", port, protocol))
                            .activatable(false)
                            .build();

                        let remove_btn = build_trash_button("Remove port from zone");
                        let z_name = zone_name.to_string();
                        let p_name = port.clone();
                        let pr_name = protocol.clone();
                        let listb = settings_listbox.clone();
                        let st = stack.clone();
                        let det = details_page.clone();
                        let cl = client.clone();
                        let ol = toast_overlay.clone();

                        remove_btn.connect_clicked(move |_| {
                            let z = z_name.clone();
                            let p = p_name.clone();
                            let pr = pr_name.clone();
                            let listb = listb.clone();
                            let st = st.clone();
                            let det = det.clone();
                            let cl = cl.clone();
                            let ol = ol.clone();

                            glib::spawn_future_local(async move {
                                let dialog = adw::AlertDialog::builder()
                                    .heading(&format!("Remove Port {}/{}?", p, pr))
                                    .body(&format!("Are you sure you want to remove the port '{}/{}' from the '{}' zone?", p, pr, z))
                                    .build();
                                
                                dialog.add_response("cancel", "Cancel");
                                dialog.add_response("remove", "Remove");
                                dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
                                
                                if dialog.choose_future(&listb).await == "remove" {
                                    match cl.remove_port_from_zone(&z, &p, &pr).await {
                                        Ok(_) => load_zone_details(&cl, &z, &st, &listb, &det, &ol).await,
                                        Err(e) => show_toast(&ol, &format!("Error removing port: {}", e)),
                                    }
                                }
                            });
                        });
                        sub.add_suffix(&remove_btn);
                        expander.add_row(&sub);
                    }

                    let add_row = build_add_row("Add Port");
                    let z_n = zone_name.to_string();
                    let l_box = settings_listbox.clone();
                    let stk = stack.clone();
                    let detail = details_page.clone();
                    let c_clone = client.clone();
                    let o_clone = toast_overlay.clone();
                    add_row.connect_activated(move |_| {
                        show_add_port_to_zone_dialog(c_clone.clone(), z_n.clone(), l_box.clone(), stk.clone(), detail.clone(), o_clone.clone());
                    });
                    expander.add_row(&add_row);
                    settings_listbox.append(&expander);
                } else if key == &"forward-ports" {
                    let mut items = Vec::new();
                    if let Some(variant) = variant_opt {
                        if let Ok(arr) = Vec::<(String, String, String, String)>::try_from(variant) {
                            items = arr;
                        }
                    }

                    let expander = adw::ExpanderRow::builder()
                        .title("FORWARD PORTS")
                        .subtitle(&format!("{} items", items.len()))
                        .build();

                    for (port, protocol, toport, toaddr) in items {
                        let formatted_title = if toaddr.is_empty() {
                            format!("{}/{} -> :{}", port, protocol, toport)
                        } else if toport.is_empty() {
                            format!("{}/{} -> {}", port, protocol, toaddr)
                        } else {
                            format!("{}/{} -> {}:{}", port, protocol, toaddr, toport)
                        };

                        let sub = adw::ActionRow::builder()
                            .title(&formatted_title)
                            .activatable(false)
                            .build();

                        let remove_btn = build_trash_button("Remove forward port from zone");
                        let z_name = zone_name.to_string();
                        let p = port.clone();
                        let pr = protocol.clone();
                        let tp = toport.clone();
                        let ta = toaddr.clone();
                        let listb = settings_listbox.clone();
                        let st = stack.clone();
                        let det = details_page.clone();
                        let cl = client.clone();
                        let ol = toast_overlay.clone();

                        remove_btn.connect_clicked(move |_| {
                            let z = z_name.clone();
                            let p = p.clone();
                            let pr = pr.clone();
                            let tp = tp.clone();
                            let ta = ta.clone();
                            let listb = listb.clone();
                            let st = st.clone();
                            let det = det.clone();
                            let cl = cl.clone();
                            let ol = ol.clone();

                            glib::spawn_future_local(async move {
                                let dialog = adw::AlertDialog::builder()
                                    .heading(&format!("Remove Forward Port {}/{}?", p, pr))
                                    .body(&format!("Are you sure you want to remove this forward port from the '{}' zone?", z))
                                    .build();
                                
                                dialog.add_response("cancel", "Cancel");
                                dialog.add_response("remove", "Remove");
                                dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
                                
                                if dialog.choose_future(&listb).await == "remove" {
                                    match cl.remove_forward_port_from_zone(&z, &p, &pr, &tp, &ta).await {
                                        Ok(_) => load_zone_details(&cl, &z, &st, &listb, &det, &ol).await,
                                        Err(e) => show_toast(&ol, &format!("Error removing forward port: {}", e)),
                                    }
                                }
                            });
                        });
                        sub.add_suffix(&remove_btn);
                        expander.add_row(&sub);
                    }

                    let add_row = build_add_row("Add Forward Port");
                    let z_n = zone_name.to_string();
                    let l_box = settings_listbox.clone();
                    let stk = stack.clone();
                    let detail = details_page.clone();
                    let c_clone = client.clone();
                    let o_clone = toast_overlay.clone();
                    add_row.connect_activated(move |_| {
                        show_add_forward_to_zone_dialog(c_clone.clone(), z_n.clone(), l_box.clone(), stk.clone(), detail.clone(), o_clone.clone());
                    });
                    expander.add_row(&add_row);
                    settings_listbox.append(&expander);
                } else {
                    if let Some(variant) = variant_opt {
                        if let Ok(val) = String::try_from(variant.clone()) {
                            let row = adw::ActionRow::builder()
                                .title(key.to_uppercase().as_str())
                                .subtitle(&val)
                                .build();
                            settings_listbox.append(&row);
                        } else if let Ok(val) = bool::try_from(variant.clone()) {
                            let text = if val { "Enabled" } else { "Disabled" };
                            let row = adw::ActionRow::builder()
                                .title(key.to_uppercase().as_str())
                                .subtitle(text)
                                .build();
                            settings_listbox.append(&row);
                        }
                    }
                }
            }
        }
        Err(e) => show_toast(toast_overlay, &format!("Error fetching details: {}", e)),
    }

    stack.set_visible_child_name("zone_details");
}

fn build_trash_button(tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text(tooltip)
        .css_classes(vec!["flat".to_string(), "destructive-action".to_string()])
        .build()
}

fn build_add_row(title: &str) -> adw::ActionRow {
    let add_row = adw::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build();
    let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
    add_row.add_prefix(&add_icon);
    add_row
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
            .margin_top(8).margin_bottom(8).margin_start(8).margin_end(8)
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

        if dialog.choose_future(&listbox).await == "add" {
            let selected_idx = combo.selected() as usize;
            let timeout = timeout_entry.text().parse::<i32>().unwrap_or(0);
            if let Some(new_svc) = services.get(selected_idx) {
                match client.add_service_to_zone(&zone, new_svc, timeout).await {
                    Ok(_) => load_zone_details(&client, &zone, &stack, &listbox, &details_page, &toast_overlay).await,
                    Err(e) => show_toast(&toast_overlay, &format!("Failed to add service: {}", e)),
                }
            }
        }
    });
}

pub fn show_add_port_to_zone_dialog(
    client: FirewallClient,
    zone: String,
    listbox: gtk::ListBox,
    stack: gtk::Stack,
    details_page: adw::StatusPage,
    toast_overlay: adw::ToastOverlay,
) {
    glib::spawn_future_local(async move {
        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Add Port to {}", zone))
            .build();

        let port_entry = adw::EntryRow::builder()
            .title("Port (e.g., 8080 or 1000-2000)")
            .build();

        let proto_combo = gtk::DropDown::from_strings(&["tcp", "udp", "sctp", "dccp"]);
        
        let timeout_entry = adw::EntryRow::builder()
            .title("Timeout (seconds, 0 for permanent)")
            .text("0")
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8).margin_bottom(8).margin_start(8).margin_end(8)
            .build();

        content_box.append(&port_entry);
        content_box.append(&gtk::Label::builder().label("Protocol:").halign(gtk::Align::Start).margin_top(8).margin_bottom(4).build());
        content_box.append(&proto_combo);
        content_box.append(&gtk::Label::builder().label("").margin_top(8).build());
        content_box.append(&timeout_entry);
        
        dialog.set_extra_child(Some(&content_box));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("add", "Add");
        dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("add"));
        dialog.set_close_response("cancel");

        if dialog.choose_future(&listbox).await == "add" {
            let port = port_entry.text().to_string();
            let protos = ["tcp", "udp", "sctp", "dccp"];
            let proto = protos[proto_combo.selected() as usize];
            let timeout = timeout_entry.text().parse::<i32>().unwrap_or(0);
            
            if !port.is_empty() {
                match client.add_port_to_zone(&zone, &port, proto, timeout).await {
                    Ok(_) => load_zone_details(&client, &zone, &stack, &listbox, &details_page, &toast_overlay).await,
                    Err(e) => show_toast(&toast_overlay, &format!("Failed to add port: {}", e)),
                }
            }
        }
    });
}

pub fn show_add_source_to_zone_dialog(
    client: FirewallClient,
    zone: String,
    listbox: gtk::ListBox,
    stack: gtk::Stack,
    details_page: adw::StatusPage,
    toast_overlay: adw::ToastOverlay,
) {
    glib::spawn_future_local(async move {
        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Add Source to {}", zone))
            .build();

        let source_entry = adw::EntryRow::builder()
            .title("Source IP or Subnet (e.g., 192.168.1.0/24)")
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8).margin_bottom(8).margin_start(8).margin_end(8)
            .build();

        content_box.append(&source_entry);
        dialog.set_extra_child(Some(&content_box));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("add", "Add");
        dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("add"));
        dialog.set_close_response("cancel");

        if dialog.choose_future(&listbox).await == "add" {
            let source = source_entry.text().to_string();
            if !source.is_empty() {
                // add_source_zone runtime doesnt take a timeout param directly in firewalld DBus!
                match client.add_source_to_zone(&zone, &source).await {
                    Ok(_) => load_zone_details(&client, &zone, &stack, &listbox, &details_page, &toast_overlay).await,
                    Err(e) => show_toast(&toast_overlay, &format!("Failed to add source: {}", e)),
                }
            }
        }
    });
}

pub fn show_add_forward_to_zone_dialog(
    client: FirewallClient,
    zone: String,
    listbox: gtk::ListBox,
    stack: gtk::Stack,
    details_page: adw::StatusPage,
    toast_overlay: adw::ToastOverlay,
) {
    glib::spawn_future_local(async move {
        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Add Forward Port to {}", zone))
            .build();

        let port_entry = adw::EntryRow::builder().title("Local Port").build();
        let proto_combo = gtk::DropDown::from_strings(&["tcp", "udp", "sctp", "dccp"]);
        let toport_entry = adw::EntryRow::builder().title("Destination Port").build();
        let toaddr_entry = adw::EntryRow::builder().title("Destination Address").build();
        
        let timeout_entry = adw::EntryRow::builder()
            .title("Timeout (seconds, 0 for permanent)")
            .text("0")
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8).margin_bottom(8).margin_start(8).margin_end(8)
            .build();

        content_box.append(&port_entry);
        content_box.append(&gtk::Label::builder().label("Protocol:").halign(gtk::Align::Start).margin_top(8).margin_bottom(4).build());
        content_box.append(&proto_combo);
        content_box.append(&toport_entry);
        content_box.append(&toaddr_entry);
        content_box.append(&timeout_entry);
        
        dialog.set_extra_child(Some(&content_box));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("add", "Add");
        dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("add"));
        dialog.set_close_response("cancel");

        if dialog.choose_future(&listbox).await == "add" {
            let port = port_entry.text().to_string();
            let protos = ["tcp", "udp", "sctp", "dccp"];
            let proto = protos[proto_combo.selected() as usize];
            let toport = toport_entry.text().to_string();
            let toaddr = toaddr_entry.text().to_string();
            let timeout = timeout_entry.text().parse::<i32>().unwrap_or(0);
            
            if !port.is_empty() {
                match client.add_forward_port_to_zone(&zone, &port, proto, &toport, &toaddr, timeout).await {
                    Ok(_) => load_zone_details(&client, &zone, &stack, &listbox, &details_page, &toast_overlay).await,
                    Err(e) => show_toast(&toast_overlay, &format!("Failed to add forward port: {}", e)),
                }
            }
        }
    });
}
