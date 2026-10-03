/**
 * DN-2A image-attachment vocabulary for the chat send path. Mirrors the
 * server-side limits in internal/attachment (VIVY): png/jpeg/gif/webp
 * only, content sniffed, ≤4 attachments, ≤5 MiB decoded per image, and
 * the complete serialized `vivy_call` request ≤4 MiB (ABI v1 c_input).
 */
import type { TurnAttachment } from '../api/vivy/contracts'

export const IMAGE_MIME_WHITELIST = new Set([
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
])
export const MAX_ATTACHMENTS = 4
export const MAX_IMAGE_BYTES = 5 << 20
/** vivy-bridge c_input bound on the serialized {method, params} JSON. */
export const MAX_FRAME_BYTES = 4 << 20

export type ChatAttachmentErrorCode =
  | 'unsupported_type'
  | 'empty_image'
  | 'image_too_large'
  | 'too_many'
  | 'sniff_mismatch'
  | 'frame_too_large'

export class ChatAttachmentError extends Error {
  constructor(
    readonly code: ChatAttachmentErrorCode,
    message: string,
  ) {
    super(message)
    this.name = 'ChatAttachmentError'
  }
}

/** Content-based image sniff — the file extension and the browser's MIME
 * claim are ignored, matching the backend's SniffMIME vocabulary. */
export function sniffImageMime(bytes: Uint8Array): string | null {
  if (bytes.length >= 4 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) {
    return 'image/png'
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return 'image/jpeg'
  }
  if (bytes.length >= 6) {
    const head = String.fromCharCode(...bytes.subarray(0, 6))
    if (head === 'GIF87a' || head === 'GIF89a') return 'image/gif'
  }
  if (bytes.length >= 12) {
    const head = String.fromCharCode(...bytes.subarray(0, 4))
    const tail = String.fromCharCode(...bytes.subarray(8, 12))
    if (head === 'RIFF' && tail === 'WEBP') return 'image/webp'
  }
  return null
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  const CHUNK = 0x8000
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK))
  }
  return btoa(binary)
}

export function base64ToBytes(data: string): Uint8Array {
  const binary = atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
  return bytes
}

/** Validate one already-decoded image against the shared server limits
 * and return its wire attachment. The claimed MIME (case/whitespace
 * tolerant) must equal the sniffed MIME. */
export function encodeImageBytes(name: string | undefined, claimedMime: string, bytes: Uint8Array): TurnAttachment {
  const mime = claimedMime.trim().toLowerCase()
  if (!IMAGE_MIME_WHITELIST.has(mime)) {
    throw new ChatAttachmentError('unsupported_type', `unsupported type "${claimedMime}" (png, jpeg, gif and webp images only)`)
  }
  if (bytes.length === 0) {
    throw new ChatAttachmentError('empty_image', 'data must not be empty')
  }
  if (bytes.length > MAX_IMAGE_BYTES) {
    throw new ChatAttachmentError('image_too_large', `image exceeds the ${MAX_IMAGE_BYTES >> 20} MiB limit`)
  }
  const detected = sniffImageMime(bytes)
  if (!detected) {
    throw new ChatAttachmentError('sniff_mismatch', 'file content is not a supported image')
  }
  if (detected !== mime) {
    throw new ChatAttachmentError('sniff_mismatch', 'MIME type does not match image content')
  }
  const attachment: TurnAttachment = { mime_type: detected, data: bytesToBase64(bytes) }
  if (name) attachment.name = name
  return attachment
}

/** Read a picked file into a validated wire attachment exactly once. The
 * sniffed content decides the MIME — the picker's accept list is UX, not
 * a trust boundary. */
export async function fileToTurnAttachment(file: File): Promise<TurnAttachment> {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const detected = sniffImageMime(bytes) ?? file.type
  return encodeImageBytes(file.name, detected, bytes)
}

/** Re-validate already-encoded attachments on the controller lane:
 * every entry is decoded, size/MIME/sniff checked, and the count bound
 * enforced — the controller never trusts an upstream encoder. */
export function validateTurnAttachments(items: TurnAttachment[]): void {
  if (items.length > MAX_ATTACHMENTS) {
    throw new ChatAttachmentError('too_many', `at most ${MAX_ATTACHMENTS} attachments are allowed per message`)
  }
  items.forEach((item, index) => {
    let bytes: Uint8Array
    try {
      bytes = base64ToBytes(item.data)
    } catch {
      throw new ChatAttachmentError('unsupported_type', `attachment ${index + 1}: data must be base64-encoded image bytes`)
    }
    try {
      encodeImageBytes(item.name, item.mime_type, bytes)
    } catch (e) {
      if (e instanceof ChatAttachmentError) {
        throw new ChatAttachmentError(e.code, `attachment ${index + 1}: ${e.message}`)
      }
      throw e
    }
  })
}

/** Byte length of the complete serialized `vivy_call` request — the
 * exact {method, params} JSON the ABI's c_input measures. */
export function framedRequestBytes(method: string, params: unknown): number {
  return new TextEncoder().encode(JSON.stringify({ method, params })).length
}

export function assertFramedRequest(method: string, params: unknown): void {
  const bytes = framedRequestBytes(method, params)
  if (bytes > MAX_FRAME_BYTES) {
    throw new ChatAttachmentError('frame_too_large', `request exceeds the 4 MiB frame bound (${bytes} bytes)`)
  }
}
