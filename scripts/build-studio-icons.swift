// Development-only macOS rasterizer for assets/icons/icon.svg's simple geometry.
// swift scripts/build-studio-icons.swift <temporary output directory>
import AppKit
import Foundation

let directory = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
for size in [16, 32, 64, 128, 256, 512, 1024] {
    let image = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    let context = NSGraphicsContext(bitmapImageRep: image)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.cgContext.scaleBy(x: CGFloat(size) / 1024, y: CGFloat(size) / 1024)
    NSColor(srgbRed: 25/255, green: 30/255, blue: 33/255, alpha: 1).setFill()
    let background = NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 206, yRadius: 206)
    background.fill()
    NSColor(srgbRed: 63/255, green: 85/255, blue: 73/255, alpha: 1).setStroke()
    background.lineWidth = 3
    background.stroke()
    NSColor(srgbRed: 182/255, green: 237/255, blue: 202/255, alpha: 1).setStroke()
    let mark = NSBezierPath()
    mark.lineWidth = 72
    mark.lineCapStyle = .round
    for x: CGFloat in [3, 11, 19] {
        mark.move(to: NSPoint(x: 242 + x * 18, y: 1024 - (260 + 23 * 18)))
        mark.line(to: NSPoint(x: 242 + (x + 8) * 18, y: 1024 - (260 + 5 * 18)))
    }
    mark.stroke()
    NSGraphicsContext.restoreGraphicsState()
    try image.representation(using: .png, properties: [:])!.write(to: directory.appendingPathComponent("\(size).png"))
}
