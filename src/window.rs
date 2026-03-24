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

            glib::spawn_future_local(glib::clone!(
                #[strong] state_label,
                #[strong] status_page,
                #[strong] zones_listbox,
                #[strong] main_stack,
                #[strong] settings_listbox,
                #[strong] details_page,
                #[strong] load_button,
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
                            status_page.set_description(Some(&format!("Your principal zone is: {}", zone)));
                        }
                        Err(e) => {
                            status_page.set_description(Some(&format!("Error reading zone: {}", e)));
                        }
                    }

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
                                                        let row = adw::ActionRow::builder().title(&key).subtitle(&val).build();
                                                        inner_settings.append(&row);
                                                    } else if let Ok(items) = <Vec<String>>::try_from(variant.clone()) {
                                                        let expander = adw::ExpanderRow::builder()
                                                            .title(&key)
                                                            .subtitle(&format!("{} items", items.len()))
                                                            .build();
                                                        for item in items {
                                                            let sub = adw::ActionRow::builder().title(&item).build();
                                                            expander.add_row(&sub);
                                                        }
                                                        inner_settings.append(&expander);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let err_row = adw::ActionRow::builder().title("Error").subtitle(&e.to_string()).build();
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
