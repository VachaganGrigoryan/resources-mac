//! Docker disk usage from the `docker` CLI, and the prune commands that free it.

use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: u64,
    /// Bytes no other image shares: what removing it frees.
    pub unique_size: u64,
    pub containers: u32,
    pub created_since: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub name: String,
    pub image: String,
    pub size: u64,
    /// `running`, `exited`, `created`, `paused`…
    pub state: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub name: String,
    pub size: u64,
    /// Containers using it, running or not.
    pub links: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildCache {
    pub id: String,
    pub size: u64,
    pub in_use: bool,
    pub shared: bool,
    pub last_used: String,
}

/// One row of plain `docker system df`: Docker's own totals per category.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    /// "Images", "Containers", "Local Volumes", "Build Cache".
    pub kind: String,
    pub count: u32,
    pub active: u32,
    pub size: u64,
    pub reclaimable: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub images: Vec<Image>,
    pub containers: Vec<Container>,
    pub volumes: Vec<Volume>,
    pub build_cache: Vec<BuildCache>,
    pub summary: Vec<Category>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prune {
    DanglingImages,
    UnusedImages,
    StoppedContainers,
    UnusedVolumes,
    BuildCache,
}

impl Prune {
    pub const ALL: [Prune; 5] = [
        Prune::DanglingImages,
        Prune::UnusedImages,
        Prune::StoppedContainers,
        Prune::UnusedVolumes,
        Prune::BuildCache,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Prune::DanglingImages => "Dangling images",
            Prune::UnusedImages => "Unused images",
            Prune::StoppedContainers => "Stopped containers",
            Prune::UnusedVolumes => "Unused volumes",
            Prune::BuildCache => "Build cache",
        }
    }
}

/// What `docker container prune` removes.
pub fn stopped(c: &Container) -> bool {
    !matches!(c.state.as_str(), "running" | "paused" | "restarting")
}

fn unused_image(i: &Image) -> bool {
    i.containers == 0
}

fn dangling(i: &Image) -> bool {
    unused_image(i) && i.tag == "<none>"
}

fn tally<T>(items: impl Iterator<Item = T>, size: impl Fn(&T) -> u64) -> (usize, u64) {
    items.fold((0, 0), |(n, b), t| (n + 1, b + size(&t)))
}

impl Usage {
    pub fn images_for(&self, p: Prune) -> impl Iterator<Item = &Image> {
        self.images.iter().filter(move |i| match p {
            Prune::DanglingImages => dangling(i),
            Prune::UnusedImages => unused_image(i),
            _ => false,
        })
    }

    pub fn stopped_containers(&self) -> impl Iterator<Item = &Container> {
        self.containers.iter().filter(|c| stopped(c))
    }

    pub fn unused_volumes(&self) -> impl Iterator<Item = &Volume> {
        self.volumes.iter().filter(|v| v.links == 0)
    }

    pub fn idle_build_cache(&self) -> impl Iterator<Item = &BuildCache> {
        self.build_cache.iter().filter(|b| !b.in_use)
    }

    /// How many items `p` would remove and the bytes that frees.
    pub fn reclaimable(&self, p: Prune) -> (usize, u64) {
        match p {
            Prune::DanglingImages | Prune::UnusedImages => {
                tally(self.images_for(p), |i| i.unique_size)
            }
            Prune::StoppedContainers => tally(self.stopped_containers(), |c| c.size),
            Prune::UnusedVolumes => tally(self.unused_volumes(), |v| v.size),
            Prune::BuildCache => tally(self.idle_build_cache(), |b| b.size),
        }
    }

    /// Arguments after `docker` that remove exactly the items listed for `p`. Removals are by
    /// name, not `prune`: Docker refuses in-use items, so a list gone stale can't widen what
    /// is deleted. Build cache has no names to give and is always re-creatable.
    pub fn command(&self, p: Prune) -> Vec<String> {
        let (verb, names): ([&str; 2], Vec<String>) = match p {
            Prune::DanglingImages | Prune::UnusedImages => (
                ["image", "rm"],
                self.images_for(p).map(|i| i.id.clone()).collect(),
            ),
            Prune::StoppedContainers => (
                ["container", "rm"],
                self.stopped_containers().map(|c| c.name.clone()).collect(),
            ),
            Prune::UnusedVolumes => (
                ["volume", "rm"],
                self.unused_volumes().map(|v| v.name.clone()).collect(),
            ),
            Prune::BuildCache => {
                return ["builder", "prune", "-a", "-f"].map(String::from).to_vec();
            }
        };
        verb.iter().map(|v| (*v).to_owned()).chain(names).collect()
    }

    /// Everything Docker stores, by its own count (shared image layers once).
    pub fn total(&self) -> u64 {
        self.summary.iter().map(|c| c.size).sum()
    }
}

/// Docker's human sizes (decimal units): `"6.911GB"`, `"1.2kB"`, `"0B (virtual 161MB)"`.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.split_whitespace().next()?;
    let split = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = s.split_at(split);
    let mult = match unit {
        "B" => 1.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        "PB" => 1e15,
        _ => return None,
    };
    let n: f64 = num.parse().ok()?;
    Some((n * mult).round() as u64)
}

/// The CLI prints every field as a string.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Verbose {
    #[serde(default)]
    images: Vec<Value>,
    #[serde(default)]
    containers: Vec<Value>,
    #[serde(default)]
    volumes: Vec<Value>,
    #[serde(default)]
    build_cache: Vec<Value>,
}

fn text(v: &Value, key: &str) -> String {
    match &v[key] {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn size(v: &Value, key: &str) -> u64 {
    parse_size(&text(v, key)).unwrap_or(0)
}

fn count(v: &Value, key: &str) -> u32 {
    text(v, key).parse().unwrap_or(0)
}

fn bad_output(e: serde_json::Error) -> String {
    format!("unexpected docker output: {e}")
}

/// Parses `docker system df -v --format '{{json .}}'` and the JSON lines of plain
/// `docker system df --format '{{json .}}'`.
pub fn parse_df(verbose: &str, summary: &str) -> Result<Usage, String> {
    let v: Verbose = serde_json::from_str(verbose).map_err(bad_output)?;
    let summary = summary
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let row: Value = serde_json::from_str(line).map_err(bad_output)?;
            Ok(Category {
                kind: text(&row, "Type"),
                count: count(&row, "TotalCount"),
                active: count(&row, "Active"),
                size: size(&row, "Size"),
                reclaimable: size(&row, "Reclaimable"),
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Usage {
        images: v
            .images
            .iter()
            .map(|i| Image {
                id: text(i, "ID"),
                repository: text(i, "Repository"),
                tag: text(i, "Tag"),
                size: size(i, "Size"),
                unique_size: size(i, "UniqueSize"),
                containers: count(i, "Containers"),
                created_since: text(i, "CreatedSince"),
            })
            .collect(),
        containers: v
            .containers
            .iter()
            .map(|c| Container {
                name: text(c, "Names"),
                image: text(c, "Image"),
                size: size(c, "Size"),
                state: text(c, "State"),
                status: text(c, "Status"),
            })
            .collect(),
        volumes: v
            .volumes
            .iter()
            .map(|vol| Volume {
                name: text(vol, "Name"),
                size: size(vol, "Size"),
                links: count(vol, "Links"),
            })
            .collect(),
        build_cache: v
            .build_cache
            .iter()
            .map(|b| BuildCache {
                id: text(b, "ID"),
                size: size(b, "Size"),
                in_use: text(b, "InUse") == "true",
                shared: text(b, "Shared") == "true",
                last_used: text(b, "LastUsedSince"),
            })
            .collect(),
        summary,
    })
}

/// Runs `program args…`: stdout on success, the trimmed stderr otherwise.
pub fn run_docker(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("can't run {program}: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let err = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    Err(if err.is_empty() {
        format!("{program} exited with {}", out.status)
    } else {
        err
    })
}

/// Current usage; an error when the CLI is missing or the daemon isn't running.
pub fn df() -> Result<Usage, String> {
    let verbose = run_docker("docker", &["system", "df", "-v", "--format", "{{json .}}"])?;
    let summary = run_docker("docker", &["system", "df", "--format", "{{json .}}"])?;
    parse_df(&verbose, &summary)
}

/// Runs `docker args…`; `Ok` holds its last output line ("Total reclaimed space: 1.2GB").
pub fn remove(args: &[String]) -> Result<String, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run_docker("docker", &args)?;
    Ok(out.lines().last().unwrap_or_default().trim().to_owned())
}

/// Docker Desktop's VM disk: a sparse file, so its apparent size overstates what it uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VmDisk {
    pub path: PathBuf,
    /// Bytes it really takes on the Mac's disk.
    pub allocated: u64,
    /// Its maximum size.
    pub apparent: u64,
}

pub fn vm_disk() -> Option<VmDisk> {
    use std::os::unix::fs::MetadataExt;
    let path = std::env::home_dir()?
        .join("Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw");
    let md = std::fs::metadata(&path).ok()?;
    Some(VmDisk {
        allocated: md.blocks() * 512,
        apparent: md.len(),
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERBOSE: &str = r#"{"Images":[
      {"Containers":"1","CreatedSince":"13 days ago","ID":"sha256:aaa","Repository":"postgres","Tag":"17-alpine","Size":"161MB","SharedSize":"0B","UniqueSize":"161.3MB"},
      {"Containers":"0","CreatedSince":"2 months ago","ID":"sha256:bbb","Repository":"<none>","Tag":"<none>","Size":"1.2GB","SharedSize":"200MB","UniqueSize":"1GB"}],
     "Containers":[
      {"Names":"db","Image":"postgres:17-alpine","Size":"4.1kB","State":"running","Status":"Up 2 hours"},
      {"Names":"old","Image":"node:20","Size":"10MB (virtual 1.1GB)","State":"exited","Status":"Exited (0) 2 days ago"}],
     "Volumes":[
      {"Name":"pgdata","Links":"1","Size":"3GB"},
      {"Name":"orphan","Links":"0","Size":"72MB"}],
     "BuildCache":[
      {"ID":"abc123","CacheType":"regular","Size":"500MB","InUse":"false","Shared":"true","LastUsedSince":"3 weeks ago"}]}"#;

    const SUMMARY: &str = r#"{"Active":"29","Reclaimable":"6.911GB (42%)","Size":"16.26GB","TotalCount":"45","Type":"Images"}
{"Active":"0","Reclaimable":"0B","Size":"0B","TotalCount":"0","Type":"Build Cache"}"#;

    #[test]
    fn parses_docker_sizes() {
        assert_eq!(parse_size("6.911GB"), Some(6_911_000_000));
        assert_eq!(parse_size("10.94MB"), Some(10_940_000));
        assert_eq!(parse_size("1.2kB"), Some(1200));
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("0B (virtual 161MB)"), Some(0));
        assert_eq!(parse_size("6.911GB (42%)"), Some(6_911_000_000));
        assert_eq!(parse_size("garbage"), None);
        assert_eq!(parse_size("N/A"), None);
    }

    #[test]
    fn parses_system_df_verbose_json() {
        let u = parse_df(VERBOSE, SUMMARY).unwrap();
        assert_eq!(u.images.len(), 2);
        assert_eq!(u.images[1].tag, "<none>");
        assert_eq!(u.images[1].unique_size, 1_000_000_000);
        assert_eq!(u.containers.len(), 2);
        assert_eq!(u.volumes.len(), 2);
        assert_eq!(u.build_cache.len(), 1);
        assert!(!u.build_cache[0].in_use);
        assert_eq!(u.reclaimable(Prune::DanglingImages), (1, 1_000_000_000));
        assert_eq!(u.reclaimable(Prune::UnusedImages), (1, 1_000_000_000));
        assert_eq!(u.reclaimable(Prune::StoppedContainers), (1, 10_000_000));
        assert_eq!(u.reclaimable(Prune::UnusedVolumes), (1, 72_000_000));
        assert_eq!(u.reclaimable(Prune::BuildCache), (1, 500_000_000));
        assert_eq!(u.summary.len(), 2);
        assert_eq!(u.summary[0].kind, "Images");
        assert_eq!(u.summary[0].size, 16_260_000_000);
        assert_eq!(u.summary[0].reclaimable, 6_911_000_000);
        assert_eq!(u.total(), 16_260_000_000);
    }

    #[test]
    fn bad_json_is_an_error() {
        assert!(parse_df("not json", "").is_err());
    }

    #[test]
    fn missing_docker_binary_is_an_error() {
        let err = run_docker("definitely-not-docker-xyz", &["system", "df"]).unwrap_err();
        assert!(err.contains("definitely-not-docker-xyz"), "{err}");
    }

    #[test]
    fn failing_command_returns_stderr() {
        let err = run_docker("sh", &["-c", "echo boom >&2; exit 3"]).unwrap_err();
        assert_eq!(err, "boom");
    }

    #[test]
    fn removals_name_exactly_what_was_shown() {
        let u = parse_df(VERBOSE, SUMMARY).unwrap();
        // Named removals: Docker refuses in-use items, so nothing beyond the list can go.
        assert_eq!(u.command(Prune::UnusedVolumes), ["volume", "rm", "orphan"]);
        assert_eq!(
            u.command(Prune::StoppedContainers),
            ["container", "rm", "old"]
        );
        assert_eq!(
            u.command(Prune::UnusedImages),
            ["image", "rm", "sha256:bbb"]
        );
        assert_eq!(
            u.command(Prune::DanglingImages),
            ["image", "rm", "sha256:bbb"]
        );
        assert_eq!(
            u.command(Prune::BuildCache),
            ["builder", "prune", "-a", "-f"]
        );
    }

    /// Talks to the real daemon: `cargo test -p disk-sweep docker_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn docker_live() {
        let u = df().unwrap();
        println!(
            "images {} containers {} volumes {} cache {} total {} B; vm {:?}",
            u.images.len(),
            u.containers.len(),
            u.volumes.len(),
            u.build_cache.len(),
            u.total(),
            vm_disk()
        );
        for p in Prune::ALL {
            println!("{:?} {:?}", p, u.reclaimable(p));
        }
        assert!(!u.summary.is_empty());
    }
}
