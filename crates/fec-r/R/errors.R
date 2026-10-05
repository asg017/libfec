# Turning Rust `Err` strings into classed R conditions.
#
# `fec_read_impl()` fails with an R error whose message starts with a kind prefix (see the
# README, "Dev notes: the Rust reader"): `io: `, `header: ` or `parse: `. Each becomes a
# `libfec_error_<kind>` condition that also inherits from `libfec_error`; a message without a
# known prefix (a panic, an extendr conversion error) becomes a plain `libfec_error`.

# Call `fec_read_impl()` on an already validated path, mapping its errors.
#
# `path` is what the user passed (shown in messages); `call` is the frame errors blame.
read_impl <- function(path, raw, n_max, call = rlang::caller_env()) {
  check_file(path, call = call)
  full <- normalizePath(path, mustWork = TRUE)
  tryCatch(
    fec_read_impl(full, raw, n_max),
    error = function(e) abort_rust(conditionMessage(e), path, full, call = call)
  )
}

# A missing file is a `libfec_error_io`, like every other read failure, rather than
# `normalizePath()`'s plain error.
check_file <- function(path, call = rlang::caller_env()) {
  if (!file.exists(path)) {
    cli::cli_abort(
      c("Can't read {.file {path}}.", x = "The file doesn't exist."),
      class = c("libfec_error_io", "libfec_error"),
      path = path,
      call = call
    )
  }
  invisible(path)
}

abort_rust <- function(msg, path, full, call = rlang::caller_env()) {
  kinds <- c(io = "io", header = "header", parse = "parse")
  kind <- NA_character_
  for (k in kinds) {
    prefix <- paste0(k, ": ")
    if (startsWith(msg, prefix)) {
      kind <- k
      msg <- substring(msg, nchar(prefix) + 1L)
      break
    }
  }
  # io messages repeat the path (`/x/y.fec: No such file ...`); the headline already has it.
  for (p in c(full, path)) {
    if (startsWith(msg, paste0(p, ": "))) {
      msg <- substring(msg, nchar(p) + 3L)
      break
    }
  }
  if (nzchar(msg)) {
    msg <- paste0(toupper(substr(msg, 1L, 1L)), substring(msg, 2L))
  }
  headline <- switch(
    if (is.na(kind)) "other" else kind,
    io = "Can't read {.file {path}}.",
    header = "{.file {path}} isn't a readable FEC filing.",
    parse = "Can't parse {.file {path}}.",
    other = "Failed to read {.file {path}}."
  )
  class <- if (is.na(kind)) "libfec_error" else c(paste0("libfec_error_", kind), "libfec_error")
  cli::cli_abort(
    c(headline, x = "{msg}"),
    class = class,
    path = path,
    call = call
  )
}
