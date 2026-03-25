use gtk::prelude::*;
use adw::prelude::*;
use std::convert::TryFrom;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/rodrigofilipefaria/FirewallManager/window.ui")]
    pub struct FirewallManagerWindow {
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

            let status_page = self.status_page.clone();
            let state_label = self.state_label.clone();
            let zones_listbox = self.zones_listbox.clone();
            let load_button = self.load_button.clone();
            let main_stack = self.main_stack.clone();
            let settings_listbox = self.settings_listbox.clone();
            let back_button = self.back_button.clone();
            let details_page = self.details_page.clone();
            let services_listbox = self.services_listbox.clone();
            let add_service_button = self.add_service_button.clone();
            let interfaces_listbox = self.interfaces_listbox.clone();

            glib::spawn_future_local(glib::clone!(
                #[strong] state_label,
                #[strong] status_page,
                #[strong] zones_listbox,
                #[strong] main_stack,
                #[strong] settings_listbox,
                #[strong] details_page,
                #[strong] load_button,
                #[strong] interfaces_listbox,
                async move {
                    match crate::firewall_dbus_api::fetch_state().await {
                        Ok(state) => {
                            state_label.set_label(&format!("Service state: {}", state));
                            if state.trim().to_lowercase() == "running" {
                                load_button.set_label("Disable Firewall");
                                load_button.remove_css_class("suggested-action");
                                load_button.add_css_class("destructive-action");
                            } else {
                                load_button.set_label("Enable Firewall");
                                load_button.remove_css_class("destructive-action");
                                load_button.add_css_class("suggested-action");
                            }
                            load_button.set_sensitive(true);
                        }
                        Err(_) => {
                            state_label.set_label("Service state: stopped");
                            load_button.set_label("Enable Firewall");
                            load_button.remove_css_class("destructive-action");
                            load_button.add_css_class("suggested-action");
                            load_button.set_sensitive(true);
                        }
                    }

                    match crate::firewall_dbus_api::fetch_default_zone().await {
                        Ok(zone) => {
                            status_page.set_description(Some(&format!("Fallback Zone: {}", zone)));
                        }
                        Err(e) => {
                            status_page.set_description(Some(&format!("Error reading zone: {}", e)));
                        }
                    }

                    reload_interfaces(&interfaces_listbox).await;

                    while let Some(child) = zones_listbox.first_child() {
                        zones_listbox.remove(&child);
                    }

                    match crate::firewall_dbus_api::fetch_zones().await {
                        Ok(zones) => {
                            for zone_name in zones {
                                let row = adw::ActionRow::builder()
                                    .title(&zone_name)
                                    .activatable(true)
                                    .build();

                                let arrow = gtk::Image::from_icon_name("go-next-symbolic");
                                row.add_suffix(&arrow);

                                let current_zone = zone_name.clone();
                                let stack_ref = main_stack.clone();
                                let settings_ref = settings_listbox.clone();
                                let details_ref = details_page.clone();

                                row.connect_activated(move |_| {
                                    let zone_to_fetch = current_zone.clone();
                                    let inner_stack = stack_ref.clone();
                                    let inner_settings = settings_ref.clone();
                                    let inner_details = details_ref.clone();

                                    glib::spawn_future_local(async move {
                                        while let Some(child) = inner_settings.first_child() {
                                            inner_settings.remove(&child);
                                        }

                                        inner_details.set_title(&format!("Zone: {}", zone_to_fetch));

                                        match crate::firewall_dbus_api::fetch_zone_settings(&zone_to_fetch).await {
                                            Ok(settings) => {
                                                for (key, variant) in settings {
                                                    if let Ok(val) = String::try_from(variant.clone()) {
                                                        let row = adw::ActionRow::builder()
                                                            .title(&key)
                                                            .subtitle(&val)
                                                            .build();
                                                        inner_settings.append(&row);
                                                    } else if let Ok(items) = Vec::<String>::try_from(variant.clone()) {
                                                        let expander = adw::ExpanderRow::builder()
                                                            .title(&key)
                                                            .subtitle(&format!("{} items", items.len()))
                                                            .build();
                                                        for item in items {
                                                            let sub = adw::ActionRow::builder()
                                                                .title(&item)
                                                                .build();
                                                            expander.add_row(&sub);
                                                        }
                                                        inner_settings.append(&expander);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let err_row = adw::ActionRow::builder()
                                                    .title("Error")
                                                    .subtitle(&e.to_string())
                                                    .build();
                                                inner_settings.append(&err_row);
                                            }
                                        }

                                        inner_stack.set_visible_child_name("zone_details");
                                    });
                                });

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
                }
            ));

            glib::spawn_future_local(glib::clone!(
                #[strong] services_listbox,
                #[strong] add_service_button,
                async move {
                    reload_services(&services_listbox).await;

                    add_service_button.connect_clicked(glib::clone!(
                        #[strong] services_listbox,
                        move |_| {
                            let list = services_listbox.clone();
                            show_add_service_dialog(list);
                        }
                    ));
                }
            ));

            back_button.connect_clicked(glib::clone!(
                #[strong] main_stack,
                move |_| {
                    main_stack.set_visible_child_name("Zones");
                }
            ));

            load_button.connect_clicked(glib::clone!(
                #[strong(rename_to = btn)] load_button,
                #[strong] state_label,
                move |_| {
                    btn.set_sensitive(false);
                    let btn_async = btn.clone();
                    let lbl_async = state_label.clone();

                    let is_running = btn_async.label().unwrap_or_default() == "Disable Firewall";

                    glib::spawn_future_local(async move {
                        if is_running {
                            match crate::firewall_dbus_api::disable_firewall().await {
                                Ok(_) => {
                                    lbl_async.set_label("Service state: stopped");
                                    btn_async.set_label("Enable Firewall");
                                    btn_async.remove_css_class("destructive-action");
                                    btn_async.add_css_class("suggested-action");
                                }
                                Err(e) => lbl_async.set_label(&format!("Error: {}", e)),
                            }
                        } else {
                            match crate::firewall_dbus_api::enable_firewall().await {
                                Ok(_) => {
                                    lbl_async.set_label("Service state: running");
                                    btn_async.set_label("Disable Firewall");
                                    btn_async.remove_css_class("suggested-action");
                                    btn_async.add_css_class("destructive-action");
                                }
                                Err(e) => lbl_async.set_label(&format!("Error: {}", e)),
                            }
                        }
                        btn_async.set_sensitive(true);
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


async fn reload_services(listbox: &gtk::ListBox) {
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }
    match crate::firewall_dbus_api::fetch_services().await {
        Ok(services) => {
            for svc in services {
                let row = build_service_row(&svc, listbox);
                listbox.append(&row);
            }
        }
        Err(e) => {
            let err_row = adw::ActionRow::builder()
                .title(&format!("Error: {}", e))
                .build();
            listbox.append(&err_row);
        }
    }
}

fn build_service_row(service_name: &str, services_listbox: &gtk::ListBox) -> adw::ActionRow {
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
    edit_btn.connect_clicked(move |_| {
        let sn = svc_name_edit.clone();
        let list = list_edit.clone();
        glib::spawn_future_local(async move {
            match crate::firewall_dbus_api::fetch_service_settings(&sn).await {
                Ok((_ver, _name, desc, ports, _mods, _dests, _includes, _src_ports)) => {
                    show_edit_service_dialog(sn, desc, ports, list);
                }
                Err(e) => eprintln!("Error fetching service settings: {e}"),
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
    remove_btn.connect_clicked(move |_| {
        let sn = svc_name_rm.clone();
        let list = list_rm.clone();

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
                match crate::firewall_dbus_api::remove_service(&sn).await {
                    Ok(_) => reload_services(&list).await,
                    Err(e) => eprintln!("Error removing service: {e}"),
                }
            }
        });
    });

    row.add_suffix(&edit_btn);
    row.add_suffix(&remove_btn);
    row
}

fn show_add_service_dialog(listbox: gtk::ListBox) {
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
            match crate::firewall_dbus_api::add_service(&name, &desc, ports).await {
                Ok(_) => reload_services(&listbox).await,
                Err(e) => eprintln!("Error adding service: {e}"),
            }
        }
    });
}

fn show_edit_service_dialog(
    service_name: String,
    current_desc: String,
    current_ports: Vec<(String, String)>,
    listbox: gtk::ListBox,
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
            match crate::firewall_dbus_api::edit_service(&service_name, &new_desc, new_ports).await {
                Ok(_) => reload_services(&listbox).await,
                Err(e) => eprintln!("Error editing service: {e}"),
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

async fn reload_interfaces(listbox: &gtk::ListBox) {
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }

    match crate::firewall_dbus_api::fetch_interfaces().await {
        Ok(interfaces) => {
            if interfaces.is_empty() {
                let empty_row = adw::ActionRow::builder()
                    .title("No interfaces found.")
                    .build();
                listbox.append(&empty_row);
                return;
            }

            for iface in interfaces {
                let current_zone = crate::firewall_dbus_api::fetch_zone_of_interface(&iface)
                    .await
                    .unwrap_or_else(|_| "Unknown".to_string());

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

                change_btn.connect_clicked(move |_| {
                    show_change_zone_dialog(iface_clone.clone(), zone_clone.clone(), list_clone.clone());
                });

                row.add_suffix(&change_btn);
                listbox.append(&row);
            }
        }
        Err(e) => {
            let err_row = adw::ActionRow::builder()
                .title(&format!("Error fetching interfaces: {}", e))
                .build();
            listbox.append(&err_row);
        }
    }
}

fn show_change_zone_dialog(interface: String, current_zone: String, listbox: gtk::ListBox) {
    glib::spawn_future_local(async move {
        let zones = crate::firewall_dbus_api::fetch_zones().await.unwrap_or_default();

        let dialog = adw::AlertDialog::builder()
            .heading(&format!("Change Zone for {}", interface))
            .build();

        let zone_strs: Vec<&str> = zones.iter().map(|s| s.as_str()).collect();
        let combo = gtk::DropDown::from_strings(&zone_strs);

        if let Some(pos) = zones.iter().position(|z| z == &current_zone) {
            combo.set_selected(pos as u32);
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
            let selected_idx = combo.selected();
            if let Some(new_zone) = zones.get(selected_idx as usize) {
                match crate::firewall_dbus_api::change_zone_interface(new_zone, &interface).await {
                    Ok(_) => reload_interfaces(&listbox).await,
                    Err(e) => eprintln!("Error changing zone: {}", e),
                }
            }
        }
    });
}
