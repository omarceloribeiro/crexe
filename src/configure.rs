//! Native, optional desktop configuration. CLI execution never starts a GUI runtime.
use super::{
    config,
    config_editor::{Editor, KeyChange},
};
use anyhow::Result;
use eframe::egui;
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

pub(crate) fn open(path: Option<&Path>, import: Option<&Path>) -> Result<()> {
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        anyhow::bail!(
            "Configurações requer um desktop. Use --help e --config no terminal sem display."
        );
    }
    let path = path
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(config::default_path)?;
    let mut editor = Editor::load(&path)?;
    if let Some(import) = import {
        editor.import(import)?;
    }
    let imported = import.is_some();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 640.0])
            .with_min_inner_size([460.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native("CREXE — Configurações", options, Box::new(move |cc| {
        #[cfg(windows)]
        unsafe { windows_sys::Win32::System::Console::FreeConsole(); }
        cc.egui_ctx.set_theme(egui::Theme::Light);
        Ok(Box::new(Window { editor, key: String::new(), remove_key: false, models: Vec::new(), import_path: String::new(),
            status: if imported { "TOML importado para revisão. Clique em Salvar para aplicar.".into() } else { String::new() },
            pending: None }))
    })).map_err(|_| anyhow::anyhow!("Não foi possível abrir a tela de configuração. Verifique se há uma sessão gráfica disponível."))
}

enum Reply {
    Saved(Editor, String),
    Models(Vec<String>),
    Loaded(Editor),
}
struct Window {
    editor: Editor,
    key: String,
    remove_key: bool,
    models: Vec<String>,
    import_path: String,
    status: String,
    pending: Option<mpsc::Receiver<Result<Reply>>>,
}

impl Window {
    fn key_change(&self) -> KeyChange {
        if self.remove_key {
            KeyChange::Remove
        } else if self.key.is_empty() {
            KeyChange::Keep
        } else {
            KeyChange::Replace(self.key.clone())
        }
    }
    fn work(&mut self, ctx: &egui::Context, task: impl FnOnce() -> Result<Reply> + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        self.status = "Aguarde…".into();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(task());
            ctx.request_repaint();
        });
    }
    fn clear_key(&mut self) {
        self.key.clear();
        self.remove_key = false;
    }
}

impl eframe::App for Window {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(Reply::Saved(editor, message)) => {
                            self.editor = editor;
                            self.clear_key();
                            self.status = message;
                        }
                        Ok(Reply::Models(models)) => {
                            self.status = format!(
                                "Conexão OK. {} modelos disponíveis. Nenhum programa foi gerado.",
                                models.len()
                            );
                            self.models = models;
                        }
                        Ok(Reply::Loaded(editor)) => {
                            self.editor = editor;
                            self.clear_key();
                            self.models.clear();
                            self.status = "Configuração carregada. Revise antes de salvar.".into();
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.status = "A operação não foi concluída. Recarregue a configuração antes de tentar novamente.".into();
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.pending.is_some() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, ui.visuals().panel_fill);
        egui::Frame::central_panel(ui.style()).inner_margin(18.0).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.heading("Creative Executable");
            ui.label("Escolha quem vai gerar seus programas.");
            ui.add_space(12.0);
            ui.add_enabled_ui(self.pending.is_none(), |ui| {
                let mut selected = self.editor.profile.clone();
                egui::ComboBox::from_label("Perfil / provider").selected_text(&selected).show_ui(ui, |ui| {
                    for name in self.editor.profiles() { ui.selectable_value(&mut selected, name.clone(), name); }
                });
                if selected != self.editor.profile {
                    match self.editor.select(&selected) {
                        Ok(()) => { self.clear_key(); self.models.clear(); self.status = "Perfil selecionado. Edições não salvas do perfil anterior foram descartadas.".into(); }
                        Err(e) => self.status = e.to_string(),
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Serviço:");
                    ui.label(self.editor.provider.kind.label());
                });
                ui.label("Modelo");
                ui.add(egui::TextEdit::singleline(&mut self.editor.provider.model).desired_width(f32::INFINITY));
                if !self.models.is_empty() {
                    egui::ComboBox::from_label("Modelos disponíveis").selected_text("Escolher da lista").show_ui(ui, |ui| {
                        for model in &self.models { ui.selectable_value(&mut self.editor.provider.model, model.clone(), model); }
                    });
                }
                ui.small("Você também pode digitar o nome do modelo. A lista não garante compatibilidade de geração.");
                ui.add_space(12.0);
                ui.label(if self.editor.provider.credential.is_some() { "Chave: referência salva no cofre (verificada ao testar ou usar)." }
                    else if self.editor.provider.api_key_env.is_some() { "Chave: não salva no cofre. Configuração por variável de ambiente disponível." }
                    else { "Chave: não configurada (normal no Ollama local)." });
                ui.add_enabled_ui(!self.remove_key, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.key).password(true).hint_text("Nova API key — deixe vazio para manter").desired_width(f32::INFINITY));
                });
                if self.editor.provider.credential.is_some() {
                    ui.checkbox(&mut self.remove_key, "Remover chave salva ao salvar");
                    if self.remove_key && self.editor.provider.api_key_env.is_some() { ui.small("Após remover, a configuração legada por variável de ambiente volta a valer."); }
                }
                ui.small("A chave digitada só é persistida ao salvar, no cofre deste usuário.");
                if ui.button("Testar conexão / atualizar modelos").clicked() {
                    let editor = self.editor.clone(); let change = self.key_change();
                    self.work(&ctx, move || editor.models(&change).map(Reply::Models));
                }
                ui.small("Consulta a lista de modelos; não gera código nem consome tokens de geração.");
                ui.add_space(10.0);
                ui.collapsing("Avançado", |ui| {
                    ui.label("Endereço do provider");
                    ui.add(egui::TextEdit::singleline(&mut self.editor.provider.base_url).desired_width(f32::INFINITY));
                    ui.small("Trocar o endereço exige informar novamente ou remover a chave salva.");
                    ui.horizontal(|ui| { ui.label("Timeout (segundos)"); ui.add(egui::DragValue::new(&mut self.editor.provider.timeout_seconds).range(1..=1800)); });
                    ui.horizontal(|ui| { ui.label("Tokens de saída"); ui.add(egui::DragValue::new(&mut self.editor.provider.max_output_tokens).range(128..=65536)); });
                    if self.editor.provider.kind == config::Kind::Ollama {
                        ui.horizontal(|ui| { ui.label("Contexto"); ui.add(egui::DragValue::new(&mut self.editor.provider.context_tokens).range(512..=131072)); });
                    }
                    if self.editor.provider.kind == config::Kind::Deepseek {
                        let mut thinking = self.editor.provider.thinking.unwrap_or(false);
                        if ui.checkbox(&mut thinking, "Habilitar raciocínio").changed() {
                            self.editor.provider.thinking = Some(thinking);
                        }
                        ui.small("Raciocínio pode aumentar o tempo e o uso de tokens; pode exigir mais tokens de saída.");
                    }
                });
                ui.collapsing("Importar configurações de um TOML", |ui| {
                    ui.label("Caminho do arquivo (por exemplo, config.example.toml editado)");
                    ui.add(egui::TextEdit::singleline(&mut self.import_path).desired_width(f32::INFINITY));
                    ui.small("Importa o provider padrão e seus parâmetros. Preserva as regras de execução locais.");
                    if ui.button("Carregar para revisão").clicked() {
                        let path = PathBuf::from(self.import_path.trim().trim_matches('"')); let mut editor = self.editor.clone();
                        self.work(&ctx, move || { editor.import(&path)?; Ok(Reply::Loaded(editor)) });
                    }
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Salvar").clicked() {
                        let editor = self.editor.clone(); let change = self.key_change();
                        self.work(&ctx, move || editor.save(change).map(|(e,m)| Reply::Saved(e,m)));
                    }
                    if ui.button("Recarregar").clicked() {
                        let path = self.editor.path.clone();
                        self.work(&ctx, move || Editor::load(&path).map(Reply::Loaded));
                    }
                    if ui.button("Fechar").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });
            });
            ui.add_space(8.0);
            if self.pending.is_some() { ui.spinner(); }
            ui.label(&self.status);
            ui.separator();
            ui.small(format!("Configuração ativa: {}", self.editor.path.display()));
            ui.small("As mudanças afetam a próxima execução. Nenhum provider pago é escolhido automaticamente.");
        });
        });
    }
}
