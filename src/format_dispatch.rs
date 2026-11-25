//! Explicit recipe/package/bundle dispatch. No archive fallback to model prompts.
use crate::*;
use std::io::Read;

const BUNDLE_MARKER: &str = "META-INF/crexe-bundle.json";

pub(crate) fn is_crexe_format(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            ["crexe", "crexp", "crexb"]
                .iter()
                .any(|item| ext.eq_ignore_ascii_case(item))
        })
}

fn archive_required(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("crexp") || ext.eq_ignore_ascii_case("crexb"))
}

fn is_zip(path: &Path) -> Result<bool> {
    paths::reject_links(path)?;
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        bail!("Expected a regular recipe or archive file");
    }
    let mut magic = Vec::with_capacity(4);
    file.take(4).read_to_end(&mut magic)?;
    Ok(secure_archive::has_zip_magic(&magic))
}

pub(crate) fn require_text(path: &Path) -> Result<()> {
    if is_zip(path)? || archive_required(path) {
        bail!("This is a package/bundle; use crexe inspect, or extract its recipe before editing");
    }
    Ok(())
}

enum Input {
    Text,
    Package(secure_archive::Archive),
    Bundle,
}

fn detect(path: &Path) -> Result<Input> {
    if !is_zip(path)? {
        if archive_required(path) {
            bail!("A .crexp or .crexb must contain a valid ZIP archive");
        }
        return Ok(Input::Text);
    }
    let archive = secure_archive::read(path, &secure_archive::Limits::bundle())?;
    if archive.files.contains_key(BUNDLE_MARKER) {
        // A malformed bundle must never fall back to package or text generation.
        return Ok(Input::Bundle);
    }
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("crexb"))
    {
        bail!("A .crexb must contain a CREXB bundle manifest; it never falls back to generation. Use .crexp for a specification package");
    }
    let archive = secure_archive::from_bytes(archive.raw_bytes, &secure_archive::Limits::recipe())?;
    Ok(Input::Package(archive))
}

pub(crate) fn exec(
    file: &Path,
    overrides: BTreeMap<String, String>,
    rebuild: bool,
    fresh: bool,
    cache_dir: Option<&Path>,
    cli: &Cli,
) -> Result<()> {
    match detect(file)? {
        Input::Text => orchestration::exec(file, overrides, rebuild, fresh, cache_dir, cli),
        Input::Package(archive) => {
            let package = crexp::load(&archive)?;
            let original = crexb::OriginalRecipe::new("crexp-package", archive.raw_bytes)?;
            let recipe = crexp::materialize(&package)?;
            println!("CREXP: {}; validated specification documents will be sent to the configured generator", package.name);
            orchestration::exec_with_original(
                &recipe, overrides, rebuild, fresh, cache_dir, cli, &original, file,
            )
        }
        Input::Bundle => {
            if rebuild || fresh || !overrides.is_empty() || cache_dir.is_some() {
                bail!("Bundle opening never regenerates. Extract the recipe and explicitly execute it to generate a new build; --cache-dir is not supported for bundle opening");
            }
            run_bundle(file, cli)
        }
    }
}

fn run_bundle(file: &Path, cli: &Cli) -> Result<()> {
    if cli.profile.is_some()
        || cli.provider.is_some()
        || cli.model.is_some()
        || cli.env_file.is_some()
        || cli.config.is_some()
        || cli.max_repairs.is_some()
        || cli.export.is_some()
    {
        bail!("Generation/configuration/export flags do not apply to bundle opening; extract the recipe to regenerate explicitly");
    }
    if cli.no_run {
        crexb::verify(file)?;
        println!("Bundle verified; --no-run does not hydrate or execute it");
        Ok(())
    } else {
        crexb::run(file, cli)
    }
}

pub(crate) fn inspect(file: &Path) -> Result<bool> {
    let info = match detect(file)? {
        Input::Text => return Ok(false),
        Input::Package(archive) => {
            let package = crexp::load(&archive)?;
            json!({"format":"crexp", "name":package.name,
                "content_sha256":package.content_sha256, "recipe_sha256":package.recipe_sha256,
                "inventory":package.inventory, "execution":"not performed", "generation":"not performed"})
        }
        Input::Bundle => crexb::inspect(file)?,
    };
    println!("{}", serde_json::to_string_pretty(&info)?);
    Ok(true)
}

pub(crate) fn bundle(command: &BundleCommands, cli: &Cli) -> Result<()> {
    match command {
        BundleCommands::Create {
            recipe,
            output,
            cache_dir,
            ..
        } => {
            let document = match detect(recipe)? {
                Input::Text => recipe.canonicalize()?,
                Input::Package(archive) => {
                    crexp::load(&archive)?;
                    recipe.canonicalize()?
                }
                Input::Bundle => bail!("Extract the recipe before creating a new bundle"),
            };
            let root = match cache_dir {
                Some(path) => std::path::absolute(path)?,
                None => default_cache_root()?,
            };
            paths::reject_links(&root)?;
            let accepted = history::History::new(&root, &document)?.load()?
                .context("No accepted generation found for this recipe and cache. Generate and validate it first")?;
            crexb::create(recipe, &accepted, output)
        }
        BundleCommands::Verify { file } => {
            crexb::verify(file)?;
            println!("Bundle integrity verified (hashes do not authenticate its publisher)");
            Ok(())
        }
        BundleCommands::Inspect { file } => {
            println!("{}", serde_json::to_string_pretty(&crexb::inspect(file)?)?);
            Ok(())
        }
        BundleCommands::Run { file } => run_bundle(file, cli),
        BundleCommands::ExtractRecipe { file, output } => crexb::extract_recipe(file, output),
    }
}
