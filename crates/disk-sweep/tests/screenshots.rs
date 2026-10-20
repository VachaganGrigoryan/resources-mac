//! Renders every page offscreen with sample data for the README and the website.
//! Run: `cargo test -p disk-sweep --test screenshots -- --ignored`; images land in
//! `docs/screenshots/disk-sweep/` (committed).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use disk_sweep::app::App;
use disk_sweep::classify::DETECTORS;
use disk_sweep::docker::{VmDisk, parse_df};
use disk_sweep::pages::settings::Settings;
use disk_sweep::scan::{Kind, Node};
use disk_sweep::suggest::suggestions;
use disk_sweep::worker::{RootScan, State};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

const ROOT: &str = "/Users/you/WS";
const GB: u64 = 1_000_000_000;
const MB: u64 = 1_000_000;

/// A file or folder that is all one kind; caches get the detector their name matches.
fn leaf(path: &str, kind: Kind, bytes: u64) -> Node {
    let name: String = path.rsplit('/').next().unwrap().into();
    let mut by_kind = [0; 5];
    by_kind[kind.index()] = bytes;
    Node {
        detector: DETECTORS.iter().find(|d| d.names.contains(&name.as_str())),
        name,
        path: PathBuf::from(path),
        kind,
        is_dir: true,
        bytes,
        files: bytes / 40_000 + 1,
        by_kind,
        children: Vec::new(),
    }
}

/// A project folder: the sum of its children, largest first.
fn dir(path: &str, mut children: Vec<Node>) -> Node {
    children.sort_by_key(|c| std::cmp::Reverse(c.bytes));
    let mut by_kind = [0; 5];
    for c in &children {
        for (s, b) in by_kind.iter_mut().zip(c.by_kind) {
            *s += b;
        }
    }
    Node {
        name: path.rsplit('/').next().unwrap().into(),
        path: PathBuf::from(path),
        kind: Kind::Source,
        is_dir: true,
        bytes: by_kind.iter().sum(),
        files: children.iter().map(|c| c.files).sum(),
        by_kind,
        detector: None,
        children,
    }
}

fn workspace() -> Node {
    let p = |rel: &str| format!("{ROOT}/{rel}");
    dir(
        ROOT,
        vec![
            dir(
                &p("web-app"),
                vec![
                    leaf(&p("web-app/src"), Kind::Source, 420 * MB),
                    leaf(&p("web-app/node_modules"), Kind::Cache, 1_840 * MB),
                    leaf(&p("web-app/.next"), Kind::Cache, 640 * MB),
                    leaf(&p("web-app/.idea"), Kind::Editor, 2 * MB),
                    leaf(&p("web-app/.git"), Kind::Git, 310 * MB),
                ],
            ),
            dir(
                &p("api"),
                vec![
                    leaf(&p("api/src"), Kind::Source, 60 * MB),
                    leaf(&p("api/target"), Kind::Cache, 7_400 * MB),
                    leaf(&p("api/.git"), Kind::Git, 120 * MB),
                ],
            ),
            dir(
                &p("ml-notebooks"),
                vec![
                    leaf(&p("ml-notebooks/notebooks"), Kind::Source, 180 * MB),
                    leaf(&p("ml-notebooks/.venv"), Kind::Cache, 3_100 * MB),
                    leaf(&p("ml-notebooks/__pycache__"), Kind::Cache, 40 * MB),
                    leaf(&p("ml-notebooks/data"), Kind::Ignored, 900 * MB),
                    leaf(&p("ml-notebooks/.vscode"), Kind::Editor, MB),
                    leaf(&p("ml-notebooks/.git"), Kind::Git, 80 * MB),
                ],
            ),
        ],
    )
}

fn sample_state() -> State {
    let tree = workspace();
    let mut scan = RootScan::new(PathBuf::from(ROOT));
    scan.scanning = false;
    scan.generation = 1;
    scan.suggestions = suggestions(&tree);
    scan.result = Some(Ok(tree));
    let docker = parse_df(
        r#"{"Images":[
            {"Containers":"1","ID":"sha256:a1","Repository":"postgres","Tag":"16","Size":"438MB","UniqueSize":"0B","CreatedSince":"3 weeks ago"},
            {"Containers":"0","ID":"sha256:b2","Repository":"node","Tag":"20","Size":"1.1GB","UniqueSize":"1.1GB","CreatedSince":"2 months ago"},
            {"Containers":"0","ID":"sha256:c3","Repository":"<none>","Tag":"<none>","Size":"2.3GB","UniqueSize":"2.3GB","CreatedSince":"4 months ago"},
            {"Containers":"0","ID":"sha256:d4","Repository":"python","Tag":"3.12","Size":"1GB","UniqueSize":"1GB","CreatedSince":"5 months ago"}],
          "Containers":[
            {"Names":"db","Image":"postgres:16","Size":"63MB","State":"running","Status":"Up 3 hours"},
            {"Names":"old-build","Image":"node:20","Size":"210MB","State":"exited","Status":"Exited (0) 9 days ago"}],
          "Volumes":[
            {"Name":"pgdata","Links":"1","Size":"1.4GB"},
            {"Name":"orphan-cache","Links":"0","Size":"640MB"}],
          "BuildCache":[
            {"ID":"x1","Size":"3.2GB","InUse":"false","Shared":"false","LastUsedSince":"6 weeks ago"},
            {"ID":"x2","Size":"420MB","InUse":"false","Shared":"true","LastUsedSince":"2 days ago"}]}"#,
        concat!(
            r#"{"Active":"1","Reclaimable":"4.4GB (91%)","Size":"4.8GB","TotalCount":"4","Type":"Images"}"#,
            "\n",
            r#"{"Active":"1","Reclaimable":"210MB (76%)","Size":"273MB","TotalCount":"2","Type":"Containers"}"#,
            "\n",
            r#"{"Active":"1","Reclaimable":"640MB (31%)","Size":"2GB","TotalCount":"2","Type":"Local Volumes"}"#,
            "\n",
            r#"{"Active":"0","Reclaimable":"3.2GB","Size":"3.6GB","TotalCount":"2","Type":"Build Cache"}"#,
        ),
    )
    .expect("sample docker output parses");
    State {
        roots: vec![scan],
        docker: Some(Ok(docker)),
        docker_vm: Some(VmDisk {
            path: PathBuf::from(
                "~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw",
            ),
            allocated: 18 * GB,
            apparent: 64 * GB,
        }),
        docker_busy: false,
    }
}

fn shoot(theme: egui::Theme, name: &str) {
    let settings = Settings {
        roots: vec![PathBuf::from(ROOT)],
        theme: theme.into(),
    };
    let app = App::new(Arc::new(Mutex::new(sample_state()))).with_settings(settings);
    let mut h = Harness::builder()
        .with_size([1000.0, 680.0])
        .with_pixels_per_point(2.0)
        .with_theme(theme)
        .with_render_cursor(false)
        .with_context_setup(move |ctx| {
            resources_ui::theme::apply(ctx);
            ctx.set_theme(theme);
        })
        .wgpu()
        .build_ui_state(|ui, app: &mut App| app.show(ui), app);
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/disk-sweep");
    std::fs::create_dir_all(&dir).unwrap();
    for (label, slug) in [
        ("Overview", "overview"),
        ("WS", "folder"),
        ("Docker", "docker"),
        ("Settings", "settings"),
    ] {
        h.query_all_by_label(label).next().expect(label).click();
        h.run_steps(8);
        let img = h.render().expect("render");
        img.save(dir.join(format!("{name}-{slug}.png"))).unwrap();
    }
}

#[test]
#[ignore = "rewrites docs/screenshots/disk-sweep"]
fn screenshots_light() {
    shoot(egui::Theme::Light, "light");
}

#[test]
#[ignore = "rewrites docs/screenshots/disk-sweep"]
fn screenshots_dark() {
    shoot(egui::Theme::Dark, "dark");
}
