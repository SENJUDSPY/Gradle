/**
 * Fixture factory for generating synthetic ZIP archives, WASM binaries, and telemetry objects.
 */
import { Buffer } from 'node:buffer'

/** Table-based CRC32 implementation for pure JS ZIP creation */
const CRC_TABLE = new Uint32Array(256)
for (let i = 0; i < 256; i++) {
  let c = i
  for (let k = 0; k < 8; k++) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  }
  CRC_TABLE[i] = c
}

export function crc32(buffer) {
  let crc = 0xffffffff
  for (let i = 0; i < buffer.length; i++) {
    crc = CRC_TABLE[(crc ^ buffer[i]) & 0xff] ^ (crc >>> 8)
  }
  return (crc ^ 0xffffffff) >>> 0
}

/**
 * Creates a valid in-memory ZIP archive from a map of entryName -> Buffer/String.
 * Allows malicious paths like `../../escape.txt` without filesystem sanitization.
 */
export function createZipBuffer(entries) {
  const localHeaders = []
  const centralHeaders = []
  let offset = 0

  const entryList = Object.entries(entries).map(([name, content]) => {
    const data = Buffer.isBuffer(content) ? content : Buffer.from(content, 'utf8')
    return { name, data, crc: crc32(data) }
  })

  for (const entry of entryList) {
    const nameBuf = Buffer.from(entry.name, 'utf8')
    const localHeader = Buffer.alloc(30)
    localHeader.writeUInt32LE(0x04034b50, 0) // Local header signature
    localHeader.writeUInt16LE(20, 4) // Version needed
    localHeader.writeUInt16LE(0, 6) // Flags
    localHeader.writeUInt16LE(0, 8) // Stored (no compression)
    localHeader.writeUInt16LE(0, 10) // Mod time
    localHeader.writeUInt16LE(0, 12) // Mod date
    localHeader.writeUInt32LE(entry.crc, 14) // CRC32
    localHeader.writeUInt32LE(entry.data.length, 18) // Compressed size
    localHeader.writeUInt32LE(entry.data.length, 22) // Uncompressed size
    localHeader.writeUInt16LE(nameBuf.length, 26) // Name length
    localHeader.writeUInt16LE(0, 28) // Extra length

    const fullLocal = Buffer.concat([localHeader, nameBuf, entry.data])
    localHeaders.push(fullLocal)

    const centralHeader = Buffer.alloc(46)
    centralHeader.writeUInt32LE(0x02014b50, 0) // Central directory header
    centralHeader.writeUInt16LE(20, 4) // Version made by
    centralHeader.writeUInt16LE(20, 6) // Version needed
    centralHeader.writeUInt16LE(0, 8) // Flags
    centralHeader.writeUInt16LE(0, 10) // Stored
    centralHeader.writeUInt16LE(0, 12) // Mod time
    centralHeader.writeUInt16LE(0, 14) // Mod date
    centralHeader.writeUInt32LE(entry.crc, 16) // CRC32
    centralHeader.writeUInt32LE(entry.data.length, 20) // Compressed size
    centralHeader.writeUInt32LE(entry.data.length, 24) // Uncompressed size
    centralHeader.writeUInt16LE(nameBuf.length, 28) // Name length
    centralHeader.writeUInt16LE(0, 30) // Extra length
    centralHeader.writeUInt16LE(0, 32) // Comment length
    centralHeader.writeUInt16LE(0, 34) // Disk start
    centralHeader.writeUInt16LE(0, 36) // Internal attr
    centralHeader.writeUInt32LE(0, 38) // External attr
    centralHeader.writeUInt32LE(offset, 42) // Local header offset

    centralHeaders.push(Buffer.concat([centralHeader, nameBuf]))
    offset += fullLocal.length
  }

  const centralDirOffset = offset
  const centralDir = Buffer.concat(centralHeaders)
  const centralDirSize = centralDir.length

  const eocd = Buffer.alloc(22)
  eocd.writeUInt32LE(0x06054b50, 0) // EOCD signature
  eocd.writeUInt16LE(0, 4) // Disk number
  eocd.writeUInt16LE(0, 6) // CD start disk
  eocd.writeUInt16LE(entryList.length, 8) // Total entries on disk
  eocd.writeUInt16LE(entryList.length, 10) // Total entries
  eocd.writeUInt32LE(centralDirSize, 12) // CD size
  eocd.writeUInt32LE(centralDirOffset, 16) // CD offset
  eocd.writeUInt16LE(0, 20) // Comment length

  return Buffer.concat([...localHeaders, centralDir, eocd])
}

/**
 * Encodes a non-negative integer into LEB128 byte array.
 */
export function encodeLeb128(value) {
  const bytes = []
  let v = value >>> 0
  do {
    let byte = v & 0x7f
    v >>>= 7
    if (v !== 0) {
      byte |= 0x80
    }
    bytes.push(byte)
  } while (v !== 0)
  return bytes
}

/**
 * Creates a synthetic WASM binary with configured memory limits.
 */
export function createWasmBinary(options = {}) {
  const magic = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])

  // Optional memory section (section ID 5)
  let memorySection = Buffer.alloc(0)
  if (options.memory) {
    const { flags = 0, initial = 1, max = null } = options.memory
    const memBody = [0x01, flags, ...encodeLeb128(initial)]
    if (max !== null && max !== undefined) {
      memBody.push(...encodeLeb128(max))
    }
    memorySection = Buffer.from([0x05, memBody.length, ...memBody])
  }

  // Custom crafted section (e.g. For LEB128 overflow testing)
  let customSection = Buffer.alloc(0)
  if (options.customSection) {
    customSection = Buffer.from(options.customSection)
  }

  return Buffer.concat([magic, memorySection, customSection])
}

/**
 * Creates a WASM binary containing a malformed LEB128 varint with shift >= 32.
 */
export function createLeb128OverflowWasm() {
  return Buffer.from([
    0x00, 0x61, 0x73, 0x6d, // Magic "\0asm"
    0x01, 0x00, 0x00, 0x00, // Version 1
    0x05, 0x08,             // Memory section (id 5, length 8)
    0x01,                   // 1 memory descriptor
    0x00,                   // flags: 0 (no max)
    0x80, 0x80, 0x80, 0x80, 0x80, 0x01 // 6-byte LEB128 sequence causing shift >= 35
  ])
}

/**
 * Creates a synthetic ContentCache payload buffer:
 * MAGIC [8 bytes: "GRADLE_C"] + VERSION [4 bytes u32] + RAW_LEN [4 bytes u32] + COMPRESSED_BYTES.
 */
export function createContentCacheBuffer(magic = 'GRADLE_C', version = 1, rawLen = 0, payloadBytes = Buffer.alloc(0)) {
  const magicBuf = Buffer.alloc(8)
  magicBuf.write(magic, 'utf8')

  const header = Buffer.alloc(8)
  header.writeUInt32LE(version, 0)
  header.writeUInt32LE(rawLen, 4)

  return Buffer.concat([magicBuf, header, payloadBytes])
}

/**
 * Creates synthetic telemetry snapshot fixture.
 */
export function createTelemetrySnapshot(overrides = {}) {
  return {
    cpu_usage_percent: 15.2,
    total_memory_bytes: 17179869184,
    used_memory_bytes: 8589934592,
    available_memory_bytes: 8589934592,
    disk_total_bytes: 512110190592,
    disk_available_bytes: 274877906944,
    launcher_rss_bytes: 314572800,
    launcher_cpu_percent: 1.8,
    ...overrides
  }
}

/**
 * Creates synthetic process metrics snapshot fixture.
 */
export function createProcessMetricsSnapshot(overrides = {}) {
  return {
    pid: 51230,
    cpu_percent: 34.5,
    rss_bytes: 2147483648,
    jvm_max_memory_bytes: 4294967296,
    uptime_seconds: 120,
    ...overrides
  }
}

/**
 * Creates synthetic state inspector dump fixture.
 */
export function createStateInspectorDump(overrides = {}) {
  return {
    timestamp: new Date().toISOString(),
    os_info: 'Linux 6.8.0-x86_64',
    app_version: '0.22.01',
    active_processes: [],
    memory_summary: createTelemetrySnapshot(),
    store_usage: {
      cache_size_bytes: 4294967296,
      instance_size_bytes: 12884901888
    },
    ...overrides
  }
}
