use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub fn install(root: &Path, id: &str) {
    let plugins = root.join(".config/omarchy/plugins");
    fs::create_dir_all(&plugins).unwrap();
    std::os::unix::fs::symlink(root, plugins.join(id)).unwrap();
    fs::write(
        root.join("enabled.json"),
        serde_json::json!([{"id":id,"enabled":true}]).to_string(),
    )
    .unwrap();
    let program = root.join("bin/omarchy");
    fs::write(&program, "#!/usr/bin/python3\nimport os,sys\nfrom pathlib import Path\nassert sys.argv[1:]==['plugin','list','--json']\nprint((Path(os.environ['HOME'])/'enabled.json').read_text())\n").unwrap();
    fs::set_permissions(program, fs::Permissions::from_mode(0o700)).unwrap();
}

pub fn configure(command: &mut Command, root: &Path) {
    command.env("HOME", root).env(
        "PATH",
        format!("{}:/usr/bin:/bin", root.join("bin").display()),
    );
}
