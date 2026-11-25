//! The configuration window launches readers without sharing its unsaved form.
//! Child output is drained on separate threads; an open reader never blocks the UI.
use anyhow::{bail, Context, Result};
use eframe::egui;
use std::{
    collections::VecDeque,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

const MAX_LINE: usize = 2048;
const READY: &str = "CREXE_READER_READY";

enum Event {
    Phase(String),
    Ready,
    Finished(Result<()>),
}

struct Job {
    file: PathBuf,
    events: Receiver<Event>,
    phase: String,
    ready: bool,
    finished: bool,
    failed: bool,
}

#[derive(Default)]
pub(crate) struct Controller {
    // Each request owns its channel. Events from an older open reader cannot
    // replace the status of a newer document or complete the wrong operation.
    jobs: VecDeque<Job>,
    notice: String,
}

impl Controller {
    pub fn preparing(&self) -> bool {
        self.jobs.iter().any(|job| !job.ready && !job.finished)
    }

    pub fn poll(&mut self) {
        for job in &mut self.jobs {
            loop {
                match job.events.try_recv() {
                    Ok(Event::Phase(message)) if !job.ready => job.phase = message,
                    Ok(Event::Phase(_)) => {}
                    Ok(Event::Ready) => {
                        job.ready = true;
                        job.phase = "Leitor aberto. Você pode selecionar outro arquivo.".into();
                    }
                    Ok(Event::Finished(result)) => {
                        job.finished = true;
                        job.failed = result.is_err();
                        job.phase = match result {
                            Ok(()) => "Abertura concluída; leitor finalizado.".into(),
                            Err(error) => format!("{error:#}"),
                        };
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if !job.finished {
                            job.finished = true;
                            job.failed = true;
                            job.phase = "O acompanhamento foi interrompido. Confira o leitor antes de tentar novamente.".into();
                        }
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
        }
        // Retain active readers and the three most recent completed requests.
        let mut completed = self.jobs.iter().filter(|job| job.finished).count();
        self.jobs.retain(|job| {
            if job.finished && completed > 3 {
                completed -= 1;
                false
            } else {
                true
            }
        });
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, config: &Path, settings_busy: bool) {
        ui.group(|ui| {
            ui.heading("Gerar com CREXE");
            ui.label("Selecione um documento para gerar ou reutilizar um programa que o abra.");
            let preparing = self.preparing();
            if ui
                .add_enabled(
                    !preparing && !settings_busy,
                    egui::Button::new("Selecionar arquivo e gerar programa…"),
                )
                .clicked()
            {
                // Keep the native picker on the UI thread for AppKit support.
                // pick_file allows exactly one file and has no extension filter.
                let selected = rfd::FileDialog::new()
                    .set_title("Gerar com CREXE — selecione um arquivo")
                    .pick_file();
                match self.start(selected, config, ui.ctx()) {
                    Ok(()) => {}
                    Err(error) => self.notice = format!("{error:#}"),
                }
            }
            ui.small("Usa a configuração salva. Alterações não salvas nesta tela são preservadas e não são aplicadas à abertura.");
            ui.small("Um arquivo por seleção. O leitor desta prévia abre em modo de leitura.");
            if !self.notice.is_empty() {
                ui.label(&self.notice);
            }
            for job in &self.jobs {
                ui.separator();
                ui.label(job.file.display().to_string());
                ui.horizontal_wrapped(|ui| {
                    if !job.ready && !job.finished {
                        ui.spinner();
                    }
                    if job.failed {
                        ui.colored_label(ui.visuals().error_fg_color, &job.phase);
                    } else {
                        ui.label(&job.phase);
                    }
                });
            }
            if preparing {
                ui.small("Aguarde a preparação para fechar esta janela. As configurações continuam preservadas.");
            }
        });
    }

    fn start(
        &mut self,
        selected: Option<PathBuf>,
        config: &Path,
        ctx: &egui::Context,
    ) -> Result<()> {
        let Some(file) = selected else {
            self.notice = "Seleção cancelada. Nenhuma geração foi iniciada.".into();
            return Ok(());
        };
        if self.preparing() {
            bail!("Aguarde a preparação atual e selecione o próximo arquivo novamente.");
        }
        let file = file
            .canonicalize()
            .context("Não foi possível localizar o arquivo selecionado")?;
        if !file.is_file() {
            bail!("Selecione um arquivo local, não uma pasta.");
        }
        let engine = std::env::current_exe().context("Não foi possível localizar a engine")?;
        let default = super::config::default_path()?;
        let command = reader_command(&engine, config, &default, &file)?;
        let (send, receive) = mpsc::sync_channel(64);
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = monitor(command, &send, &ctx);
            let _ = send.send(Event::Finished(result));
            ctx.request_repaint();
        });
        self.jobs.push_back(Job {
            file,
            events: receive,
            phase: "Preparando a abertura…".into(),
            ready: false,
            finished: false,
            failed: false,
        });
        self.notice.clear();
        Ok(())
    }
}

fn reader_command(engine: &Path, active: &Path, default: &Path, file: &Path) -> Result<Command> {
    let active = std::path::absolute(active)?;
    let default = std::path::absolute(default)?;
    let mut command = Command::new(engine);
    if active.is_file() {
        command.arg("--config").arg(&active);
    } else if active != default || active.exists() {
        bail!("A configuração selecionada não está salva em um arquivo válido. Salve-a ou selecione a configuração ativa antes de gerar.");
    }
    // The parent owns progress/error presentation. --ui would detach the Windows
    // console and invalidate redirected standard handles in the child.
    command.arg("open-generating").arg("--").arg(file);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    Ok(command)
}

fn monitor(mut command: Command, send: &SyncSender<Event>, ctx: &egui::Context) -> Result<()> {
    let mut child = command
        .spawn()
        .context("Não foi possível iniciar a engine para abrir o arquivo")?;
    let stdout = child
        .stdout
        .take()
        .context("Saída de progresso indisponível")?;
    let stderr = child
        .stderr
        .take()
        .context("Saída de diagnóstico indisponível")?;
    let stdout_send = send.clone();
    let stdout_ctx = ctx.clone();
    let output = thread::spawn(move || {
        drain(stdout, |line| {
            if line.trim() == READY {
                // This signal must not be discarded when progress fills the queue.
                let _ = stdout_send.send(Event::Ready);
                stdout_ctx.request_repaint();
            } else if let Some(phase) = progress_line(line) {
                let _ = stdout_send.try_send(Event::Phase(phase.into()));
                stdout_ctx.request_repaint();
            }
        })
    });
    let diagnostics = thread::spawn(move || {
        let mut tail = String::new();
        drain(stderr, |line| {
            tail.push_str(line);
            tail.push('\n');
            truncate_tail(&mut tail, MAX_LINE);
        })
        .map(|()| tail)
    });
    let status = child
        .wait()
        .context("Não foi possível acompanhar a engine")?;
    output
        .join()
        .map_err(|_| anyhow::anyhow!("Acompanhamento de progresso interrompido"))??;
    let error = diagnostics
        .join()
        .map_err(|_| anyhow::anyhow!("Acompanhamento de diagnóstico interrompido"))??;
    if !status.success() {
        if error.trim().is_empty() {
            bail!("A engine terminou com {status}. Confira o provider, o modelo e o SDK do perfil e tente novamente.");
        }
        bail!("A abertura falhou ({status}): {}", error.trim());
    }
    Ok(())
}

fn progress_line(line: &str) -> Option<&str> {
    let line = line.trim();
    [
        "Planejando",
        "Gerando",
        "Compilando",
        "Preparando",
        "Verificando",
        "Corrigindo",
        "Abrindo",
        "Aguardando",
        "Reutilizando",
        "Leitor existente",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
    .then_some(line)
}

fn truncate_tail(value: &mut String, limit: usize) {
    if value.len() > limit {
        let mut start = value.len() - limit;
        while !value.is_char_boundary(start) {
            start += 1;
        }
        value.drain(..start);
    }
}

// A generated application may inherit stdout. Always drain, but never accumulate
// an unbounded line or forward arbitrary document output into the status panel.
fn drain(mut input: impl Read, mut line: impl FnMut(&str)) -> std::io::Result<()> {
    let mut pending = Vec::new();
    let mut oversized = false;
    let mut buffer = [0; 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            if !pending.is_empty() && !oversized {
                line(&String::from_utf8_lossy(&pending));
            }
            return Ok(());
        }
        for byte in &buffer[..count] {
            if *byte == b'\n' {
                if !oversized {
                    line(&String::from_utf8_lossy(&pending));
                }
                pending.clear();
                oversized = false;
            } else if !oversized {
                if pending.len() == MAX_LINE {
                    pending.clear();
                    oversized = true;
                } else {
                    pending.push(*byte);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn cancellation_does_not_resolve_engine_or_configuration_or_create_a_job() {
        let mut controller = Controller::default();
        controller
            .start(
                None,
                Path::new("missing-config.toml"),
                &egui::Context::default(),
            )
            .unwrap();
        assert!(controller.jobs.is_empty());
        assert!(!controller.preparing());
        assert!(controller.notice.contains("cancelada"));
    }

    #[test]
    fn command_uses_only_saved_settings_and_preserves_paths_as_single_arguments() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("configuração salva.toml");
        std::fs::write(&config, "saved settings sentinel").unwrap();
        let document = directory.path().join("-reunião & dados ;.formato-novo");
        let command = reader_command(Path::new("engine.exe"), &config, &config, &document).unwrap();
        let args: Vec<OsString> = command.get_args().map(OsString::from).collect();
        assert_eq!(
            args,
            vec![
                OsString::from("--config"),
                config.as_os_str().into(),
                "open-generating".into(),
                "--".into(),
                document.as_os_str().into()
            ]
        );
        assert_eq!(command.get_program(), "engine.exe");
        assert_eq!(
            std::fs::read_to_string(&config).unwrap(),
            "saved settings sentinel"
        );
    }

    #[test]
    fn absent_default_uses_built_in_defaults_but_missing_custom_file_fails() {
        let directory = tempfile::tempdir().unwrap();
        let default = directory.path().join("default.toml");
        let command =
            reader_command(Path::new("engine"), &default, &default, Path::new("data")).unwrap();
        assert_eq!(command.get_args().next().unwrap(), "open-generating");
        assert!(!default.exists());
        assert!(reader_command(
            Path::new("engine"),
            &directory.path().join("custom.toml"),
            &default,
            Path::new("data")
        )
        .is_err());
    }

    #[test]
    fn reader_ready_releases_selection_without_waiting_for_the_program_to_exit() {
        let (send, receive) = mpsc::sync_channel(8);
        let mut controller = Controller::default();
        controller.jobs.push_back(Job {
            file: "one.md".into(),
            events: receive,
            phase: String::new(),
            ready: false,
            finished: false,
            failed: false,
        });
        assert!(controller.preparing());
        send.send(Event::Ready).unwrap();
        controller.poll();
        assert!(!controller.preparing());
        assert!(!controller.jobs[0].finished);
        send.send(Event::Finished(Ok(()))).unwrap();
        controller.poll();
        assert!(controller.jobs[0].finished);
    }

    #[test]
    fn each_document_retains_its_own_progress_and_failure() {
        let mut controller = Controller::default();
        let mut senders = Vec::new();
        for name in ["old.md", "new.md"] {
            let (send, receive) = mpsc::sync_channel(8);
            senders.push(send);
            controller.jobs.push_back(Job {
                file: name.into(),
                events: receive,
                phase: "initial".into(),
                ready: false,
                finished: false,
                failed: false,
            });
        }
        senders[0].send(Event::Ready).unwrap();
        senders[1].send(Event::Phase("Compilando…".into())).unwrap();
        controller.poll();
        assert!(controller.jobs[0].ready);
        assert_eq!(controller.jobs[1].phase, "Compilando…");
        senders[0]
            .send(Event::Finished(Err(anyhow::anyhow!("old failure"))))
            .unwrap();
        controller.poll();
        assert!(controller.jobs[0].failed);
        assert!(!controller.jobs[1].finished);
        assert_eq!(controller.jobs[1].phase, "Compilando…");
    }

    #[test]
    fn output_is_bounded_and_document_text_is_not_a_progress_message() {
        let input = format!("{}\nGerando leitor…\n{READY}\n", "x".repeat(MAX_LINE * 20));
        let mut lines = Vec::new();
        drain(input.as_bytes(), |line| lines.push(line.to_owned())).unwrap();
        assert_eq!(lines, vec!["Gerando leitor…", READY]);
        assert_eq!(progress_line("Gerando leitor…"), Some("Gerando leitor…"));
        assert!(progress_line("document secret sentinel").is_none());
        let mut tail = "á".repeat(20);
        truncate_tail(&mut tail, 9);
        assert!(tail.len() <= 9);
        assert!(!tail.is_empty());
    }

    #[test]
    fn child_monitor_drains_both_pipes_and_reports_exit_failures() {
        for outcome in ["success", "failure"] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "reader_desktop::tests::monitor_subprocess_fixture",
                    "--ignored",
                    "--nocapture",
                ])
                .env("CREXE_DESKTOP_MONITOR_FIXTURE", outcome)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
            }
            let (send, receive) = mpsc::sync_channel(8);
            let result = monitor(command, &send, &egui::Context::default());
            assert!(receive
                .try_iter()
                .any(|event| matches!(event, Event::Ready)));
            if outcome == "success" {
                result.unwrap();
            } else {
                let error = format!("{:#}", result.unwrap_err());
                assert!(error.contains("fixture failure after opening"));
                assert!(error.len() < MAX_LINE + 256);
            }
        }
    }

    #[test]
    #[ignore = "helper subprocess; exercised by child_monitor_drains_both_pipes_and_reports_exit_failures"]
    fn monitor_subprocess_fixture() {
        let Ok(outcome) = std::env::var("CREXE_DESKTOP_MONITOR_FIXTURE") else {
            return;
        };
        println!("{READY}");
        // Much larger than a pipe buffer: neither pipe may wait for the other.
        println!("{}", "arbitrary document output".repeat(10000));
        eprintln!("{}", "unbounded diagnostic".repeat(10000));
        if outcome == "failure" {
            eprintln!("fixture failure after opening");
            std::process::exit(23);
        }
    }
}
