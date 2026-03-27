use gtk::prelude::*;
use adw::prelude::*;
use std::convert::TryFrom;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use crate::firewall_dbus_api::FirewallClient;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/rodrigofilipefaria/FirewallManager/window.ui")]
    pub struct FirewallManagerWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub load_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub state_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub zones_listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub settings_listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub details_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub services_listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub add_service_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub interfaces_listbox: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FirewallManagerWindow {
        const NAME: &'static str = "FirewallManagerWindow";
        type Type = super::FirewallManagerWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FirewallManagerWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().clone();

            glib::spawn_future_local(async move {
                match FirewallClient::new().await {
                    Ok(client) => {
                        let imp = obj.imp();
                        let toast_overlay = imp.toast_overlay.get();

                        imp.setup_firewall_state(&client, &toast_overlay);
                        imp.setup_zones_list(&client, &toast_overlay);
                        imp.setup_services(&client, &toast_overlay);
                        imp.setup_navigation(&client, &toast_overlay);
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to D-Bus: {}", e);
                    }
                }
            });
        }
    }

    impl FirewallManagerWindow {
        fn setup_firewall_state(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            let state_label = self.state_label.clone();
            let load_button = self.load_button.clone();
            let status_page = self.status_page.clone();
            let interfaces_listbox = self.interfaces_listbox.clone();
            let client = client.clone();
            let overlay = toast_overlay.clone();

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

                reload_interfaces(&client, &interfaces_listbox, &overlay).await;
            });
        }

        fn setup_zones_list(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            let zones_listbox = self.zones_listbox.clone();
            let main_stack = self.main_stack.clone();
            let settings_listbox = self.settings_listbox.clone();
            let details_page = self.details_page.clone();
            let client = client.clone();
            let overlay = toast_overlay.clone();

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
                                &overlay,
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

        fn setup_services(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            let services_listbox = self.services_listbox.clone();
            let add_service_button = self.add_service_button.clone();

            let client_for_reload = client.clone();
            let client_for_dialog = client.clone();
            let overlay = toast_overlay.clone();
            let overlay_dialog = toast_overlay.clone();

            glib::spawn_future_local(async move {
                reload_services(&client_for_reload, &services_listbox, &overlay).await;

                add_service_button.connect_clicked(glib::clone!(
                    #[strong] services_listbox,
                    move |_| show_add_service_dialog(client_for_dialog.clone(), services_listbox.clone(), overlay_dialog.clone())
                ));
            });
        }

        fn setup_navigation(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            let main_stack = self.main_stack.clone();
            let load_button = self.load_button.clone();
            let state_label = self.state_label.clone();
            let client = client.clone();
            let overlay = toast_overlay.clone();

            self.back_button.connect_clicked(glib::clone!(
                #[strong] main_stack,
                move |_| main_stack.set_visible_child_name("Zones")
            ));

            load_button.connect_clicked(glib::clone!(
                #[strong(rename_to = btn)] load_button,
                #[strong] state_label,
                move |_| {
                    btn.set_sensitive(false);
                    let btn = btn.clone();
                    let lbl = state_label.clone();
                    let is_running = btn.label().unwrap_or_default() == "Disable Firewall";
                    let client = client.clone();
                    let overlay = overlay.clone();

                    glib::spawn_future_local(async move {
                        if is_running {
                            match client.disable_firewall().await {
                                Ok(_) => {
                                    lbl.set_label("Service state: stopped");
                                    update_firewall_button(&btn, false);
                                }
                                Err(e) => {
                                    show_toast(&overlay, &format!("Failed to stop firewall: {}", e));
                                    btn.set_sensitive(true);
                                }
                            }
                        } else {
                            match client.enable_firewall().await {
                                Ok(_) => {
                                    lbl.set_label("Service state: running");
                                    update_firewall_button(&btn, true);
                                }
                                Err(e) => {
                                    show_toast(&overlay, &format!("Failed to start firewall: {}", e));
                                    btn.set_sensitive(true);
                                }
                            }
                        }
                    });
                }
            ));
        }
    }

    impl WidgetImpl for FirewallManagerWindow {}
    impl WindowImpl for FirewallManagerWindow {}
    impl ApplicationWindowImpl for FirewallManagerWindow {}
    impl AdwApplicationWindowImpl for FirewallManagerWindow {}
}

glib::wrapper! {
    pub struct FirewallManagerWindow(ObjectSubclass<imp::FirewallManagerWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FirewallManagerWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}

fn show_toast(overlay: &adw::ToastOverlay, message: &str) {
    let toast = adw::Toast::builder()
        .title(message)
        .timeout(3)
        .build();
    overlay.add_toast(toast);
}

fn clear_listbox(listbox: &gtk::ListBox) {
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }
}

fn update_firewall_button(button: &gtk::Button, is_running: bool) {
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

fn build_zone_row(
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

async fn load_zone_details(
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
                                    match c.remove_service_to_zone(&z, &s).await {
                                        Ok(_) => {
                                            load_zone_details(&c, &z, &stack, &listbox, &details, &overlay).await;
                                        }
                                        Err(e) => show_toast(&overlay, &format!("Error removing service from zone: {}", e)),
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

fn show_add_service_to_zone_dialog(
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

async fn reload_services(client: &FirewallClient, listbox: &gtk::ListBox, toast_overlay: &adw::ToastOverlay) {
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

fn build_service_row(
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

    row.add_suffix(&edit_btn);
    row.add_suffix(&remove_btn);
    row
}

fn show_add_service_dialog(client: FirewallClient, listbox: gtk::ListBox, toast_overlay: adw::ToastOverlay) {
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

fn show_edit_service_dialog(
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

fn parse_ports(input: &str) -> Vec<(String, String)> {
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

async fn reload_interfaces(client: &FirewallClient, listbox: &gtk::ListBox, toast_overlay: &adw::ToastOverlay) {
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

fn show_change_zone_dialog(client: FirewallClient, interface: String, current_zone: String, listbox: gtk::ListBox, toast_overlay: adw::ToastOverlay) {
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
