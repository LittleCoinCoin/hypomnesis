// SPDX-License-Identifier: MIT OR Apache-2.0
//
// gpujob [SECONDS]
//
// A resident GPU job for `sandbox.sh PROFILE --job`: one 256 MiB shared Metal
// buffer, written in full so its pages are resident, held for SECONDS (default
// 20). Prints `gpujob pid=<pid>` once the buffer is resident.

import Foundation
import Metal

let length = 256 * 1024 * 1024

guard let device = MTLCreateSystemDefaultDevice() else {
    FileHandle.standardError.write(Data("gpujob: no Metal device\n".utf8))
    exit(1)
}
guard let buffer = device.makeBuffer(length: length, options: .storageModeShared) else {
    FileHandle.standardError.write(Data("gpujob: cannot allocate \(length) bytes\n".utf8))
    exit(1)
}
memset(buffer.contents(), 0xAB, length)

print("gpujob pid=\(getpid())")
fflush(stdout)

let seconds = CommandLine.arguments.count > 1 ? (Double(CommandLine.arguments[1]) ?? 20) : 20
withExtendedLifetime(buffer) {
    Thread.sleep(forTimeInterval: seconds)
}
