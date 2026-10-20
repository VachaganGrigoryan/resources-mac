// App icons for Resources for Mac and Disk Sweep, drawn with CoreGraphics so they are
// reproducible and need no design tools. Original designs: Resources shows the app's own
// sidebar mini graphs, Disk Sweep its stacked "space by kind" bars.
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let srgb = CGColorSpace(name: CGColorSpace.sRGB)!

func rgb(_ hex: UInt32, _ a: CGFloat = 1) -> CGColor {
    CGColor(colorSpace: srgb, components: [
        CGFloat((hex >> 16) & 0xFF) / 255, CGFloat((hex >> 8) & 0xFF) / 255,
        CGFloat(hex & 0xFF) / 255, a,
    ])!
}

// Upstream GNOME Resources graph colours, as in src/theme.rs.
let CPU: UInt32 = 0x3584E4, MEM: UInt32 = 0xC52F90, DRIVE: UInt32 = 0xFF7800
// Disk Sweep kind colours, as in crates/disk-sweep/src/widgets.rs (Other ignored, Git history).
let GREY: UInt32 = 0x8E8E93, GIT: UInt32 = 0x26A269

func gradient(_ colors: [CGColor], _ locs: [CGFloat]? = nil) -> CGGradient {
    CGGradient(colorsSpace: srgb, colors: colors as CFArray, locations: locs)!
}

/// Deterministic "live usage" trace in 0...1.
func trace(_ n: Int, base: Double, amp: Double, freq: Double, phase: Double, spike: Int? = nil) -> [Double] {
    (0..<n).map { i in
        let t = Double(i)
        var v = base + amp * (0.6 * sin(t * freq + phase) + 0.4 * sin(t * freq * 2.3 + phase * 1.7))
        if let s = spike, i == s { v += amp * 1.6 }
        return min(max(v, 0.02), 0.98)
    }
}

/// Smooth area under `values` inside `r` (CG coordinates, origin bottom-left):
/// Catmull-Rom through the samples, as cubic Béziers.
func areaPath(_ r: CGRect, _ values: [Double]) -> (area: CGPath, line: CGPath) {
    let step = r.width / CGFloat(values.count - 1)
    let pts = values.enumerated().map { i, v in
        CGPoint(x: r.minX + CGFloat(i) * step, y: r.minY + CGFloat(v) * r.height)
    }
    let line = CGMutablePath()
    line.move(to: pts[0])
    for i in 0..<pts.count - 1 {
        let p0 = pts[max(i - 1, 0)], p1 = pts[i], p2 = pts[i + 1], p3 = pts[min(i + 2, pts.count - 1)]
        line.addCurve(to: p2,
                      control1: CGPoint(x: p1.x + (p2.x - p0.x) / 6, y: p1.y + (p2.y - p0.y) / 6),
                      control2: CGPoint(x: p2.x - (p3.x - p1.x) / 6, y: p2.y - (p3.y - p1.y) / 6))
    }
    let area = line.mutableCopy()!
    area.addLine(to: CGPoint(x: r.maxX, y: r.minY))
    area.addLine(to: CGPoint(x: r.minX, y: r.minY))
    area.closeSubpath()
    return (area, line)
}

/// macOS icon body: 824 pt rounded square centred in the 1024 canvas, with drop shadow,
/// background gradient and a soft top sheen. `content` draws clipped to the body.
func body(_ c: CGContext, _ k: CGFloat, top: UInt32, bottom: UInt32, content: (CGRect) -> Void) {
    let r = CGRect(x: 100 * k, y: 100 * k, width: 824 * k, height: 824 * k)
    let shape = CGPath(roundedRect: r, cornerWidth: 185 * k, cornerHeight: 185 * k, transform: nil)
    c.saveGState()
    c.setShadow(offset: CGSize(width: 0, height: -10 * k), blur: 24 * k, color: rgb(0x000000, 0.35))
    c.addPath(shape); c.setFillColor(rgb(bottom)); c.fillPath()
    c.restoreGState()
    c.saveGState()
    c.addPath(shape); c.clip()
    c.drawLinearGradient(gradient([rgb(top), rgb(bottom)]), start: CGPoint(x: 0, y: r.maxY),
                         end: CGPoint(x: 0, y: r.minY), options: [])
    content(r)
    c.drawLinearGradient(gradient([rgb(0xFFFFFF, 0.16), rgb(0xFFFFFF, 0)]),
                         start: CGPoint(x: 0, y: r.maxY), end: CGPoint(x: 0, y: r.midY), options: [])
    c.restoreGState()
    c.addPath(shape); c.setStrokeColor(rgb(0xFFFFFF, 0.10)); c.setLineWidth(2 * k); c.strokePath()
}

/// The icon: three of the app's sidebar graph strips (CPU, Memory, Drive) on a graphite body.
func icon(_ c: CGContext, _ k: CGFloat) {
    body(c, k, top: 0x3B404D, bottom: 0x1B1E26) { r in
        let strips: [(UInt32, [Double])] = [
            (CPU, trace(13, base: 0.36, amp: 0.22, freq: 0.9, phase: 0.3, spike: 8)),
            (MEM, trace(13, base: 0.56, amp: 0.07, freq: 0.6, phase: 1.1)),
            (DRIVE, trace(13, base: 0.20, amp: 0.24, freq: 1.1, phase: 2.0, spike: 4)),
        ]
        let h = 150 * k, gap = 46 * k, x = 190 * k, w = 644 * k
        var y = r.midY + (3 * h + 2 * gap) / 2 - h
        for (color, values) in strips {
            let strip = CGRect(x: x, y: y, width: w, height: h)
            let rounded = CGPath(roundedRect: strip, cornerWidth: 30 * k, cornerHeight: 30 * k, transform: nil)
            c.saveGState()
            c.addPath(rounded); c.clip()
            c.setFillColor(rgb(color, 0.20)); c.fill(strip)
            let (area, line) = areaPath(strip.insetBy(dx: 0, dy: 8 * k), values)
            c.addPath(area); c.setFillColor(rgb(color, 0.55)); c.fillPath()
            c.addPath(line); c.setStrokeColor(rgb(color)); c.setLineWidth(max(9 * k, 1.2))
            c.setLineJoin(.round); c.strokePath()
            c.restoreGState()
            y -= h + gap
        }
    }
}

/// Disk Sweep: three of the app's stacked kind bars (project files, caches & builds, editor
/// metadata, other ignored, git history), each shorter than the last, as space is freed.
func sweep(_ c: CGContext, _ k: CGFloat) {
    body(c, k, top: 0x3B404D, bottom: 0x1B1E26) { r in
        let bars: [[(UInt32, CGFloat)]] = [
            [(CPU, 0.34), (DRIVE, 0.40), (MEM, 0.10), (GREY, 0.08), (GIT, 0.08)],
            [(CPU, 0.34), (DRIVE, 0.18), (MEM, 0.06), (GIT, 0.08)],
            [(CPU, 0.34), (GIT, 0.08)],
        ]
        let h = 150 * k, gap = 46 * k, x = 190 * k, w = 644 * k
        var y = r.midY + (3 * h + 2 * gap) / 2 - h
        for parts in bars {
            let track = CGRect(x: x, y: y, width: w, height: h)
            let rounded = CGPath(roundedRect: track, cornerWidth: 30 * k, cornerHeight: 30 * k, transform: nil)
            c.saveGState()
            c.addPath(rounded); c.clip()
            c.setFillColor(rgb(0xFFFFFF, 0.08)); c.fill(track)
            var left = x
            for (color, frac) in parts {
                let seg = CGRect(x: left, y: y, width: w * frac, height: h)
                c.setFillColor(rgb(color, 0.85)); c.fill(seg)
                left += w * frac
                // Hairline between parts, like the app's bar.
                c.setFillColor(rgb(0x1B1E26, 0.9)); c.fill(CGRect(x: left - 3 * k, y: y, width: 6 * k, height: h))
            }
            c.restoreGState()
            y -= h + gap
        }
    }
}

func render(_ px: Int, _ draw: (CGContext, CGFloat) -> Void, to path: String) {
    let c = CGContext(data: nil, width: px, height: px, bitsPerComponent: 8, bytesPerRow: 0,
                      space: srgb, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    c.interpolationQuality = .high
    draw(c, CGFloat(px) / 1024)
    let url = URL(fileURLWithPath: path) as CFURL
    let dest = CGImageDestinationCreateWithURL(url, UTType.png.identifier as CFString, 1, nil)!
    CGImageDestinationAddImage(dest, c.makeImage()!, nil)
    CGImageDestinationFinalize(dest)
}


/// Writes `<dir>/macos/AppIcon.icns`, `<dir>/assets/icon-256.png` and the 512 px `docs` PNG.
func write(_ draw: (CGContext, CGFloat) -> Void, crate dir: String, docs: String) throws {
    let iconset = "\(dir)/macos/AppIcon.iconset"
    try FileManager.default.createDirectory(atPath: iconset, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(atPath: "\(dir)/assets", withIntermediateDirectories: true)
    for pt in [16, 32, 128, 256, 512] {
        render(pt, draw, to: "\(iconset)/icon_\(pt)x\(pt).png")
        render(pt * 2, draw, to: "\(iconset)/icon_\(pt)x\(pt)@2x.png")
    }
    render(256, draw, to: "\(dir)/assets/icon-256.png")
    render(512, draw, to: docs)
    let iconutil = Process()
    iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
    iconutil.arguments = ["-c", "icns", iconset, "-o", "\(dir)/macos/AppIcon.icns"]
    try iconutil.run()
    iconutil.waitUntilExit()
    guard iconutil.terminationStatus == 0 else { fatalError("iconutil failed") }
    try FileManager.default.removeItem(atPath: iconset)
}

// Usage: swift scripts/icon.swift   (from the repo root; redraws both icons)
try write(icon, crate: "crates/resources", docs: "docs/icon.png")
try write(sweep, crate: "crates/disk-sweep", docs: "docs/disk-sweep-icon.png")
