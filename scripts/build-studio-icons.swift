// Development packaging helper. Run from the repository root:
// swift scripts/build-studio-icons.swift <temporary output directory>
// Copies reviewed Moxcrew v1 PNG exports and assembles platform containers.
// SVG source and export provenance live in design/moxcrew; no runtime exporter.
import Foundation

let directory = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let source = URL(fileURLWithPath: "design/moxcrew/icons", isDirectory: true)
let files = FileManager.default
try files.createDirectory(at: directory, withIntermediateDirectories: true)
for size in [16, 32, 64, 128, 256, 512, 1024] {
    let data = try Data(contentsOf: source.appendingPathComponent("moxcrew-app-\(size).png"))
    try data.write(to: directory.appendingPathComponent("\(size).png"))
}
let iconset = directory.appendingPathComponent("Moxcrew.iconset", isDirectory: true)
try files.createDirectory(at: iconset, withIntermediateDirectories: true)
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let suffix = scale == 2 ? "@2x" : ""
        let data = try Data(contentsOf: directory.appendingPathComponent("\(size * scale).png"))
        try data.write(to: iconset.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
    }
}
// Modern ICNS chunks carry PNG payloads; retain explicit big-endian lengths.
var chunks = Data()
for (type, size) in [("icp4",16),("icp5",32),("icp6",64),("ic07",128),("ic08",256),("ic09",512),("ic10",1024)] {
    let data = try Data(contentsOf: directory.appendingPathComponent("\(size).png"))
    chunks.append(type.data(using: .ascii)!)
    var length = UInt32(data.count + 8).bigEndian
    chunks.append(Data(bytes: &length, count: 4))
    chunks.append(data)
}
var icns = "icns".data(using: .ascii)!
var icnsLength = UInt32(chunks.count + 8).bigEndian
icns.append(Data(bytes: &icnsLength, count: 4))
icns.append(chunks)
try icns.write(to: directory.appendingPathComponent("icon.icns"))
var ico = Data()
func u16(_ value: UInt16) { var n = value.littleEndian; ico.append(Data(bytes: &n, count: 2)) }
func u32(_ value: UInt32) { var n = value.littleEndian; ico.append(Data(bytes: &n, count: 4)) }
let sizes = [16, 32, 64, 128, 256]
let images = try sizes.map { try Data(contentsOf: directory.appendingPathComponent("\($0).png")) }
u16(0); u16(1); u16(UInt16(sizes.count))
var offset = UInt32(6 + 16 * sizes.count)
for (size, data) in zip(sizes, images) {
    ico.append(contentsOf: [UInt8(size % 256), UInt8(size % 256), 0, 0])
    u16(1); u16(32); u32(UInt32(data.count)); u32(offset)
    offset += UInt32(data.count)
}
for data in images { ico.append(data) }
try ico.write(to: directory.appendingPathComponent("icon.ico"))
