// Development export: swift export-wordmark.swift <Manrope-Semibold.ttf> <wordmark-path.json>
import Foundation
import CoreText
import CoreGraphics
let data = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
let provider = CGDataProvider(data: data as CFData)!
let font = CTFontCreateWithGraphicsFont(CGFont(provider)!, 116, nil, nil)
let line = CTLineCreateWithAttributedString(NSAttributedString(string: "Moxcrew", attributes: [NSAttributedString.Key(kCTFontAttributeName as String): font]))
var commands = ""
func number(_ v: CGFloat) -> String { String(format: "%.3f", Double(v)) }
for run in CTLineGetGlyphRuns(line) as! [CTRun] {
    let count = CTRunGetGlyphCount(run)
    var glyphs = [CGGlyph](repeating: 0, count: count)
    var positions = [CGPoint](repeating: .zero, count: count)
    CTRunGetGlyphs(run, CFRange(location: 0, length: 0), &glyphs)
    CTRunGetPositions(run, CFRange(location: 0, length: 0), &positions)
    for i in 0..<count {
        var transform = CGAffineTransform(translationX: positions[i].x, y: positions[i].y)
        guard let glyph = CTFontCreatePathForGlyph(font, glyphs[i], &transform) else { continue }
        glyph.applyWithBlock { ptr in
            let e = ptr.pointee
            func p(_ n: Int) -> String { "\(number(e.points[n].x)) \(number(e.points[n].y))" }
            switch e.type {
            case .moveToPoint: commands += "M\(p(0))"
            case .addLineToPoint: commands += "L\(p(0))"
            case .addQuadCurveToPoint: commands += "Q\(p(0)) \(p(1))"
            case .addCurveToPoint: commands += "C\(p(0)) \(p(1)) \(p(2))"
            case .closeSubpath: commands += "Z"
            @unknown default: break
            }
        }
    }
}
let object: [String: Any] = ["text": "Moxcrew", "font": "Manrope Semibold 650", "advance": CTLineGetTypographicBounds(line, nil, nil, nil), "path": commands]
try JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys]).write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
