//! Overview: every app as a bubble around the Mac with its processes packed inside, and
//! lines for who launched whom. Double-click a bubble to see that app's process tree.

use std::collections::HashMap;

use egui::epaint::{CubicBezierShape, QuadraticBezierShape};
use egui::{
    Align, Align2, Color32, FontId, Layout, Painter, Pos2, Rect, Response, RichText, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use crate::bubbles::{self, Sim};
use crate::icons;
use crate::pages::apps::AppActions;
use crate::sample::{AppUsage, ProcInfo};
use crate::sampler::State;
use crate::table::{IconCache, Search};
use crate::theme::{ACCENT, color, palette};
use crate::widgets::{fmt_bytes, icon_button};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeBy {
    #[default]
    Memory,
    Cpu,
}

/// Radius of the "This Mac" hub in the middle, in world units.
const HUB: f32 = 64.0;
/// Process box in the tree view, and the distance between neighbouring boxes.
const NODE: Vec2 = vec2(150.0, 46.0);
const TREE_SPACING: Vec2 = vec2(170.0, 96.0);
/// Relaxation steps per frame; enough to settle in about a second.
const STEPS: usize = 4;

pub struct OverviewView {
    pub search: Search,
    size_by: SizeBy,
    sim: Sim,
    /// Screen points per world unit, and the world origin's offset from the canvas centre.
    zoom: f32,
    pan: Vec2,
    /// Keep everything on screen until the user pans or zooms.
    auto_fit: bool,
    selected: Option<u32>,
    /// Bubble under the pointer last frame, to highlight its links.
    hovered: Option<u32>,
    /// App whose process tree is shown instead of the bubbles.
    focus: Option<u32>,
    actions: AppActions,
    icons: IconCache,
}

impl Default for OverviewView {
    fn default() -> Self {
        Self {
            search: Search::default(),
            size_by: SizeBy::default(),
            sim: Sim::default(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            auto_fit: true,
            selected: None,
            hovered: None,
            focus: None,
            actions: AppActions::default(),
            icons: IconCache::default(),
        }
    }
}

/// Load colour: teal when idle, orange at 50 %, red at a full core and beyond.
pub fn heat(cpu: f32) -> Color32 {
    let t = (cpu / 100.0).clamp(0.0, 1.0);
    if t < 0.5 {
        color::NETWORK.lerp_to_gamma(color::DRIVE, t * 2.0)
    } else {
        color::DRIVE.lerp_to_gamma(color::GPU, t * 2.0 - 1.0)
    }
}

/// Arc from 12 o'clock, clockwise, covering `frac` of the circle.
fn arc(painter: &Painter, c: Pos2, r: f32, frac: f32, stroke: Stroke) {
    let n = (64.0 * frac).ceil().max(2.0) as usize;
    let pts = (0..=n)
        .map(|i| {
            let a =
                -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac * i as f32 / n as f32;
            c + Vec2::angled(a) * r
        })
        .collect();
    painter.add(Shape::line(pts, stroke));
}

fn pct(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        used as f32 / total as f32 * 100.0
    }
}

fn truncate(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        name.to_owned()
    } else {
        format!("{}…", name.chars().take(max - 1).collect::<String>())
    }
}

fn proc_details(p: &ProcInfo) -> String {
    format!("{} · {:.1} % · {}", p.pid, p.cpu, fmt_bytes(p.mem))
}

/// One app bubble placed in world space.
struct Bubble<'a> {
    app: &'a AppUsage,
    pos: Vec2,
    r: f32,
}

impl OverviewView {
    fn to_screen(&self, rect: Rect, w: Vec2) -> Pos2 {
        rect.center() + self.pan + w * self.zoom
    }

    fn metric(&self, cpu: f32, mem: u64) -> f64 {
        match self.size_by {
            SizeBy::Memory => mem as f64,
            SizeBy::Cpu => f64::from(cpu),
        }
    }

    fn matches(&self, name: &str) -> bool {
        let q = self.search.query.trim().to_lowercase();
        q.is_empty() || name.to_lowercase().contains(&q)
    }

    pub fn show(&mut self, ui: &mut Ui, s: &State) {
        self.search.show(ui, "Search apps");
        self.icons.retain(&s.apps);
        if self
            .focus
            .is_some_and(|f| !s.apps.iter().any(|a| a.app.pid == f))
        {
            self.focus = None; // the app quit
        }
        if self.focus.is_some()
            && !self.search.open
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.focus = None;
            self.auto_fit = true;
        }
        let selected = self
            .selected
            .and_then(|pid| s.apps.iter().find(|a| a.app.pid == pid));
        self.actions.show(ui, s, selected);
        self.toolbar(ui, s);

        let (rect, canvas) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        canvas.widget_info(|| WidgetInfo::labeled(egui::Role::Canvas, true, "Overview"));
        self.navigate(ui, rect, &canvas);
        if canvas.clicked() {
            self.selected = None;
        }
        let painter = ui.painter_at(rect);
        match self.focus {
            Some(pid) => self.tree(ui, &painter, rect, s, pid),
            None => self.bubbles(ui, &painter, rect, s),
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, s: &State) {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            if let Some(pid) = self.focus {
                if icon_button(ui, icons::ARROW_LEFT, "Back to Overview").clicked() {
                    self.focus = None;
                    self.auto_fit = true;
                }
                let name = s
                    .apps
                    .iter()
                    .find(|a| a.app.pid == pid)
                    .map_or("", |a| a.app.name.as_str());
                ui.label(RichText::new(format!("{name} · Process Tree")).strong());
            } else {
                ui.label(RichText::new("Size by").small().weak());
                ui.selectable_value(&mut self.size_by, SizeBy::Memory, "Memory");
                ui.selectable_value(&mut self.size_by, SizeBy::Cpu, "Processor");
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Ring: processor load · Lines: launched by")
                        .small()
                        .weak(),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                if icon_button(ui, icons::CORNERS_OUT, "Fit to Window").clicked() {
                    self.auto_fit = true;
                }
            });
        });
    }

    /// Drag or scroll to pan, pinch (or ⌘-scroll) to zoom around the pointer.
    fn navigate(&mut self, ui: &Ui, rect: Rect, canvas: &Response) {
        if canvas.dragged() {
            self.pan += canvas.drag_delta();
            self.auto_fit = false;
        }
        let Some(ptr) = ui.ctx().pointer_hover_pos().filter(|p| rect.contains(*p)) else {
            return;
        };
        let (zoom, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
        if scroll != Vec2::ZERO {
            self.pan += scroll;
            self.auto_fit = false;
        }
        if zoom != 1.0 {
            let world = (ptr - rect.center() - self.pan) / self.zoom;
            self.zoom = (self.zoom * zoom).clamp(0.15, 4.0);
            self.pan = ptr - rect.center() - world * self.zoom;
            self.auto_fit = false;
        }
    }

    /// Zooms and centres so the world-space `bounds` fill the canvas.
    fn fit(&mut self, rect: Rect, bounds: Rect) {
        let room = rect.size() - Vec2::splat(48.0);
        self.zoom = (room.x / bounds.width())
            .min(room.y / bounds.height())
            .clamp(0.15, 1.5);
        self.pan = -bounds.center().to_vec2() * self.zoom;
    }

    fn bubbles(&mut self, ui: &mut Ui, painter: &Painter, rect: Rect, s: &State) {
        let max = s
            .apps
            .iter()
            .map(|a| self.metric(a.cpu, a.mem))
            .fold(0.0, f64::max);
        let mut nodes: Vec<(u32, f32)> = s
            .apps
            .iter()
            .map(|a| (a.app.pid, bubbles::radius(self.metric(a.cpu, a.mem), max)))
            .collect();
        // Biggest first: they settle closest to the hub and are drawn underneath.
        nodes.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let links = bubbles::links(&s.apps, &s.procs);
        let mut moved = 0.0;
        for _ in 0..STEPS {
            moved = self.sim.step(&nodes, &links, HUB);
        }
        if moved > 0.1 {
            ui.ctx().request_repaint();
        }
        let by_pid: HashMap<u32, &AppUsage> = s.apps.iter().map(|a| (a.app.pid, a)).collect();
        let placed: Vec<Bubble> = nodes
            .iter()
            .filter_map(|&(pid, r)| {
                Some(Bubble {
                    app: by_pid.get(&pid)?,
                    pos: self.sim.pos(pid)?,
                    r,
                })
            })
            .collect();
        if self.auto_fit {
            let bounds = placed.iter().fold(
                Rect::from_center_size(Pos2::ZERO, Vec2::splat(HUB * 2.0)),
                |b, x| {
                    b.union(Rect::from_center_size(
                        x.pos.to_pos2(),
                        Vec2::splat(x.r * 2.0 + 24.0),
                    ))
                },
            );
            self.fit(rect, bounds);
        }

        let p = palette(ui.visuals());
        let text = ui.visuals().text_color();
        let z = self.zoom;
        let pos: HashMap<u32, Pos2> = placed
            .iter()
            .map(|b| (b.app.app.pid, self.to_screen(rect, b.pos)))
            .collect();
        let hub = self.to_screen(rect, Vec2::ZERO);

        // Who launched whom; everything else hangs off the hub.
        let parent: HashMap<u32, u32> = links
            .into_iter()
            .map(|(parent, child)| (child, parent))
            .collect();
        for b in &placed {
            let pid = b.app.app.pid;
            let from = parent
                .get(&pid)
                .and_then(|pp| pos.get(pp).copied())
                .unwrap_or(hub);
            let lit = [Some(pid), parent.get(&pid).copied()]
                .iter()
                .any(|id| id.is_some() && (*id == self.selected || *id == self.hovered));
            let (stroke, launched) = if lit {
                (Stroke::new(2.0, ACCENT), 2.0)
            } else {
                (Stroke::new(1.0, text.gamma_multiply(0.18)), 1.5)
            };
            let to = pos[&pid];
            if from == hub {
                painter.line_segment([from, to], stroke);
            } else {
                // App-to-app links bow away from the hub so they don't vanish behind it.
                let mid = from + (to - from) / 2.0;
                let out = (mid - hub).normalized() * from.distance(to) * 0.35;
                painter.add(QuadraticBezierShape::from_points_stroke(
                    [from, mid + out, to],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(stroke.width * launched, stroke.color),
                ));
            }
        }

        // Hub: processor (outer) and memory (inner) rings.
        let hr = HUB * z;
        painter.circle(hub, hr, p.card, Stroke::new(1.0, p.border));
        let cpu = s.cpu_total.last().unwrap_or(0.0) as f32;
        let mem = pct(s.mem_used, s.mem_total);
        for (ring, frac, col) in [
            (hr - 5.0 * z, cpu, color::CPU),
            (hr - 12.0 * z, mem, color::MEMORY),
        ] {
            painter.circle_stroke(hub, ring, Stroke::new(4.0 * z, col.gamma_multiply(0.15)));
            arc(painter, hub, ring, frac / 100.0, Stroke::new(4.0 * z, col));
        }
        if hr > 34.0 {
            let small = FontId::proportional(11.0);
            painter.text(
                hub - vec2(0.0, 14.0),
                Align2::CENTER_CENTER,
                "This Mac",
                small.clone(),
                text.gamma_multiply(0.6),
            );
            painter.text(
                hub,
                Align2::CENTER_CENTER,
                format!("CPU {cpu:.0} %"),
                small.clone(),
                color::CPU,
            );
            painter.text(
                hub + vec2(0.0, 14.0),
                Align2::CENTER_CENTER,
                format!("Mem {mem:.0} %"),
                small,
                color::MEMORY,
            );
        }

        let procs: HashMap<u32, &ProcInfo> = s.procs.iter().map(|p| (p.pid, p)).collect();
        let fill = match self.size_by {
            SizeBy::Memory => color::MEMORY,
            SizeBy::Cpu => color::CPU,
        };
        let ptr = ui.ctx().pointer_hover_pos();
        let mut hovered = None;
        for b in &placed {
            let a = b.app;
            let c = pos[&a.app.pid];
            let r = b.r * z;
            let dim = if self.matches(&a.app.name) { 1.0 } else { 0.25 };
            painter.circle(
                c,
                r,
                p.card.gamma_multiply(dim),
                Stroke::new(2.0, heat(a.cpu).gamma_multiply(dim)),
            );

            // Subprocesses packed inside.
            let kids: Vec<&ProcInfo> = a
                .pids
                .iter()
                .filter_map(|pid| procs.get(pid).copied())
                .collect();
            let mut under_ptr = None;
            if r > 14.0 {
                let weights: Vec<f64> = kids.iter().map(|k| self.metric(k.cpu, k.mem)).collect();
                for (k, (off, kr)) in kids.iter().zip(bubbles::pack(&weights, r - 4.0)) {
                    let kc = c + off;
                    let t = (k.cpu / 100.0).clamp(0.0, 1.0);
                    painter.circle_filled(kc, kr, fill.gamma_multiply((0.15 + 0.55 * t) * dim));
                    if ptr.is_some_and(|q| q.distance(kc) <= kr) {
                        under_ptr = Some(*k);
                    }
                }
            }

            let side = (r * 0.9).min(56.0);
            if side > 12.0 {
                painter.circle_filled(c, side * 0.55, p.card.gamma_multiply(0.8 * dim));
                let at = Rect::from_center_size(c, Vec2::splat(side));
                if let Some(tex) = self.icons.texture(ui.ctx(), a) {
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    painter.image(tex.id(), at, uv, Color32::WHITE.gamma_multiply(dim));
                } else {
                    let glyph = if a.app.pid == 0 {
                        icons::PUZZLE_PIECE
                    } else {
                        icons::APP_WINDOW
                    };
                    painter.text(
                        c,
                        Align2::CENTER_CENTER,
                        glyph,
                        FontId::proportional(side * 0.6),
                        text.gamma_multiply(dim),
                    );
                }
            }
            // Inside big bubbles, under the icon; under small ones, where GAP leaves room.
            if r > 10.0 {
                let at = if r >= 48.0 {
                    c + vec2(0.0, side / 2.0 + 4.0)
                } else {
                    c + vec2(0.0, r + 2.0)
                };
                painter.text(
                    at,
                    Align2::CENTER_TOP,
                    truncate(&a.app.name, 22),
                    FontId::proportional(11.0),
                    text.gamma_multiply(dim),
                );
            }
            if self.selected == Some(a.app.pid) {
                painter.circle_stroke(c, r + 4.0, Stroke::new(2.0, ACCENT));
            }

            let hit = Rect::from_center_size(c, Vec2::splat(r * 2.0)).intersect(rect);
            if !hit.is_positive() {
                continue;
            }
            let resp = ui.interact(
                hit,
                ui.scope_id().with(("bubble", a.app.pid)),
                Sense::click(),
            );
            resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &a.app.name));
            if resp.hovered() {
                hovered = Some(a.app.pid);
            }
            if resp.double_clicked() && a.app.pid != 0 {
                self.focus = Some(a.app.pid);
                self.auto_fit = true;
            }
            if resp.clicked() {
                self.selected = Some(a.app.pid);
            }
            resp.on_hover_ui(|ui| {
                ui.label(RichText::new(&a.app.name).strong());
                ui.label(format!(
                    "Processor {:.1} % · Memory {} · {} processes",
                    a.cpu,
                    fmt_bytes(a.mem),
                    a.pids.len()
                ));
                if let Some(k) = under_ptr {
                    ui.separator();
                    ui.label(&k.name);
                    ui.label(RichText::new(proc_details(k)).small().weak());
                }
                if a.app.pid != 0 {
                    ui.label(
                        RichText::new("Double-click for the process tree")
                            .small()
                            .weak(),
                    );
                }
            });
        }
        self.hovered = hovered;
    }

    fn tree(&mut self, ui: &mut Ui, painter: &Painter, rect: Rect, s: &State, root: u32) {
        let (nodes, edges) = bubbles::tree(root, &s.procs);
        let world = bubbles::tree_positions(&nodes, TREE_SPACING);
        if self.auto_fit {
            let bounds = world.values().fold(Rect::NOTHING, |b, p| {
                b.union(Rect::from_center_size(
                    p.to_pos2(),
                    NODE + Vec2::splat(24.0),
                ))
            });
            if bounds.is_positive() {
                self.fit(rect, bounds);
            }
        }
        let p = palette(ui.visuals());
        let text = ui.visuals().text_color();
        let z = self.zoom;
        let procs: HashMap<u32, &ProcInfo> = s.procs.iter().map(|p| (p.pid, p)).collect();
        let at = |pid: &u32| self.to_screen(rect, world[pid]);

        for (from, to) in &edges {
            let a = at(from) + vec2(0.0, NODE.y / 2.0 * z);
            let b = at(to) - vec2(0.0, NODE.y / 2.0 * z);
            let bend = vec2(0.0, (b.y - a.y) / 2.0);
            painter.add(CubicBezierShape::from_points_stroke(
                [a, a + bend, b - bend, b],
                false,
                Color32::TRANSPARENT,
                Stroke::new(1.0, text.gamma_multiply(0.3)),
            ));
        }
        for (pid, ..) in &nodes {
            let Some(proc) = procs.get(pid) else { continue };
            let box_ = Rect::from_center_size(at(pid), NODE * z);
            let dim = if self.matches(&proc.name) { 1.0 } else { 0.3 };
            painter.rect(
                box_,
                10.0 * z,
                p.card.gamma_multiply(dim),
                Stroke::new(1.5, heat(proc.cpu).gamma_multiply(dim)),
                StrokeKind::Inside,
            );
            if z > 0.45 {
                let left = box_.left_center() + vec2(10.0 * z, 0.0);
                painter.text(
                    left - vec2(0.0, 8.0 * z),
                    Align2::LEFT_CENTER,
                    truncate(&proc.name, 20),
                    FontId::proportional(12.0 * z),
                    text.gamma_multiply(dim),
                );
                painter.text(
                    left + vec2(0.0, 9.0 * z),
                    Align2::LEFT_CENTER,
                    proc_details(proc),
                    FontId::proportional(10.0 * z),
                    text.gamma_multiply(0.55 * dim),
                );
            }
            let hit = box_.intersect(rect);
            if hit.is_positive() {
                let resp = ui.interact(hit, ui.scope_id().with(("proc", *pid)), Sense::hover());
                resp.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, &proc.name));
                resp.on_hover_ui(|ui| {
                    ui.label(RichText::new(&proc.name).strong());
                    ui.label(format!(
                        "PID {} · {} · Processor {:.1} % · Memory {}",
                        proc.pid,
                        proc.user,
                        proc.cpu,
                        fmt_bytes(proc.mem)
                    ));
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_runs_teal_orange_red() {
        assert_eq!(heat(0.0), color::NETWORK);
        assert_eq!(heat(50.0), color::DRIVE);
        assert_eq!(heat(250.0), color::GPU);
    }

    #[test]
    fn truncate_keeps_short_names() {
        assert_eq!(truncate("Safari", 22), "Safari");
        assert_eq!(truncate("abcdef", 4), "abc…");
    }
}
