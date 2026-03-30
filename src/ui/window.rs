use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use crate::backend::FirewallClient;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/rodrigofilipefaria/FirewallManager/ui/window.ui")]
    pub struct FirewallManagerWindow {
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub runtime_action_bar: TemplateChild<gtk::ActionBar>,
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
        #[template_child]
        pub mode_runtime_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub mode_permanent_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub make_permanent_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub reload_firewall_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub revert_changes_button: TemplateChild<gtk::Button>,
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
                        imp.init_services(&client, &toast_overlay);
                        
                        let is_perm = client.is_permanent_mode();
                        imp.add_service_button.set_sensitive(is_perm);
                        if !is_perm {
                            imp.add_service_button.set_tooltip_text(Some("Switch to Permanent mode to define new services."));
                        } else {
                            imp.add_service_button.set_tooltip_text(Some("Add a new custom service definition."));
                        }
                        
                        let c_init = client.clone();
                        let o_init = toast_overlay.clone();
                        let l_init = imp.services_listbox.get();
                        glib::spawn_future_local(async move {
                            crate::ui::services::reload_services(&c_init, &l_init, &o_init).await;
                        });
                        imp.setup_navigation(&client, &toast_overlay);
                        imp.setup_modes(&client, &toast_overlay);
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to D-Bus: {}", e);
                    }
                }
            });
        }
    }

    impl FirewallManagerWindow {
        fn setup_modes(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            let client_mode = client.clone();
            let overlay_mode = toast_overlay.clone();
            let obj = self.obj().clone();
            
            let mut unsaved_rx = client.unsaved_changes_rx.clone();
            let runtime_action_bar = self.runtime_action_bar.clone();
            let client_watch = client.clone();
            glib::spawn_future_local(async move {
                while unsaved_rx.changed().await.is_ok() {
                    let has_unsaved = *unsaved_rx.borrow();
                    let is_permanent = client_watch.is_permanent_mode();
                    runtime_action_bar.set_revealed(has_unsaved && !is_permanent);
                }
            });
            
            self.mode_permanent_button.connect_toggled(move |btn| {
                let is_permanent = btn.is_active();
                client_mode.set_permanent_mode(is_permanent);
                
                let imp = obj.imp();
                imp.setup_firewall_state(&client_mode, &overlay_mode);
                imp.setup_zones_list(&client_mode, &overlay_mode);
                
                imp.add_service_button.set_sensitive(is_permanent);
                if !is_permanent {
                    imp.add_service_button.set_tooltip_text(Some("Switch to Permanent mode to define new services."));
                } else {
                    imp.add_service_button.set_tooltip_text(Some("Add a new custom service definition."));
                }
                
                let c_reload = client_mode.clone();
                let o_reload = overlay_mode.clone();
                let l_reload = imp.services_listbox.get();
                glib::spawn_future_local(async move {
                    crate::ui::services::reload_services(&c_reload, &l_reload, &o_reload).await;
                });
                
                let msg = if is_permanent { "Switched to Permanent Configuration" } else { "Switched to Runtime Configuration" };
                crate::ui::utils::show_toast(&overlay_mode, msg);
                
                let has_unsaved = *client_mode.unsaved_changes_rx.borrow();
                imp.runtime_action_bar.set_revealed(has_unsaved && !is_permanent);
            });

            let client_save = client.clone();
            let overlay_save = toast_overlay.clone();
            self.make_permanent_button.connect_clicked(move |_| {
                let c = client_save.clone();
                let o = overlay_save.clone();
                glib::spawn_future_local(async move {
                    match c.runtime_to_permanent().await {
                        Ok(_) => crate::ui::utils::show_toast(&o, "Runtime configuration saved to permanent."),
                        Err(e) => crate::ui::utils::show_toast(&o, &format!("Failed to save config: {}", e)),
                    }
                });
            });

            let client_reload = client.clone();
            let overlay_reload = toast_overlay.clone();
            self.reload_firewall_button.connect_clicked(move |_| {
                let c = client_reload.clone();
                let o = overlay_reload.clone();
                glib::spawn_future_local(async move {
                    match c.reload_firewall().await {
                        Ok(_) => crate::ui::utils::show_toast(&o, "Firewall reloaded successfully."),
                        Err(e) => crate::ui::utils::show_toast(&o, &format!("Failed to reload firewall: {}", e)),
                    }
                });
            });

            let client_revert = client.clone();
            let overlay_revert = toast_overlay.clone();
            self.revert_changes_button.connect_clicked(move |_| {
                let c = client_revert.clone();
                let o = overlay_revert.clone();
                glib::spawn_future_local(async move {
                    match c.reload_firewall().await {
                        Ok(_) => crate::ui::utils::show_toast(&o, "Unsaved changes reverted successfully."),
                        Err(e) => crate::ui::utils::show_toast(&o, &format!("Failed to revert changes: {}", e)),
                    }
                });
            });
        }

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

        fn init_services(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            crate::ui::services::init_services(
                client.clone(),
                toast_overlay.clone(),
                self.services_listbox.get(),
                self.add_service_button.get(),
                self.services_search_entry.get(),
            );
        }

        fn setup_navigation(&self, client: &FirewallClient, toast_overlay: &adw::ToastOverlay) {
            let main_stack = self.main_stack.clone();
            let split_view = self.split_view.clone();
            let load_button = self.load_button.clone();
            let state_label = self.state_label.clone();
            let client = client.clone();
            let overlay = toast_overlay.clone();

            main_stack.connect_visible_child_notify(glib::clone!(
                #[strong] split_view,
                move |_| {
                    split_view.set_show_content(true);
                }
            ));

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
