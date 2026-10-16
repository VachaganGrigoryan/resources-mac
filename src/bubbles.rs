//! Pure geometry for the Overview page: bubble sizes, subprocesses packed inside an app's
//! bubble, a small force layout that keeps bubbles apart, who-launched-whom links between
//! apps, and the process tree of one app.

use std::collections::{HashMap, HashSet};

use egui::{Vec2, vec2};

use crate::sample::{AppUsage, ProcInfo};

pub const MIN_R: f32 = 22.0;
pub const MAX_R: f32 = 130.0;
/// Empty space kept between neighbouring bubbles.
pub const GAP: f32 = 22.0;
const GOLDEN_ANGLE: f32 = 2.399_963;
const COLLISION_PASSES: usize = 6;

/// Radius for `value` out of the largest `max`: area grows with the value, clamped so idle
/// apps stay clickable.
pub fn radius(value: f64, max: f64) -> f32 {
    if max <= 0.0 {
        return MIN_R;
    }
    (MAX_R * (value / max).sqrt() as f32).clamp(MIN_R, MAX_R)
}

/// Places one circle per weight inside a circle of radius `r` on a sunflower spiral, heaviest
/// in the middle. Returns `(offset from centre, radius)` in input order.
pub fn pack(weights: &[f64], r: f32) -> Vec<(Vec2, f32)> {
    let n = weights.len();
    if n == 1 {
        return vec![(Vec2::ZERO, r * 0.6)];
    }
    let max = weights.iter().copied().fold(0.0, f64::max);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| weights[b].total_cmp(&weights[a]));
    // Each child gets an equal slot; children never cross the parent's edge.
    let slot = 0.8 / (n as f32).sqrt();
    let inner = r / (1.0 + slot);
    let mut out = vec![(Vec2::ZERO, 0.0); n];
    for (k, &i) in order.iter().enumerate() {
        let dist = inner * ((k as f32 + 0.5) / n as f32).sqrt();
        let angle = k as f32 * GOLDEN_ANGLE;
        let scale = if max > 0.0 {
            ((weights[i] / max).sqrt() as f32).max(0.3)
        } else {
            0.3
        };
        out[i] = (Vec2::angled(angle) * dist, inner * slot * scale);
    }
    out
}

/// Bubble positions around a hub at the origin, kept between frames by id so bubbles stay
/// put while the apps underneath come and go.
#[derive(Default)]
pub struct Sim {
    pos: HashMap<u32, Vec2>,
    /// Strength of the inward pull. It cools towards 0 so the layout comes to rest (many
    /// bubbles pressed together would otherwise jitter forever); changes re-heat it.
    heat: f32,
    /// Radii of the last step, to notice changes worth re-heating for.
    radii: HashMap<u32, f32>,
}

/// Per-step cooling: from full heat to resting in about 300 steps.
const COOLING: f32 = 0.985;
/// A radius change smaller than this (memory drifting a little each second) doesn't re-heat.
const REHEAT_DR: f32 = 3.0;

impl Sim {
    pub fn pos(&self, id: u32) -> Option<Vec2> {
        self.pos.get(&id).copied()
    }

    /// One relaxation step for bubbles `(id, radius)`: a pull towards the hub of radius
    /// `hub` (or, for the child of a `(parent, child)` link, towards its parent), then pushing
    /// overlapping bubbles apart. New ids start on a golden-angle spiral
    /// outside the others; ids no longer present are forgotten.
    /// Returns how far the furthest bubble moved, so callers can stop once it settles.
    pub fn step(&mut self, nodes: &[(u32, f32)], links: &[(u32, u32)], hub: f32) -> f32 {
        let changed = nodes.len() != self.radii.len()
            || nodes.iter().any(|(id, r)| {
                self.radii
                    .get(id)
                    .is_none_or(|old| (old - r).abs() > REHEAT_DR)
            });
        if changed {
            self.heat = 1.0;
            self.radii = nodes.iter().copied().collect();
        }
        self.heat *= COOLING;
        let ids: HashSet<u32> = nodes.iter().map(|n| n.0).collect();
        self.pos.retain(|id, _| ids.contains(id));
        let far = self.pos.values().map(|p| p.length()).fold(hub, f32::max);
        let radius_of: HashMap<u32, f32> = nodes.iter().copied().collect();
        let launcher: HashMap<u32, u32> = links
            .iter()
            .filter(|(pa, _)| radius_of.contains_key(pa))
            .map(|&(pa, ch)| (ch, pa))
            .collect();
        // Launched bubbles start beside their launcher once it is placed; a launcher chain
        // deeper than the passes falls back to the spiral.
        for pass in 0..4 {
            for &(id, r) in nodes {
                if self.pos.contains_key(&id) {
                    continue;
                }
                let k = self.pos.len() as f32;
                let at = match launcher.get(&id) {
                    Some(pa) if pass < 3 => match self.pos.get(pa) {
                        Some(&pp) => {
                            let out = if pp.length() > 1e-3 {
                                pp.normalized()
                            } else {
                                Vec2::X
                            };
                            let dir = Vec2::angled(out.angle() + (k - 2.0) * 0.6);
                            pp + dir * (radius_of[pa] + r + GAP)
                        }
                        None => continue,
                    },
                    _ => Vec2::angled(k * GOLDEN_ANGLE) * (far + r + GAP),
                };
                self.pos.insert(id, at);
            }
        }

        let before: Vec<Vec2> = nodes.iter().map(|n| self.pos[&n.0]).collect();
        let mut p = before.clone();
        let index: HashMap<u32, usize> = nodes.iter().enumerate().map(|(i, n)| (n.0, i)).collect();
        let parent: HashMap<usize, usize> = links
            .iter()
            .filter_map(|(pa, ch)| Some((*index.get(ch)?, *index.get(pa)?)))
            .collect();
        for (i, &(_, r)) in nodes.iter().enumerate() {
            let (anchor, rest) = match parent.get(&i) {
                Some(&pa) => (p[pa], nodes[pa].1 + r + GAP),
                None => (Vec2::ZERO, hub + r + GAP),
            };
            let d = p[i] - anchor;
            let len = d.length();
            if len > rest {
                let pull = d / len * (len - rest) * 0.08 * self.heat;
                p[i] -= pull;
            }
        }
        // Several passes: one leaves overlaps that the inward pull keeps pressing together.
        for _ in 0..COLLISION_PASSES {
            for i in 0..nodes.len() {
                for j in i + 1..nodes.len() {
                    let d = p[j] - p[i];
                    let min = nodes[i].1 + nodes[j].1 + GAP;
                    let len = d.length();
                    if len < min {
                        let dir = if len > 1e-3 {
                            d / len
                        } else {
                            Vec2::angled(j as f32)
                        };
                        let push = dir * (min - len) / 2.0;
                        p[i] -= push;
                        p[j] += push;
                    }
                }
                let len = p[i].length();
                let min = hub + nodes[i].1 + GAP;
                if len < min {
                    let dir = if len > 1e-3 {
                        p[i] / len
                    } else {
                        Vec2::angled(i as f32)
                    };
                    p[i] = dir * min;
                }
            }
        }
        let mut moved = 0.0f32;
        for (i, &(id, _)) in nodes.iter().enumerate() {
            moved = moved.max((p[i] - before[i]).length());
            self.pos.insert(id, p[i]);
        }
        moved
    }
}

/// Who launched whom between apps: `(parent app pid, child app pid)` when the child app's
/// process was started by a process the parent app owns (e.g. Terminal → an app run from a
/// shell). Apps started by the system have no link; the Overview hangs them off the hub.
pub fn links(apps: &[AppUsage], procs: &[ProcInfo]) -> Vec<(u32, u32)> {
    let ppid: HashMap<u32, Option<u32>> = procs.iter().map(|p| (p.pid, p.ppid)).collect();
    // A process can sit in several apps' trees; the app with the smallest tree is the
    // closest owner, so it is inserted last and wins.
    let mut by_size: Vec<&AppUsage> = apps.iter().filter(|a| a.app.pid != 0).collect();
    by_size.sort_by_key(|a| std::cmp::Reverse(a.pids.len()));
    let mut owner = HashMap::new();
    for a in by_size {
        for &pid in &a.pids {
            owner.insert(pid, a.app.pid);
        }
    }
    apps.iter()
        .filter(|a| a.app.pid != 0)
        .filter_map(|a| {
            let parent = ppid.get(&a.app.pid).copied().flatten()?;
            let by = *owner.get(&parent)?;
            (by != a.app.pid).then_some((by, a.app.pid))
        })
        .collect()
}

/// A process's place in [`tree`]: `(pid, column, depth)`.
pub type Slot = (u32, f32, f32);

/// Top-down tree layout of `root` and its descendants: `(pid, column, depth)` with leaves in
/// consecutive columns and each parent centred over its children, plus `(parent, child)` edges.
pub fn tree(root: u32, procs: &[ProcInfo]) -> (Vec<Slot>, Vec<(u32, u32)>) {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in procs {
        if let Some(pp) = p.ppid {
            children.entry(pp).or_default().push(p.pid);
        }
    }
    for c in children.values_mut() {
        c.sort_unstable();
    }
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen = HashSet::new();
    let mut next_leaf = 0.0;
    place(
        root,
        0.0,
        &children,
        &mut seen,
        &mut next_leaf,
        &mut nodes,
        &mut edges,
    );
    (nodes, edges)
}

/// Lays out `pid`'s subtree; returns its column. `seen` makes a ppid cycle terminate.
fn place(
    pid: u32,
    depth: f32,
    children: &HashMap<u32, Vec<u32>>,
    seen: &mut HashSet<u32>,
    next_leaf: &mut f32,
    nodes: &mut Vec<Slot>,
    edges: &mut Vec<(u32, u32)>,
) -> f32 {
    seen.insert(pid);
    let mut cols = Vec::new();
    for &c in children.get(&pid).into_iter().flatten() {
        if seen.contains(&c) {
            continue;
        }
        edges.push((pid, c));
        cols.push(place(
            c,
            depth + 1.0,
            children,
            seen,
            next_leaf,
            nodes,
            edges,
        ));
    }
    let col = if cols.is_empty() {
        *next_leaf += 1.0;
        *next_leaf - 1.0
    } else {
        cols.iter().sum::<f32>() / cols.len() as f32
    };
    nodes.push((pid, col, depth));
    col
}

/// Converts `(column, depth)` slots to centred world positions `spacing` apart.
pub fn tree_positions(nodes: &[Slot], spacing: Vec2) -> HashMap<u32, Vec2> {
    let max_col = nodes.iter().map(|n| n.1).fold(0.0, f32::max);
    let max_depth = nodes.iter().map(|n| n.2).fold(0.0, f32::max);
    let centre = vec2(max_col, max_depth) / 2.0;
    nodes
        .iter()
        .map(|&(pid, c, d)| (pid, (vec2(c, d) - centre) * spacing))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::AppInfo;

    fn proc(pid: u32, ppid: Option<u32>) -> ProcInfo {
        ProcInfo {
            pid,
            ppid,
            name: format!("p{pid}"),
            user: "me".into(),
            cpu: 0.0,
            mem: 0,
            threads: None,
        }
    }

    fn app(pid: u32, pids: &[u32]) -> AppUsage {
        AppUsage {
            app: AppInfo {
                pid,
                name: format!("app{pid}"),
                bundle_id: None,
                icon: None,
            },
            cpu: 0.0,
            mem: 0,
            pids: pids.to_vec(),
        }
    }

    #[test]
    fn radius_is_clamped_and_grows_with_area() {
        assert_eq!(radius(0.0, 0.0), MIN_R);
        assert_eq!(radius(100.0, 100.0), MAX_R);
        assert_eq!(radius(0.0, 100.0), MIN_R);
        assert!((radius(25.0, 100.0) - MAX_R / 2.0).abs() < 0.01);
    }

    #[test]
    fn packed_children_stay_inside_parent() {
        for n in [1, 2, 5, 40, 400] {
            let weights: Vec<f64> = (0..n).map(|i| (i % 7) as f64).collect();
            for (off, r) in pack(&weights, 100.0) {
                assert!(off.length() + r <= 100.0 + 1e-3, "n={n}");
                assert!(r > 0.0);
            }
        }
    }

    #[test]
    fn heaviest_child_sits_in_the_middle() {
        let out = pack(&[1.0, 9.0, 4.0], 100.0);
        assert!(out[1].0.length() < out[0].0.length());
        assert!(out[1].1 > out[0].1);
    }

    #[test]
    fn sim_settles_without_overlaps() {
        let nodes: Vec<(u32, f32)> = (1..=30)
            .map(|i| (i, 20.0 + (i % 5) as f32 * 15.0))
            .collect();
        let mut sim = Sim::default();
        for _ in 0..600 {
            sim.step(&nodes, &[], 60.0);
        }
        assert!(sim.step(&nodes, &[], 60.0) < 0.5, "still moving");
        for (i, a) in nodes.iter().enumerate() {
            let pa = sim.pos(a.0).unwrap();
            assert!(pa.length() >= 60.0 + a.1 + GAP / 2.0, "inside hub");
            for b in &nodes[i + 1..] {
                let d = (sim.pos(b.0).unwrap() - pa).length();
                assert!(
                    d >= a.1 + b.1 + GAP / 2.0,
                    "{} overlaps {}: {d} < {}",
                    a.0,
                    b.0,
                    a.1 + b.1 + GAP
                );
            }
        }
    }

    #[test]
    fn sim_settles_for_a_busy_mac() {
        // ~45 apps with mixed sizes, as on a real desktop; the page stops repainting once
        // a step moves less than its threshold.
        let nodes: Vec<(u32, f32)> = (1..=45)
            .map(|i| (i, MIN_R + ((i * 37) % 100) as f32))
            .collect();
        let links = [(3, 40), (3, 41), (40, 42)];
        let mut sim = Sim::default();
        let mut frames = 0;
        while sim.step(&nodes, &links, 64.0) > 0.1 {
            frames += 1;
            assert!(frames < 4000, "never settles");
        }
    }

    #[test]
    fn sim_forgets_gone_ids_and_keeps_the_rest() {
        let mut sim = Sim::default();
        for _ in 0..200 {
            sim.step(&[(1, 30.0), (2, 30.0)], &[], 50.0);
        }
        let before = sim.pos(1).unwrap();
        sim.step(&[(1, 30.0)], &[], 50.0);
        assert!(sim.pos(2).is_none());
        assert!((sim.pos(1).unwrap() - before).length() < 5.0);
    }

    #[test]
    fn launched_app_settles_next_to_its_launcher() {
        let nodes: Vec<(u32, f32)> = (1..=12).map(|i| (i, 30.0)).collect();
        let mut sim = Sim::default();
        for _ in 0..600 {
            sim.step(&nodes, &[(1, 12)], 60.0);
        }
        let d = (sim.pos(1).unwrap() - sim.pos(12).unwrap()).length();
        assert!(d < 60.0 + GAP * 1.5, "launcher {d} away");
    }

    #[test]
    fn links_follow_the_launching_app() {
        // 10 = Terminal → 11 shell → 20 app launched from the shell; 30 launched by launchd.
        let procs = [
            proc(1, None),
            proc(10, Some(1)),
            proc(11, Some(10)),
            proc(20, Some(11)),
            proc(21, Some(20)),
            proc(30, Some(1)),
        ];
        let apps = [
            app(10, &[10, 11, 20, 21]),
            app(20, &[20, 21]),
            app(30, &[30]),
            app(0, &[1]),
        ];
        assert_eq!(links(&apps, &procs), [(10, 20)]);
    }

    #[test]
    fn links_prefer_the_closest_owner() {
        let procs = [proc(10, Some(1)), proc(20, Some(10)), proc(30, Some(20))];
        let apps = [app(10, &[10, 20, 30]), app(20, &[20, 30]), app(30, &[30])];
        let mut l = links(&apps, &procs);
        l.sort();
        assert_eq!(l, [(10, 20), (20, 30)]);
    }

    #[test]
    fn tree_centres_parents_over_children() {
        let procs = [
            proc(1, None),
            proc(2, Some(1)),
            proc(3, Some(1)),
            proc(4, Some(3)),
            proc(9, None),
        ];
        let (nodes, mut edges) = tree(1, &procs);
        let at = |pid| nodes.iter().find(|n| n.0 == pid).map(|n| (n.1, n.2));
        assert_eq!(at(2), Some((0.0, 1.0)));
        assert_eq!(at(4), Some((1.0, 2.0)));
        assert_eq!(at(3), Some((1.0, 1.0)));
        assert_eq!(at(1), Some((0.5, 0.0)));
        assert_eq!(at(9), None);
        edges.sort();
        assert_eq!(edges, [(1, 2), (1, 3), (3, 4)]);
    }

    #[test]
    fn tree_survives_ppid_cycle() {
        let procs = [proc(1, Some(2)), proc(2, Some(1))];
        let (nodes, edges) = tree(1, &procs);
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges, [(1, 2)]);
    }
}
