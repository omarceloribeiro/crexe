//! Document execution, incremental generation and publication of approved revisions.
use crate::*;

pub(crate) fn exec(
    file: &Path,
    overrides: BTreeMap<String, String>,
    rebuild: bool,
    fresh: bool,
    cache_dir: Option<&Path>,
    cli: &Cli,
) -> Result<()> {
    exec_inner(file, overrides, rebuild, fresh, cache_dir, cli, None, None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn exec_with_original(
    file: &Path,
    overrides: BTreeMap<String, String>,
    rebuild: bool,
    fresh: bool,
    cache_dir: Option<&Path>,
    cli: &Cli,
    original: &crexb::OriginalRecipe,
    source: &Path,
) -> Result<()> {
    exec_inner(
        file,
        overrides,
        rebuild,
        fresh,
        cache_dir,
        cli,
        None,
        Some((original, source)),
    )
}

pub(crate) fn exec_reader(
    recipe: &Path,
    document: &Path,
    cli: &Cli,
    settings: &config::Settings,
    generation: &mut generator::Session,
    prepared: &mut dyn FnMut(&Path) -> Result<bool>,
) -> Result<()> {
    exec_inner(
        recipe,
        BTreeMap::new(),
        false,
        false,
        None,
        cli,
        Some(ReaderInvocation {
            document,
            settings,
            generation,
            prepared,
        }),
        None,
    )
}

struct ReaderInvocation<'a> {
    document: &'a Path,
    settings: &'a config::Settings,
    generation: &'a mut generator::Session,
    // true: caller owns final launch (e.g. a persisted reader); false: launch here.
    prepared: &'a mut dyn FnMut(&Path) -> Result<bool>,
}

#[allow(clippy::too_many_arguments)]
fn exec_inner(
    file: &Path,
    overrides: BTreeMap<String, String>,
    rebuild: bool,
    fresh: bool,
    cache_dir: Option<&Path>,
    cli: &Cli,
    reader: Option<ReaderInvocation<'_>>,
    original: Option<(&crexb::OriginalRecipe, &Path)>,
) -> Result<()> {
    let reader_mode = reader.is_some();
    let (runtime_args, supplied_settings, mut supplied_generation, mut reader_prepared) =
        if let Some(reader) = reader {
            (
                vec![reader.document.as_os_str().to_owned()],
                Some(reader.settings.clone()),
                Some(reader.generation),
                Some(reader.prepared),
            )
        } else {
            (Vec::new(), None, None, None)
        };
    let path = file
        .canonicalize()
        .with_context(|| format!("CREXE not found: {}", file.display()))?;
    // Keep a package's history stable across edits of its immutable resolved recipe.
    let document =
        original.map_or_else(|| Ok(path.clone()), |(_, source)| source.canonicalize())?;
    let raw = format::read(&path)?;
    if format::is_empty(&raw) {
        if cli.ui && cfg!(windows) {
            if presentation::confirm_edit_empty()? {
                edit::open(&path)?;
            }
            return Ok(());
        }
        bail!(
            "O arquivo está vazio. Abra para edição: crexe edit \"{}\"",
            path.display()
        );
    }
    let text_original;
    let original = match original {
        Some((original, _)) => original,
        None => {
            text_original = crexb::OriginalRecipe::new("crexe-text", raw.as_bytes().to_vec())?;
            &text_original
        }
    };
    let mut intent = None;
    let mut spec = match format::parse(&raw, &path)? {
        format::Document::Yaml(spec) => {
            if get_path(&spec, &["engine_project"]).is_some()
                || get_path(&spec, &["engine_profile"]).is_some()
            {
                bail!("Reserved engine fields in YAML");
            }
            spec
        }
        format::Document::Intent {
            name,
            prompt,
            profile,
            format,
        } => {
            println!("Format: {format}; app: {name}");
            let selected = cli.profile.clone().or(profile);
            let spec = profiles::resolve(&name, &prompt, selected.as_deref())?;
            intent = Some((name, prompt, selected));
            spec
        }
    };
    let mut settings = if let Some(settings) = supplied_settings {
        settings
    } else {
        config::Settings::load(
            cli.config.as_deref(),
            cli.provider.as_deref(),
            cli.model.as_deref(),
            cli.env_file.as_deref(),
        )?
    };
    if let Some(seconds) =
        get_path(&spec, &["policies", "timeoutSeconds", "generate"]).and_then(Value::as_u64)
    {
        settings.provider.timeout_seconds = settings.provider.timeout_seconds.min(seconds);
    }
    if get_path(&spec, &["generator"]).is_some() {
        println!("Legacy generator settings are ignored; provider and credentials come from local configuration.");
    }
    let inputs = resolve_inputs(&spec, overrides)?;
    let runtime = detect_runtime_vars();
    let mut ctx = inputs.clone();
    for (key, value) in [
        ("OS", &runtime.os),
        ("ARCH", &runtime.arch),
        ("EXE_EXT", &runtime.exe_ext),
        ("USER_HOME", &runtime.user_home),
    ] {
        ctx.insert(key.into(), value.clone());
    }
    let target = select_target(&spec, &ctx)?;
    println!(
        "Target: {target} (OS={}, ARCH={})",
        runtime.os, runtime.arch
    );
    validate_commands(&spec, &target, &ctx)?;
    let mut identity = raw.as_bytes().to_vec();
    identity.extend(serde_yaml::to_string(&spec)?.as_bytes());
    let mut legacy_identity = identity.clone();
    legacy_identity.extend(settings.legacy_identity()?);
    identity.extend(settings.identity()?);
    let request = compute_fingerprint(&identity, &spec, &inputs)?;
    let mut signature = identity.clone();
    signature.extend(serde_json::to_vec(&json!({"inputs": inputs, "rebuild": rebuild, "fresh": fresh,
        "no_run": cli.no_run, "export": cli.export, "repairs": cli.max_repairs, "execution": settings.execution, "cache_dir": cache_dir}))?);
    // Reader requests already hold a per-type lease. They must each reach launch,
    // rather than being coalesced like repeated opens of one .crexe recipe.
    let mut preparation = if reader_mode {
        None
    } else {
        let Some(session) = preparation::Session::acquire(&document, &signature)? else {
            return Ok(());
        };
        Some(session)
    };
    let _progress = if reader_mode {
        None
    } else {
        Some(presentation::Progress::start(cli.ui)?)
    };
    let max_attempts = settings
        .execution
        .attempts(settings.provider.max_output_tokens, cli.max_repairs)
        .min(
            supplied_generation
                .as_ref()
                .map_or(u8::MAX, |session| session.remaining_requests()),
        );
    presentation::details(&format!(
        "Provider: {} ({})\nModelo: {}\nTentativa 1 de {max_attempts} — até {} correções",
        settings.profile,
        settings.provider.kind.label(),
        settings.provider.model,
        max_attempts.saturating_sub(1)
    ));
    if max_attempts < settings.execution.max_provider_requests {
        println!("Limite configurado: {}; limite efetivo: {max_attempts} (orçamento de tokens e/ou --max-repairs).", settings.execution.max_provider_requests);
    }
    let mut ready = || {
        presentation::finish();
        if let Some(session) = preparation.as_mut() {
            session.release();
        }
        if !cli.no_run && matches!(&cli.command, Commands::OpenGenerating { .. }) {
            println!("CREXE_READER_READY");
        }
    };
    let old_cache = get_cache_state(&document, &legacy_identity, &spec, &inputs, cache_dir)?;
    let root = old_cache.root.parent().context("Invalid cache root")?;
    let history = history::History::new(root, &document)?;
    let mut baseline = if fresh {
        None
    } else {
        history
            .load()
            .context("Não foi possível ler o baseline aprovado. Use --fresh para gerar sem ele")?
    };
    let (_, user_prompt) = build_prompt(&spec, &target, &ctx)?;
    if baseline.is_none() && !fresh {
        baseline = adopt_legacy(
            &old_cache,
            &history,
            &document,
            history::Metadata::new(
                request.clone(),
                None,
                user_prompt.clone(),
                spec.clone(),
                serde_json::from_slice(&settings.identity()?)?,
            ),
        )?;
    }
    if !rebuild && !fresh {
        if let Some(base) = &baseline {
            if base.metadata.request == request && base.cached() {
                println!("Cache hit: skipping generate/build");
                if reader_mode {
                    presentation::phase("Abrindo com leitor existente…");
                }
                presentation::details(&format!(
                    "Provider: {}\nModelo: {}\nRevisão aprovada — nenhuma tentativa necessária",
                    settings.profile, settings.provider.model
                ));
                if !cli.no_run {
                    check_graphics(
                        &base.metadata.spec,
                        &target,
                        &ctx,
                        &base.revision,
                        &settings,
                    )?;
                }
                if let Some(prepared) = reader_prepared.as_deref_mut() {
                    if prepared(&base.revision)? {
                        return Ok(());
                    }
                }
                return launch(
                    &base.metadata.spec,
                    &base.revision,
                    &ctx,
                    &settings,
                    cli,
                    &runtime_args,
                    &mut ready,
                );
            }
        }
    }
    if baseline.is_none() {
        println!("Sem baseline verificável para este documento; iniciando uma nova revisão.");
    }
    if max_attempts == 0 {
        bail!("Local output-token budget exhausted; no provider request was made");
    }
    settings.execution.max_provider_requests = max_attempts;
    let parent = baseline.as_ref().map(history::Baseline::id);
    let cache = entry(
        root,
        &document,
        &request,
        parent.as_deref().unwrap_or("initial"),
    );
    let _cache_lock = cache.lock()?;
    let workspace = tempfile::Builder::new().prefix("crexe-work-").tempdir()?;
    let workspace_path = workspace.path().canonicalize()?;
    println!("Workspace: {}", workspace_path.display());
    let result = (|| -> Result<PathBuf> {
        let mut allowed = Vec::new();
        if let Some((name, prompt, fixed)) = &intent {
            if fixed.is_none() {
                if let Some(base) = &baseline {
                    if base.metadata.profile != "legacy" {
                        spec = profiles::resolve(name, prompt, Some(&base.metadata.profile))?;
                    }
                }
                allowed = profiles::available(name, prompt, &workspace_path, &settings)?;
                let current = get_path(&spec, &["engine_profile"])
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !allowed.iter().any(|p| p == current) {
                    spec = profiles::resolve(name, prompt, Some(&allowed[0]))?;
                }
            }
        }
        preflight(&spec, &target, &ctx, &workspace_path, &settings)?;
        let (mut system, _) = build_prompt(&spec, &target, &ctx)?;
        if !allowed.is_empty() {
            let (name, prompt, _) = intent.as_ref().unwrap();
            system = profiles::selection_prompt(
                name,
                prompt,
                &allowed,
                get_path(&spec, &["engine_profile"])
                    .and_then(Value::as_str)
                    .unwrap(),
            )?;
        }
        let guidance = "\nWhen previous approved sources are supplied, evolve that application: preserve unrelated behavior, layout, tests and source code. A complete rewrite or a supported profile change is allowed when the new request requires it. The CURRENT REQUEST takes precedence over the PREVIOUS REQUEST. Return a FULL files[] snapshot, not a patch. Optional metadata: profile (chosen supported profile), change_kind (incremental or recreate), reason (short explanation; required for a profile change). A visual 3D effect does not automatically require OpenGL; choose according to actual requirements. Engine-owned project files supplied as context must not be returned in files[].";
        system.push_str(guidance);
        if reader_mode
            && get_path(&spec, &["engine_profile"])
                .and_then(Value::as_str)
                .is_some_and(|profile| profile.starts_with("dotnet-"))
        {
            system.push_str("\nFor --crexe-reader-check, serialize an object with lowercase kind, content and truncated keys using System.Text.Json.JsonSerializer. Never construct JSON using string interpolation or manual escaping. Use new UTF8Encoding(false, true) to detect invalid UTF-8; Encoding.UTF8.GetString silently replaces invalid bytes and does not satisfy the contract. Invalid UTF-8 must produce hex even without NUL bytes. The headless self-test must assert actual preview/decoding logic, succeed for correct behavior, and run before GUI initialization. Preserve exact whitespace and Unicode in the unrendered check content.");
            if get_path(&spec, &["engine_profile"]).and_then(Value::as_str)
                == Some("dotnet-winforms")
            {
                system.push_str("\nFor the Windows Forms reader UI, preserve visible line breaks and blank lines from both LF and CRLF documents. Keep the raw preview unchanged for --crexe-reader-check; normalize only the display string with rawContent.ReplaceLineEndings(Environment.NewLine) before assigning it to a TextBox. Use a read-only multiline control with a readable font, adequate contrast, word wrapping and vertical scrolling, filling the resizable content area. Keep display formatting separate from decoding and include headless self-test assertions for LF, CRLF and blank-line presentation without changing the raw check content.");
            }
        }
        let previous = baseline.as_ref().map(|b| b.metadata.prompt.as_str());
        let baseline_sources = baseline
            .as_ref()
            .map(history::Baseline::files)
            .transpose()?
            .unwrap_or_default();
        let request_prompt = if let Some(base) = &baseline {
            format!("PREVIOUS REQUEST:\n{}\nPREVIOUS APPROVED SOURCE FILES (including read-only engine project files):\n{}\nCURRENT REQUEST:\n{user_prompt}", base.metadata.prompt, serde_json::to_string(&baseline_sources)?)
        } else {
            format!("CURRENT REQUEST:\n{user_prompt}")
        };
        presentation::phase(if baseline.is_some() {
            "Atualizando seu programa a partir do baseline…"
        } else {
            "Gerando seu programa…"
        });
        let mut local_generation = generator::Session::new(&settings);
        let generation = supplied_generation
            .as_deref_mut()
            .unwrap_or(&mut local_generation);
        let mut generated = generation.generate(&system, &request_prompt)?;
        let mut change_kind = None;
        let mut change_reason = None;
        for attempt in 0..max_attempts {
            if executor::cancelled() {
                bail!("Cancelled; generation was not published");
            }
            if generated
                .change_kind
                .as_deref()
                .is_some_and(|k| !matches!(k, "incremental" | "recreate"))
            {
                bail!("Invalid generation change_kind");
            }
            if generated.reason.as_ref().is_some_and(|r| r.len() > 2000) {
                bail!("Generation reason exceeds limit");
            }
            if let Some(chosen) = &generated.profile {
                let current = get_path(&spec, &["engine_profile"])
                    .and_then(Value::as_str)
                    .unwrap_or("legacy");
                if chosen != current {
                    if !allowed.contains(chosen) {
                        bail!("Provider selected a profile that is unavailable or explicitly fixed: {chosen}");
                    }
                    if generated
                        .reason
                        .as_ref()
                        .is_none_or(|r| r.trim().is_empty())
                    {
                        bail!("Profile changes require an explanation");
                    }
                    let (name, prompt, _) = intent.as_ref().unwrap();
                    spec = profiles::resolve(name, prompt, Some(chosen))?;
                    generated.change_kind = Some("recreate".into());
                }
            }
            let current_profile = get_path(&spec, &["engine_profile"])
                .and_then(Value::as_str)
                .unwrap_or("legacy");
            if !allowed.is_empty()
                && baseline.as_ref().is_some_and(|base| {
                    base.metadata.profile != "legacy" && base.metadata.profile != current_profile
                })
            {
                if generated
                    .reason
                    .as_ref()
                    .or(change_reason.as_ref())
                    .is_none_or(|r: &String| r.trim().is_empty())
                {
                    bail!("Profile changes require an explanation");
                }
                generated.change_kind = Some("recreate".into());
            }
            // Older responses can omit metadata, including on a repair. Retain the
            // effective choice and its explanation across the entire generation.
            generated.profile = Some(current_profile.to_owned());
            if generated.change_kind.is_some() {
                change_kind.clone_from(&generated.change_kind);
            }
            if generated.reason.is_some() {
                change_reason.clone_from(&generated.reason);
            }
            if change_kind.as_deref() == Some("recreate") {
                presentation::phase("Recriando seu programa para atender ao novo pedido…");
            }
            preflight(&spec, &target, &ctx, &workspace_path, &settings)?;
            let attempt_path = workspace_path.join(format!("attempt-{}", attempt + 1));
            fs::create_dir(&attempt_path)?;
            profiles::validate_files(&spec, &generated.files)?;
            apply_generated_files(&attempt_path, &generated.files, &ctx)?;
            profiles::scaffold(&spec, &attempt_path)?;
            if baseline
                .as_ref()
                .is_some_and(|base| base.metadata.profile == "dotnet-opengl")
                && get_path(&spec, &["engine_profile"]).and_then(Value::as_str)
                    == Some("dotnet-opengl")
            {
                if let Some(lock) = baseline_sources
                    .iter()
                    .find(|f| f.path == "packages.lock.json")
                {
                    fs::write(attempt_path.join("packages.lock.json"), &lock.content)?;
                }
            }
            let attempt_spec = profiles::materialize(&spec, &attempt_path, &generated.files, &ctx)?;
            let built = build(
                &attempt_spec,
                &target,
                &ctx,
                &attempt_path,
                &settings,
                !cli.no_run,
            )
            .and_then(|()| {
                if reader_mode {
                    presentation::phase("Verificando a leitura de arquivos de teste…");
                    reader::validate_application(
                        &attempt_spec,
                        &target,
                        &ctx,
                        &attempt_path,
                        &settings,
                    )
                } else {
                    Ok(())
                }
            });
            match built {
                Ok(()) => {
                    presentation::phase("Preparando os arquivos…");
                    package::create(
                        &attempt_spec,
                        &target,
                        &ctx,
                        &attempt_path,
                        &generated.files,
                    )?;
                    let mut metadata = history::Metadata::new(
                        request.clone(),
                        parent.clone(),
                        user_prompt.clone(),
                        attempt_spec,
                        serde_json::from_slice(&settings.identity()?)?,
                    );
                    metadata.change_kind = change_kind;
                    metadata.reason = change_reason;
                    metadata.write(&attempt_path)?;
                    crexb::write_original(original, &attempt_path)?;
                    if executor::cancelled() {
                        bail!("Cancelled; generation was not published");
                    }
                    let revision = cache.publish(&attempt_path)?;
                    if executor::cancelled() {
                        bail!("Cancelled; document history was not changed");
                    }
                    history.publish(&revision)?;
                    return Ok(revision);
                }
                Err(error) => {
                    let diagnostics = format!("{error:#}");
                    fs::write(
                        workspace_path.join(format!("attempt-{}.log", attempt + 1)),
                        &diagnostics,
                    )?;
                    if executor::cancelled() {
                        return Err(error);
                    }
                    if attempt + 1 == max_attempts {
                        return Err(error).context(format!("Limite de {max_attempts} tentativas atingido (local provider request budget exhausted)"));
                    }
                    println!(
                        "Build/test failed; requesting correction {}/{}",
                        attempt + 1,
                        max_attempts - 1
                    );
                    let repair = format!("PREVIOUS REQUEST:\n{}\nCURRENT REQUEST:\n{user_prompt}\nCurrent complete source files and selected profile:\n{}\nCompiler/test diagnostics:\n{}\nCorrect the actual failure. Return the FULL corrected files[] snapshot, not a patch. Preserve requested behavior and tests. Do not change engine-owned commands or install global tools.", previous.unwrap_or("None"), serde_json::to_string(&generated)?, truncate(&diagnostics, 24000));
                    if !allowed.is_empty() {
                        let (name, prompt, _) = intent.as_ref().unwrap();
                        system =
                            profiles::selection_prompt(name, prompt, &allowed, current_profile)?;
                        system.push_str(guidance);
                    }
                    presentation::phase("Corrigindo erros de compilação…");
                    generated = generation.generate(&system, &repair)?;
                }
            }
        }
        unreachable!()
    })();
    let revision = match result {
        Ok(revision) => revision,
        Err(error) => {
            let preserved = workspace.keep();
            return Err(error).with_context(|| {
                format!(
                    "Generation failed; previous cache preserved. Diagnostic workspace: {}",
                    preserved.display()
                )
            });
        }
    };
    println!("Validated revision: {}", revision.display());
    drop(_cache_lock);
    if let Some(prepared) = reader_prepared {
        if prepared(&revision)? {
            return Ok(());
        }
    }
    launch(
        &spec,
        &revision,
        &ctx,
        &settings,
        cli,
        &runtime_args,
        &mut ready,
    )
}

fn adopt_legacy(
    old: &cache::Entry,
    history: &history::History,
    document: &Path,
    mut metadata: history::Metadata,
) -> Result<Option<history::Baseline>> {
    if !old.root.is_dir() {
        return Ok(None);
    }
    let _lock = old.lock()?;
    let Some(revision) = old.current() else {
        return Ok(None);
    };
    // Never mutate an old immutable revision. Only adopt a verified source inventory.
    let work = tempfile::tempdir()?;
    cache::copy_tree(
        &revision,
        &revision,
        work.path(),
        &mut BTreeMap::new(),
        &mut 0,
    )?;
    if let Err(error) = metadata.write(work.path()) {
        println!("Cache antigo sem baseline verificável: {error:#}");
        return Ok(None);
    }
    let adoption = entry(
        old.root.parent().context("Invalid cache root")?,
        document,
        &metadata.request,
        "legacy-adoption",
    );
    let _guard = adoption.lock()?;
    let adopted = adoption.publish(work.path())?;
    history.publish(&adopted)?;
    history.load()
}

fn entry(root: &Path, document: &Path, request: &str, parent: &str) -> cache::Entry {
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(format!(
            "lineage-v1:{}:{request}:{parent}",
            document.display()
        ))
    );
    cache::Entry {
        root: root.join(&fingerprint),
        fingerprint,
    }
}

fn validate_commands(spec: &Value, target: &str, ctx: &BTreeMap<String, String>) -> Result<()> {
    let mut commands = get_build_steps(spec, target)?;
    commands.push(get_cmd(spec, &["targets", target, "run", "cmd"])?);
    for operation in ["test", "publish"] {
        if let Some(steps) =
            get_path(spec, &["targets", target, operation, "steps"]).and_then(Value::as_sequence)
        {
            for step in steps {
                commands.push(value_to_cmd(get_path(step, &["cmd"]).unwrap_or(step))?);
            }
        }
    }
    for command in commands {
        for arg in command {
            check_template(&render_template(&arg, spec, ctx))?;
        }
    }
    Ok(())
}

fn preflight(
    spec: &Value,
    target: &str,
    ctx: &BTreeMap<String, String>,
    workspace: &Path,
    settings: &config::Settings,
) -> Result<()> {
    validate_commands(spec, target, ctx)?;
    for cmd in get_build_steps(spec, target)? {
        let command = render_template(&cmd[0], spec, ctx);
        executor::resolve_tool(&command)?;
        settings.execution.authorize(&command)?;
        check_allowlist(spec, &[command], std::env::consts::OS, "build", workspace)?;
    }
    profiles::preflight(spec, workspace, settings)
}

fn build(
    spec: &Value,
    target: &str,
    ctx: &BTreeMap<String, String>,
    path: &Path,
    settings: &config::Settings,
    graphics: bool,
) -> Result<()> {
    for cmd in get_build_steps(spec, target)? {
        presentation::phase(if cmd.get(1).is_some_and(|s| s == "restore") {
            "Restaurando bibliotecas do projeto…"
        } else {
            "Compilando seu programa…"
        });
        run_command(
            spec,
            &cmd,
            path,
            "build",
            std::env::consts::OS,
            ctx,
            settings,
            None,
        )?;
    }
    if let Some(steps) =
        get_path(spec, &["targets", target, "test", "steps"]).and_then(Value::as_sequence)
    {
        presentation::phase("Verificando seu programa…");
        for step in steps {
            let cmd = value_to_cmd(get_path(step, &["cmd"]).unwrap_or(step))?;
            run_command(
                spec,
                &cmd,
                path,
                "test",
                std::env::consts::OS,
                ctx,
                settings,
                None,
            )?;
        }
    }
    let run = get_cmd(spec, &["targets", target, "run", "cmd"])?;
    resolve_run_command(
        run.iter().map(|s| render_template(s, spec, ctx)).collect(),
        path,
    )?;
    if graphics {
        check_graphics(spec, target, ctx, path, settings)?;
    }
    Ok(())
}

fn check_graphics(
    spec: &Value,
    target: &str,
    ctx: &BTreeMap<String, String>,
    path: &Path,
    settings: &config::Settings,
) -> Result<()> {
    if get_path(spec, &["engine_profile"])
        .and_then(Value::as_str)
        .is_some_and(|profile| profile.ends_with("-opengl"))
    {
        presentation::phase("Verificando suporte OpenGL e renderização…");
        let mut command = get_cmd(spec, &["targets", target, "run", "cmd"])?;
        command.push("--crexe-graphics-check".into());
        run_command(spec, &command, path, "test", std::env::consts::OS, ctx, settings, None)
            .context("A verificação gráfica OpenGL falhou. Confira o contexto, os shaders, a sessão gráfica e o driver deste host")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn launch(
    spec: &Value,
    revision: &Path,
    ctx: &BTreeMap<String, String>,
    settings: &config::Settings,
    cli: &Cli,
    runtime_args: &[std::ffi::OsString],
    ready: &mut dyn FnMut(),
) -> Result<()> {
    package::export(revision, cli.export.as_deref())?;
    if cli.no_run {
        ready();
        return Ok(());
    }
    let project = project::Project::load(revision)?;
    run_command_args(
        spec,
        &project.commands["run"][0],
        revision,
        "run",
        std::env::consts::OS,
        ctx,
        settings,
        runtime_args,
        Some(ready),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_adoption_verifies_sources_without_modifying_old_revision() {
        let temp = tempfile::tempdir().unwrap();
        let document = temp.path().join("app.crexe");
        fs::write(&document, "request").unwrap();
        let root = temp.path().join("cache");
        let old = cache::Entry {
            root: root.join("old"),
            fingerprint: "old".into(),
        };
        let workspace = temp.path().join("work");
        fs::create_dir(&workspace).unwrap();
        fs::write(workspace.join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(workspace.join("app.exe"), [0, 255, 0]).unwrap();
        fs::write(
            workspace.join(project::MANIFEST),
            serde_json::to_vec(&json!({
                "schema": 1, "name": "legacy", "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH, "profile": "legacy", "sources": ["main.rs"],
                "commands": {"build": [["rustc", "main.rs"]], "run": [["app.exe"]]}
            }))
            .unwrap(),
        )
        .unwrap();
        let guard = old.lock().unwrap();
        let previous = old.publish(&workspace).unwrap();
        drop(guard);
        let original_pointer = fs::read(old.root.join("current.json")).unwrap();
        let history = history::History::new(&root, &document).unwrap();
        let metadata = || {
            history::Metadata::new(
                "request".into(),
                None,
                "prompt".into(),
                Value::Null,
                json!({}),
            )
        };
        let base = adopt_legacy(&old, &history, &document, metadata())
            .unwrap()
            .unwrap();
        assert_ne!(base.revision, previous);
        assert!(base.cached());
        assert_eq!(base.files().unwrap().len(), 1);
        assert_eq!(base.files().unwrap()[0].path, "main.rs");
        assert_eq!(base.metadata.prompt, "prompt");
        assert!(!previous.join(history::METADATA).exists());
        assert_eq!(
            original_pointer,
            fs::read(old.root.join("current.json")).unwrap()
        );
        let copy = temp.path().join("copy.crexe");
        fs::copy(&document, &copy).unwrap();
        assert!(history::History::new(&root, &copy)
            .unwrap()
            .load()
            .unwrap()
            .is_none());
        // Compiled-artifact damage loses the cache, not the approved source context.
        fs::write(base.revision.join("app.exe"), "damaged").unwrap();
        assert!(history.load().unwrap().is_some());
        assert!(!base.cached());
        fs::write(base.revision.join("main.rs"), "damaged").unwrap();
        assert!(history.load().is_err());
        // An unverified legacy cache is never adopted.
        fs::write(previous.join("main.rs"), "damaged").unwrap();
        assert!(adopt_legacy(&old, &history, &document, metadata())
            .unwrap()
            .is_none());
    }
}
