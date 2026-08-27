const WINDOWS_VERBATIM_PREFIX = '\\\\?\\';
const WINDOWS_VERBATIM_UNC_PREFIX = '\\\\?\\UNC\\';

/** Converts Windows verbatim paths into the familiar user-facing form. */
export function formatDisplayPath(path: string): string {
  const upperPath = path.toUpperCase();
  if (upperPath.startsWith(WINDOWS_VERBATIM_UNC_PREFIX)) {
    return `\\\\${path.slice(WINDOWS_VERBATIM_UNC_PREFIX.length)}`;
  }
  if (upperPath.startsWith(WINDOWS_VERBATIM_PREFIX)) {
    return path.slice(WINDOWS_VERBATIM_PREFIX.length);
  }
  return path;
}
