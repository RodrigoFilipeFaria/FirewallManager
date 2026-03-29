use gtk::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use crate::backend::FirewallClient;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/rodrigofilipefaria/FirewallManager/ui/window.ui")]
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
        pub services_search_entry: TemplateChild<gtk::SearchEntry>,
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
            crate::ui::dashboard::setup_firewall_state(
                client.clone(),
                toast_overlay.clone(),
                self.state_label.get(),
                self.load_button.get(),
                self.status_page.get(),
                self.interfaces_listbox.get(),
            );
        }

        fn setup_zones_list(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            crate::ui::zones::setup_zones_list(
                client.clone(),
                toast_overlay.clone(),
                self.zones_listbox.get(),
                self.main_stack.get(),
                self.settings_listbox.get(),
                self.details_page.get(),
            );
        }

        fn setup_services(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            crate::ui::services::setup_services(
                client.clone(),
                toast_overlay.clone(),
                self.services_listbox.get(),
                self.add_service_button.get(),
                self.services_search_entry.get(),
            );
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
                                    crate::ui::dashboard::update_firewall_button(&btn, false);
                                }
                                Err(e) => {
                                    crate::ui::utils::show_toast(&overlay, &format!("Failed to stop firewall: {}", e));
                                    btn.set_sensitive(true);
                                }
                            }
                        } else {
                            match client.enable_firewall().await {
                                Ok(_) => {
                                    lbl.set_label("Service state: running");
                                    crate::ui::dashboard::update_firewall_button(&btn, true);
                                }
                                Err(e) => {
                                    crate::ui::utils::show_toast(&overlay, &format!("Failed to start firewall: {}", e));
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
