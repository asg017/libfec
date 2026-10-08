//// Errors from opening and reading a filing.

pub type FecError {
  /// The file couldn't be opened or read.
  IoError(message: String)
  /// The header names a format version the parser doesn't support
  /// (`""` if the header has no version at all).
  UnsupportedVersion(version: String)
  /// A malformed header, cover or row. `line` is 0 when unknown (errors in
  /// the header or cover).
  ParseError(line: Int, message: String)
  /// A row whose type has no column mapping for the filing's version.
  MissingMapping(row_type: String, version: String, line: Int)
  /// The reader was closed.
  Closed
}
