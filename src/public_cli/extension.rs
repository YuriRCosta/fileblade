use super::*;
use crate::extension_template::{self, Request};
use std::path::Path;

#[derive(Clone, Debug, Subcommand)]
pub enum ExtensionCommand {
    /// Write a starter FileBlade extension (an Omarchy plugin with one blade module) into a new directory.
    Template(ExtensionTemplateArgs),
}

#[derive(Clone, Debug, Args)]
pub struct ExtensionTemplateArgs {
    /// Omarchy plugin id as publisher.name, for example acme.fileblade-weather.
    pub id: String,
    /// Directory to create; defaults to ./<id>.
    pub directory: Option<String>,
    /// Extension name for tabs and the README; defaults to the module id in title case.
    #[arg(long)]
    pub name: Option<String>,
    /// Blade module id; defaults to the plugin name without its fileblade- prefix.
    #[arg(long)]
    pub module: Option<String>,
    /// Manifest author and LICENSE holder; defaults to $USER.
    #[arg(long)]
    pub author: Option<String>,
    /// One line for the manifest and the module picker.
    #[arg(long)]
    pub description: Option<String>,
    /// Git URL used by the README install command; defaults to https://github.com/<publisher>/<name>.git.
    #[arg(long)]
    pub repository: Option<String>,
    /// Write into a directory that already has files, replacing only the template's own files.
    #[arg(long)]
    pub force: bool,
}

pub(super) fn extension(action: ExtensionCommand) -> AppResult<PublicResult> {
    match action {
        ExtensionCommand::Template(options) => template(options),
    }
}

fn template(options: ExtensionTemplateArgs) -> AppResult<PublicResult> {
    let request = Request {
        id: options.id,
        name: options.name,
        module: options.module,
        author: options.author,
        description: options.description,
        repository: options.repository,
    };
    let scaffold = extension_template::scaffold(&request)?;
    let directory = options.directory.unwrap_or_else(|| scaffold.id.clone());
    let directory = absolute_directory(&directory)?;
    let files = extension_template::render(&scaffold)?;
    let written = extension_template::write(Path::new(&directory), &files, options.force)?;
    let next = extension_template::next_steps(&scaffold, &directory);
    let mut lines = vec![format!(
        "Created {} ({}) in {directory}",
        scaffold.id, scaffold.name
    )];
    lines.extend(written.iter().map(|path| format!("  {path}")));
    lines.push(String::new());
    lines.push("Next:".to_string());
    lines.extend(next.iter().map(|step| format!("  {step}")));
    let document = json!({
        "ok": true,
        "id": scaffold.id,
        "name": scaffold.name,
        "module": format!("{}/{}", scaffold.id, scaffold.module),
        "directory": directory,
        "files": written,
        "next": next,
    });
    Ok(PublicResult::lines(lines, document))
}

fn absolute_directory(value: &str) -> AppResult<String> {
    let path = PathBuf::from(value);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    absolute
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::invalid("the directory must be valid UTF-8"))
}
