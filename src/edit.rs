//! A small plain-text editor. Saving never invokes generation or configuration.
use super::document_editor::Document;
use anyhow::Result;
use eframe::egui;
use std::path::Path;

pub(crate) fn open(path: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        anyhow::bail!("O editor CREXE requer uma sessão gráfica. Edite o arquivo em um editor de texto disponível.");
    }
    let document = Document::load(path)?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([460.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CREXE — Editor",
        options,
        Box::new(move |cc| {
            #[cfg(windows)]
            unsafe {
                windows_sys::Win32::System::Console::FreeConsole();
            }
            cc.egui_ctx.set_theme(egui::Theme::Light);
            Ok(Box::new(Window {
                document,
                pending: None,
                closing: false,
                focus_text: true,
                status: String::new(),
            }))
        }),
    )
    .map_err(|_| {
        anyhow::anyhow!(
            "Não foi possível abrir o editor. Verifique se há uma sessão gráfica disponível."
        )
    })
}

#[derive(Clone, Copy)]
enum Action {
    Close,
    Reload,
}

struct Window {
    document: Document,
    pending: Option<Action>,
    closing: bool,
    focus_text: bool,
    status: String,
}

impl Window {
    fn save(&mut self) -> bool {
        match self.document.save() {
            Ok(()) => {
                self.status = "Arquivo salvo. Abra o .crexe para gerar o programa.".into();
                true
            }
            Err(error) => {
                self.status = format!("{error:#}");
                false
            }
        }
    }
    fn perform(&mut self, action: Action, ctx: &egui::Context) {
        self.pending = None;
        match action {
            Action::Close => {
                self.closing = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Reload => match self.document.reload() {
                Ok(()) => self.status = "Arquivo recarregado.".into(),
                Err(error) => self.status = format!("{error:#}"),
            },
        }
    }
    fn request(&mut self, action: Action, ctx: &egui::Context) {
        if self.document.dirty() {
            self.pending = Some(action);
        } else {
            self.perform(action, ctx);
        }
    }
}

impl Window {
    fn logic(&mut self, ctx: &egui::Context) {
        if self.pending.is_none() {
            let replacement = ctx.input_mut(|input| {
                let mut replacement = None;
                input.consume_accesskit_action_requests(
                    egui::Id::new("document-text"),
                    |request| {
                        if request.action == egui::accesskit::Action::SetValue {
                            if let Some(egui::accesskit::ActionData::Value(value)) = &request.data {
                                replacement = Some(value.to_string());
                            }
                            true
                        } else {
                            false
                        }
                    },
                );
                replacement
            });
            if let Some(text) = replacement {
                self.document.text = text.replace("\r\n", "\n");
            }
        }
        if !self.closing && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request(Action::Close, ctx);
        }
        if save_shortcut(ctx) && self.pending.is_none() {
            self.save();
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}{} — CREXE Editor",
            self.document
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            if self.document.dirty() { " *" } else { "" }
        )));
    }
    fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Frame::central_panel(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Salvar").clicked() {
                        self.save();
                    }
                    if ui.button("Recarregar").clicked() {
                        self.request(Action::Reload, &ctx);
                    }
                    if ui.button("Fechar").clicked() {
                        self.request(Action::Close, &ctx);
                    }
                    if self.document.dirty() {
                        ui.label("Alterações não salvas");
                    }
                });
                ui.small(self.document.path.display().to_string());
                ui.label(&self.status);
                ui.separator();
                let size = ui.available_size();
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let response = ui.add_sized(
                            size,
                            egui::TextEdit::multiline(&mut self.document.text)
                                .id(egui::Id::new("document-text"))
                                .font(egui::TextStyle::Monospace)
                                .desired_width(size.x)
                                .hint_text("Descreva o programa que deseja criar…"),
                        );
                        if self.focus_text {
                            response.request_focus();
                            self.focus_text = false;
                        }
                    });
            });
        if let Some(action) = self.pending {
            egui::Modal::new(egui::Id::new("unsaved-document")).show(&ctx, |ui| {
                ui.heading("Há alterações não salvas");
                ui.label("Deseja salvar antes de continuar?");
                ui.horizontal(|ui| {
                    if ui.button("Salvar").clicked() && self.save() {
                        self.perform(action, &ctx);
                    }
                    if ui.button("Descartar").clicked() {
                        self.perform(action, &ctx);
                    }
                    if ui.button("Cancelar").clicked() {
                        self.pending = None;
                    }
                });
                ui.label(&self.status);
            });
        }
    }
}

fn save_shortcut(ctx: &egui::Context) -> bool {
    ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
}

impl eframe::App for Window {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.logic(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.ui(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_control_handles_selection_paste_undo_redo_and_save_shortcut_without_a_display() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("document.crexe");
        std::fs::write(&path, "original").unwrap();
        let mut window = Window {
            document: Document::load(&path).unwrap(),
            pending: None,
            closing: false,
            focus_text: true,
            status: String::new(),
        };
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let mut frame = |window: &mut Window, events: Vec<egui::Event>| {
            time += 1.0;
            let mut save = false;
            ctx.run_ui(
                egui::RawInput {
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ui| {
                    save = save_shortcut(ui.ctx());
                    window.ui(ui);
                },
            )
            .drop_without_applying_deltas();
            save
        };
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        frame(&mut window, vec![]);
        frame(
            &mut window,
            vec![key(egui::Key::A, egui::Modifiers::COMMAND)],
        );
        frame(
            &mut window,
            vec![egui::Event::Paste("ação\nsegunda linha".into())],
        );
        assert_eq!(window.document.text, "ação\nsegunda linha");
        frame(
            &mut window,
            vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
        );
        assert_eq!(window.document.text, "original");
        frame(
            &mut window,
            vec![key(egui::Key::Y, egui::Modifiers::COMMAND)],
        );
        assert_eq!(window.document.text, "ação\nsegunda linha");
        assert!(frame(
            &mut window,
            vec![key(egui::Key::S, egui::Modifiers::COMMAND)]
        ));
        assert!(!frame(
            &mut window,
            vec![key(egui::Key::S, egui::Modifiers::NONE)]
        ));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "original");
    }
}
