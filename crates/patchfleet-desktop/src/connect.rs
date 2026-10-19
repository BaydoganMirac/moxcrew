use super::*;

fn connections(snapshot: &Value) -> &[Value] {
    let diagnostic = &snapshot["providerConnections"];
    if text(diagnostic, "reasonCode") != "PROVIDER_CONNECTIONS_READ" {
        return &[];
    }
    array(&diagnostic["providers"])
}

pub(super) fn can_start(snapshot: &Value) -> bool {
    connections(snapshot).iter().any(|provider| {
        let version: Option<Vec<u64>> = text(provider, "version")
            .split('+')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|part| part.parse().ok())
            .collect();
        text(provider, "id") == "codex"
            && provider["installed"] == true
            && provider["authentication"] == "present"
            && provider["controlSupported"] == true
            && version.is_some_and(|version| {
                version.len() == 3 && version.as_slice() >= [0, 155, 1].as_slice()
            })
    })
}

fn setup_url(id: &str) -> Option<&'static str> {
    match id {
        "codex" => Some("https://learn.chatgpt.com/docs/codex/cli"),
        "claude" => Some("https://code.claude.com/docs/en/setup"),
        "gemini" => Some("https://geminicli.com/docs/get-started/installation/"),
        _ => None,
    }
}

impl Console {
    pub(super) fn connection_dialog(
        &mut self,
        ui: &mut Ui,
        snapshot: &Value,
        commands: &mut Vec<Command>,
    ) {
        if !self.create.chat.connections_open {
            return;
        }
        let ready = ui.is_enabled();
        let id = egui::Id::new("provider-connections");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .show(ui.ctx(), |ui| {
                ui.set_width((ui.ctx().viewport_rect().width()-64.0).clamp(280.0,680.0));
                ui.label(studio::title(t(ui,"Connect your crew"),28.0));
                hint(ui,"One supported connection is enough. Your existing provider account stays yours.");
                hint(ui,"After installing or updating a provider CLI, restart Moxcrew and check connections again.");
                ui.add_space(8.0);
                egui::ScrollArea::vertical().id_salt("connection-cards")
                    .max_height((ui.ctx().viewport_rect().height()-365.0).max(110.0))
                    .show(ui,|ui| {
                        for (id,name) in [("codex","Codex"),("claude","Claude Code"),("gemini","Gemini CLI")] {
                            let provider=connections(snapshot).iter().find(|provider|text(provider,"id")==id);
                            ui.push_id(id,|ui| {
                                egui::Frame::new().fill(colors(ui).raised).corner_radius(14).inner_margin(14).show(ui,|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(studio::title(name,18.0));
                                        if let Some(version)=provider.and_then(|provider|provider["version"].as_str()) {raw_hint(ui,version);}
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                                            if let Some(url)=setup_url(id) {
                                                if ui.add_enabled(ready,egui::Link::new(t(ui,"Official setup guide"))).clicked() {ui.ctx().open_url(egui::OpenUrl::new_tab(url));}
                                            }
                                        });
                                    });
                                    ui.add_space(8.0);
                                    ui.columns(3,|columns| {
                                        for (column,label,value) in [
                                            (0,"Installation",match provider {Some(provider) if provider["installed"]==true=>"Installed",Some(_)=>"Not installed",None=>"Not checked"}),
                                            (1,"Sign-in",match provider.map(|provider|text(provider,"authentication")) {Some("present")=>"Saved sign-in found",Some("missing")=>"Sign-in needed",_=>"Unknown"}),
                                            (2,"Task control",if id!="codex" {"Not supported yet"} else if provider.is_some_and(|provider|provider["controlSupported"]==true) {"Supported"} else {"Install or update Codex"}),
                                        ] {
                                            let ui=&mut columns[column];
                                            ui.label(RichText::new(t(ui,label)).size(11.0).color(colors(ui).muted));
                                            ui.label(RichText::new(t(ui,value)).size(13.0).strong());
                                        }
                                    });
                                    if let Some(provider)=provider {
                                        ui.collapsing(t(ui,"Diagnostic details"),|ui| {
                                            ui.monospace(text(provider,"reasonCode"));
                                            if id!="codex" {hint(ui,"Task execution for this provider is not available yet.");}
                                        });
                                    }
                                });
                            });
                            ui.add_space(10.0);
                        }
                    });
                ui.add_space(6.0);
                hint(ui,"Saved sign-in does not verify model access or available quota. Checking never logs you in.");
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(ready,|ui| {
                        action(ui,commands,"Check connections","provider_connections_read",!self.hold_automatic_reads,||json!({}));
                    });
                    let mut reduced=snapshot["desktopPreferences"]["reduceMotion"].as_bool().unwrap_or(false);
                    let response=ui.add_enabled(ready,egui::Checkbox::new(&mut reduced,t(ui,"Reduce motion")));
                    #[cfg(test)]
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new(("test-action","desktop_motion","Reduce motion")),response.rect);
                        data.insert_temp(egui::Id::new(("test-action-state","desktop_motion","Reduce motion")),(response.enabled(),response.rect,ui.clip_rect()));
                    });
                    if response.changed() && commands.is_empty() {commands.push(Command::new("desktop_motion",json!({"reduceMotion":reduced})));}
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                        if control(ui,"Close",true).clicked() {self.create.chat.connections_open=false;}
                    });
                });
            });
        if modal.should_close() {
            self.create.chat.connections_open = false;
        }
        if self.create.chat.connections_open
            && !self.create.chat.connection_requested
            && connections(snapshot).is_empty()
            && ready
            && !self.hold_automatic_reads
            && commands.is_empty()
        {
            self.create.chat.connection_requested = true;
            commands.push(Command::new("provider_connections_read", json!({})));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_auth_alone_never_enables_an_unsupported_provider_or_old_codex() {
        let mut snapshot = json!({"providerConnections":{"reasonCode":"PROVIDER_CONNECTIONS_READ","providers":[
            {"id":"claude","installed":true,"authentication":"present","controlSupported":true,"version":"9.0.0"}]}});
        assert!(!can_start(&snapshot));
        snapshot["providerConnections"]["providers"][0]["id"] = json!("codex");
        snapshot["providerConnections"]["providers"][0]["version"] = json!("0.155.0");
        assert!(!can_start(&snapshot));
        snapshot["providerConnections"]["providers"][0]["version"] = json!("0.155.1");
        assert!(can_start(&snapshot));
        snapshot["providerConnections"]["providers"][0]["version"] = json!("0.bad.155.1");
        assert!(!can_start(&snapshot));
        snapshot["providerConnections"]["providers"][0]["version"] = json!("0.155.1");
        for auth in ["missing", "unknown"] {
            snapshot["providerConnections"]["providers"][0]["authentication"] = json!(auth);
            assert!(!can_start(&snapshot));
        }
        assert!(setup_url("foreign").is_none());
        assert!(setup_url("codex")
            .unwrap()
            .starts_with("https://learn.chatgpt.com/"));
    }
}
