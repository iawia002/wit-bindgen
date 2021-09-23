use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use witx_bindgen_gen_core::Generator;

test_helpers::runtime_tests!("ts");

fn execute(name: &str, wasm: &Path, ts: &Path, imports: &Path, exports: &Path) {
    let mut dir = PathBuf::from(env!("OUT_DIR"));
    dir.push(name);
    drop(fs::remove_dir_all(&dir));
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&dir.join("imports")).unwrap();
    fs::create_dir_all(&dir.join("exports")).unwrap();

    println!("OUT_DIR = {:?}", dir);
    println!("Generating bindings...");
    let imports = witx_bindgen_gen_core::witx2::Interface::parse_file(imports).unwrap();
    let exports = witx_bindgen_gen_core::witx2::Interface::parse_file(exports).unwrap();
    // TODO: should combine these calls into one
    let mut import_files = Default::default();
    let mut export_files = Default::default();
    witx_bindgen_gen_js::Opts::default()
        .build()
        .generate_all(&[imports], &[], &mut import_files);
    witx_bindgen_gen_js::Opts::default()
        .build()
        .generate_all(&[], &[exports], &mut export_files);
    for (file, contents) in import_files.iter() {
        fs::write(dir.join("imports").join(file), contents).unwrap();
    }
    for (file, contents) in export_files.iter() {
        fs::write(dir.join("exports").join(file), contents).unwrap();
    }

    let (cmd, args) = if cfg!(windows) {
        ("cmd.exe", &["/c", "npx.cmd"] as &[&str])
    } else {
        ("npx", &[] as &[&str])
    };

    fs::copy(ts, dir.join("host.ts")).unwrap();
    fs::copy("tests/helpers.d.ts", dir.join("helpers.d.ts")).unwrap();
    fs::copy("tests/helpers.js", dir.join("helpers.js")).unwrap();
    let config = dir.join("tsconfig.json");
    fs::write(
        &config,
        format!(
            r#"
                {{
                    "files": ["host.ts"],
                    "compilerOptions": {{
                        "module": "esnext",
                        "target": "es2020",
                        "strict": true,
                        "strictNullChecks": true,
                        "baseUrl": {0:?},
                        "outDir": {0:?}
                    }}
                }}
            "#,
            dir,
        ),
    )
    .unwrap();

    run(Command::new(cmd)
        .args(args)
        .arg("tsc")
        .arg("--project")
        .arg(&config));

    // Currently there's mysterious uvwasi errors creating a `WASI` on Windows.
    // Unsure what's happening so let's ignore these tests for now since there's
    // not much Windows-specific here anyway.
    if cfg!(windows) {
        return;
    }

    fs::write(dir.join("package.json"), "{\"type\":\"module\"}").unwrap();
    let mut path = Vec::new();
    path.push(env::current_dir().unwrap());
    path.push(dir.clone());
    println!("{:?}", std::env::join_paths(&path));
    run(Command::new("node")
        .arg("--experimental-wasi-unstable-preview1")
        .arg(dir.join("host.js"))
        .env("NODE_PATH", std::env::join_paths(&path).unwrap())
        .arg(wasm));
}

fn run(cmd: &mut Command) {
    println!("running {:?}", cmd);
    let output = cmd.output().expect("failed to executed");
    println!("status: {}", output.status);
    println!(
        "stdout:\n  {}",
        String::from_utf8_lossy(&output.stdout).replace("\n", "\n  ")
    );
    println!(
        "stderr:\n  {}",
        String::from_utf8_lossy(&output.stderr).replace("\n", "\n  ")
    );
    assert!(output.status.success());
}