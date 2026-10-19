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
        let modal=egui::Modal::new(id).area(egui::Modal::default_area(id).fade_in(false)).show(ui.ctx(),|ui| {
            ui.set_width((ui.ctx().viewport_rect().width()-64.0).clamp(280.0,640.0));
            ui.label(studio::title(t(ui,"Connect your crew"),24.0));
            hint(ui,"One supported connection is enough. Your existing provider account stays yours.");
            hint(ui,"After installing or updating a provider CLI, restart Moxcrew and check connections again.");
            ui.add_enabled_ui(ready,|ui| {
                action(ui,commands,"Check connections","provider_connections_read",!self.hold_automatic_reads,||json!({}));
            });
            egui::ScrollArea::vertical().id_salt("connection-cards").max_height((ui.ctx().viewport_rect().height()-320.0).max(120.0)).show(ui,|ui| {
                for (id,name) in [("codex","Codex"),("claude","Claude Code"),("gemini","Gemini CLI")] {
                    ui.push_id(id,|ui|card(ui,|ui| {
                        ui.label(RichText::new(name).strong().size(17.0));
                        let provider=connections(snapshot).iter().find(|provider|text(provider,"id")==id);
                        ui.horizontal_wrapped(|ui| {
                            hint(ui,"Installation");
                            status(ui,match provider {Some(provider) if provider["installed"]==true=>"Installed",Some(_)=>"Not installed",None=>"Not checked"});
                            if let Some(version)=provider.and_then(|provider|provider["version"].as_str()) {raw_hint(ui,version);}
                        });
                        ui.horizontal_wrapped(|ui| {
                            hint(ui,"Sign-in");
                            status(ui,match provider.map(|provider|text(provider,"authentication")) {
                                Some("present")=>"Saved sign-in found",Some("missing")=>"Sign-in needed",_=>"Unknown",
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            hint(ui,"Task control");
                            status(ui,if id!="codex" {"Not supported yet"} else if provider.is_some_and(|provider|provider["controlSupported"]==true) {"Supported"} else {"Install or update Codex"});
                        });
                        if id!="codex" {hint(ui,"Task execution for this provider is not available yet.");}
                        if let Some(url)=setup_url(id) {
                            if control(ui,"Official setup guide",ready).clicked() {ui.ctx().open_url(egui::OpenUrl::new_tab(url));}
                        }
                        if let Some(provider)=provider {
                            ui.collapsing(t(ui,"Diagnostic details"),|ui| {ui.monospace(text(provider,"reasonCode"));});
                        }
                    }));
                    ui.add_space(10.0);
                }
            });
            hint(ui,"Saved sign-in does not verify model access or available quota. Checking never logs you in.");
            ui.horizontal_wrapped(|ui| {
                let mut reduced=snapshot["desktopPreferences"]["reduceMotion"].as_bool().unwrap_or(false);
                let response=ui.add_enabled(ready,egui::Checkbox::new(&mut reduced,t(ui,"Reduce motion")));
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new(("test-action","desktop_motion","Reduce motion")),response.rect);
                    data.insert_temp(egui::Id::new(("test-action-state","desktop_motion","Reduce motion")),(response.enabled(),response.rect,ui.clip_rect()));
                });
                if response.changed() && commands.is_empty() {commands.push(Command::new("desktop_motion",json!({"reduceMotion":reduced})));}
                if control(ui,"Close",true).clicked() {self.create.chat.connections_open=false;}
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
